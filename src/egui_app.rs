//! Egui-based interactive viewer for Simulink systems (feature = "egui").
//!
//! This module exposes small, well-documented utilities along with a ready-to-run
//! `SubsystemApp` that renders a Simulink subsystem and allows navigation and
//! basic interaction. It is intended to be reused by applications or examples.
//!
//! Enable using the `egui` feature, construct a [`SubsystemApp`], and call
//! `eframe::run_native` from your binary or example to open a viewer window.

#![cfg(feature = "egui")]

use std::collections::{BTreeMap, HashMap, HashSet};

use eframe::egui::{self, Align2, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use egui::text::LayoutJob;
// phosphor icons are configured via block_types; no direct use here
use crate::block_types::{self, BlockTypeConfig, IconSpec, Rgb};
use crate::label_place::{self, Config as LabelConfig, Measurer as LabelMeasurer, Vec2f as V2, RectF as RF};

use crate::model::{Block, Chart, EndpointRef, System};

/// Resolve a subsystem by an absolute path string, e.g. "/Top/Sub".
/// Returns `Some(&System)` when the path resolves within `root`, otherwise `None`.
pub fn resolve_subsystem_by_path<'a>(root: &'a System, path: &str) -> Option<&'a System> {
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

/// Resolve a subsystem by a vector of names relative to the `root` system.
pub fn resolve_subsystem_by_vec<'a>(root: &'a System, path: &[String]) -> Option<&'a System> {
    let mut cur: &System = root;
    for name in path {
        let mut found = None;
        for b in &cur.blocks {
            if b.block_type == "SubSystem" && &b.name == name {
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

/// Collect all non-chart subsystem paths for search/autocomplete.
/// Returns a vector of paths, each path is represented as `Vec<String>` of names from root.
pub fn collect_subsystems_paths(root: &System) -> Vec<Vec<String>> {
    fn rec(cur: &System, path: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
        for b in &cur.blocks {
            if b.block_type == "SubSystem" {
                if let Some(sub) = &b.subsystem {
                    if sub.chart.is_none() {
                        path.push(b.name.clone());
                        out.push(path.clone());
                        rec(sub, path, out);
                        path.pop();
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    let mut p = Vec::new();
    rec(root, &mut p, &mut out);
    out
}

/// Simple case-insensitive highlighter that builds a LayoutJob for `text`,
/// highlighting occurrences of `query`.
pub fn highlight_query_job(text: &str, query: &str) -> LayoutJob {
    let mut job = LayoutJob::default();
    let t = text;
    let tl = t.to_lowercase();
    let ql = query.to_lowercase();
    if ql.is_empty() {
        job.append(t, 0.0, egui::TextFormat::default());
        return job;
    }
    let mut i = 0;
    while let Some(pos) = tl[i..].find(&ql) {
        let start = i + pos;
        if start > i {
            job.append(&t[i..start], 0.0, egui::TextFormat::default());
        }
        let end = start + ql.len();
        let mut fmt = egui::TextFormat::default();
        fmt.background = Color32::YELLOW.into();
        job.append(&t[start..end], 0.0, fmt);
        i = end;
    }
    if i < t.len() {
        job.append(&t[i..], 0.0, egui::TextFormat::default());
    }
    job
}

/// MATLAB syntax highlighter using syntect. Lazily loads the syntax set and theme.
pub fn matlab_syntax_job(script: &str) -> LayoutJob {
    use egui::{FontId, TextFormat};
    use once_cell::sync::OnceCell;
    use syntect::easy::HighlightLines;
    use syntect::highlighting::{Style, ThemeSet};
    use syntect::parsing::SyntaxSet;
    use syntect::util::LinesWithEndings;

    static SYNTAX_SET: OnceCell<SyntaxSet> = OnceCell::new();
    static THEME_SET: OnceCell<ThemeSet> = OnceCell::new();

    let ss = SYNTAX_SET.get_or_init(|| SyntaxSet::load_defaults_newlines());
    let ts = THEME_SET.get_or_init(|| ThemeSet::load_defaults());
    // Important: Don't select by ".m" file extension as syntect often resolves that to Objective‑C.
    // Prefer the explicit MATLAB scope or well-known names and only then fall back to plain text.
    let syntax = {
        use syntect::parsing::Scope;
        // Try by scope first (most reliable)
        let by_scope = Scope::new("source.matlab").ok().and_then(|s| ss.find_syntax_by_scope(s));
        if let Some(s) = by_scope { s } else {
            // Try a few common names that appear across sublime grammars
            ss.find_syntax_by_name("Matlab")
                .or_else(|| ss.find_syntax_by_name("MATLAB"))
                .or_else(|| ss.find_syntax_by_name("Matlab (Octave)"))
                .or_else(|| ss.find_syntax_by_name("MATLAB (Octave)"))
                .unwrap_or_else(|| ss.find_syntax_plain_text())
        }
    };
    let theme = ts
        .themes
        .get("InspiredGitHub")
        .or_else(|| ts.themes.values().next())
        .unwrap();

    let mut h = HighlightLines::new(syntax, theme);
    let mut job = LayoutJob::default();
    let mono = FontId::monospace(14.0);

    for line in LinesWithEndings::from(script) {
        let regions: Vec<(Style, &str)> = h.highlight(line, ss);
        for (style, text) in regions {
            let color = Color32::from_rgba_premultiplied(
                style.foreground.r,
                style.foreground.g,
                style.foreground.b,
                style.foreground.a,
            );
            let tf = TextFormat { font_id: mono.clone(), color, ..Default::default() };
            job.append(text, 0.0, tf);
        }
    }
    job
}

/// Parse the block rectangle from a Simulink block's `Position` property.
/// Expects a string of the form "[l, t, r, b]".
pub fn parse_block_rect(b: &Block) -> Option<Rect> {
    let pos = b.position.as_deref()?;
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

/// Compute a port anchor position on a block's rectangle.
/// Ports are distributed vertically. `port_type` is "in" or "out".
pub fn port_anchor_pos(r: Rect, port_type: &str, port_index: u32, num_ports: Option<u32>) -> Pos2 {
    let idx1 = if port_index == 0 { 1 } else { port_index };
    let n = num_ports.unwrap_or(idx1).max(idx1);
    let total_segments = n * 2 + 1;
    let y0 = r.top();
    let y1 = r.bottom();
    let dy = (y1 - y0) / (total_segments as f32);
    let y = y0 + ((2 * idx1) as f32 - 0.5) * dy;
    match port_type {
        "out" => Pos2::new(r.right(), y),
        _ => Pos2::new(r.left(), y),
    }
}

/// Variant that tries to match a target Y (e.g., last polyline Y) to keep the final segment horizontal
pub fn endpoint_pos_with_target(r: Rect, ep: &EndpointRef, num_ports: Option<u32>, target_y: Option<f32>) -> Pos2 {
    let mut p = endpoint_pos(r, ep, num_ports);
    if let Some(ty) = target_y {
        let mut y = ty;
        y = y.max(r.top()).min(r.bottom());
        p.y = y;
    }
    p
}

/// Helper to compute a port anchor position given an endpoint reference.
pub fn endpoint_pos(r: Rect, ep: &EndpointRef, num_ports: Option<u32>) -> Pos2 {
    port_anchor_pos(r, ep.port_type.as_str(), ep.port_index, num_ports)
}

/// Render an icon in the center of the block according to its type using the phosphor font.
pub fn render_block_icon(painter: &egui::Painter, block: &Block, rect: &Rect) {
    let icon_size = 24.0;
    let icon_center = rect.center();
    let font = egui::FontId::new(icon_size, egui::FontFamily::Name("phosphor".into()));
    let dark_icon = Color32::from_rgb(40, 40, 40); // dark color for icons
    // Lookup icon from centralized registry
    let cfg = get_block_type_cfg(&block.block_type);
    if let Some(icon) = cfg.icon {
        match icon {
            IconSpec::Phosphor(glyph) => {
                painter.text(icon_center, Align2::CENTER_CENTER, glyph, font, dark_icon);
            }
        }
    }
}

fn rgb_to_color32(c: Rgb) -> Color32 { Color32::from_rgb(c.0, c.1, c.2) }

fn get_block_type_cfg(block_type: &str) -> BlockTypeConfig {
    let map = block_types::get_block_type_config_map();
    if let Ok(g) = map.read() {
        g.get(block_type).cloned().unwrap_or_default()
    } else {
        BlockTypeConfig::default()
    }
}

/// Data needed to open a chart popup.
#[derive(Clone)]
pub struct ChartView {
    pub title: String,
    pub script: String,
    pub open: bool,
}

/// Data for a selected signal information dialog.
#[derive(Clone)]
pub struct SignalDialog {
    pub title: String,
    pub line_idx: usize,
    pub open: bool,
}

/// Interactive Egui application that displays and navigates a Simulink subsystem tree.
#[derive(Clone)]
pub struct SubsystemApp {
    pub root: System,
    pub path: Vec<String>,
    pub all_subsystems: Vec<Vec<String>>,
    pub search_query: String,
    pub search_matches: Vec<Vec<String>>,
    pub zoom: f32,
    pub pan: Vec2,
    pub reset_view: bool,
    pub chart_view: Option<ChartView>,
    pub charts: BTreeMap<u32, Chart>,
    pub chart_map: BTreeMap<String, u32>,
    pub signal_view: Option<SignalDialog>,
}

impl SubsystemApp {
    /// Create a new app showing the provided `root` system.
    pub fn new(root: System, initial_path: Vec<String>, charts: BTreeMap<u32, Chart>, chart_map: BTreeMap<String, u32>) -> Self {
        let all = collect_subsystems_paths(&root);
        Self {
            root,
            path: initial_path,
            all_subsystems: all,
            search_query: String::new(),
            search_matches: Vec::new(),
            zoom: 1.0,
            pan: Vec2::ZERO,
            reset_view: true,
            chart_view: None,
            charts,
            chart_map,
            signal_view: None,
        }
    }

    /// Get the current subsystem based on `self.path`.
    pub fn current_system(&self) -> Option<&System> {
        resolve_subsystem_by_vec(&self.root, &self.path)
    }

    /// Navigate one level up, if possible.
    pub fn go_up(&mut self) {
        if !self.path.is_empty() {
            self.path.pop();
            self.reset_view = true;
        }
    }

    /// Navigate to the given path, if it resolves.
    pub fn navigate_to_path(&mut self, p: Vec<String>) {
        if resolve_subsystem_by_vec(&self.root, &p).is_some() {
            self.path = p;
            self.reset_view = true;
        }
    }

    /// If the block is a non-chart subsystem, open it and return true.
    pub fn open_block_if_subsystem(&mut self, b: &Block) -> bool {
        if b.block_type == "SubSystem" {
            if let Some(sub) = &b.subsystem {
                if sub.chart.is_none() {
                    self.path.push(b.name.clone());
                    self.reset_view = true;
                    return true;
                }
            }
        }
        false
    }

    /// Update `search_matches` based on `search_query`.
    pub fn update_search_matches(&mut self) {
        let q = self.search_query.trim();
        if q.is_empty() {
            self.search_matches.clear();
            return;
        }
        let ql = q.to_lowercase();
        let mut m: Vec<Vec<String>> = self
            .all_subsystems
            .iter()
            .filter(|p| p.last().map(|n| n.to_lowercase().contains(&ql)).unwrap_or(false))
            .cloned()
            .collect();
        m.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
        m.truncate(30);
        self.search_matches = m;
    }
}

impl eframe::App for SubsystemApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut navigate_to: Option<Vec<String>> = None;
        let mut clear_search = false;
        let path_snapshot = self.path.clone();

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let up = ui.add_enabled(!path_snapshot.is_empty(), egui::Button::new("↑ Up"));
                if up.clicked() {
                    let mut p = path_snapshot.clone();
                    p.pop();
                    navigate_to = Some(p);
                }
                ui.separator();
                ui.label(RichText::new("Path:").strong());
                if ui.link("Root").clicked() {
                    navigate_to = Some(Vec::new());
                }
                for (i, name) in path_snapshot.iter().enumerate() {
                    ui.label("/");
                    if ui.link(name).clicked() {
                        navigate_to = Some(path_snapshot[..=i].to_vec());
                    }
                }
            });
            ui.horizontal(|ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("Search subsystems by name…"),
                );
                if resp.changed() {
                    self.update_search_matches();
                }
            });
            if !self.search_query.trim().is_empty() && !self.search_matches.is_empty() {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        for p in self.search_matches.clone() {
                            let label = format!("/{}", p.join("/"));
                            let job = highlight_query_job(&label, &self.search_query);
                            let resp = ui.add(egui::Label::new(job).sense(Sense::click()));
                            if resp.clicked() {
                                navigate_to = Some(p);
                                clear_search = true;
                            }
                        }
                    });
                });
            }
        });

        let Some(current_system) = self.current_system() else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.colored_label(Color32::RED, "Invalid path – nothing to render");
            });
            return;
        };

        let mut navigate_to_from_block: Option<Vec<String>> = None;
        let mut open_chart: Option<ChartView> = None;
    let mut open_signal: Option<SignalDialog> = None;
        let mut staged_zoom = self.zoom;
        let mut staged_pan = self.pan;
        let mut staged_reset = self.reset_view;

        egui::CentralPanel::default().show(ctx, |ui| {
            let blocks: Vec<(&Block, Rect)> = current_system
                .blocks
                .iter()
                .filter_map(|b| parse_block_rect(b).map(|r| (b, r)))
                .collect();
            if blocks.is_empty() {
                ui.colored_label(Color32::YELLOW, "No blocks with positions to render");
                return;
            }

            let mut bb = blocks[0].1;
            for (_, r) in &blocks {
                bb = bb.union(*r);
            }

            let margin = 20.0;
            let avail = ui.available_rect_before_wrap();
            let avail_size = avail.size();
            let width = (bb.width()).max(1.0);
            let height = (bb.height()).max(1.0);
            let sx = (avail_size.x - 2.0 * margin) / width;
            let sy = (avail_size.y - 2.0 * margin) / height;
            let base_scale = sx.min(sy).max(0.1);

            if staged_reset {
                staged_zoom = 1.0;
                staged_pan = Vec2::ZERO;
                staged_reset = false;
            }

            let canvas_resp = ui.interact(avail, ui.id().with("canvas"), Sense::drag());
            if canvas_resp.dragged() {
                let d = canvas_resp.drag_delta();
                staged_pan += d;
            }
            let scroll_y = ctx.input(|i| i.raw_scroll_delta.y);
            if scroll_y.abs() > 0.0 && canvas_resp.hovered() {
                let factor = (1.0_f32 + scroll_y as f32 * 0.001_f32).max(0.1_f32);
                let old_zoom = staged_zoom;
                let new_zoom = (old_zoom * factor).clamp(0.2, 10.0);
                if (new_zoom - old_zoom).abs() > f32::EPSILON {
                    let origin = Pos2::new(avail.left() + margin, avail.top() + margin);
                    let s_old = base_scale * old_zoom;
                    let s_new = base_scale * new_zoom;
                    let cursor = canvas_resp.hover_pos().unwrap_or(avail.center());
                    let world_x = (cursor.x - origin.x - staged_pan.x) / s_old + bb.left();
                    let world_y = (cursor.y - origin.y - staged_pan.y) / s_old + bb.top();
                    staged_zoom = new_zoom;
                    staged_pan.x = cursor.x - ((world_x - bb.left()) * s_new + origin.x);
                    staged_pan.y = cursor.y - ((world_y - bb.top()) * s_new + origin.y);
                }
            }

            egui::Area::new("zoom_controls".into())
                .fixed_pos(Pos2::new(avail.left() + 8.0, avail.top() + 8.0))
                .show(ui.ctx(), |ui| {
                    egui::Frame::menu(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let center = avail.center();
                            let origin = Pos2::new(avail.left() + margin, avail.top() + margin);
                            if ui.small_button("−").clicked() {
                                let old_zoom = staged_zoom;
                                let new_zoom = (old_zoom * 0.9).clamp(0.2, 10.0);
                                let s_old = base_scale * old_zoom;
                                let s_new = base_scale * new_zoom;
                                let world_x = (center.x - origin.x - staged_pan.x) / s_old + bb.left();
                                let world_y = (center.y - origin.y - staged_pan.y) / s_old + bb.top();
                                staged_zoom = new_zoom;
                                staged_pan.x = center.x - ((world_x - bb.left()) * s_new + origin.x);
                                staged_pan.y = center.y - ((world_y - bb.top()) * s_new + origin.y);
                            }
                            if ui.small_button("+").clicked() {
                                let old_zoom = staged_zoom;
                                let new_zoom = (old_zoom * 1.1).clamp(0.2, 10.0);
                                let s_old = base_scale * old_zoom;
                                let s_new = base_scale * new_zoom;
                                let world_x = (center.x - origin.x - staged_pan.x) / s_old + bb.left();
                                let world_y = (center.y - origin.y - staged_pan.y) / s_old + bb.top();
                                staged_zoom = new_zoom;
                                staged_pan.x = center.x - ((world_x - bb.left()) * s_new + origin.x);
                                staged_pan.y = center.y - ((world_y - bb.top()) * s_new + origin.y);
                            }
                            if ui.small_button("Reset").clicked() {
                                staged_reset = true;
                            }
                        });
                    });
                });

            let to_screen = |p: Pos2| -> Pos2 {
                let s = base_scale * staged_zoom;
                let x = (p.x - bb.left()) * s + avail.left() + margin + staged_pan.x;
                let y = (p.y - bb.top()) * s + avail.top() + margin + staged_pan.y;
                Pos2::new(x, y)
            };

            let mut sid_map: HashMap<u32, Rect> = HashMap::new(); // world-space rects
            let mut sid_screen_map: HashMap<u32, Rect> = HashMap::new(); // screen-space rects
            let mut block_views: Vec<(&Block, Rect, bool)> = Vec::new();
            for (b, r) in &blocks {
                if let Some(sid) = b.sid {
                    sid_map.insert(sid, *r);
                }
                let r_screen = Rect::from_min_max(to_screen(r.min), to_screen(r.max));
                if let Some(sid) = b.sid { sid_screen_map.insert(sid, r_screen); }
                // Draw block background with configured color if provided
                let cfg = get_block_type_cfg(&b.block_type);
                let bg = cfg.background.map(rgb_to_color32).unwrap_or_else(|| Color32::from_rgb(210, 210, 210));
                ui.painter().rect_filled(r_screen, 6.0, bg);
                let resp = ui.allocate_rect(r_screen, Sense::click());
                block_views.push((b, r_screen, resp.clicked()));
            }

            // Precompute a block SID -> block name map for labeling fallbacks and lookup
            let mut sid_to_name: HashMap<u32, String> = HashMap::new();
            let mut sid_to_block: HashMap<u32, &Block> = HashMap::new();
            for (b, _r) in &blocks {
                if let Some(sid) = b.sid {
                    sid_to_name.insert(sid, b.name.clone());
                    sid_to_block.insert(sid, b);
                }
            }

            // Build adjacency of lines: two lines are adjacent if they share a src/dst SID
            // This helps us assign distinct rainbow colors to neighboring signals deterministically.
            let mut line_adjacency: Vec<Vec<usize>> = vec![Vec::new(); current_system.lines.len()];
            // Index endpoints per SID to compute adjacency quickly
            let mut sid_to_lines: HashMap<u32, Vec<usize>> = HashMap::new();
            for (i, l) in current_system.lines.iter().enumerate() {
                if let Some(src) = &l.src { sid_to_lines.entry(src.sid).or_default().push(i); }
                if let Some(dst) = &l.dst { sid_to_lines.entry(dst.sid).or_default().push(i); }
                // also include branch destinations
                fn collect_branch_sids(br: &crate::model::Branch, out: &mut Vec<u32>) {
                    if let Some(dst) = &br.dst { out.push(dst.sid); }
                    for sub in &br.branches { collect_branch_sids(sub, out); }
                }
                let mut br_sids = Vec::new();
                for br in &l.branches { collect_branch_sids(br, &mut br_sids); }
                for sid in br_sids { sid_to_lines.entry(sid).or_default().push(i); }
            }
            for (_sid, idxs) in &sid_to_lines {
                for a in 0..idxs.len() {
                    for b in (a+1)..idxs.len() {
                        let i = idxs[a];
                        let j = idxs[b];
                        if !line_adjacency[i].contains(&j) { line_adjacency[i].push(j); }
                        if !line_adjacency[j].contains(&i) { line_adjacency[j].push(i); }
                    }
                }
            }

            // Deterministic adjacency-aware rainbow color assignment with brightness guard
            fn circular_dist(a: f32, b: f32) -> f32 {
                let d = (a - b).abs();
                d.min(1.0 - d)
            }
            fn hsv_to_color32(h: f32, s: f32, v: f32) -> Color32 {
                let h6 = (h * 6.0) % 6.0;
                let c = v * s;
                let x = c * (1.0 - ((h6 % 2.0) - 1.0).abs());
                let (r1, g1, b1) = if h6 < 1.0 { (c, x, 0.0) }
                else if h6 < 2.0 { (x, c, 0.0) }
                else if h6 < 3.0 { (0.0, c, x) }
                else if h6 < 4.0 { (0.0, x, c) }
                else if h6 < 5.0 { (x, 0.0, c) } else { (c, 0.0, x) };
                let m = v - c;
                let (r, g, b) = (r1 + m, g1 + m, b1 + m);
                Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
            }
            fn hue_to_color32(h: f32) -> Color32 { hsv_to_color32(h, 0.85, 0.95) }
            fn rel_luminance(c: Color32) -> f32 {
                fn to_lin(u: u8) -> f32 {
                    let s = (u as f32) / 255.0;
                    if s <= 0.04045 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
                }
                let r = to_lin(c.r());
                let g = to_lin(c.g());
                let b = to_lin(c.b());
                0.2126 * r + 0.7152 * g + 0.0722 * b
            }
            let n_lines = current_system.lines.len();
            // Build a rich set of hue candidates and filter out those too light on the background
            let sample_count = (n_lines.max(1) * 8).max(64);
            let mut candidates: Vec<f32> = Vec::new();
            for i in 0..sample_count { candidates.push((i as f32) / (sample_count as f32)); }
            let bg_lum = rel_luminance(Color32::from_gray(245)); // approx UI bg
            let max_lum = (bg_lum - 0.25).clamp(0.0, 1.0); // ensure reasonable contrast; forbid too-light colors (e.g., yellows)
            candidates.retain(|&h| rel_luminance(hue_to_color32(h)) <= max_lum);
            if candidates.is_empty() {
                // Fallback if filtering removed all hues
                for i in 0..sample_count { candidates.push((i as f32) / (sample_count as f32)); }
            }
            // Order nodes by degree desc, then by index for determinism
            let mut order: Vec<usize> = (0..n_lines).collect();
            order.sort_by_key(|&i| (-(line_adjacency[i].len() as isize), i as isize));
            let mut assigned_hues: Vec<Option<f32>> = vec![None; n_lines];
            let mut remaining: Vec<f32> = candidates.clone();
            for i in order {
                // Choose hue from remaining that maximizes minimum circular distance to already assigned neighbors
                let neigh_hues: Vec<f32> = line_adjacency[i]
                    .iter()
                    .filter_map(|&j| assigned_hues[j])
                    .collect();
                let mut best_h = 0.0;
                let mut best_score = -1.0f32;
                for &h in &remaining {
                    let score: f32 = if neigh_hues.is_empty() {
                        // spread globally if no neighbor colors yet: maximize min distance to all used hues
                        let used: Vec<f32> = assigned_hues.iter().flatten().copied().collect();
                        if used.is_empty() { 1.0_f32 } else { used.iter().map(|&u| circular_dist(h, u)).fold(1.0_f32, |a, d| f32::min(a, d)) }
                    } else {
                        neigh_hues.iter().map(|&u| circular_dist(h, u)).fold(1.0_f32, |a, d| f32::min(a, d))
                    };
                    let s = score as f32;
                    if s > best_score || (s == best_score && h < best_h) {
                        best_score = s;
                        best_h = h;
                    }
                }
                assigned_hues[i] = Some(best_h);
                if let Some(pos) = remaining.iter().position(|&x| (x - best_h).abs() < f32::EPSILON) {
                    remaining.remove(pos);
                }
            }
            let line_colors: Vec<Color32> = assigned_hues
                .into_iter()
                .enumerate()
                .map(|(i, h)| {
                    let default_h = (i as f32) / (n_lines.max(1) as f32);
                    let c = hue_to_color32(h.unwrap_or(default_h));
                    // As a final guard, darken if still too light
                    if rel_luminance(c) > max_lum { hsv_to_color32(h.unwrap_or(default_h), 0.85, 0.75) } else { c }
                })
                .collect();

            let line_stroke_default = Stroke::new(2.0, Color32::LIGHT_GREEN);
            let mut port_counts: HashMap<(u32, u8), u32> = HashMap::new();
            fn reg_ep(ep: &EndpointRef, port_counts: &mut HashMap<(u32, u8), u32>) {
                let key = (ep.sid, if ep.port_type == "out" { 1 } else { 0 });
                let idx1 = if ep.port_index == 0 { 1 } else { ep.port_index };
                port_counts.entry(key).and_modify(|v| *v = (*v).max(idx1)).or_insert(idx1);
            }
            fn reg_branch(br: &crate::model::Branch, port_counts: &mut HashMap<(u32, u8), u32>) {
                if let Some(dst) = &br.dst {
                    reg_ep(dst, port_counts);
                }
                for sub in &br.branches {
                    reg_branch(sub, port_counts);
                }
            }
            for line in &current_system.lines {
                if let Some(src) = &line.src {
                    reg_ep(src, &mut port_counts);
                }
                if let Some(dst) = &line.dst {
                    reg_ep(dst, &mut port_counts);
                }
                for br in &line.branches {
                    reg_branch(br, &mut port_counts);
                }
            }

            let mut line_views: Vec<(&crate::model::Line, Vec<Pos2>, Pos2, bool, usize)> = Vec::new();
            // Requests to draw port labels inside blocks after blocks are drawn: (sid, port_index, is_input, y_screen)
            let mut port_label_requests: Vec<(u32, u32, bool, f32)> = Vec::new();
            for (li, line) in current_system.lines.iter().enumerate() {
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
                let mut screen_pts: Vec<Pos2> = offsets_pts.iter().map(|p| to_screen(*p)).collect();
                // Record output port label aligned with the source anchor Y
                if let Some(src_ep) = line.src.as_ref() {
                    let src_screen = *screen_pts.get(0).unwrap_or(&to_screen(cur));
                    port_label_requests.push((src_ep.sid, src_ep.port_index, false, src_screen.y));
                }
                if let Some(dst) = line.dst.as_ref() {
                    if let Some(dr) = sid_map.get(&dst.sid) {
                        let num_dst = port_counts.get(&(dst.sid, if dst.port_type == "out" { 1 } else { 0 })).copied();
                        let dst_pt = endpoint_pos_with_target(*dr, dst, num_dst, Some(cur.y));
                        let dst_screen = to_screen(dst_pt);
                        screen_pts.push(dst_screen);
                        // Record input port label aligned with the actual destination contact Y
                        if dst.port_type == "in" {
                            port_label_requests.push((dst.sid, dst.port_index, true, dst_screen.y));
                        }
                    }
                }
                if screen_pts.is_empty() {
                    continue;
                }
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
                let hit_rect = Rect::from_min_max(
                    Pos2::new(min_x - pad, min_y - pad),
                    Pos2::new(max_x + pad, max_y + pad),
                );
                let resp = ui.allocate_rect(hit_rect, Sense::click());
                let clicked = resp.clicked();
                let main_anchor = *offsets_pts.last().unwrap_or(&cur);
                line_views.push((line, screen_pts, main_anchor, clicked, li));
            }

            // Draw an arrow head at the end of a segment pointing to `tip`
            fn draw_arrowhead(painter: &egui::Painter, tail: Pos2, tip: Pos2, color: Color32) {
                // TODO(arrowheads): consider scaling with zoom
                let size = 8.0_f32;
                let dir = Vec2::new(tip.x - tail.x, tip.y - tail.y);
                let len = (dir.x * dir.x + dir.y * dir.y).sqrt().max(1e-3);
                let ux = dir.x / len;
                let uy = dir.y / len;
                // perpendicular
                let px = -uy;
                let py = ux;
                let base = Pos2::new(tip.x - ux * size, tip.y - uy * size);
                let left = Pos2::new(base.x + px * (size * 0.6), base.y + py * (size * 0.6));
                let right = Pos2::new(base.x - px * (size * 0.6), base.y - py * (size * 0.6));
                painter.add(egui::Shape::convex_polygon(vec![tip, left, right], color, Stroke::NONE));
            }

            fn draw_branch_rec(
                painter: &egui::Painter,
                to_screen: &dyn Fn(Pos2) -> Pos2,
                sid_map: &HashMap<u32, Rect>,
                port_counts: &HashMap<(u32, u8), u32>,
                start: Pos2,
                br: &crate::model::Branch,
                color: Color32,
                port_label_requests: &mut Vec<(u32, u32, bool, f32)>,
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
                        let end_pt = endpoint_pos_with_target(*dr, dstb, num_dst, Some(cur.y));
                        let last = *pts.last().unwrap_or(&cur);
                        let a = to_screen(last);
                        let b = to_screen(end_pt);
                        painter.line_segment([a, b], Stroke::new(2.0, color));
                        // Draw arrowhead only when contacting an input
                        if dstb.port_type == "in" { draw_arrowhead(painter, a, b, color); }
                        // Record input port label aligned with actual branch destination contact
                        if dstb.port_type == "in" { port_label_requests.push((dstb.sid, dstb.port_index, true, b.y)); }
                    } else {
                        eprintln!("Cannot draw branch to dst SID {}", dstb.sid);
                    }
                }
                for sub in &br.branches {
                    draw_branch_rec(painter, to_screen, sid_map, port_counts, *pts.last().unwrap_or(&cur), sub, color, port_label_requests);
                }
            }

            let painter = ui.painter();
            for (line, screen_pts, main_anchor, clicked, li) in &line_views {
                let color = line_colors.get(*li).copied().unwrap_or(line_stroke_default.color);
                let stroke = Stroke::new(2.0, color);
                for seg in screen_pts.windows(2) {
                    painter.line_segment([seg[0], seg[1]], stroke);
                }
                // Draw arrowhead for main destination if present
                if let Some(dst) = &line.dst {
                    if dst.port_type == "in" && screen_pts.len() >= 2 {
                        let n = screen_pts.len();
                        let a = screen_pts[n - 2];
                        let b = screen_pts[n - 1];
                        draw_arrowhead(&painter, a, b, color);
                    }
                }
                for br in &line.branches {
                    draw_branch_rec(&painter, &to_screen, &sid_map, &port_counts, *main_anchor, br, color, &mut port_label_requests);
                }
                if *clicked {
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
                            if dist < min_dist {
                                min_dist = dist;
                            }
                        }
                        let threshold = 8.0;
                        if min_dist <= threshold {
                            let title = line
                                .name
                                .clone()
                                .unwrap_or_else(|| sid_to_name.get(&line.src.as_ref().map(|s| s.sid).unwrap_or(0)).cloned().unwrap_or("<signal>".into()));
                            open_signal = Some(SignalDialog { title, line_idx: *li, open: true });
                        }
                    }
                }
            }

            // Place and render signal labels with collision avoidance via label_place module
            // Font sizes: blocks use ~14.0; signals were ~0.75x; increase another 1.5x => ~1.125x
            let block_label_font = 14.0f32;
            // Previously signal font was approx half the block font, then 1.5x. Increase by another 1.5x now.
            let signal_font = (block_label_font * 0.5 * 1.5 * 1.5).round().max(7.0);

            struct EguiMeasurer<'a> { ctx: &'a egui::Context, font: egui::FontId, color: Color32 }
            impl<'a> LabelMeasurer for EguiMeasurer<'a> {
                fn measure(&self, text: &str) -> (f32, f32) {
                    let galley = self.ctx.fonts(|f| f.layout_no_wrap(text.to_string(), self.font.clone(), self.color));
                    let s = galley.size();
                    (s.x, s.y)
                }
            }

            // Use a smaller perpendicular offset so labels appear visually attached to lines
            let cfg = LabelConfig { expand_factor: 1.5, step_fraction: 0.25, perp_offset: 2.0 };
            let mut placed_label_rects: Vec<Rect> = Vec::new();

            // Collect segments for a branch tree (model coords in, screen-space segments out)
            fn collect_branch_segments_rec(
                to_screen: &dyn Fn(Pos2) -> Pos2,
                sid_map: &HashMap<u32, Rect>,
                port_counts: &HashMap<(u32, u8), u32>,
                start: Pos2,
                br: &crate::model::Branch,
                out: &mut Vec<(Pos2, Pos2)>,
            ) {
                let mut pts: Vec<Pos2> = vec![start];
                let mut cur = start;
                for off in &br.points {
                    cur = Pos2::new(cur.x + off.x as f32, cur.y + off.y as f32);
                    pts.push(cur);
                }
                for seg in pts.windows(2) {
                    out.push((to_screen(seg[0]), to_screen(seg[1])));
                }
                if let Some(dstb) = &br.dst {
                    if let Some(dr) = sid_map.get(&dstb.sid) {
                        let num_dst = port_counts.get(&(dstb.sid, if dstb.port_type == "out" { 1 } else { 0 })).copied();
                        let end_pt = endpoint_pos_with_target(*dr, dstb, num_dst, Some(cur.y));
                        out.push((to_screen(*pts.last().unwrap_or(&cur)), to_screen(end_pt)));
                    }
                }
                for sub in &br.branches {
                    collect_branch_segments_rec(to_screen, sid_map, port_counts, *pts.last().unwrap_or(&cur), sub, out);
                }
            }

            let mut signal_label_rects: Vec<(Rect, usize)> = Vec::new();
            let mut draw_line_labels = |line: &crate::model::Line, screen_pts: &Vec<Pos2>, main_anchor: Pos2, color: Color32, line_idx: usize| {
                if screen_pts.len() < 2 { return; }
                let label_text = if let Some(name) = line.name.as_ref().and_then(|s| if s.trim().is_empty() { None } else { Some(s) }) {
                    name.clone()
                } else {
                    let src_s = line.src.as_ref().and_then(|e| sid_to_name.get(&e.sid)).cloned().unwrap_or_else(|| format!("SID{}", line.src.as_ref().map(|e| e.sid).unwrap_or(0)));
                    let dst_s = line.dst.as_ref().and_then(|e| sid_to_name.get(&e.sid)).cloned().unwrap_or_else(|| format!("SID{}", line.dst.as_ref().map(|e| e.sid).unwrap_or(0)));
                    // Use a Unicode triangle arrow for a nicer arrow glyph
                    if dst_s == "SID0" {
                        format!("{} ⏵", src_s)
                    } else {
                        format!("{} ⏵ {}", src_s, dst_s)
                    }
                };

                // Build list of all segments for this line, including branches, in screen space
                let mut segments: Vec<(Pos2, Pos2)> = Vec::new();
                for seg in screen_pts.windows(2) { segments.push((seg[0], seg[1])); }
                for br in &line.branches {
                    collect_branch_segments_rec(&to_screen, &sid_map, &port_counts, main_anchor, br, &mut segments);
                }
                // Choose the longest segment as preferred placement target
                let mut best_len2 = -1.0f32;
                let mut best_seg: Option<(Pos2, Pos2)> = None;
                for (a, b) in &segments {
                    let dx = b.x - a.x; let dy = b.y - a.y;
                    let l2 = dx*dx + dy*dy;
                    if l2 > best_len2 { best_len2 = l2; best_seg = Some((*a, *b)); }
                }
                let Some((sa, sb)) = best_seg else { return; };
                let poly: Vec<V2> = vec![V2{ x: sa.x, y: sa.y }, V2{ x: sb.x, y: sb.y }];
                // Avoid collisions with already placed labels, with blocks, and with this line's own segments
                let mut avoid_rects: Vec<RF> = placed_label_rects
                    .iter()
                    .map(|r| RF::from_min_max(V2{ x: r.left(), y: r.top() }, V2{ x: r.right(), y: r.bottom() }))
                    .collect();
                // Add all block rects as obstacles
                for (_b, br, _clicked) in &block_views {
                    avoid_rects.push(RF::from_min_max(V2{ x: br.left(), y: br.top() }, V2{ x: br.right(), y: br.bottom() }));
                }
                // Convert each segment to a very thin rectangle; algorithm expands by expand_factor internally,
                // so keep this tiny to avoid visible detachment while still preventing overlap
                let line_thickness = 0.8f32;
                for (a, b) in &segments {
                    let min_x = a.x.min(b.x) - line_thickness;
                    let max_x = a.x.max(b.x) + line_thickness;
                    let min_y = a.y.min(b.y) - line_thickness;
                    let max_y = a.y.max(b.y) + line_thickness;
                    avoid_rects.push(RF::from_min_max(V2{ x: min_x, y: min_y }, V2{ x: max_x, y: max_y }));
                }
                let already: Vec<RF> = avoid_rects;
                // Try: normal text, then wrapped text, then progressively smaller font
                let mut final_drawn = false;
                let mut font_size = signal_font;
                let mut tried_wrap = false;
                let mut wrap_text = label_text.clone();
                while !final_drawn {
                    let font_id = egui::FontId::proportional(font_size);
                    let meas = EguiMeasurer { ctx, font: font_id.clone(), color };
                    let candidate_texts: Vec<String> = if !tried_wrap && label_text.contains(' ') {
                        // Build a two-line wrap at closest space to center
                        let bytes: Vec<(usize, char)> = label_text.char_indices().collect();
                        let mut best_split = None;
                        let mut best_dist = usize::MAX;
                        for (i, ch) in bytes.iter() {
                            if *ch == ' ' {
                                let dist = (*i as isize - (label_text.len() as isize)/2).abs() as usize;
                                if dist < best_dist { best_dist = dist; best_split = Some(*i); }
                            }
                        }
                        if let Some(split) = best_split {
                            wrap_text = format!("{}\n{}", &label_text[..split].trim_end(), &label_text[split+1..].trim_start());
                            vec![label_text.clone(), wrap_text.clone()]
                        } else {
                            vec![label_text.clone()]
                        }
                    } else {
                        vec![label_text.clone(), wrap_text.clone()]
                    };

                    for candidate in candidate_texts.into_iter().filter(|s| !s.is_empty()) {
                        if let Some(result) = label_place::place_label(&poly, &candidate, &meas, cfg, &already) {
                            let oriented_text = if result.horizontal {
                                candidate.clone()
                            } else {
                                let mut s = String::new();
                                for (i, ch) in candidate.chars().enumerate() {
                                    if i > 0 { s.push('\n'); }
                                    s.push(ch);
                                }
                                s
                            };
                            let galley = ctx.fonts(|f| f.layout_no_wrap(oriented_text.clone(), font_id.clone(), color));
                            let draw_pos = Pos2::new(result.rect.min.x, result.rect.min.y);
                            painter.galley(draw_pos, galley, color);
                            let w = result.rect.max.x - result.rect.min.x;
                            let h = result.rect.max.y - result.rect.min.y;
                            /*println!(
                                "label: text='{}' orientation={} at ({:.2}, {:.2}) size {:.2}x{:.2}",
                                candidate.replace('\n', "\\n"),
                                if result.horizontal { "horizontal" } else { "vertical" },
                                result.rect.min.x,
                                result.rect.min.y,
                                w,
                                h
                            );*/
                            let rect = Rect::from_min_max(
                                Pos2::new(result.rect.min.x, result.rect.min.y),
                                Pos2::new(result.rect.max.x, result.rect.max.y),
                            );
                            placed_label_rects.push(rect);
                            signal_label_rects.push((rect, line_idx));
                            final_drawn = true;
                            break;
                        }
                    }
                    if final_drawn { break; }
                    if !tried_wrap && label_text.contains(' ') {
                        tried_wrap = true;
                    } else {
                        font_size *= 0.9; // shrink and retry
                        if font_size < 9.0 { break; }
                    }
                }
            };

            // Draw labels for each line using the computed colors and screen polylines
            for (line, screen_pts, _main_anchor, _clicked, li) in &line_views {
                let color = line_colors.get(*li).copied().unwrap_or(line_stroke_default.color);
                draw_line_labels(line, screen_pts, *_main_anchor, color, *li);
            }

            // Make signal labels clickable to open the info dialog
            for (r, li) in signal_label_rects {
                let resp = ui.interact(r, ui.id().with(("signal_label", li)), Sense::click());
                if resp.clicked() {
                    let line = &current_system.lines[li];
                    let title = line
                        .name
                        .clone()
                        .unwrap_or_else(|| line.src.as_ref().and_then(|s| sid_to_name.get(&s.sid)).cloned().unwrap_or("<signal>".into()));
                    open_signal = Some(SignalDialog { title, line_idx: li, open: true });
                }
            }

            for (b, r_screen, clicked) in &block_views {
                let cfg = get_block_type_cfg(&b.block_type);
                // background already filled earlier; now draw border using config
                let border_rgb = cfg.border.unwrap_or(Rgb(180, 180, 200));
                let stroke = Stroke::new(2.0, rgb_to_color32(border_rgb));
                painter.rect_stroke(*r_screen, 4.0, stroke, egui::StrokeKind::Inside);

                render_block_icon(&painter, b, r_screen);

                let lines: Vec<&str> = b.name.split('\n').collect();
                let line_height = 16.0;
                let mut y = r_screen.bottom() + 2.0;
                for line in lines {
                    let pos = Pos2::new(r_screen.center().x, y);
                    painter.text(
                        pos,
                        Align2::CENTER_TOP,
                        line,
                        egui::FontId::proportional(14.0),
                        Color32::from_rgb(40, 40, 40),
                    );
                    y += line_height;
                }
                if *clicked {
                    // Open MATLAB Function dialog for either block_type == "MATLAB Function" or SubSystem with is_matlab_function
                    if b.block_type == "MATLAB Function" || (b.block_type == "SubSystem" && b.is_matlab_function) {
                        // Prefer SID-based mapping if chart_map keys are SIDs
                        let sid_key = b.sid.map(|s| s.to_string());
                        let by_sid = sid_key.as_ref().and_then(|k| self.chart_map.get(k)).cloned();
                        // Fallback: name-based mapping from chart name in chart_*.xml
                        let mut instance_name = if path_snapshot.is_empty() {
                            b.name.clone()
                        } else {
                            format!("{}/{}", path_snapshot.join("/"), b.name)
                        };
                        instance_name = instance_name.trim_matches('/').to_string();
                        let cid_opt = by_sid.or_else(|| self.chart_map.get(&instance_name).cloned());
                        if let Some(cid) = cid_opt {
                            if let Some(chart) = self.charts.get(&cid) {
                                let title = chart
                                    .name
                                    .clone()
                                    .or(chart.eml_name.clone())
                                    .unwrap_or_else(|| b.name.clone());
                                let script = chart.script.clone().unwrap_or_default();
                                open_chart = Some(ChartView { title, script, open: true });
                            } else {
                                println!("MATLAB Function clicked but chart id {} not found", cid);
                            }
                        } else {
                            let available_instances: Vec<&String> = self.chart_map.keys().collect();
                            println!(
                                "MATLAB Function clicked but instance not found in mapping: {}. Available instances: {:?}",
                                instance_name,
                                available_instances
                            );
                        }
                    } else if b.block_type == "SubSystem" {
                        if let Some(_sub) = &b.subsystem {
                            let mut np = path_snapshot.clone();
                            np.push(b.name.clone());
                            navigate_to_from_block = Some(np);
                        } else {
                            println!(
                                "Clicked subsystem block (unresolved): name='{}' sid={:?}",
                                b.name, b.sid
                            );
                        }
                    } else {
                        println!(
                            "Clicked block: type='{}' name='{}' sid={:?}",
                            b.block_type, b.name, b.sid
                        );
                    }
                }
            }

            // Draw port labels after blocks so they are visible on top
            // TODO(port labels): Consider caching measurements per unique (text, font)
            let mut seen_port_labels: HashSet<(u32, u32, bool, i32)> = HashSet::new(); // (sid, index, is_input, y_center_rounded)
            let font_id = egui::FontId::proportional(12.0);
            for (sid, index, is_input, y) in port_label_requests {
                let key = (sid, index, is_input, y.round() as i32);
                if !seen_port_labels.insert(key) { continue; }
                let Some(brect) = sid_screen_map.get(&sid).copied() else { continue; };
                let Some(block) = sid_to_block.get(&sid) else { continue; };
                // Respect configuration for showing port labels
                let cfg = get_block_type_cfg(&block.block_type);
                if (is_input && !cfg.show_input_port_labels) || (!is_input && !cfg.show_output_port_labels) {
                    continue;
                }
                // Choose the correct port set
                let pname = block
                    .ports
                    .iter()
                    .filter(|p| p.port_type == if is_input { "in" } else { "out" } && p.index.unwrap_or(0) == index)
                    .filter_map(|p| {
                        p.properties.get("Name").cloned()
                            .or_else(|| p.properties.get("PropagatedSignals").cloned())
                            .or_else(|| p.properties.get("name").cloned())
                            .or_else(|| Some(format!("{}{}", if is_input { "In" } else { "Out" }, index)))
                    })
                    .next()
                    .unwrap_or_else(|| format!("{}{}", if is_input { "In" } else { "Out" }, index));

                let galley = ctx.fonts(|f| f.layout_no_wrap(pname.clone(), font_id.clone(), Color32::from_rgb(40,40,40)));
                let size = galley.size();
                let avail = brect.width() - 8.0; // margin
                if size.x <= avail {
                    // Vertically center the text at the requested Y and clamp inside block bounds.
                    // Use safe min/max to handle cases where text is taller than the block height.
                    let half_h = size.y * 0.5;
                    let y_min = brect.top();
                    let y_max = (brect.bottom() - size.y).max(y_min);
                    let y_top = (y - half_h).max(y_min).min(y_max);
                    let pos = if is_input {
                        // Left inside
                        Pos2::new(brect.left() + 4.0, y_top)
                    } else {
                        // Right inside, right-align text
                        Pos2::new(brect.right() - 4.0 - size.x, y_top)
                    };
                    painter.galley(pos, galley, Color32::from_rgb(40,40,40));
                }
            }
        });

        if let Some(p) = navigate_to_from_block.or(navigate_to) {
            self.navigate_to_path(p);
        }
        self.zoom = staged_zoom;
        self.pan = staged_pan;
        self.reset_view = staged_reset;
        if let Some(cv) = open_chart {
            self.chart_view = Some(cv);
        }
        if let Some(sd) = open_signal {
            self.signal_view = Some(sd);
        }
        if clear_search {
            self.search_query.clear();
            self.search_matches.clear();
        }

        if let Some(cv) = &mut self.chart_view {
            let mut open_flag = cv.open;
            egui::Window::new(format!("Chart: {}", cv.title))
                .open(&mut open_flag)
                .resizable(true)
                .vscroll(true)
                .min_width(400.0)
                .min_height(200.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                        let job = matlab_syntax_job(&cv.script);
                        ui.add(egui::Label::new(job).wrap());
                    });
                });
            cv.open = open_flag;
            if !cv.open {
                self.chart_view = None;
            }
        }

        // Render signal info dialog if open
        if let Some(sd) = &self.signal_view {
            let mut open_flag = sd.open;
            let title = format!("Signal: {}", sd.title);
            // Move required data out before mutably borrowing self
            let sys = self.current_system().map(|s| s.clone());
            let line_idx = sd.line_idx;
            egui::Window::new(title)
                .open(&mut open_flag)
                .resizable(true)
                .vscroll(true)
                .min_width(360.0)
                .min_height(200.0)
                .show(ctx, |ui| {
                    if let Some(sys) = &sys {
                        if let Some(line) = sys.lines.get(line_idx) {
                            // Basic info
                            ui.label(RichText::new("General").strong());
                            ui.horizontal_wrapped(|ui| {
                                ui.label(format!("Name: {}", line.name.clone().unwrap_or("<unnamed>".into())));
                                if let Some(z) = &line.zorder { ui.label(format!("Z: {}", z)); }
                            });
                            ui.separator();

                            // Collect inputs (source) and outputs (dst + branches)
                            let mut outputs: Vec<EndpointRef> = Vec::new();
                            fn collect_branch_dsts(br: &crate::model::Branch, out: &mut Vec<EndpointRef>) {
                                if let Some(d) = &br.dst { out.push(d.clone()); }
                                for s in &br.branches { collect_branch_dsts(s, out); }
                            }
                            if let Some(d) = &line.dst { outputs.push(d.clone()); }
                            for b in &line.branches { collect_branch_dsts(b, &mut outputs); }

                            egui::CollapsingHeader::new("Inputs").default_open(true).show(ui, |ui| {
                                if let Some(src) = &line.src {
                                    let bname = sys
                                        .blocks
                                        .iter()
                                        .find(|b| b.sid == Some(src.sid))
                                        .map(|b| b.name.clone())
                                        .unwrap_or_else(|| format!("SID{}", src.sid));
                                    let pname = sys
                                        .blocks
                                        .iter()
                                        .find(|b| b.sid == Some(src.sid))
                                        .and_then(|b| b.ports.iter().find(|p| p.port_type == src.port_type && p.index.unwrap_or(0) == src.port_index))
                                        .and_then(|p| p.properties.get("Name").cloned().or_else(|| p.properties.get("name").cloned()))
                                        .unwrap_or_else(|| format!("{}{}", if src.port_type=="in"{"In"}else{"Out"}, src.port_index));
                                    ui.label(format!("{} • {}{} ({}): {}", bname, if src.port_type=="in"{"In"}else{"Out"}, src.port_index, src.port_type, pname));
                                } else {
                                    ui.label("<no source>");
                                }
                            });
                            egui::CollapsingHeader::new("Outputs").default_open(true).show(ui, |ui| {
                                if outputs.is_empty() { ui.label("<none>"); }
                                for d in outputs {
                                    let bname = sys
                                        .blocks
                                        .iter()
                                        .find(|b| b.sid == Some(d.sid))
                                        .map(|b| b.name.clone())
                                        .unwrap_or_else(|| format!("SID{}", d.sid));
                                    let pname = sys
                                        .blocks
                                        .iter()
                                        .find(|b| b.sid == Some(d.sid))
                                        .and_then(|b| b.ports.iter().find(|p| p.port_type == d.port_type && p.index.unwrap_or(0) == d.port_index))
                                        .and_then(|p| p.properties.get("Name").cloned().or_else(|| p.properties.get("name").cloned()))
                                        .unwrap_or_else(|| format!("{}{}", if d.port_type=="in"{"In"}else{"Out"}, d.port_index));
                                    ui.label(format!("{} • {}{} ({}): {}", bname, if d.port_type=="in"{"In"}else{"Out"}, d.port_index, d.port_type, pname));
                                }
                            });
                        } else {
                            ui.colored_label(Color32::RED, "Selected signal no longer exists in this view");
                        }
                    }
                });
            // Now update the open flag mutably
            if let Some(sd_mut) = &mut self.signal_view {
                sd_mut.open = open_flag;
                if !sd_mut.open { self.signal_view = None; }
            }
        }
    }
}

// Intentionally no function here that opens an eframe window: the example/binary
// remains responsible for creating the native window and running the event loop.

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_system() -> System {
        let sub_child = System { properties: Default::default(), blocks: vec![], lines: vec![], chart: None };
        let sub_block = Block {
            block_type: "SubSystem".into(),
            name: "Child".into(),
            sid: Some(2),
            position: Some("[100, 100, 160, 140]".into()),
            zorder: None,
            commented: false,
            is_matlab_function: false,
            properties: Default::default(),
            ports: vec![],
            subsystem: Some(Box::new(sub_child)),
        };
        let root = System { properties: Default::default(), blocks: vec![sub_block], lines: vec![], chart: None };
        root
    }

    #[test]
    fn test_resolve_subsystem_path_and_vec() {
        let root = simple_system();
        assert!(resolve_subsystem_by_path(&root, "/Child").is_some());
        assert!(resolve_subsystem_by_vec(&root, &["Child".to_string()]).is_some());
        assert!(resolve_subsystem_by_path(&root, "/Nope").is_none());
    }

    #[test]
    fn test_collect_paths() {
        let root = simple_system();
        let paths = collect_subsystems_paths(&root);
        assert_eq!(paths, vec![vec!["Child".to_string()]]);
    }

    #[test]
    fn test_parse_block_rect_and_ports() {
        let root = simple_system();
        let b = &root.blocks[0];
        let r = parse_block_rect(b).unwrap();
        assert_eq!(r.left(), 100.0);
        let p1 = port_anchor_pos(r, "in", 1, Some(2));
        let p2 = port_anchor_pos(r, "out", 2, Some(2));
        assert!(p1.y < p2.y);
        assert!(p1.x < p2.x);
    }

    #[test]
    fn test_highlight_job() {
        let job = highlight_query_job("/A/B", "b");
        // Should build a non-empty job; exact fields depend on egui internals
        assert!(job.sections.len() >= 1);
    }
}
