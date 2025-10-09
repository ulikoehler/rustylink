mod model;
mod parser;

use anyhow::{Context, Result};
use camino::{Utf8PathBuf};
use parser::{FsSource, SimulinkParser, ZipSource};
use clap::Parser;


#[derive(Parser, Debug)]
#[command(author, version, about = "Parse Simulink .slx or XML system files to JSON", long_about = None)]
struct Cli {
    /// Simulink .slx file or system XML file
    #[arg(value_name = "SIMULINK_FILE")]
    simulink_file: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let path = Utf8PathBuf::from(&cli.simulink_file);
    let root_dir = Utf8PathBuf::from(".");
    let system = if path.extension() == Some("slx") {
        // Read from .slx zip: the root system is at simulink/systems/system_root.xml
        let file = std::fs::File::open(&path).with_context(|| format!("Open {}", path))?;
        let reader = std::io::BufReader::new(file);
        let mut parser = SimulinkParser::new("", ZipSource::new(reader)?);
        let root = Utf8PathBuf::from("simulink/systems/system_root.xml");
        parser.parse_system_file(&root)?
    } else {
        // Fallback: parse a standalone XML file from FS
        let mut parser = SimulinkParser::new(&root_dir, FsSource);
        parser
            .parse_system_file(&path)
            .with_context(|| format!("Failed to parse {}", path))?
    };

    let json = serde_json::to_string_pretty(&system)?;
    println!("{}", json);
    Ok(())
}

// No longer needed: resolve_default_root_xml and path_exists
