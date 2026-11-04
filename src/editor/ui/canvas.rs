//! Grid, ports, arrows and branch drawing on the editor canvas.

#![cfg(feature = "egui")]

//! Editor UI — the interactive egui interface for model editing.
//!
//! This module provides the main rendering and interaction functions for
//! the Simulink model editor. It extends the viewer UI with:
//!
//! - Block dragging with arrow-key support
//! - Connection drawing with auto-snap to ports
//! - Rectangle selection of blocks and lines
//! - Block browser popup (hotkey "A")
//! - Context menus for blocks, lines, and canvas
//! - Code editor for MATLAB Function / CFunction blocks
//! - Keyboard shortcuts (Ctrl+Z/Y, Delete, Ctrl+C/V, R, M, etc.)
//! - Grid overlay

use crate::editor::state::{DragMode, EditorState};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::HashMap;
// ────────────────────────────────────────────────────────────────────────────
// Color utilities — re-exported from canonical `egui_app::ui::colors`
// ────────────────────────────────────────────────────────────────────────────

pub(super) fn draw_port_indicators(
    ui: &mut egui::Ui,
    block: &crate::model::Block,
    r_screen: &Rect,
    font_scale: f32,
) {
    fn paint_port_chevron(
        painter: &egui::Painter,
        outline: Pos2,
        is_left_side: bool,
        font_scale: f32,
        color: Color32,
    ) {
        let (h, w, stroke_w) = crate::egui_app::geometry::port_chevron_size(font_scale);

        let (base_x, tip_x) = if is_left_side {
            let tip_x = outline.x - stroke_w / 2.0;
            (tip_x - w, tip_x)
        } else {
            let base_x = outline.x + stroke_w / 2.0;
            (base_x, base_x + w)
        };

        let points = vec![
            Pos2::new(base_x, outline.y - h / 2.0),
            Pos2::new(tip_x, outline.y),
            Pos2::new(base_x, outline.y + h / 2.0),
        ];

        painter.add(egui::Shape::Path(egui::epaint::PathShape::line(
            points,
            Stroke::new(stroke_w, color),
        )));
    }

    let in_count = block.port_counts.as_ref().and_then(|p| p.ins).unwrap_or(0);
    let out_count = block.port_counts.as_ref().and_then(|p| p.outs).unwrap_or(0);
    let mirrored = block.block_mirror.unwrap_or(false);

    let (in_x, out_x) = if mirrored {
        (r_screen.right(), r_screen.left())
    } else {
        (r_screen.left(), r_screen.right())
    };

    let ins_left_side = !mirrored;
    let outs_left_side = mirrored;

    // Input ports — (i - 0.5)/n cell-centred fractions, matching the line
    // endpoint anchors (`port_anchor_pos`) so chevrons sit exactly on arrows.
    for i in 0..in_count {
        let n = in_count.max(1);
        let y = r_screen.top() + r_screen.height() * ((i as f32 + 0.5) / (n as f32));
        paint_port_chevron(
            ui.painter(),
            Pos2::new(in_x, y),
            ins_left_side,
            font_scale,
            Color32::from_rgb(60, 60, 200),
        );
    }

    // Output ports — same anchor formula as inputs.
    for i in 0..out_count {
        let n = out_count.max(1);
        let y = r_screen.top() + r_screen.height() * ((i as f32 + 0.5) / (n as f32));
        paint_port_chevron(
            ui.painter(),
            Pos2::new(out_x, y),
            outs_left_side,
            font_scale,
            Color32::from_rgb(200, 60, 60),
        );
    }
}

pub(super) fn draw_grid(
    ui: &mut egui::Ui,
    avail: &Rect,
    to_screen: &dyn Fn(Pos2) -> Pos2,
    from_screen: &dyn Fn(Pos2) -> Pos2,
    grid_size: i32,
    _zoom: f32,
    _base_scale: f32,
) {
    let tl = from_screen(avail.left_top());
    let br = from_screen(avail.right_bottom());
    let grid = grid_size.max(1) as f32;

    let start_x = (tl.x / grid).floor() as i32 * grid_size;
    let end_x = (br.x / grid).ceil() as i32 * grid_size;
    let start_y = (tl.y / grid).floor() as i32 * grid_size;
    let end_y = (br.y / grid).ceil() as i32 * grid_size;

    let grid_color = Color32::from_rgba_unmultiplied(100, 100, 100, 30);
    let grid_stroke = Stroke::new(0.5_f32, grid_color);

    let mut x = start_x;
    while x <= end_x {
        let p1 = to_screen(Pos2::new(x as f32, start_y as f32));
        let p2 = to_screen(Pos2::new(x as f32, end_y as f32));
        ui.painter().line_segment([p1, p2], grid_stroke);
        x += grid_size;
    }

    let mut y = start_y;
    while y <= end_y {
        let p1 = to_screen(Pos2::new(start_x as f32, y as f32));
        let p2 = to_screen(Pos2::new(end_x as f32, y as f32));
        ui.painter().line_segment([p1, p2], grid_stroke);
        y += grid_size;
    }
}

pub(super) fn draw_arrow_with_trim(
    painter: &egui::Painter,
    tail: Pos2,
    tip: Pos2,
    color: Color32,
    stroke: Stroke,
) {
    let size = 8.0_f32;
    let dir = Vec2::new(tip.x - tail.x, tip.y - tail.y);
    let len = (dir.x * dir.x + dir.y * dir.y).sqrt().max(1e-3);
    let ux = dir.x / len;
    let uy = dir.y / len;
    let inset = size * 0.6;
    let tip_adj = Pos2::new(tip.x - ux * inset, tip.y - uy * inset);
    painter.line_segment([tail, tip_adj], stroke);

    let px = -uy;
    let py = ux;
    let base = Pos2::new(tip_adj.x - ux * size, tip_adj.y - uy * size);
    let left = Pos2::new(base.x + px * (size * 0.6), base.y + py * (size * 0.6));
    let right = Pos2::new(base.x - px * (size * 0.6), base.y - py * (size * 0.6));
    painter.add(egui::Shape::convex_polygon(
        vec![tip_adj, left, right],
        color,
        Stroke::NONE,
    ));
}

/// Shared lookup tables used while drawing a branch tree.
pub(super) struct BranchLookup<'a> {
    pub to_screen: &'a dyn Fn(Pos2) -> Pos2,
    pub sid_map: &'a HashMap<String, Rect>,
    pub port_counts: &'a HashMap<(String, u8), u32>,
    pub sid_mirrored: &'a HashMap<String, bool>,
    pub sid_port_overrides:
        &'a HashMap<String, Vec<crate::simulink_libraries::types::PortPositionOverride>>,
}

pub(super) fn draw_branch_rec(
    painter: &egui::Painter,
    lk: &BranchLookup,
    start: Pos2,
    br: &crate::model::Branch,
    stroke: Stroke,
    color: Color32,
) {
    let mut pts: Vec<Pos2> = vec![start];
    let mut cur = start;
    for off in &br.points {
        cur = Pos2::new(cur.x + off.x as f32, cur.y + off.y as f32);
        pts.push(cur);
    }
    for seg in pts.windows(2) {
        let a = (lk.to_screen)(seg[0]);
        let b = (lk.to_screen)(seg[1]);
        painter.line_segment([a, b], stroke);
    }
    if let Some(dstb) = &br.dst
        && let Some(dr) = lk.sid_map.get(&dstb.sid)
    {
        let mirrored_dst = lk.sid_mirrored.get(&dstb.sid).copied().unwrap_or(false);
        let dst_overrides = lk
            .sid_port_overrides
            .get(&dstb.sid)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let end_pt = crate::egui_app::ui::signal_routing::endpoint_pos(
            *dr,
            dstb,
            lk.port_counts,
            mirrored_dst,
            dst_overrides,
        );
        let a = (lk.to_screen)(*pts.last().unwrap_or(&cur));
        let b = (lk.to_screen)(end_pt);
        let is_in_dst = dstb.port_type == "in"
            || crate::egui_app::geometry::is_control_port_type(&dstb.port_type);
        if is_in_dst {
            draw_arrow_with_trim(painter, a, b, color, stroke);
        } else {
            painter.line_segment([a, b], stroke);
        }
    }
    // Draw a junction dot at the sub-branch point when there are sub-branches.
    if !br.branches.is_empty() {
        painter.circle_filled((lk.to_screen)(*pts.last().unwrap_or(&cur)), 4.0, color);
    }
    for sub in &br.branches {
        draw_branch_rec(painter, lk, *pts.last().unwrap_or(&cur), sub, stroke, color);
    }
}

pub fn compute_line_colors(
    lines: &[crate::model::Line],
    _port_counts: &HashMap<(String, u8), u32>,
) -> Vec<Color32> {
    let n = lines.len();
    if n == 0 {
        return Vec::new();
    }

    // Build adjacency
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut sid_to_lines: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, l) in lines.iter().enumerate() {
        if let Some(src) = &l.src {
            sid_to_lines.entry(src.sid.clone()).or_default().push(i);
        }
        if let Some(dst) = &l.dst {
            sid_to_lines.entry(dst.sid.clone()).or_default().push(i);
        }
        fn collect_bsids(br: &crate::model::Branch, out: &mut Vec<String>) {
            if let Some(d) = &br.dst {
                out.push(d.sid.clone());
            }
            for s in &br.branches {
                collect_bsids(s, out);
            }
        }
        let mut bsids = Vec::new();
        for br in &l.branches {
            collect_bsids(br, &mut bsids);
        }
        for sid in bsids {
            sid_to_lines.entry(sid).or_default().push(i);
        }
    }
    for idxs in sid_to_lines.values() {
        for a in 0..idxs.len() {
            for b in (a + 1)..idxs.len() {
                let i = idxs[a];
                let j = idxs[b];
                if !adj[i].contains(&j) {
                    adj[i].push(j);
                }
                if !adj[j].contains(&i) {
                    adj[j].push(i);
                }
            }
        }
    }

    fn circular_dist(a: f32, b: f32) -> f32 {
        let d = (a - b).abs();
        d.min(1.0 - d)
    }
    fn hue_to_color(h: f32) -> Color32 {
        let h6 = (h * 6.0) % 6.0;
        let c = 0.95 * 0.85;
        let x = c * (1.0 - ((h6 % 2.0) - 1.0).abs());
        let (r1, g1, b1) = if h6 < 1.0 {
            (c, x, 0.0)
        } else if h6 < 2.0 {
            (x, c, 0.0)
        } else if h6 < 3.0 {
            (0.0, c, x)
        } else if h6 < 4.0 {
            (0.0, x, c)
        } else if h6 < 5.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };
        let m = 0.95 - c;
        Color32::from_rgb(
            ((r1 + m) * 255.0) as u8,
            ((g1 + m) * 255.0) as u8,
            ((b1 + m) * 255.0) as u8,
        )
    }

    let sample_count = (n * 8).max(64);
    let candidates: Vec<f32> = (0..sample_count)
        .map(|i| i as f32 / sample_count as f32)
        .collect();

    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| (-(adj[i].len() as isize), i as isize));

    let mut assigned: Vec<Option<f32>> = vec![None; n];
    let mut remaining = candidates.clone();
    for i in order {
        let neigh: Vec<f32> = adj[i].iter().filter_map(|&j| assigned[j]).collect();
        let mut best_h = 0.0;
        let mut best_score = -1.0f32;
        for &h in &remaining {
            let used = if neigh.is_empty() {
                assigned.iter().flatten().copied().collect()
            } else {
                neigh.clone()
            };
            let score = if used.is_empty() {
                1.0
            } else {
                used.iter()
                    .map(|&u| circular_dist(h, u))
                    .fold(1.0, f32::min)
            };
            if score > best_score || (score == best_score && h < best_h) {
                best_score = score;
                best_h = h;
            }
        }
        assigned[i] = Some(best_h);
        if let Some(pos) = remaining
            .iter()
            .position(|&x| (x - best_h).abs() < f32::EPSILON)
        {
            remaining.remove(pos);
        }
    }

    assigned
        .into_iter()
        .enumerate()
        .map(|(i, h)| {
            let default_h = i as f32 / n.max(1) as f32;
            hue_to_color(h.unwrap_or(default_h))
        })
        .collect()
}

// ────────────────────────────────────────────────────────────────────────────
// Port interaction areas (for initiating connection drag)
// ────────────────────────────────────────────────────────────────────────────

/// Draw invisible interaction areas over port chevrons to initiate connection dragging.
pub(super) fn draw_port_interaction_areas(
    ui: &mut egui::Ui,
    block: &crate::model::Block,
    r_screen: &Rect,
    font_scale: f32,
    _block_idx: usize,
    state: &mut EditorState,
) {
    let in_count = block.port_counts.as_ref().and_then(|p| p.ins).unwrap_or(0);
    let out_count = block.port_counts.as_ref().and_then(|p| p.outs).unwrap_or(0);
    let mirrored = block.block_mirror.unwrap_or(false);

    let (in_x, out_x) = if mirrored {
        (r_screen.right(), r_screen.left())
    } else {
        (r_screen.left(), r_screen.right())
    };

    let scale = font_scale.max(0.2);
    // Keep the clickable target comfortably larger than the (now smaller) visual
    // chevron without overlapping neighbouring blocks.
    let hit_size = (16.0 * scale).max(10.0);

    let sid = match &block.sid {
        Some(s) => s.clone(),
        None => return,
    };

    // Input ports — centre hit areas on the anchor positions so clicks/drags
    // land on the chevrons that the line endpoints actually use.
    for i in 0..in_count {
        let n = in_count.max(1);
        let y = r_screen.top() + r_screen.height() * ((i as f32 + 0.5) / (n as f32));
        let port_center = Pos2::new(in_x, y);
        let hit_rect = Rect::from_center_size(port_center, Vec2::splat(hit_size));
        let resp = ui.allocate_rect(hit_rect, Sense::click_and_drag());

        if resp.drag_started() {
            state.drag_mode = DragMode::Connection {
                src_sid: sid.clone(),
                src_port_type: "in".to_string(),
                src_port_index: i + 1,
                current_x: port_center.x,
                current_y: port_center.y,
            };
        }
    }

    // Output ports — same anchor formula as inputs.
    for i in 0..out_count {
        let n = out_count.max(1);
        let y = r_screen.top() + r_screen.height() * ((i as f32 + 0.5) / (n as f32));
        let port_center = Pos2::new(out_x, y);
        let hit_rect = Rect::from_center_size(port_center, Vec2::splat(hit_size));
        let resp = ui.allocate_rect(hit_rect, Sense::click_and_drag());

        if resp.drag_started() {
            state.drag_mode = DragMode::Connection {
                src_sid: sid.clone(),
                src_port_type: "out".to_string(),
                src_port_index: i + 1,
                current_x: port_center.x,
                current_y: port_center.y,
            };
        }
    }
}
