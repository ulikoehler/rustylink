//! List all blocks in a Simulink model recursively.
//!
//! Usage:
//!   cargo run --example list_blocks -- <file.slx|system.xml>

use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use clap::Parser;
use rustylink::{
    parser::{FsSource, SimulinkParser, ZipSource},
};

#[derive(Parser, Debug)]
#[command(author, version, about = "List all blocks in a Simulink model", long_about = None)]
struct Args {
    /// Simulink .slx file or System XML file
    #[arg(value_name = "SIMULINK_FILE")]
    file: String,

    /// Additional directories to search for library `.slx` files. Can be repeated.
    #[arg(short = 'L', long = "lib")]
    lib: Vec<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let path = Utf8PathBuf::from(&args.file);

    // Build library search paths
    let mut lib_paths: Vec<Utf8PathBuf> = Vec::new();
    if let Some(parent) = path.parent() {
        if parent.as_str() != "" {
            lib_paths.push(parent.to_path_buf());
        }
    }
    lib_paths.extend(args.lib.iter().map(|s| Utf8PathBuf::from(s)));

    let mut root_system = if path.extension() == Some("slx") {
        let file = std::fs::File::open(&path).with_context(|| format!("Open {}", path))?;
        let reader = std::io::BufReader::new(file);
        let mut parser = SimulinkParser::new("", ZipSource::new(reader)?);
        let root = Utf8PathBuf::from("simulink/systems/system_root.xml");
        parser.parse_system_file(&root)?
    } else {
        let root_dir = Utf8PathBuf::from(".");
        let mut parser = SimulinkParser::new(&root_dir, FsSource);
        parser.parse_system_file(&path)?
    };

    // Resolve library references
    if !lib_paths.is_empty() {
        println!("Resolving library references using paths:");
        for p in &lib_paths {
            println!("  - {}", p);
        }
        SimulinkParser::<FsSource>::resolve_library_references(&mut root_system, &lib_paths)
            .with_context(|| "Failed to resolve library references")?;
    }

    println!("\nBlocks in {}:", args.file);
    println!("================");
    list_blocks_recursive(&root_system, "");

    Ok(())
}

fn list_blocks_recursive(sys: &rustylink::model::System, prefix: &str) {
    for block in &sys.blocks {
        let path = if prefix.is_empty() {
            format!("/{}", block.name)
        } else {
            format!("{}/{}", prefix, block.name)
        };
        
        let mut block_info = if block.block_type == "Reference" {
            if let Some(src) = block.properties.get("SourceBlock") {
                format!("Reference (SourceBlock: {})", src)
            } else {
                "Reference".to_string()
            }
        } else {
            block.block_type.clone()
        };

        if let Some(lib_src) = &block.library_source {
            block_info.push_str(&format!(" [resolved from: {}]", lib_src));
        }
        
        let subsys_info = if block.subsystem.is_some() {
            " [has subsystem]"
        } else {
            ""
        };
        
        println!("{:<40} {:<50} {}", path, block_info, subsys_info);
        
        if let Some(sub) = &block.subsystem {
            let next_prefix = if prefix.is_empty() {
                block.name.clone()
            } else {
                format!("{}/{}", prefix, block.name)
            };
            list_blocks_recursive(sub, &next_prefix);
        }
    }
}
