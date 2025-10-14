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
use rustylink::{egui_app, model::System, parser::{FsSource, SimulinkParser, ZipSource}};
#[cfg(feature = "egui")]
use eframe::egui;

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
}

#[cfg(feature = "egui")]
fn main() -> Result<()> {
    let args = Args::parse();
    let path = Utf8PathBuf::from(&args.file);

    // Parse system and collect optional charts
    let (root_system, charts, chart_map) = if path.extension() == Some("slx") {
        let file = std::fs::File::open(&path).with_context(|| format!("Open {}", path))?;
        let reader = std::io::BufReader::new(file);
        let mut parser = SimulinkParser::new("", ZipSource::new(reader)?);
        let root = Utf8PathBuf::from("simulink/systems/system_root.xml");
        let sys = parser.parse_system_file(&root)?;
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
        let sys = parser.parse_system_file(&path).with_context(|| format!("Failed to parse {}", path))?;
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

    let mut app = egui_app::SubsystemApp::new(root_system, initial_path, charts, chart_map);

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
            println!("Block context action: {} ({})", block.name, block.block_type);
        },
    );

    // Create and run the native window here to keep windowing in the example.
    // Load and apply window icon from the repository (embedded at compile time)
    let mut viewport = egui::ViewportBuilder::default().with_maximized(true);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../docs/RustyLinkIconSmall.png")) {
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
            // Register phosphor font once at startup so icons render.
            let mut font_definitions = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut font_definitions, egui_phosphor::Variant::Regular);
            font_definitions
                .families
                .insert(egui::FontFamily::Name("phosphor".into()), vec!["phosphor".into()]);
            cc.egui_ctx.set_fonts(font_definitions);
            cc.egui_ctx.set_visuals(egui::Visuals::light());
            Ok(Box::new(app.clone()))
        }),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

#[cfg(not(feature = "egui"))]
fn main() {
    eprintln!("This example requires the 'egui' feature. Try: cargo run --features egui --example egui_viewer -- <file> -s <system>");
}
