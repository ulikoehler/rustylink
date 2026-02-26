use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use rustylink::model::System;
use rustylink::parser::{
    is_virtual_library, FsSource, GraphicalInterface, LibraryResolver, SimulinkParser, ZipSource,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::BufReader;

#[derive(Parser, Debug)]
#[command(author, version, about = "Check for missing Simulink libraries and list unimplemented virtual library blocks")]
struct Cli {
    /// Path to .slx (zip), an extracted simulink root directory, or a system XML file
    #[arg(value_name = "SIMULINK_FILE_OR_DIR")]
    simulink_file: String,

    /// Optional library search paths (order matters). Used to locate LIBNAME.slx files.
    #[arg(short = 'L', long = "lib-path")]
    lib_paths: Vec<String>,

    /// Do not print block usage instances (host paths).
    #[arg(long = "no-usage")]
    no_usage: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let input_path = Utf8PathBuf::from(&cli.simulink_file);

    let (system, gi_opt) = load_system_and_gi(&input_path)?;

    let mut library_uses: BTreeMap<String, BTreeMap<String, BTreeSet<String>>> = BTreeMap::new();
    collect_library_uses(&system, &mut library_uses);

    // Determine the full set of libraries (system uses + GI references).
    let mut libraries: BTreeSet<String> = library_uses.keys().cloned().collect();
    if let Some(gi) = &gi_opt {
        for lib in gi.library_names() {
            libraries.insert(lib);
        }
    }

    // Build resolver search paths: default to model directory/root, then append user-provided ones.
    let resolver_paths = default_library_search_paths(&input_path)
        .into_iter()
        .chain(cli.lib_paths.iter().map(Utf8PathBuf::from))
        .collect::<Vec<_>>();
    let resolver = LibraryResolver::new(resolver_paths.iter());

    let show_usage = !cli.no_usage;

    // Determine missing libraries (excluding virtual ones)
    let mut missing: BTreeSet<String> = BTreeSet::new();
    for lib in &libraries {
        if is_virtual_library(lib) {
            continue;
        }
        let lookup_name = normalize_lib_name_for_lookup(lib);
        let res = resolver.locate(std::iter::once(lookup_name.as_str()));
        if res.found.is_empty() {
            missing.insert(lib.clone());
        }
    }

    // Print missing libraries overview (tree with ASCII pipes)
    if missing.is_empty() {
        println!("No missing libraries detected");
    } else {
        for lib in &missing {
            print_library_tree(lib, library_uses.get(lib), show_usage);
        }
    }

    // Print all unimplemented virtual library blocks
    let virtual_unimplemented = collect_virtual_unimplemented_blocks(&library_uses);
    if !virtual_unimplemented.is_empty() {
        println!();
        println!("UNIMPLEMENTED VIRTUAL LIBRARY BLOCKS");
        for (lib, blocks) in virtual_unimplemented {
            print_library_tree(&lib, Some(&blocks), show_usage);
        }
    }

    Ok(())
}

fn load_system_and_gi(input_path: &Utf8Path) -> Result<(System, Option<GraphicalInterface>)> {
    if input_path.extension() == Some("slx") {
        let file = File::open(input_path)
            .with_context(|| format!("Open Simulink model {}", input_path))?;
        let reader = BufReader::new(file);
        let mut parser = SimulinkParser::new("", ZipSource::new(reader)?);
        let system_root = Utf8PathBuf::from("simulink/systems/system_root.xml");
        let system = parser.parse_system_file(&system_root)?;

        let gi_path = Utf8PathBuf::from("simulink/graphicalInterface.json");
        let gi = parser.parse_graphical_interface_file(&gi_path).ok();
        return Ok((system, gi));
    }

    let mut parser = SimulinkParser::new(".", FsSource);

    // If a directory is provided, assume an extracted SLX root.
    if input_path.is_dir() {
        let system_root = if input_path.join("simulink/systems/system_root.xml").exists() {
            input_path.join("simulink/systems/system_root.xml")
        } else if input_path.join("systems/system_root.xml").exists() {
            // input is already the `simulink/` directory
            input_path.join("systems/system_root.xml")
        } else {
            anyhow::bail!(
                "Could not locate system_root.xml under {}",
                input_path.as_str()
            );
        };
        let system = parser
            .parse_system_file(&system_root)
            .with_context(|| format!("Failed to parse {}", system_root))?;

        let gi_path = if input_path.join("simulink/graphicalInterface.json").exists() {
            input_path.join("simulink/graphicalInterface.json")
        } else if input_path.join("graphicalInterface.json").exists() {
            input_path.join("graphicalInterface.json")
        } else {
            Utf8PathBuf::new()
        };
        let gi = if gi_path.as_str().is_empty() {
            None
        } else {
            parser.parse_graphical_interface_file(&gi_path).ok()
        };
        return Ok((system, gi));
    }

    // Otherwise, treat as a system XML file.
    let system = parser
        .parse_system_file(input_path)
        .with_context(|| format!("Failed to parse {}", input_path))?;

    // Try to locate graphicalInterface.json by walking up to find `simulink/`.
    let gi_path = find_graphical_interface_for_system_xml(input_path);
    let gi = if let Some(p) = gi_path {
        parser.parse_graphical_interface_file(&p).ok()
    } else {
        None
    };

    Ok((system, gi))
}

fn find_graphical_interface_for_system_xml(system_xml_path: &Utf8Path) -> Option<Utf8PathBuf> {
    for anc in system_xml_path.ancestors() {
        if anc.file_name() == Some("simulink") {
            let candidate = anc.join("graphicalInterface.json");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    // Fallback: same directory
    let sibling = system_xml_path
        .parent()
        .map(|p| p.join("graphicalInterface.json"));
    if let Some(sib) = sibling {
        if sib.exists() {
            return Some(sib);
        }
    }
    None
}

fn default_library_search_paths(input_path: &Utf8Path) -> Vec<Utf8PathBuf> {
    let mut out = Vec::new();
    if input_path.is_dir() {
        out.push(input_path.to_path_buf());
    } else if let Some(parent) = input_path.parent() {
        out.push(parent.to_path_buf());
    }
    out
}

fn normalize_lib_name_for_lookup(lib: &str) -> String {
    let trimmed = lib.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.ends_with(".slx") {
        trimmed[..trimmed.len().saturating_sub(4)].to_string()
    } else {
        trimmed.to_string()
    }
}

fn collect_library_uses(
    system: &System,
    out: &mut BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
) {
    let mut path: Vec<String> = Vec::new();
    system.walk_blocks(&mut path, &mut |p, blk| {
        let Some(source_block) = blk.properties.get("SourceBlock") else {
            return;
        };
        let source_block = source_block.trim();
        if source_block.is_empty() {
            return;
        }

        let (lib, _) = source_block.split_once('/').unwrap_or((source_block, ""));
        let lib = lib.trim();
        if lib.is_empty() {
            return;
        }

        let host_path = if p.is_empty() {
            format!("/{}", blk.name)
        } else {
            format!("/{}/{}", p.join("/"), blk.name)
        };

        out.entry(lib.to_string())
            .or_default()
            .entry(source_block.to_string())
            .or_default()
            .insert(host_path);
    });
}

fn collect_virtual_unimplemented_blocks(
    library_uses: &BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
) -> BTreeMap<String, BTreeMap<String, BTreeSet<String>>> {
    let mut out: BTreeMap<String, BTreeMap<String, BTreeSet<String>>> = BTreeMap::new();
    for (lib, blocks) in library_uses {
        for (source_block, host_paths) in blocks {
            // Virtual libraries may be indicated either by the lib name or by the full SourceBlock.
            if is_virtual_library(lib) || is_virtual_library(source_block) {
                out.entry(lib.clone())
                    .or_default()
                    .entry(source_block.clone())
                    .or_default()
                    .extend(host_paths.iter().cloned());
            }
        }
    }
    out
}

fn print_library_tree(
    lib: &str,
    blocks_opt: Option<&BTreeMap<String, BTreeSet<String>>>,
    show_usage: bool,
) {
    println!("{}", lib);

    let Some(blocks) = blocks_opt else {
        println!("|- <no library blocks found in model>");
        return;
    };

    if blocks.is_empty() {
        println!("|- <no library blocks found in model>");
        return;
    }

    for (source_block, hosts) in blocks {
        // Always report the number of instances, regardless of `show_usage`.
        let count = hosts.len();
        println!("|- {} ({} instance{})", source_block, count, if count == 1 { "" } else { "s" });

        if show_usage {
            if hosts.is_empty() {
                println!("|  |- <no instances found>");
                continue;
            }
            for host in hosts {
                println!("|  |- {}", host);
            }
        }
    }
}
