use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use clap::Parser;
use rustylink::model::SystemDoc;
use rustylink::parser::{FsSource, SimulinkParser, ZipSource};
use std::fs::File;
use std::io::BufReader;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Benchmark binary serialization and deserialization of Simulink models"
)]
struct Cli {
    /// Path to .slx (zip) or system XML file
    #[arg(value_name = "SIMULINK_FILE")]
    simulink_file: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let path = Utf8PathBuf::from(&cli.simulink_file);

    println!("Benchmarking file: {}", path);

    // 1. Initial Parsing
    let start_parse = Instant::now();
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
    let parse_duration = start_parse.elapsed();
    println!("Initial parsing time: {:?}", parse_duration);

    let doc = SystemDoc { system };

    // Create a temporary file for binary output
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.path();

    // 2. Serialization
    let start_ser = Instant::now();
    doc.save_to_binary(temp_path)?;
    let ser_duration = start_ser.elapsed();
    println!("Serialization time: {:?}", ser_duration);

    // 3. File Size
    let metadata = std::fs::metadata(temp_path)?;
    let file_size = metadata.len();
    println!("Binary file size: {} bytes", file_size);

    // 4. Deserialization
    let start_deser = Instant::now();
    let _loaded_doc = SystemDoc::load_from_binary(temp_path)?;
    let deser_duration = start_deser.elapsed();
    println!("Deserialization time: {:?}", deser_duration);

    Ok(())
}
