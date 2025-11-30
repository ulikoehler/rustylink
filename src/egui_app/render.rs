#![cfg(feature = "egui")]

use crate::block_types::{self, BlockTypeConfig, Rgb};
use crate::model::Block;
use eframe::egui::{self, Align2, Color32, Pos2, Rect, Stroke};

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

/// Render an icon in the center of the block according to its type.
///
/// The `font_scale` parameter scales the icon size relative to the baseline size
/// (baseline is 24.0 at 400% zoom; caller should pass zoom/4.0).
pub fn render_block_icon(painter: &egui::Painter, block: &Block, rect: &Rect, font_scale: f32) {
    let icon_size = 24.0 * font_scale.max(0.01);
    let icon_center = rect.center();
    // Use default egui font (proportional) for UTF-8 icons
    let font = egui::FontId::proportional(icon_size);
    let dark_icon = Color32::from_rgb(40, 40, 40); // dark color for icons
    // Lookup icon from centralized registry
    let cfg = get_block_type_cfg(&block.block_type);
    if let Some(icon) = cfg.icon {
        match icon {
            block_types::IconSpec::Utf8(glyph) => {
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
    let mirrored = block.block_mirror.unwrap_or(false);
    let in_side = if mirrored { PortSide::Out } else { PortSide::In };
    let out_side = if mirrored { PortSide::In } else { PortSide::Out };
    let default_in1 = super::geometry::port_anchor_pos(*rect, in_side, 1, Some(max_in));
    let default_in2 = super::geometry::port_anchor_pos(*rect, in_side, 2, Some(max_in));
    let default_out = super::geometry::port_anchor_pos(*rect, out_side, 1, Some(max_out));

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

    let (top_in_center, bot_in_center, out_center) = if !mirrored {
        (Pos2::new(rect.left() + pad, top_in_y), Pos2::new(rect.left() + pad, bot_in_y), Pos2::new(rect.right() - pad, out_y))
    } else {
        (Pos2::new(rect.right() - pad, top_in_y), Pos2::new(rect.right() - pad, bot_in_y), Pos2::new(rect.left() + pad, out_y))
    };

    // Horizontal leads from border to the pole circles up to circle edge
    if !mirrored {
        let in1_anchor = Pos2::new(rect.left(), top_in_y);
        let in2_anchor = Pos2::new(rect.left(), bot_in_y);
        let out_anchor = Pos2::new(rect.right(), out_y);
        painter.line_segment([in1_anchor, Pos2::new(top_in_center.x - r_in, top_in_center.y)], Stroke::new(stroke_w, col_active));
        painter.line_segment([in2_anchor, Pos2::new(bot_in_center.x - r_in, bot_in_center.y)], Stroke::new(stroke_w, col_active));
        painter.line_segment([Pos2::new(out_center.x + r_out, out_center.y), out_anchor], Stroke::new(stroke_w, col_active));
    } else {
        let in1_anchor = Pos2::new(rect.right(), top_in_y);
        let in2_anchor = Pos2::new(rect.right(), bot_in_y);
        let out_anchor = Pos2::new(rect.left(), out_y);
        painter.line_segment([in1_anchor, Pos2::new(top_in_center.x + r_in, top_in_center.y)], Stroke::new(stroke_w, col_active));
        painter.line_segment([in2_anchor, Pos2::new(bot_in_center.x + r_in, bot_in_center.y)], Stroke::new(stroke_w, col_active));
        painter.line_segment([Pos2::new(out_center.x - r_out, out_center.y), out_anchor], Stroke::new(stroke_w, col_active));
    }

    // Draw open-circuit poles
    let set_top = matches!(block.current_setting.as_deref(), Some("1"));
    let top_col = if set_top { col_active } else { col_inactive };
    let bot_col = if set_top { col_inactive } else { col_active };
    painter.circle_stroke(top_in_center, r_in, Stroke::new(stroke_w, top_col));
    painter.circle_stroke(bot_in_center, r_in, Stroke::new(stroke_w, bot_col));
    painter.circle_stroke(out_center, r_out, Stroke::new(stroke_w, col_active));

    // Small stubs from the circle edge OUTSIDE the circle (1/3 of the circle diameter)
    let stub = (2.0 * r_in / 3.0).max(0.8); // 1/3 diameter
    // Input stubs extend inside the block: to the right for left-side inputs, to the left for right-side inputs.
    let in1_color = top_col;
    let in2_color = bot_col;
    if !mirrored {
        let in1_edge = top_in_center.x + r_in; // rightmost point of top input circle
        let in2_edge = bot_in_center.x + r_in; // rightmost point of bottom input circle
        painter.line_segment([Pos2::new(in1_edge, top_in_center.y), Pos2::new(in1_edge + stub, top_in_center.y)], Stroke::new(stroke_w, in1_color));
        painter.line_segment([Pos2::new(in2_edge, bot_in_center.y), Pos2::new(in2_edge + stub, bot_in_center.y)], Stroke::new(stroke_w, in2_color));
        // Output stub extends to the LEFT of the output circle: [edge - stub, edge]
        let out_edge_left = out_center.x - r_out; // leftmost point of output circle
        painter.line_segment([Pos2::new(out_edge_left - stub, out_center.y), Pos2::new(out_edge_left, out_center.y)], Stroke::new(stroke_w, col_active));
        // Lever connects from active input stub end to output stub end
        let from_edge = in1_edge; let from_edge2 = in2_edge;
        let from_y_top = top_in_center.y; let from_y_bot = bot_in_center.y;
        let from_edge_sel = if set_top { from_edge } else { from_edge2 };
        let from_y_sel = if set_top { from_y_top } else { from_y_bot };
        let start = Pos2::new(from_edge_sel + stub, from_y_sel);
        let end = Pos2::new(out_edge_left - stub, out_center.y);
        painter.line_segment([start, end], Stroke::new(stroke_w, col_active));
    } else {
        let in1_edge = top_in_center.x - r_in; // leftmost point of top input circle (inputs on right)
        let in2_edge = bot_in_center.x - r_in; // leftmost point of bottom input circle
        painter.line_segment([Pos2::new(in1_edge, top_in_center.y), Pos2::new(in1_edge - stub, top_in_center.y)], Stroke::new(stroke_w, in1_color));
        painter.line_segment([Pos2::new(in2_edge, bot_in_center.y), Pos2::new(in2_edge - stub, bot_in_center.y)], Stroke::new(stroke_w, in2_color));
        // Output stub extends to the RIGHT of the output circle (output on left): [edge, edge + stub]
        let out_edge_right = out_center.x + r_out; // rightmost point of output circle
        painter.line_segment([Pos2::new(out_edge_right, out_center.y), Pos2::new(out_edge_right + stub, out_center.y)], Stroke::new(stroke_w, col_active));
        // Lever
        let from_edge = in1_edge; let from_edge2 = in2_edge;
        let from_y_top = top_in_center.y; let from_y_bot = bot_in_center.y;
        let from_edge_sel = if set_top { from_edge } else { from_edge2 };
        let from_y_sel = if set_top { from_y_top } else { from_y_bot };
        let start = Pos2::new(from_edge_sel - stub, from_y_sel);
        let end = Pos2::new(out_edge_right + stub, out_center.y);
        painter.line_segment([start, end], Stroke::new(stroke_w, col_active));
    }
}

// no re-exports; keep this module focused on rendering helpers
