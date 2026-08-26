//! Dump the effective center icon for each distinct block identity in a model,
//! so we can see which UTF-8 glyphs are used (and might render as tofu).
//!
//! Usage:
//!   cargo run --features egui,dashboard,highlight --example dump_block_icons -- <file.slx>

use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use clap::Parser;
use std::collections::BTreeMap;

use rustylink::block_types::IconSpec;
use rustylink::egui_app::render::get_block_type_cfg;
use rustylink::model::{Block, System};
use rustylink::parser::{FsSource, SimulinkParser, ZipSource};
use rustylink::simulink_libraries::metadata::extract_metadata;
use rustylink::simulink_libraries::resolve_definition;
use rustylink::simulink_libraries::types::{BlockLabelPolicy, SimulinkIcon, SimulinkShape};

#[derive(Parser, Debug)]
struct Args {
    file: String,
    #[arg(short = 'L', long = "lib")]
    lib: Vec<String>,
}

fn block_label_text(block: &Block) -> Option<String> {
    let def = resolve_definition(block);
    let metadata = extract_metadata(block, def);
    match def.block_label {
        BlockLabelPolicy::None => {}
        BlockLabelPolicy::Fixed(s) => return Some(s.to_string()),
        BlockLabelPolicy::MetadataDependent(f) => {
            if let Some(s) = f(block, &metadata) {
                return Some(s);
            }
        }
    }
    def.compute_instance_label.and_then(|f| f(block))
}

fn icon_desc(block: &Block) -> String {
    let def = resolve_definition(block);
    if def.shape == SimulinkShape::FilledBlack {
        return "<filled-black>".into();
    }
    if def.static_renderer.is_some() {
        return "<renderer>".into();
    }
    if let Some(l) = block_label_text(block)
        && !l.is_empty()
    {
        return format!("<label:{l:?}>");
    }
    if let Some(icon) = def.icon {
        return match icon {
            SimulinkIcon::Utf8(g) => glyph_desc(g),
            SimulinkIcon::Phosphor(_) => "<phosphor>".into(),
            SimulinkIcon::Math(s) => format!("<math:{s}>"),
            SimulinkIcon::Plot(s) => format!("<plot:{s}>"),
        };
    }
    if def.shape != SimulinkShape::Rectangle {
        return "<empty-shape>".into();
    }
    let cfg = get_block_type_cfg(block);
    match cfg.icon {
        Some(IconSpec::Utf8(g)) => glyph_desc(g),
        Some(IconSpec::Phosphor(_)) => "<phosphor>".into(),
        Some(IconSpec::Math(s)) => format!("<math:{s}>"),
        Some(IconSpec::Plot(s)) => format!("<plot:{s}>"),
        None => "<QUESTION>".into(),
    }
}

fn glyph_desc(g: &str) -> String {
    if g.is_empty() {
        return "<empty-utf8>".into();
    }
    let cps: Vec<String> = g.chars().map(|c| format!("U+{:04X}", c as u32)).collect();
    format!("utf8 {:?} [{}]", g, cps.join(","))
}

fn ident(block: &Block) -> String {
    block
        .library_block_path
        .as_deref()
        .or_else(|| block.properties.get("SourceBlock").map(|s| s.as_str()))
        .map(|s| s.replace(['\n', '\r'], " "))
        .unwrap_or_else(|| block.block_type.clone())
}

fn walk(sys: &System, out: &mut BTreeMap<String, String>) {
    for block in &sys.blocks {
        out.entry(ident(block)).or_insert_with(|| icon_desc(block));
        if let Some(sub) = &block.subsystem {
            walk(sub, out);
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let path = Utf8PathBuf::from(&args.file);
    let mut lib_paths: Vec<Utf8PathBuf> = Vec::new();
    if let Some(parent) = path.parent()
        && !parent.as_str().is_empty()
    {
        lib_paths.push(parent.to_path_buf());
    }
    lib_paths.extend(args.lib.iter().map(Utf8PathBuf::from));

    let mut root_system = if path.extension() == Some("slx") {
        let file = std::fs::File::open(&path).with_context(|| format!("Open {}", path))?;
        let reader = std::io::BufReader::new(file);
        let mut parser = SimulinkParser::new("", ZipSource::new(reader)?);
        let root = Utf8PathBuf::from("simulink/systems/system_root.xml");
        parser.parse_system_file(&root)?
    } else {
        let mut parser = SimulinkParser::new(Utf8PathBuf::from("."), FsSource);
        parser.parse_system_file(&path)?
    };
    SimulinkParser::<FsSource>::resolve_library_references(&mut root_system, &lib_paths)
        .with_context(|| "Failed to resolve library references")?;

    let mut out: BTreeMap<String, String> = BTreeMap::new();
    walk(&root_system, &mut out);

    for (ident, icon) in &out {
        println!("{:<50} | {}", ident, icon);
    }
    Ok(())
}
