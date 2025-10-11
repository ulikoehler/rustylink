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
use rustylink::model::System;
#[cfg(feature = "egui")]
use rustylink::parser::{FsSource, SimulinkParser, ZipSource};

#[cfg(feature = "egui")]
use {
    std::collections::HashMap,
    eframe::egui::{self, Align2, Color32, Pos2, Rect, Stroke, Sense},
    rustylink::model::{Block, EndpointRef},
    egui_phosphor::variants::regular,
};

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

    // Parse system
    let system = if path.extension() == Some("slx") {
        let file = std::fs::File::open(&path).with_context(|| format!("Open {}", path))?;
        let reader = std::io::BufReader::new(file);
        let mut parser = SimulinkParser::new("", ZipSource::new(reader)?);
        let root = Utf8PathBuf::from("simulink/systems/system_root.xml");
        parser.parse_system_file(&root)?
    } else {
        let root_dir = Utf8PathBuf::from(".");
        let mut parser = SimulinkParser::new(&root_dir, FsSource);
        parser.parse_system_file(&path).with_context(|| format!("Failed to parse {}", path))?
    };

    // Resolve subsystem to show and own it
    let system_owned: System = if let Some(p) = &args.system {
        resolve_subsystem_by_path(&system, p).cloned().unwrap_or(system)
    } else {
        system
    };

    // Run egui app in a window that starts maximized (windowed fullscreen)
    // Some platforms do not support exclusive fullscreen well; starting maximized keeps a window but fills the screen.
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_maximized(true),
        ..Default::default()
    };
    eframe::run_native(
        "rustylink egui subsystem viewer",
        options,
        Box::new(|cc| {
            // Register phosphor font once at startup
            let mut font_definitions = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut font_definitions, egui_phosphor::Variant::Regular);
            // Also bind a named family so FontFamily::Name("phosphor") works without panic
            font_definitions
                .families
                .insert(egui::FontFamily::Name("phosphor".into()), vec!["phosphor".into()]);
            cc.egui_ctx.set_fonts(font_definitions);
                // Set light theme
                cc.egui_ctx.set_visuals(egui::Visuals::light());
            Ok(Box::new(SubsystemApp::new(system_owned)))
        })
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

#[cfg(feature = "egui")]
fn resolve_subsystem_by_path<'a>(root: &'a System, path: &str) -> Option<&'a System> {
    let mut cur: &System = root;
    let p = path.trim();
    let mut parts = p.trim_start_matches('/').split('/').filter(|s| !s.is_empty());
    for name in parts.by_ref() {
        let mut found = None;
        for b in &cur.blocks {
            if b.block_type == "SubSystem" && b.name == name {
                if let Some(sub) = &b.subsystem {
                    found = Some(sub.as_ref());
                    break;
                }
            }
        }
        cur = found?;
    }
    Some(cur)
}

#[cfg(feature = "egui")]
#[derive(Clone)]
struct SubsystemApp {
    system: System,
}

#[cfg(feature = "egui")]
impl SubsystemApp {
    fn new(system: System) -> Self { Self { system } }
}

#[cfg(feature = "egui")]
impl eframe::App for SubsystemApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.label("Use Esc to exit. Rendering subsystem blocks and lines.");
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            // We'll first allocate interaction rects (which mutably borrow `ui`),
            // collecting screen-space geometry and click responses. After that
            // we can obtain the painter and draw everything (avoids borrow conflicts).

            // Collect drawable blocks (with positions)
            let blocks: Vec<(&Block, Rect)> = self
                .system
                .blocks
                .iter()
                .filter_map(|b| parse_block_rect(b).map(|r| (b, r)))
                .collect();
            if blocks.is_empty() {
                ui.colored_label(Color32::YELLOW, "No blocks with positions to render");
                return;
            }

            // Compute bounding box
            let mut bb = blocks[0].1;
            for (_, r) in &blocks { bb = bb.union(*r); }

            // Add margins
            let margin = 20.0;
            let avail = ui.available_rect_before_wrap();
            let avail_size = avail.size();
            let width = (bb.width()).max(1.0);
            let height = (bb.height()).max(1.0);
            let sx = (avail_size.x - 2.0 * margin) / width;
            let sy = (avail_size.y - 2.0 * margin) / height;
            let scale = sx.min(sy).max(0.1);

            // Transform function from model coords to screen
            let to_screen = |p: Pos2| -> Pos2 {
                let x = (p.x - bb.left()) * scale + avail.left() + margin;
                let y = (p.y - bb.top()) * scale + avail.top() + margin;
                Pos2::new(x, y)
            };

            // Build sid->block rect map and allocate interaction rects for blocks first
            let mut sid_map: HashMap<u32, Rect> = HashMap::new();
            let mut block_views: Vec<(&Block, Rect, bool)> = Vec::new();
            for (b, r) in &blocks {
                if let Some(sid) = b.sid {
                    sid_map.insert(sid, *r);
                }
                let r_screen = Rect::from_min_max(to_screen(r.min), to_screen(r.max));
                let resp = ui.allocate_rect(r_screen, Sense::click());
                block_views.push((b, r_screen, resp.clicked()));
            }

            // At this point we have block_views (interaction info) and will compute
            // line_views next. Drawing will occur after both sets are prepared.

// Helper function to render block icons based on type
fn render_block_icon(painter: &egui::Painter, block: &Block, rect: &Rect) {
    let icon_size = 24.0;
    let icon_center = rect.center();
    match block.block_type.as_str() {
        "Product" => {
            painter.text(
                icon_center,
                Align2::CENTER_CENTER,
                regular::X,
                egui::FontId::new(icon_size, egui::FontFamily::Name("phosphor".into())),
                Color32::WHITE,
            );
        },
        "Constant" => {
            painter.text(
                icon_center,
                Align2::CENTER_CENTER,
                regular::WRENCH,
                egui::FontId::new(icon_size, egui::FontFamily::Name("phosphor".into())),
                Color32::WHITE,
            );
        },
        "Scope" => {
            painter.text(
                icon_center,
                Align2::CENTER_CENTER,
                regular::WAVE_SINE,
                egui::FontId::new(icon_size, egui::FontFamily::Name("phosphor".into())),
                Color32::WHITE,
            );
        },
        "ManualSwitch" => {
            painter.text(
                icon_center,
                Align2::CENTER_CENTER,
                regular::TOGGLE_LEFT,
                egui::FontId::new(icon_size, egui::FontFamily::Name("phosphor".into())),
                Color32::WHITE,
            );
        },
        _ => {}
    }
}

            // Draw lines using polyline points (points are relative offsets)
            let line_stroke = Stroke::new(2.0, Color32::LIGHT_GREEN);
            // Build a map of max port index per (sid, port_type) visible in this view
            let mut port_counts: HashMap<(u32, u8), u32> = HashMap::new();
            fn reg_ep(ep: &EndpointRef, port_counts: &mut HashMap<(u32, u8), u32>) {
                let key = (ep.sid, if ep.port_type == "out" { 1 } else { 0 });
                let idx1 = if ep.port_index == 0 { 1 } else { ep.port_index };
                port_counts
                    .entry(key)
                    .and_modify(|v| *v = (*v).max(idx1))
                    .or_insert(idx1);
            }
            fn reg_branch(br: &rustylink::model::Branch, port_counts: &mut HashMap<(u32, u8), u32>) {
                if let Some(dst) = &br.dst { reg_ep(dst, port_counts); }
                for sub in &br.branches { reg_branch(sub, port_counts); }
            }
            for line in &self.system.lines {
                if let Some(src) = &line.src { reg_ep(src, &mut port_counts); }
                if let Some(dst) = &line.dst { reg_ep(dst, &mut port_counts); }
                for br in &line.branches { reg_branch(br, &mut port_counts); }
            }

            // Precompute screen-space polylines and allocate hit rects (mutable ui borrow)
            // Store views for later drawing with the painter.
            let mut line_views: Vec<(&rustylink::model::Line, Vec<Pos2>, Pos2, bool)> = Vec::new();
            for line in &self.system.lines {
                let Some(src) = line.src.as_ref() else {
                    eprintln!(
                        "Cannot draw line '{}': missing src endpoint",
                        line.name.as_deref().unwrap_or("<unnamed>")
                    );
                    continue;
                };
                let Some(sr) = sid_map.get(&src.sid) else {
                    eprintln!(
                        "Cannot draw line '{}': missing src SID {} in current view",
                        line.name.as_deref().unwrap_or("<unnamed>"),
                        src.sid
                    );
                    continue;
                };
                let mut offsets_pts: Vec<Pos2> = Vec::new();
                let num_src = port_counts.get(&(src.sid, if src.port_type == "out" { 1 } else { 0 })).copied();
                let mut cur = endpoint_pos(*sr, src, num_src);
                offsets_pts.push(cur);
                for off in &line.points {
                    cur = Pos2::new(cur.x + off.x as f32, cur.y + off.y as f32);
                    offsets_pts.push(cur);
                }
                // Convert offsets (model-space) to screen-space points so we can both draw and
                // perform hit-testing for clicks.
                let mut screen_pts: Vec<Pos2> = offsets_pts.iter().map(|p| to_screen(*p)).collect();
                if let Some(dst) = line.dst.as_ref() {
                    if let Some(dr) = sid_map.get(&dst.sid) {
                        let num_dst = port_counts.get(&(dst.sid, if dst.port_type == "out" { 1 } else { 0 })).copied();
                        // Snap destination Y to the last offset point to ensure a horizontal final segment
                        let dst_pt = endpoint_pos_with_target(*dr, dst, num_dst, Some(cur.y));
                        let dst_screen = to_screen(dst_pt);
                        screen_pts.push(dst_screen);
                    }
                }
                // compute bbox for hit-testing
                if screen_pts.is_empty() { continue; }
                let mut min_x = screen_pts[0].x;
                let mut max_x = screen_pts[0].x;
                let mut min_y = screen_pts[0].y;
                let mut max_y = screen_pts[0].y;
                for p in &screen_pts {
                    min_x = min_x.min(p.x);
                    max_x = max_x.max(p.x);
                    min_y = min_y.min(p.y);
                    max_y = max_y.max(p.y);
                }
                let pad = 8.0;
                let hit_rect = Rect::from_min_max(Pos2::new(min_x - pad, min_y - pad), Pos2::new(max_x + pad, max_y + pad));
                let resp = ui.allocate_rect(hit_rect, Sense::click());
                let clicked = resp.clicked();
                let main_anchor = *offsets_pts.last().unwrap_or(&cur);
                line_views.push((line, screen_pts, main_anchor, clicked));
            }

            // Helper to draw branches recursively. Defined here so it can use `painter` and other locals.
            fn draw_branch_rec(
                painter: &egui::Painter,
                to_screen: &dyn Fn(Pos2) -> Pos2,
                sid_map: &HashMap<u32, Rect>,
                port_counts: &HashMap<(u32, u8), u32>,
                start: Pos2,
                br: &rustylink::model::Branch,
                color: Color32,
            ) {
                let mut pts: Vec<Pos2> = vec![start];
                let mut cur = start;
                for off in &br.points {
                    cur = Pos2::new(cur.x + off.x as f32, cur.y + off.y as f32);
                    pts.push(cur);
                }
                for seg in pts.windows(2) {
                    painter.line_segment([to_screen(seg[0]), to_screen(seg[1])], Stroke::new(2.0, color));
                }
                if let Some(dstb) = &br.dst {
                    if let Some(dr) = sid_map.get(&dstb.sid) {
                        let num_dst = port_counts.get(&(dstb.sid, if dstb.port_type == "out" { 1 } else { 0 })).copied();
                        // Snap branch destination to last point Y for a horizontal final segment
                        let end_pt = endpoint_pos_with_target(*dr, dstb, num_dst, Some(cur.y));
                        painter.line_segment([to_screen(*pts.last().unwrap_or(&cur)), to_screen(end_pt)], Stroke::new(2.0, color));
                    } else {
                        eprintln!("Cannot draw branch to dst SID {}", dstb.sid);
                    }
                }
                for sub in &br.branches {
                    draw_branch_rec(painter, to_screen, sid_map, port_counts, *pts.last().unwrap_or(&cur), sub, color);
                }
            }

            // Now obtain painter and draw lines and branches with the painter and handle clicks using stored info
            let painter = ui.painter();
            // Now draw lines and branches with the painter and handle clicks using stored info
            for (line, screen_pts, main_anchor, clicked) in &line_views {
                for seg in screen_pts.windows(2) {
                    painter.line_segment([seg[0], seg[1]], line_stroke);
                }
                let branch_color = Color32::from_rgb(120, 220, 120);
                for br in &line.branches {
                    draw_branch_rec(&painter, &to_screen, &sid_map, &port_counts, *main_anchor, br, branch_color);
                }
                if *clicked {
                    // try to get pointer position at interaction
                    let cp = ctx.input(|i| i.pointer.interact_pos());
                    if let Some(cp) = cp {
                        let mut min_dist = f32::INFINITY;
                        for seg in screen_pts.windows(2) {
                            let a = seg[0];
                            let b = seg[1];
                            let ab_x = b.x - a.x;
                            let ab_y = b.y - a.y;
                            let ap_x = cp.x - a.x;
                            let ap_y = cp.y - a.y;
                            let ab_len2 = (ab_x * ab_x + ab_y * ab_y).max(1e-6);
                            let t = (ap_x * ab_x + ap_y * ab_y) / ab_len2;
                            let t_clamped = t.max(0.0).min(1.0);
                            let proj_x = a.x + ab_x * t_clamped;
                            let proj_y = a.y + ab_y * t_clamped;
                            let dx = cp.x - proj_x;
                            let dy = cp.y - proj_y;
                            let dist = (dx * dx + dy * dy).sqrt();
                            if dist < min_dist { min_dist = dist; }
                        }
                        let threshold = 8.0;
                        if min_dist <= threshold {
                            println!("Clicked line: '{}' (min_dist={:.1})", line.name.as_deref().unwrap_or("<unnamed>"), min_dist);
                        }
                    }
                }
            }

            // Now render blocks using painter and the earlier allocated interaction results
            for (b, r_screen, clicked) in &block_views {
                let fill = Color32::from_gray(30);
                let stroke = Stroke::new(2.0, Color32::from_rgb(180, 180, 200));
                painter.rect_filled(*r_screen, 4.0, fill);
                painter.rect_stroke(*r_screen, 4.0, stroke, egui::StrokeKind::Inside);

                // Draw icon for specific block types using a common function
                render_block_icon(&painter, b, r_screen);

                // Block name: always draw beneath the block, horizontally centered
                let lines: Vec<&str> = b.name.split('\n').collect();
                let line_height = 16.0; // Approximate line height for font size 14
                let mut y = r_screen.bottom() + 2.0;
                for line in lines {
                    let pos = Pos2::new(r_screen.center().x, y);
                    // Use dark color for text outside the block
                    painter.text(pos, Align2::CENTER_TOP, line, egui::FontId::proportional(14.0), Color32::from_rgb(40, 40, 40));
                    y += line_height;
                }
                if *clicked {
                    if b.block_type == "SubSystem" {
                        println!("Clicked subsystem block: name='{}' sid={:?}", b.name, b.sid);
                    } else {
                        println!("Clicked block: type='{}' name='{}' sid={:?}", b.block_type, b.name, b.sid);
                    }
                }
            }
        });
    }
}

#[cfg(feature = "egui")]
fn parse_block_rect(b: &Block) -> Option<Rect> {
    let pos = b.position.as_deref()?;
    // Expected format: "[l, t, r, b]"
    let inner = pos.trim().trim_start_matches('[').trim_end_matches(']');
    let nums: Vec<f32> = inner
        .split(',')
        .map(|s| s.trim())
        .filter_map(|s| s.parse::<f32>().ok())
        .collect();
    if nums.len() == 4 {
        let l = nums[0];
        let t = nums[1];
        let r = nums[2];
        let btm = nums[3];
        Some(Rect::from_min_max(Pos2::new(l, t), Pos2::new(r, btm)))
    } else {
        None
    }
}

#[cfg(feature = "egui")]
fn endpoint_pos(r: Rect, ep: &EndpointRef, num_ports: Option<u32>) -> Pos2 {
    port_anchor_pos(r, ep.port_type.as_str(), ep.port_index, num_ports)
}

// Variant that tries to match a target Y (e.g., last polyline Y) to keep the final segment horizontal
#[cfg(feature = "egui")]
fn endpoint_pos_with_target(r: Rect, ep: &EndpointRef, num_ports: Option<u32>, target_y: Option<f32>) -> Pos2 {
    let mut p = endpoint_pos(r, ep, num_ports);
    if let Some(ty) = target_y {
        p.y = ty;
        // Clamp within the block vertical range to avoid overshooting due to rounding
        p.y = p.y.max(r.top()).min(r.bottom());
    }
    p
}

// Helper to make switching to real port coordinates easier later on.
#[cfg(feature = "egui")]
fn port_anchor_pos(r: Rect, port_type: &str, port_index: u32, num_ports: Option<u32>) -> Pos2 {
    // Distribute ports vertically: (N*2)+1 segments; ports occupy the centers of odd segments.
    // Use 1-based port indices (as in Simulink UI). If 0 is provided, treat it as 1.
    let idx1 = if port_index == 0 { 1 } else { port_index };
    // Ensure N is at least idx1 to avoid overshooting when metadata is incomplete.
    let n = num_ports.unwrap_or(idx1).max(idx1);
    let total_segments = n * 2 + 1;
    let y0 = r.top();
    let y1 = r.bottom();
    let dy = (y1 - y0) / (total_segments as f32);
    // Place at center of the corresponding odd segment: position = (2*idx1 - 0.5) segments from top.
    let y = y0 + ((2 * idx1) as f32 - 0.5) * dy;
    match port_type {
        "out" => Pos2::new(r.right(), y),
        _ => Pos2::new(r.left(), y),
    }
}

#[cfg(not(feature = "egui"))]
fn main() {
    eprintln!("This example requires the 'egui' feature. Try: cargo run --features egui --example egui_viewer -- <file> -s <system>");
}
