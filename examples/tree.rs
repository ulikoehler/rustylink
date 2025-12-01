use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use clap::Parser;
use rustylink::model::*;
use rustylink::parser::{FsSource, SimulinkParser, ZipSource};
use std::fs::File;
use std::io::BufReader;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Print an ASCII tree of SubSystems in a Simulink model"
)]
struct Cli {
    /// Path to .slx (zip) or system XML file
    #[arg(value_name = "SIMULINK_FILE")]
    simulink_file: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let path = Utf8PathBuf::from(&cli.simulink_file);
    let system = if path.extension() == Some("slx") {
        // open zip
        let file = File::open(&path).with_context(|| format!("Open {}", path))?;
        let reader = BufReader::new(file);
        let mut parser = SimulinkParser::new("", ZipSource::new(reader)?);
        let root = Utf8PathBuf::from("simulink/systems/system_root.xml");
        parser.parse_system_file(&root)?
    } else {
        let mut parser = SimulinkParser::new(".", FsSource);
        parser
            .parse_system_file(&path)
            .with_context(|| format!("Failed to parse {}", path))?
    };

    // Print ASCII tree of subsystems
    print_system_tree(&system, "", true);

    Ok(())
}

fn print_system_tree(system: &System, prefix: &str, _is_last: bool) {
    // Print top-level header only when called at root: system has properties maybe 'Name'
    let title = system
        .properties
        .get("Name")
        .map(|s| s.as_str())
        .unwrap_or("<root>");

    if prefix.is_empty() {
        println!("{}", title);
    }

    let mut subs: Vec<(&str, &Block)> = system
        .blocks
        .iter()
        .filter_map(|b| {
            if b.subsystem.is_some() {
                Some((b.name.as_str(), b))
            } else {
                None
            }
        })
        .collect();

    // sort by name for deterministic output
    subs.sort_by_key(|(n, _)| n.to_string());

    for (i, (name, block)) in subs.iter().enumerate() {
        let last = i + 1 == subs.len();
        let branch = if last { "└─" } else { "├─" };
        println!("{}{} {}", prefix, branch, name);
        let new_prefix = format!("{}{}  ", prefix, if last { "   " } else { "│  " });
        if let Some(child_sys) = &block.subsystem {
            print_system_tree(child_sys, &new_prefix, last);
        }
    }
}
