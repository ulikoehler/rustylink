//! Visualize a Simulink subsystem using egui (requires `--features egui`).
//!
//! Usage:
//!   cargo run --features egui --example egui_viewer -- <file.slx|system.xml> -s "/path/to/subsystem"

#[cfg(feature = "egui")]
use anyhow::{Context, Result};
#[cfg(feature = "egui")]
use camino::Utf8PathBuf;
#[cfg(feature = "egui")]
use clap::Parser;
#[cfg(feature = "egui")]
use eframe::egui;
#[cfg(feature = "egui")]
use rustylink::{
    egui_app,
    parser::{FsSource, SimulinkParser, ZipSource, LibraryResolver},
};

#[cfg(feature = "egui")]
#[derive(Parser, Debug)]
#[command(author, version, about = "Visualize a Simulink subsystem using egui", long_about = None)]
struct Args {
    /// Simulink .slx file or System XML file
    #[arg(value_name = "SIMULINK_FILE")]
    file: String,

    /// Full path of subsystem to render (e.g. "/Top/Sub"). If omitted, render root system
    #[arg(short = 's', long = "system")]
    system: Option<String>,

    /// Additional directories to search for library `.slx` files. Can be repeated.
    /// Example: `-L /path/to/libs -L /another/path`
    #[arg(short = 'L', long = "lib")]
    lib: Vec<String>,
}

#[cfg(feature = "egui")]
fn main() -> Result<()> {
    let args = Args::parse();
    let path = Utf8PathBuf::from(&args.file);

    // Build library search paths up-front (dir of the provided file + any -L entries)
    let mut lib_paths: Vec<Utf8PathBuf> = Vec::new();
    if let Some(parent) = path.parent() {
        if parent.as_str() != "" {
            lib_paths.push(parent.to_path_buf());
        }
    }
    lib_paths.extend(args.lib.iter().map(|s| Utf8PathBuf::from(s)));

    // Collect referenced library names (may be filled inside parser branches)
    let mut referenced_lib_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Parse system and collect optional charts
    let (root_system, charts, chart_map) = if path.extension() == Some("slx") {
        let file = std::fs::File::open(&path).with_context(|| format!("Open {}", path))?;
        let reader = std::io::BufReader::new(file);
        let mut parser = SimulinkParser::new("", ZipSource::new(reader)?);
        let root = Utf8PathBuf::from("simulink/systems/system_root.xml");
        let mut sys = parser.parse_system_file(&root)?;

        // Resolve library references (using previously-built search paths)
        if !lib_paths.is_empty() {
            SimulinkParser::<FsSource>::resolve_library_references(&mut sys, &lib_paths)
                .with_context(|| "Failed to resolve library references")?;
        }

        // Collect library names referenced from the system (SourceBlock) recursively
        fn collect_sys_libs(sys: &rustylink::model::System, acc: &mut std::collections::HashSet<String>) {
            for b in &sys.blocks {
                if let Some(src) = b.properties.get("SourceBlock") {
                    if let Some((lib, _)) = src.split_once('/') {
                        acc.insert(lib.to_string());
                    }
                }
                if let Some(sub) = &b.subsystem {
                    collect_sys_libs(sub, acc);
                }
            }
        }
        collect_sys_libs(&sys, &mut referenced_lib_names);

        // Also include library names from graphicalInterface.json where present
        if let Ok(names) = parser.graphical_interface_library_names(Utf8PathBuf::from("simulink/graphicalInterface.json")) {
            for n in names {
                referenced_lib_names.insert(n);
            }
        }

        let charts = parser.get_charts().clone();
        // Build combined chart map: prefer SID-based keys, also include name-based keys
        let mut chart_map: std::collections::BTreeMap<String, u32> = parser
            .get_sid_to_chart_map()
            .iter()
            .map(|(sid, cid)| (sid.to_string(), *cid))
            .collect();
        for (name, cid) in parser.get_system_to_chart_map().iter() {
            chart_map.entry(name.clone()).or_insert(*cid);
        }
        (sys, charts, chart_map)
    } else {
        let root_dir = Utf8PathBuf::from(".");
        let mut parser = SimulinkParser::new(&root_dir, FsSource);
        let mut sys = parser
            .parse_system_file(&path)
            .with_context(|| format!("Failed to parse {}", path))?;

        // Resolve library references (using previously-built search paths)
        if !lib_paths.is_empty() {
            SimulinkParser::<FsSource>::resolve_library_references(&mut sys, &lib_paths)
                .with_context(|| "Failed to resolve library references")?;
        }

        // Collect library names referenced from the system (SourceBlock) recursively
        fn collect_sys_libs(sys: &rustylink::model::System, acc: &mut std::collections::HashSet<String>) {
            for b in &sys.blocks {
                if let Some(src) = b.properties.get("SourceBlock") {
                    if let Some((lib, _)) = src.split_once('/') {
                        acc.insert(lib.to_string());
                    }
                }
                if let Some(sub) = &b.subsystem {
                    collect_sys_libs(sub, acc);
                }
            }
        }
        collect_sys_libs(&sys, &mut referenced_lib_names);

        // Also include library names from graphicalInterface.json where present
        if let Ok(names) = parser.graphical_interface_library_names(Utf8PathBuf::from("simulink/graphicalInterface.json")) {
            for n in names {
                referenced_lib_names.insert(n);
            }
        }

        let charts = parser.get_charts().clone();
        let mut chart_map: std::collections::BTreeMap<String, u32> = parser
            .get_sid_to_chart_map()
            .iter()
            .map(|(sid, cid)| (sid.to_string(), *cid))
            .collect();
        for (name, cid) in parser.get_system_to_chart_map().iter() {
            chart_map.entry(name.clone()).or_insert(*cid);
        }
        (sys, charts, chart_map)
    };

    // Compute initial path vector relative to root_system
    let initial_path: Vec<String> = if let Some(p) = &args.system {
        let parts: Vec<String> = p
            .trim()
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        if egui_app::resolve_subsystem_by_vec(&root_system, &parts).is_some() {
            parts
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    let mut app = egui_app::SubsystemApp::new(root_system.clone(), initial_path, charts, chart_map);

    // Propagate library search paths (if any) into the app so the UI can report them
    app.library_search_paths = lib_paths.clone();

    // Print any referenced libraries that could not be found in the provided search paths
    let mut lookup_opt = None;
    if !referenced_lib_names.is_empty() {
        let resolver = LibraryResolver::new(lib_paths.iter());
        let lookup = resolver.locate(referenced_lib_names.iter().map(|s| s.as_str()));
        lookup_opt = Some(lookup);
        if let Some(lu) = &lookup_opt {
            if !lu.not_found.is_empty() {
                eprintln!("[rustylink] Libraries referenced by model but NOT found in search paths:");
                for n in &lu.not_found {
                    eprintln!("  - {}", n);
                }
            }
        }
    }

    // Collect SourceBlock references that remain unresolved (library file missing or block not present)
    let mut unresolved_blocks: Vec<(String, String, String)> = Vec::new();
    fn collect_unresolved_blocks(sys: &rustylink::model::System, prefix: &str, acc: &mut Vec<(String, String, String)>) {
        for b in &sys.blocks {
            let host_path = if prefix.is_empty() { format!("/{}", b.name) } else { format!("{}/{}", prefix, b.name) };
            if let Some(src) = b.properties.get("SourceBlock") {
                if let Some((lib, blk)) = src.split_once('/') {
                    if b.library_block_path.is_none() {
                        acc.push((lib.to_string(), blk.to_string(), host_path.clone()));
                    }
                }
            }
            if let Some(sub) = &b.subsystem {
                let next_prefix = if prefix.is_empty() { b.name.clone() } else { format!("{}/{}", prefix, b.name) };
                collect_unresolved_blocks(sub, &next_prefix, acc);
            }
        }
    }
    collect_unresolved_blocks(&root_system, "", &mut unresolved_blocks);

    if !unresolved_blocks.is_empty() {
        eprintln!("[rustylink] Blocks referenced from libraries but NOT found:");
        for (lib, blk, host) in &unresolved_blocks {
            let lib_missing = lookup_opt
                .as_ref()
                .map_or(false, |lu| lu.not_found.iter().any(|n| n == lib));
            if lib_missing {
                eprintln!("  - {}/{} referenced by {} (library not found)", lib, blk, host);
            } else {
                eprintln!("  - {}/{} referenced by {} (library found but block missing)", lib, blk, host);
            }
        }
    }

    // Example: print current entities and listen for subsystem changes
    if let Some(ents) = app.current_entities() {
        println!(
            "Initial subsystem at /{}: {} blocks, {} lines, {} annotations",
            app.path.join("/"),
            ents.blocks.len(),
            ents.lines.len(),
            ents.annotations.len()
        );
    }
    app.add_subsystem_change_listener(|path, entities| {
        let p = if path.is_empty() {
            String::from("")
        } else {
            format!("/{}", path.join("/"))
        };
        println!(
            "Subsystem changed to {} ({} blocks, {} lines, {} annotations)",
            p,
            entities.blocks.len(),
            entities.lines.len(),
            entities.annotations.len()
        );
    });

    // Demo: add custom context menu items for signals and blocks
    app.add_signal_context_menu_item(
        "Print signal name",
        |_| true,
        |line| {
            let name = line.name.clone().unwrap_or_else(|| "<unnamed>".to_string());
            println!("Signal context action: {}", name);
        },
    );
    app.add_block_context_menu_item(
        "Print block name",
        |_| true,
        |block| {
            println!(
                "Block context action: {} ({})",
                block.name, block.block_type
            );
        },
    );

    // Example: observe block clicks but allow the default behavior to run
    app.set_block_click_handler(|_app, block| {
        println!("[click] Block: {} ({})", block.name, block.block_type);
        // Return false to let the default behavior (open subsystem / show dialogs) execute
        false
    });

    // Create and run the native window here to keep windowing in the example.
    // Load and apply window icon from the repository (embedded at compile time)
    let mut viewport = egui::ViewportBuilder::default().with_maximized(true);
    if let Ok(icon) =
        eframe::icon_data::from_png_bytes(include_bytes!("../docs/RustyLinkIconSmall.png"))
    {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "rustylink egui subsystem viewer",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::light());
            Ok(Box::new(app.clone()))
        }),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

#[cfg(not(feature = "egui"))]
fn main() {
    eprintln!(
        "This example requires the 'egui' feature. Try: cargo run --features egui --example egui_viewer -- <file> -s <system>"
    );
}
