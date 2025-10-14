// Use the library crate's modules instead of redefining them here.

use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use clap::Parser;
use rustylink::parser::{FsSource, SimulinkParser, ZipSource};

#[derive(Parser, Debug)]
#[command(author, version, about = "Parse Simulink .slx or XML system files to JSON", long_about = None)]
struct Cli {
    /// Simulink .slx file or system XML file
    #[arg(value_name = "SIMULINK_FILE")]
    simulink_file: String,

    /// Print output as JSON (full tree)
    #[arg(short = 'j', long = "json")]
    json: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let path = Utf8PathBuf::from(&cli.simulink_file);
    let root_dir = Utf8PathBuf::from(".");

    if cli.json {
        // Print the complete JSON tree
        let system = if path.extension() == Some("slx") {
            let file = std::fs::File::open(&path).with_context(|| format!("Open {}", path))?;
            let reader = std::io::BufReader::new(file);
            let mut parser = SimulinkParser::new("", ZipSource::new(reader)?);
            let root = Utf8PathBuf::from("simulink/systems/system_root.xml");
            parser.parse_system_file(&root)?
        } else {
            let mut parser = SimulinkParser::new(&root_dir, FsSource);
            parser
                .parse_system_file(&path)
                .with_context(|| format!("Failed to parse {}", path))?
        };
        let json = serde_json::to_string_pretty(&system)?;
        println!("{}", json);
    } else {
        // Report unknown tags and block types
        let mut unknown_tags = std::collections::BTreeSet::new();
        let mut unknown_block_types = std::collections::BTreeSet::new();
        let known_tags = [
            "System",
            "Block",
            "Line",
            "P",
            "PortCounts",
            "PortProperties",
            "Port",
            "Branch",
        ];
        let known_block_types = [
            "SubSystem",
            "Inport",
            "Outport",
            "Gain",
            "Sum",
            "Product",
            "Constant",
            "Scope",
            "Integrator",
            "S-Function",
            "Switch",
            "Mux",
            "Demux",
            "UnitDelay",
            "DiscreteTransferFcn",
            "DiscreteFilter",
            "DiscreteStateSpace",
            "TransferFcn",
            "StateSpace",
            "From",
            "Goto",
            "Selector",
            "Display",
            "Saturate",
            "RelationalOperator",
            "LogicalOperator",
            "CompareToZero",
            "CompareToConstant",
            "Lookup_n-D",
            "Lookup",
            "Fcn",
            "MATLABFcn",
            "DataStoreRead",
            "DataStoreWrite",
            "DataStoreMemory",
            "Merge",
            "MultiPortSwitch",
            "RateTransition",
            "ZeroOrderHold",
            "TriggeredSubsystem",
            "EnabledSubsystem",
            "ActionPort",
            "If",
            "IfActionSubsystem",
            "ForEach",
            "ForEachSubsystem",
            "WhileIterator",
            "WhileSubsystem",
            "ModelReference",
            "BusCreator",
            "BusSelector",
            "BusAssignment",
            "BusElement",
            "BusToVector",
            "VectorToBus",
            "SignalConversion",
            "Sqrt",
            "Abs",
            "MinMax",
            "MaxMin",
            "Min",
            "Max",
            "SumOfElements",
            "SineWave",
            "Step",
            "Ramp",
            "PulseGenerator",
            "RandomNumber",
            "UniformRandomNumber",
            "RepeatingSequence",
            "RepeatingSequenceStair",
            "RepeatingSequenceRamp",
            "TriggeredDelay",
            "TriggeredSampleAndHold",
            "TriggeredToWorkspace",
            "TriggeredWriteToFile",
            "TriggeredReadFromFile",
            "TriggeredFromWorkspace",
            "TriggeredReadFromFile",
            "TriggeredWriteToFile",
            "TriggeredToWorkspace",
            "TriggeredFromWorkspace",
            "TriggeredSubsystem",
            "EnabledSubsystem",
            "IfActionSubsystem",
            "ForEachSubsystem",
            "WhileSubsystem",
            "ModelReference",
            "BusCreator",
            "BusSelector",
            "BusAssignment",
            "BusElement",
            "BusToVector",
            "VectorToBus",
            "SignalConversion",
            "Sqrt",
            "Abs",
            "MinMax",
            "MaxMin",
            "Min",
            "Max",
            "SumOfElements",
            "SineWave",
            "Step",
            "Ramp",
            "PulseGenerator",
            "RandomNumber",
            "UniformRandomNumber",
            "RepeatingSequence",
            "RepeatingSequenceStair",
            "RepeatingSequenceRamp",
        ];
        fn scan_xml(
            path: &Utf8PathBuf,
            unknown_tags: &mut std::collections::BTreeSet<String>,
            unknown_block_types: &mut std::collections::BTreeSet<String>,
            known_tags: &[&str],
            known_block_types: &[&str],
        ) -> Result<()> {
            let text = std::fs::read_to_string(path)?;
            let doc = roxmltree::Document::parse(&text)?;
            for node in doc.descendants().filter(|n| n.is_element()) {
                let tag = node.tag_name().name();
                if !known_tags.contains(&tag) {
                    unknown_tags.insert(tag.to_string());
                }
                if tag == "Block" {
                    if let Some(bt) = node.attribute("BlockType") {
                        if !known_block_types.contains(&bt) {
                            unknown_block_types.insert(bt.to_string());
                        }
                    }
                }
            }
            Ok(())
        }
        let mut xml_files = Vec::new();
        let simulink_dir = std::path::Path::new("simulink");
        if simulink_dir.exists() {
            for entry in walkdir::WalkDir::new(simulink_dir) {
                let entry = entry?;
                if entry
                    .path()
                    .extension()
                    .map(|e| e == "xml")
                    .unwrap_or(false)
                {
                    xml_files.push(Utf8PathBuf::from_path_buf(entry.path().to_path_buf()).unwrap());
                }
            }
        }
        for xml in &xml_files {
            let _ = scan_xml(
                xml,
                &mut unknown_tags,
                &mut unknown_block_types,
                &known_tags,
                &known_block_types,
            );
        }
        let result = serde_json::json!({
            "unknown_tags": unknown_tags,
            "unknown_block_types": unknown_block_types
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    }
    Ok(())
}

// No longer needed: resolve_default_root_xml and path_exists
