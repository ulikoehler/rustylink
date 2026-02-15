use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use clap::Parser;
use rustylink::parser::{FsSource, LibraryResolver, SimulinkParser, ZipSource};
use serde_json::json;
use std::fs::File;
use std::io::BufReader;

#[derive(Parser, Debug)]
#[command(author, version, about = "Analyze libraries used by a Simulink model")] 
struct Cli {
    /// Path to .slx (zip) or simulink root / system XML file
    #[arg(value_name = "SIMULINK_FILE_OR_DIR")]
    simulink_file: String,

    /// Output JSON instead of human-readable text
    #[arg(long)]
    json: bool,

    /// Optional library search paths (order matters). Used to locate LIBNAME.slx files.
    #[arg(short = 'L', long = "lib-path")]
    lib_paths: Vec<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let path = Utf8PathBuf::from(&cli.simulink_file);

    // Load GraphicalInterface JSON either from .slx or from filesystem
    let gi = if path.extension() == Some("slx") {
        let file = File::open(&path).with_context(|| format!("Open {}", path))?;
        let reader = BufReader::new(file);
        let mut parser = SimulinkParser::new("", ZipSource::new(reader)?);
        parser.parse_graphical_interface_file(Utf8PathBuf::from("simulink/graphicalInterface.json"))?
    } else {
        // If user supplied a system XML path (e.g. simulink/systems/system_root.xml), try to resolve
        // the simulink root. Otherwise assume the provided path is the root containing `simulink`.
        let candidate = if path.is_file() && path.as_str().contains("/systems/") {
            // path like .../simulink/systems/system_root.xml -> take parent parent
            path.parent().and_then(|p| p.parent()).map(|p| p.join("graphicalInterface.json"))
        } else if path.is_dir() {
            Some(path.join("graphicalInterface.json"))
        } else {
            // Try a few reasonable fallbacks
            let p1 = path.join("simulink/graphicalInterface.json");
            let p2 = path.with_file_name("graphicalInterface.json");
            if p1.exists() {
                Some(p1)
            } else {
                Some(p2)
            }
        };
        let gi_path = candidate.ok_or_else(|| anyhow::anyhow!("Cannot locate graphicalInterface.json"))?;
        let mut parser = SimulinkParser::new(".", FsSource);
        parser.parse_graphical_interface_file(gi_path)?
    };

    // Group ExternalFileReferences by library (first segment of Reference)
    use std::collections::BTreeMap;
    let mut by_lib: BTreeMap<String, Vec<_>> = BTreeMap::new();
    for r in &gi.external_file_references {
        // only LIBRARY_BLOCK entries are considered libraries
        if r.r#type != rustylink::parser::ExternalFileReferenceType::LibraryBlock {
            continue;
        }
        let lib = r.reference.split_once('/').map(|(a, _)| a).unwrap_or(r.reference.as_str());
        by_lib.entry(lib.to_string()).or_default().push(r);
    }

    // Optionally resolve library .slx files
    let resolver = if !cli.lib_paths.is_empty() {
        Some(LibraryResolver::new(cli.lib_paths.iter().map(|p| Utf8PathBuf::from(p.clone()))))
    } else {
        None
    };

    if cli.json {
        // Build JSON object
        let mut libs = serde_json::Map::new();
        for (lib, refs) in &by_lib {
            let found_path = resolver.as_ref()
                .and_then(|r| r.locate(std::iter::once(lib.as_str())).found.into_iter().next().map(|(_, p)| p.as_str().to_string()));
            let arr = serde_json::to_value(refs).unwrap_or(json!([]));
            let mut obj = serde_json::Map::new();
            obj.insert("found_at".to_string(), serde_json::Value::String(found_path.unwrap_or_default()));
            obj.insert("blocks".to_string(), arr);
            libs.insert(lib.clone(), serde_json::Value::Object(obj));
        }
        let out = json!({ "libraries": serde_json::Value::Object(libs) });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    // Human-readable output
    for (lib, refs) in &by_lib {
        println!("Library: {}", lib);
        if let Some(r) = &resolver {
            let lr = r.locate(std::iter::once(lib.as_str()));
            if let Some((_, p)) = lr.found.get(0) {
                println!("  located: {}", p);
            } else {
                println!("  located: <not found in provided lib-paths>");
            }
        }
        for rr in refs {
            println!("  - Path: {}", rr.path);
            println!("    Reference: {}", rr.reference);
            println!("    SID: {}", rr.sid);
            println!("    Type: {:?}", rr.r#type);
        }
        println!("");
    }

    Ok(())
}
