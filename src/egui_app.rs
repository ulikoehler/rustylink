//! Egui-based interactive viewer for Simulink systems (feature = "egui").
//!
//! This module exposes small, well-documented utilities along with a ready-to-run
//! `SubsystemApp` that renders a Simulink subsystem and allows navigation and
//! basic interaction. It is intended to be reused by applications or examples.
//!
//! Enable using the `egui` feature, construct a [`SubsystemApp`], and call
//! `eframe::run_native` from your binary or example to open a viewer window.

#![cfg(feature = "egui")]

use std::collections::{BTreeMap, HashMap};

use eframe::egui::{self, Align2, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use egui::text::LayoutJob;
use egui_phosphor::variants::regular;
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
    match block.block_type.as_str() {
        "Product" => {
            painter.text(icon_center, Align2::CENTER_CENTER, regular::X, font, dark_icon);
        }
        "Constant" => {
            painter.text(icon_center, Align2::CENTER_CENTER, regular::WRENCH, font, dark_icon);
        }
        "Scope" => {
            painter.text(icon_center, Align2::CENTER_CENTER, regular::WAVE_SINE, font, dark_icon);
        }
        "ManualSwitch" => {
            painter.text(icon_center, Align2::CENTER_CENTER, regular::TOGGLE_LEFT, font, dark_icon);
        }
        _ => {}
    }
}

/// Data needed to open a chart popup.
#[derive(Clone)]
pub struct ChartView {
    pub title: String,
    pub script: String,
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

            let mut sid_map: HashMap<u32, Rect> = HashMap::new();
            let mut block_views: Vec<(&Block, Rect, bool)> = Vec::new();
            for (b, r) in &blocks {
                if let Some(sid) = b.sid {
                    sid_map.insert(sid, *r);
                }
                let r_screen = Rect::from_min_max(to_screen(r.min), to_screen(r.max));
                // Draw block background with light gray color
                let light_gray = Color32::from_rgb(230, 230, 230);
                ui.painter().rect_filled(r_screen, 6.0, light_gray);
                let resp = ui.allocate_rect(r_screen, Sense::click());
                block_views.push((b, r_screen, resp.clicked()));
            }

            // Precompute a block SID -> block name map for labeling fallbacks
            let mut sid_to_name: HashMap<u32, String> = HashMap::new();
            for (b, _r) in &blocks {
                if let Some(sid) = b.sid {
                    sid_to_name.insert(sid, b.name.clone());
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

            // Deterministic adjacency-aware rainbow color assignment
            fn circular_dist(a: f32, b: f32) -> f32 {
                let d = (a - b).abs();
                d.min(1.0 - d)
            }
            fn hue_to_color32(h: f32) -> Color32 {
                // Convert hue in [0,1) to RGB (simple HSV with s=0.85, v=0.95)
                let s = 0.85f32;
                let v = 0.95f32;
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
            let n_lines = current_system.lines.len();
            let palette: Vec<f32> = (0..n_lines).map(|i| (i as f32) / (n_lines.max(1) as f32)).collect();
            // Order nodes by degree desc, then by index for determinism
            let mut order: Vec<usize> = (0..n_lines).collect();
            order.sort_by_key(|&i| (-(line_adjacency[i].len() as isize), i as isize));
            let mut assigned_hues: Vec<Option<f32>> = vec![None; n_lines];
            let mut remaining: Vec<f32> = palette.clone();
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
                    hue_to_color32(h.unwrap_or(default_h))
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
                if let Some(dst) = line.dst.as_ref() {
                    if let Some(dr) = sid_map.get(&dst.sid) {
                        let num_dst = port_counts.get(&(dst.sid, if dst.port_type == "out" { 1 } else { 0 })).copied();
                        let dst_pt = endpoint_pos_with_target(*dr, dst, num_dst, Some(cur.y));
                        let dst_screen = to_screen(dst_pt);
                        screen_pts.push(dst_screen);
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

            fn draw_branch_rec(
                painter: &egui::Painter,
                to_screen: &dyn Fn(Pos2) -> Pos2,
                sid_map: &HashMap<u32, Rect>,
                port_counts: &HashMap<(u32, u8), u32>,
                start: Pos2,
                br: &crate::model::Branch,
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
                        let end_pt = endpoint_pos_with_target(*dr, dstb, num_dst, Some(cur.y));
                        painter.line_segment(
                            [to_screen(*pts.last().unwrap_or(&cur)), to_screen(end_pt)],
                            Stroke::new(2.0, color),
                        );
                    } else {
                        eprintln!("Cannot draw branch to dst SID {}", dstb.sid);
                    }
                }
                for sub in &br.branches {
                    draw_branch_rec(painter, to_screen, sid_map, port_counts, *pts.last().unwrap_or(&cur), sub, color);
                }
            }

            let painter = ui.painter();
            for (line, screen_pts, main_anchor, clicked, li) in &line_views {
                let color = line_colors.get(*li).copied().unwrap_or(line_stroke_default.color);
                let stroke = Stroke::new(2.0, color);
                for seg in screen_pts.windows(2) {
                    painter.line_segment([seg[0], seg[1]], stroke);
                }
                for br in &line.branches {
                    draw_branch_rec(&painter, &to_screen, &sid_map, &port_counts, *main_anchor, br, color);
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
                            println!(
                                "Clicked line: '{}' (min_dist={:.1})",
                                line.name.as_deref().unwrap_or("<unnamed>"),
                                min_dist
                            );
                        }
                    }
                }
            }

            // Place and render signal labels with collision avoidance via label_place module
            // Font sizes: blocks use ~14.0; signals use ~half
            let block_label_font = 14.0f32;
            // Previously signal font was approx half the block font. Increase by 1.5x.
            let signal_font = (block_label_font * 0.5 * 1.5).round().max(7.0);
            let sig_font_id = egui::FontId::proportional(signal_font);

            struct EguiMeasurer<'a> { ctx: &'a egui::Context, font: egui::FontId, color: Color32 }
            impl<'a> LabelMeasurer for EguiMeasurer<'a> {
                fn measure(&self, text: &str) -> (f32, f32) {
                    let galley = self.ctx.fonts(|f| f.layout_no_wrap(text.to_string(), self.font.clone(), self.color));
                    let s = galley.size();
                    (s.x, s.y)
                }
            }

            let cfg = LabelConfig::default();
            let mut placed_label_rects: Vec<Rect> = Vec::new();

            let mut draw_line_labels = |line: &crate::model::Line, screen_pts: &Vec<Pos2>, color: Color32| {
                if screen_pts.len() < 2 { return; }
                let label_text = if let Some(name) = line.name.as_ref().and_then(|s| if s.trim().is_empty() { None } else { Some(s) }) {
                    name.clone()
                } else {
                    let src_s = line.src.as_ref().and_then(|e| sid_to_name.get(&e.sid)).cloned().unwrap_or_else(|| format!("SID{}", line.src.as_ref().map(|e| e.sid).unwrap_or(0)));
                    let dst_s = line.dst.as_ref().and_then(|e| sid_to_name.get(&e.sid)).cloned().unwrap_or_else(|| format!("SID{}", line.dst.as_ref().map(|e| e.sid).unwrap_or(0)));
                    format!("{} → {}", src_s, dst_s)
                };

                let poly: Vec<V2> = screen_pts.iter().map(|p| V2{ x: p.x, y: p.y }).collect();
                let meas = EguiMeasurer { ctx, font: sig_font_id.clone(), color };
                let already: Vec<RF> = placed_label_rects.iter().map(|r| RF::from_min_max(V2{ x: r.left(), y: r.top() }, V2{ x: r.right(), y: r.bottom() })).collect();
                if let Some(result) = label_place::place_label(&poly, &label_text, &meas, cfg, &already) {
                    // Recreate galley in chosen orientation; place_label encodes vertical as stacked characters
                    let oriented_text = if result.horizontal {
                        label_text.clone()
                    } else {
                        let mut s = String::new();
                        for (i, ch) in label_text.chars().enumerate() {
                            if i > 0 { s.push('\n'); }
                            s.push(ch);
                        }
                        s
                    };
                    let galley = ctx.fonts(|f| f.layout_no_wrap(oriented_text, sig_font_id.clone(), color));
                    let draw_pos = Pos2::new(result.rect.min.x, result.rect.min.y);
                    painter.galley(draw_pos, galley, color);
                    // Debug print for every label: text + position/size in screen space
                    let w = result.rect.max.x - result.rect.min.x;
                    let h = result.rect.max.y - result.rect.min.y;
                    println!(
                        "label: text='{}' orientation={} at ({:.2}, {:.2}) size {:.2}x{:.2}",
                        label_text,
                        if result.horizontal { "horizontal" } else { "vertical" },
                        result.rect.min.x,
                        result.rect.min.y,
                        w,
                        h
                    );
                    placed_label_rects.push(Rect::from_min_max(
                        Pos2::new(result.rect.min.x, result.rect.min.y),
                        Pos2::new(result.rect.max.x, result.rect.max.y),
                    ));
                }
            };

            // Draw labels for each line using the computed colors and screen polylines
            for (line, screen_pts, _main_anchor, _clicked, li) in &line_views {
                let color = line_colors.get(*li).copied().unwrap_or(line_stroke_default.color);
                draw_line_labels(line, screen_pts, color);
            }

            for (b, r_screen, clicked) in &block_views {
                let fill = Color32::from_rgb(210, 210, 210); // light gray
                let stroke = Stroke::new(2.0, Color32::from_rgb(180, 180, 200));
                painter.rect_filled(*r_screen, 4.0, fill);
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
                    if b.block_type == "SubSystem" {
                        if b.is_matlab_function {
                            let mut instance_name = if path_snapshot.is_empty() {
                                b.name.clone()
                            } else {
                                format!("{}/{}", path_snapshot.join("/"), b.name)
                            };
                            instance_name = instance_name.trim_matches('/').to_string();
                            if let Some(cid) = self.chart_map.get(&instance_name).cloned() {
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
                                println!(
                                    "MATLAB Function clicked but instance not found in mapping: {}",
                                    instance_name
                                );
                            }
                        } else if let Some(_sub) = &b.subsystem {
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
