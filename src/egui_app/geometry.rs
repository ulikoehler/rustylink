#![cfg(feature = "egui")]

use crate::model::{Block, EndpointRef};
use eframe::egui::{Pos2, Rect};

/// Side of a block where a port resides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortSide {
    In,
    Out,
}

/// Parse the block rectangle from a Simulink block's `Position` property.
/// Expects a string of the form "[l, t, r, b]".
pub fn parse_block_rect(b: &Block) -> Option<Rect> {
    let pos = b.position.as_deref()?;
    parse_rect_str(pos)
}

/// Parse a rectangle string of the form "[l, t, r, b]" into an egui Rect
pub fn parse_rect_str(pos: &str) -> Option<Rect> {
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
/// Ports are distributed vertically.
pub fn port_anchor_pos(r: Rect, side: PortSide, port_index: u32, num_ports: Option<u32>) -> Pos2 {
    let idx1 = if port_index == 0 { 1 } else { port_index };
    let n = num_ports.unwrap_or(idx1).max(idx1);
    let total_segments = n * 2 + 1;
    let y0 = r.top();
    let y1 = r.bottom();
    let dy = (y1 - y0) / (total_segments as f32);
    let y = y0 + ((2 * idx1) as f32 - 0.5) * dy;
    match side {
        PortSide::Out => Pos2::new(r.right(), y),
        PortSide::In => Pos2::new(r.left(), y),
    }
}

/// Helper to compute a port anchor position given an endpoint reference.
pub fn endpoint_pos(r: Rect, ep: &EndpointRef, num_ports: Option<u32>) -> Pos2 {
    let side = if ep.port_type == "out" {
        PortSide::Out
    } else {
        PortSide::In
    };
    port_anchor_pos(r, side, ep.port_index, num_ports)
}

/// Variant that tries to match a target Y (e.g., last polyline Y) to keep the final segment horizontal
pub fn endpoint_pos_with_target(
    r: Rect,
    ep: &EndpointRef,
    num_ports: Option<u32>,
    target_y: Option<f32>,
) -> Pos2 {
    let mut p = endpoint_pos(r, ep, num_ports);
    if let Some(ty) = target_y {
        let mut y = ty;
        y = y.max(r.top()).min(r.bottom());
        p.y = y;
    }
    p
}

/// Determine the port side on screen for a given endpoint type, considering mirroring.
pub fn port_side_for(port_type: &str, mirrored: bool) -> PortSide {
    match (port_type, mirrored) {
        ("out", false) | ("in", true) => PortSide::Out,
        ("in", false) | ("out", true) => PortSide::In,
        (_other, _m) => PortSide::In,
    }
}

/// Compute endpoint position considering BlockMirror (inputs on right, outputs on left when true).
pub fn endpoint_pos_maybe_mirrored(
    r: Rect,
    ep: &EndpointRef,
    num_ports: Option<u32>,
    mirrored: bool,
) -> Pos2 {
    let side = port_side_for(&ep.port_type, mirrored);
    port_anchor_pos(r, side, ep.port_index, num_ports)
}

/// Variant with target Y matching, considering mirroring.
pub fn endpoint_pos_with_target_maybe_mirrored(
    r: Rect,
    ep: &EndpointRef,
    num_ports: Option<u32>,
    target_y: Option<f32>,
    mirrored: bool,
) -> Pos2 {
    let mut p = endpoint_pos_maybe_mirrored(r, ep, num_ports, mirrored);
    if let Some(ty) = target_y {
        let mut y = ty;
        let top = r.top();
        let bottom = r.bottom();
        if y < top {
            y = top;
        }
        if y > bottom {
            y = bottom;
        }
        p.y = y;
    }
    p
}

/// Compute the positions of port indicators to draw for a block.
///
/// These indicators are purely visual (useful even when the model has no
/// connected lines) and are derived from the block's port counts.
pub fn port_indicator_positions(
    r: Rect,
    in_count: u32,
    out_count: u32,
    mirrored: bool,
) -> (Vec<Pos2>, Vec<Pos2>) {
    port_indicator_positions_with_overrides(r, in_count, out_count, mirrored, &[])
}

/// Like [`port_indicator_positions`], but honours [`PortPositionOverride`] entries.
///
/// Ports that have an override are placed on the specified side at the given
/// fraction.  Ports without an override use the standard evenly-distributed
/// layout.
pub fn port_indicator_positions_with_overrides(
    r: Rect,
    in_count: u32,
    out_count: u32,
    mirrored: bool,
    overrides: &[crate::builtin_libraries::virtual_library::PortPositionOverride],
) -> (Vec<Pos2>, Vec<Pos2>) {
    let (in_side, out_side) = if mirrored {
        (PortSide::Out, PortSide::In)
    } else {
        (PortSide::In, PortSide::Out)
    };

    let mut ins = Vec::new();
    for i in 1..=in_count {
        if let Some(ovr) = overrides.iter().find(|o| o.is_input && o.port_index == i) {
            ins.push(placement_pos(r, ovr.placement, ovr.fraction));
        } else {
            ins.push(port_anchor_pos(r, in_side, i, Some(in_count.max(1))));
        }
    }
    let mut outs = Vec::new();
    for i in 1..=out_count {
        if let Some(ovr) = overrides.iter().find(|o| !o.is_input && o.port_index == i) {
            outs.push(placement_pos(r, ovr.placement, ovr.fraction));
        } else {
            outs.push(port_anchor_pos(r, out_side, i, Some(out_count.max(1))));
        }
    }
    (ins, outs)
}

/// Convert a [`PortPlacement`] + fraction to a concrete position on a block rect.
fn placement_pos(
    r: Rect,
    placement: crate::builtin_libraries::virtual_library::PortPlacement,
    fraction: f32,
) -> Pos2 {
    use crate::builtin_libraries::virtual_library::PortPlacement;
    let f = fraction.clamp(0.0, 1.0);
    match placement {
        PortPlacement::Left => Pos2::new(r.left(), r.top() + f * r.height()),
        PortPlacement::Right => Pos2::new(r.right(), r.top() + f * r.height()),
        PortPlacement::Top => Pos2::new(r.left() + f * r.width(), r.top()),
        PortPlacement::Bottom => Pos2::new(r.left() + f * r.width(), r.bottom()),
    }
}

/// Determine the chevron direction for an overridden port.
///
/// Returns `true` when the chevron tip should point **into** the block
/// (i.e. the port is on the same side as the block's left edge when not
/// mirrored).  The caller can use this to mirror the chevron shape.
///
/// For standard Left/Right placements the result is the same as the
/// `is_left_side` flag used for the default layout.  For Top/Bottom
/// overrides the value is always `true` (the chevron faces inward).
pub fn port_override_is_left_side(
    placement: crate::builtin_libraries::virtual_library::PortPlacement,
    _mirrored: bool,
) -> bool {
    use crate::builtin_libraries::virtual_library::PortPlacement;
    match placement {
        PortPlacement::Left => true,
        PortPlacement::Right => false,
        PortPlacement::Top | PortPlacement::Bottom => true,
    }
}

// tests moved to tests/ module
