//! Edit a Simulink model interactively using egui (requires `--features egui`).
//!
//! Usage:
//!   cargo run --features egui --example egui_editor -- <file.slx|system.xml> [-s "/path/to/subsystem"] [-L /lib/path]

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
    editor,
    parser::{FsSource, SimulinkParser, ZipSource},
};

#[cfg(feature = "egui")]
#[derive(Parser, Debug)]
#[command(author, version, about = "Edit a Simulink model using egui", long_about = None)]
struct Args {
    /// Simulink .slx file or System XML file
    #[arg(value_name = "SIMULINK_FILE")]
    file: String,

    /// Full path of subsystem to open initially (e.g. "/Top/Sub")
    #[arg(short = 's', long = "system")]
    system: Option<String>,

    /// Additional directories to search for library `.slx` files. Can be repeated.
    #[arg(short = 'L', long = "lib")]
    lib: Vec<String>,
}

#[cfg(feature = "egui")]
fn main() -> Result<()> {
    let args = Args::parse();
    let path = Utf8PathBuf::from(&args.file);

    // Build library search paths
    let mut lib_paths: Vec<Utf8PathBuf> = Vec::new();
    if let Some(parent) = path.parent() {
        if !parent.as_str().is_empty() {
            lib_paths.push(parent.to_path_buf());
        }
    }
    lib_paths.extend(args.lib.iter().map(|s| Utf8PathBuf::from(s)));

    // Parse system
    let (root_system, charts, chart_map) = if path.extension() == Some("slx") {
        let file = std::fs::File::open(&path).with_context(|| format!("Open {}", path))?;
        let reader = std::io::BufReader::new(file);
        let mut parser = SimulinkParser::new("", ZipSource::new(reader)?);
        let root = Utf8PathBuf::from("simulink/systems/system_root.xml");
        let mut sys = parser.parse_system_file(&root)?;

        // Resolve library references
        if !lib_paths.is_empty() {
            SimulinkParser::<FsSource>::resolve_library_references(&mut sys, &lib_paths)
                .with_context(|| "Failed to resolve library references")?;
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
    } else {
        let root_dir = Utf8PathBuf::from(".");
        let mut parser = SimulinkParser::new(&root_dir, FsSource);
        let mut sys = parser
            .parse_system_file(&path)
            .with_context(|| format!("Failed to parse {}", path))?;

        if !lib_paths.is_empty() {
            SimulinkParser::<FsSource>::resolve_library_references(&mut sys, &lib_paths)
                .with_context(|| "Failed to resolve library references")?;
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

    // Compute initial path
    let initial_path: Vec<String> = if let Some(p) = &args.system {
        p.trim()
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    } else {
        Vec::new()
    };

    let mut state = editor::EditorState::new(root_system, initial_path, charts, chart_map);
    state.app.library_search_paths = lib_paths;

    // Print initial info
    if let Some(system) = state.current_system() {
        println!(
            "Opened model at /{}: {} blocks, {} lines",
            state.app.path.join("/"),
            system.blocks.len(),
            system.lines.len(),
        );
    }

    println!("Keyboard shortcuts:");
    println!("  A        — Open block browser");
    println!("  Delete   — Delete selection");
    println!("  Ctrl+Z   — Undo");
    println!("  Ctrl+Y   — Redo");
    println!("  Ctrl+C   — Copy");
    println!("  Ctrl+V   — Paste");
    println!("  R        — Rotate selection");
    println!("  M        — Mirror selection");
    println!("  Arrows   — Move selection (Ctrl+Arrow for 1px)");
    println!("  Shift+Drag — Rectangle selection");
    println!("  Escape   — Clear selection");

    // Create and run the native window
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
        "rustylink egui model editor",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::light());
            Ok(Box::new(state))
        }),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

#[cfg(not(feature = "egui"))]
fn main() {
    eprintln!(
        "This example requires the 'egui' feature. Try: cargo run --features egui --example egui_editor -- <file> -s <system>"
    );
}
