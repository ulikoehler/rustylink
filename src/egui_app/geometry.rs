#![cfg(feature = "egui")]

use eframe::egui::{Pos2, Rect};
use crate::model::{Block, EndpointRef};

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

/// Helper to compute a port anchor position given an endpoint reference.
pub fn endpoint_pos(r: Rect, ep: &EndpointRef, num_ports: Option<u32>) -> Pos2 {
    port_anchor_pos(r, ep.port_type.as_str(), ep.port_index, num_ports)
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

// tests moved to tests/ module
