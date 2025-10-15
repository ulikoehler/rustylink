#![cfg(feature = "egui")]

use crate::block_types::{self, BlockTypeConfig, IconSpec, Rgb};
use crate::model::Block;
use eframe::egui::{self, Align2, Color32, Pos2, Rect, Stroke, Vec2};

pub(crate) fn rgb_to_color32(c: Rgb) -> Color32 {
    Color32::from_rgb(c.0, c.1, c.2)
}

pub(crate) fn get_block_type_cfg(block_type: &str) -> BlockTypeConfig {
    let map = block_types::get_block_type_config_map();
    if let Ok(g) = map.read() {
        g.get(block_type).cloned().unwrap_or_default()
    } else {
        BlockTypeConfig::default()
    }
}

/// Render an icon in the center of the block according to its type using the phosphor font.
///
/// The `font_scale` parameter scales the icon size relative to the baseline size
/// (baseline is 24.0 at 400% zoom; caller should pass zoom/4.0).
pub fn render_block_icon(painter: &egui::Painter, block: &Block, rect: &Rect, font_scale: f32) {
    let icon_size = 24.0 * font_scale.max(0.01);
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

/// Screen-space Y coordinates computed for a block's ports (as used by the UI when placing
/// port labels and clamped within the block rect). Keys are 1-based port indices.
#[derive(Clone, Debug, Default)]
pub struct ComputedPortYCoordinates {
    pub inputs: std::collections::HashMap<u32, f32>,
    pub outputs: std::collections::HashMap<u32, f32>,
}

/// Custom renderer for a ManualSwitch block.
///
/// Draws a simple switch symbol with two input poles (left) and one output pole (right).
/// The pole centers are aligned to the exact y-positions of the ports so that
/// connecting lines meet them cleanly. The lever connects from the selected input
/// (current_setting: "0" => bottom, "1" => top; default "0") to the output pole.
pub fn render_manual_switch(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    _font_scale: f32,
    coords: Option<&ComputedPortYCoordinates>,
) {
    // Determine how many ports to align (fall back to common defaults)
    let mut max_in: u32 = 0;
    let mut max_out: u32 = 0;
    for p in &block.ports {
        let idx = p.index.unwrap_or(0).max(1);
        if p.port_type == "in" { max_in = max_in.max(idx); }
        if p.port_type == "out" { max_out = max_out.max(idx); }
    }
    if max_in == 0 { max_in = 2; }
    if max_out == 0 { max_out = 1; }

    // Compute port anchors (in screen space)
    use super::geometry::PortSide;
    let default_in1 = super::geometry::port_anchor_pos(*rect, PortSide::In, 1, Some(max_in));
    let default_in2 = super::geometry::port_anchor_pos(*rect, PortSide::In, 2, Some(max_in));
    let default_out = super::geometry::port_anchor_pos(*rect, PortSide::Out, 1, Some(max_out));

    // Place pole centers slightly inside the block border so the circles are fully visible
    let pad = 8.0_f32; // horizontal inset from the border for the circle centers
    let r_in = (rect.height() * 0.06).clamp(2.0, 6.0) * 0.8; // 20% smaller
    let r_out = r_in;
    let stroke_w = 1.5_f32; // thinner
    let col_active = Color32::from_rgb(32, 32, 32);
    let col_inactive = Color32::from_rgb(110, 110, 110); // dark gray for inactive

    let top_in_y = coords.and_then(|c| c.inputs.get(&1).copied()).unwrap_or(default_in1.y);
    let bot_in_y = coords.and_then(|c| c.inputs.get(&2).copied()).unwrap_or(default_in2.y);
    let out_y = coords.and_then(|c| c.outputs.get(&1).copied()).unwrap_or(default_out.y);

    let top_in_center = Pos2::new(rect.left() + pad, top_in_y);
    let bot_in_center = Pos2::new(rect.left() + pad, bot_in_y);
    let out_center = Pos2::new(rect.right() - pad, out_y);

    // Horizontal leads from border to the pole circles up to circle edge
    let in1_anchor = Pos2::new(rect.left(), top_in_y);
    let in2_anchor = Pos2::new(rect.left(), bot_in_y);
    let out_anchor = Pos2::new(rect.right(), out_y);
    painter.line_segment([in1_anchor, Pos2::new(top_in_center.x - r_in, top_in_center.y)], Stroke::new(stroke_w, col_active));
    painter.line_segment([in2_anchor, Pos2::new(bot_in_center.x - r_in, bot_in_center.y)], Stroke::new(stroke_w, col_active));
    painter.line_segment([Pos2::new(out_center.x + r_out, out_center.y), out_anchor], Stroke::new(stroke_w, col_active));

    // Draw open-circuit poles
    painter.circle_stroke(top_in_center, r_in, Stroke::new(stroke_w, col_active));
    painter.circle_stroke(bot_in_center, r_in, Stroke::new(stroke_w, col_active));
    painter.circle_stroke(out_center, r_out, Stroke::new(stroke_w, col_active));

    // Small stubs from circle edge inwards (approx. 0.3x diameter = 0.6r)
    let stub = (0.6 * r_in).max(0.8);
    let set_top = matches!(block.current_setting.as_deref(), Some("1"));
    // Input stubs drawn inside towards center: [edge - stub, edge]
    let in1_color = if set_top { col_active } else { col_inactive };
    let in2_color = if set_top { col_inactive } else { col_active };
    let in1_edge = top_in_center.x + r_in;
    let in2_edge = bot_in_center.x + r_in;
    painter.line_segment([Pos2::new(in1_edge - stub, top_in_center.y), Pos2::new(in1_edge, top_in_center.y)], Stroke::new(stroke_w, in1_color));
    painter.line_segment([Pos2::new(in2_edge - stub, bot_in_center.y), Pos2::new(in2_edge, bot_in_center.y)], Stroke::new(stroke_w, in2_color));
    // Output stub inside from circle edge to center
    let out_edge_left = out_center.x - r_out;
    painter.line_segment([Pos2::new(out_edge_left, out_center.y), Pos2::new(out_edge_left + stub, out_center.y)], Stroke::new(stroke_w, col_active));

    // Lever connecting selected input to output (from circle centers)
    let from = if set_top { top_in_center } else { bot_in_center };
    // Start lever at the circle perimeter towards the output side to avoid overlapping the circle stroke
    let dir = Vec2::new(out_center.x - from.x, out_center.y - from.y);
    let len = (dir.x * dir.x + dir.y * dir.y).sqrt().max(1e-3);
    let ux = dir.x / len; let uy = dir.y / len;
    let start = Pos2::new(from.x + ux * r_in, from.y + uy * r_in);
    let end = Pos2::new(out_center.x - ux * r_out, out_center.y - uy * r_out);
    painter.line_segment([start, end], Stroke::new(stroke_w, col_active));
}

// no re-exports; keep this module focused on rendering helpers
