use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use clap::Parser;
use rustylink::parser::{FsSource, SimulinkParser, ZipSource};
use std::fs::File;
use std::io::BufReader;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Find and print every ManualSwitch block in a Simulink model"
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

    // Find ManualSwitch blocks and print with their subsystem path
    let found = system.find_blocks_by_type("ManualSwitch");
    if found.is_empty() {
        println!("No ManualSwitch blocks found");
        return Ok(());
    }

    for (path, blk) in found {
        let full_path = if path.is_empty() {
            format!("/{}", blk.name)
        } else {
            format!("/{}/{}", path.join("/"), blk.name)
        };
        println!(
            "Found ManualSwitch: {} (type: {})",
            full_path, blk.block_type
        );
        if !blk.properties.is_empty() {
            println!("  properties:");
            for (k, v) in &blk.properties {
                println!("    {} = {}", k, v);
            }
        }
        if !blk.ports.is_empty() {
            println!("  ports:");
            for p in &blk.ports {
                println!("    {} idx={:?}", p.port_type, p.index);
            }
        }
    }

    Ok(())
}
