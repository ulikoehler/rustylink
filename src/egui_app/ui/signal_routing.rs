//! Pure-function signal routing utilities.
//!
//! These functions handle orthogonal polyline expansion, signal point
//! manipulation, and branch-tree traversal — all without any egui dependency
//! beyond basic types (`Pos2`).  This makes them easy to unit-test.

use eframe::egui::Pos2;

// ---------------------------------------------------------------------------
// Orthogonal polyline helpers
// ---------------------------------------------------------------------------

/// Insert corner points so that every segment in the polyline is either
/// horizontal or vertical (orthogonal routing, horizontal-first).
pub fn orthogonalize_polyline(points: &[Pos2]) -> Vec<Pos2> {
    if points.len() <= 1 {
        return points.to_vec();
    }
    let mut out = vec![points[0]];
    for pair in points.windows(2) {
        let a = pair[0];
        let b = pair[1];
        if (a.x - b.x).abs() > f32::EPSILON && (a.y - b.y).abs() > f32::EPSILON {
            // Diagonal segment — insert a corner (horizontal first)
            let corner = Pos2::new(b.x, a.y);
            if out.last().copied() != Some(corner) {
                out.push(corner);
            }
        }
        if out.last().copied() != Some(b) {
            out.push(b);
        }
    }
    out
}

/// Convert an orthogonalized polyline into `(start, end)` segment pairs.
pub fn push_orthogonal_segments(points: &[Pos2], out: &mut Vec<(Pos2, Pos2)>) {
    let ortho = orthogonalize_polyline(points);
    for seg in ortho.windows(2) {
        out.push((seg[0], seg[1]));
    }
}

// ---------------------------------------------------------------------------
// Signal point manipulation (used when dragging corners/branches)
// ---------------------------------------------------------------------------

/// Move a single point in a line's point list by `(dx, dy)`, compensating the
/// *next* point so that all downstream geometry is unchanged.
pub fn move_line_point(line: &mut crate::model::Line, point_index: usize, dx: i32, dy: i32) {
    if let Some(point) = line.points.get_mut(point_index) {
        point.x += dx;
        point.y += dy;
    }
    // Compensate the next point so the endpoint after the moved one stays put.
    if let Some(next) = line.points.get_mut(point_index + 1) {
        next.x -= dx;
        next.y -= dy;
    }
}

/// Move a single point in a branch's point list, compensating the next point.
pub fn move_branch_point(branch: &mut crate::model::Branch, point_index: usize, dx: i32, dy: i32) {
    if let Some(point) = branch.points.get_mut(point_index) {
        point.x += dx;
        point.y += dy;
    }
    if let Some(next) = branch.points.get_mut(point_index + 1) {
        next.x -= dx;
        next.y -= dy;
    }
}

/// Shift *all* points in a line (and its entire branch tree) by `(dx, dy)`.
pub fn move_line_layout(line: &mut crate::model::Line, dx: i32, dy: i32) {
    for point in &mut line.points {
        point.x += dx;
        point.y += dy;
    }
    move_branch_layouts(&mut line.branches, dx, dy);
}

/// Recursively shift all points in a branch slice by `(dx, dy)`.
pub fn move_branch_layouts(branches: &mut [crate::model::Branch], dx: i32, dy: i32) {
    for branch in branches {
        for point in &mut branch.points {
            point.x += dx;
            point.y += dy;
        }
        move_branch_layouts(&mut branch.branches, dx, dy);
    }
}

// ---------------------------------------------------------------------------
// Branch tree traversal
// ---------------------------------------------------------------------------

/// Collect the screen-space positions of all branch corner points for interactive
/// handle rendering.  Each entry is `(branch_path, point_index, screen_pos)`.
pub fn collect_branch_handle_positions(
    start: Pos2,
    branches: &[crate::model::Branch],
    to_screen: &dyn Fn(Pos2) -> Pos2,
    path_prefix: &mut Vec<usize>,
    out: &mut Vec<(Vec<usize>, usize, Pos2)>,
) {
    for (branch_index, branch) in branches.iter().enumerate() {
        path_prefix.push(branch_index);
        let mut cur = start;
        for (point_index, point) in branch.points.iter().enumerate() {
            cur = Pos2::new(cur.x + point.x as f32, cur.y + point.y as f32);
            out.push((path_prefix.clone(), point_index, to_screen(cur)));
        }
        collect_branch_handle_positions(cur, &branch.branches, to_screen, path_prefix, out);
        path_prefix.pop();
    }
}

/// Navigate the branch tree by an index path and return a mutable reference to
/// the target branch.
pub fn get_branch_mut<'a>(
    branches: &'a mut [crate::model::Branch],
    path: &[usize],
) -> Option<&'a mut crate::model::Branch> {
    let (first, rest) = path.split_first()?;
    let branch = branches.get_mut(*first)?;
    if rest.is_empty() {
        Some(branch)
    } else {
        get_branch_mut(&mut branch.branches, rest)
    }
}

// ---------------------------------------------------------------------------
// Port-count accumulation
// ---------------------------------------------------------------------------

/// The port-count bucket an endpoint belongs to: input side, output side, or
/// the control ports on the top edge.  Control ports are counted separately so
/// an `enable:1` endpoint is not mistaken for data input 1.
pub fn port_kind(port_type: &str) -> u8 {
    match port_type {
        "out" => 1,
        other if crate::egui_app::geometry::is_control_port_type(other) => 2,
        _ => 0,
    }
}

/// Bucket holding the top-edge *slot* of a control port type rather than a
/// port count; no endpoint maps to it, so it cannot collide with the counts.
const CONTROL_SLOT_KIND: u8 = 3;

fn control_slot_key(sid: &str, port_type: &str) -> (String, u8) {
    (
        format!("{sid}\u{1}{}", port_type.to_ascii_lowercase()),
        CONTROL_SLOT_KIND,
    )
}

/// Where an endpoint attaches to its block.
///
/// Control ports are numbered per type (`enable:1` *and* `trigger:1`), so their
/// index says nothing about where they sit on the top edge; the slot recorded
/// by [`compute_port_info`] does.
pub fn endpoint_pos(
    rect: eframe::egui::Rect,
    ep: &crate::model::EndpointRef,
    port_counts: &std::collections::HashMap<(String, u8), u32>,
    mirrored: bool,
) -> Pos2 {
    let kind = port_kind(&ep.port_type);
    let num_ports = port_counts.get(&(ep.sid.clone(), kind)).copied();
    let index = if kind == 2 {
        port_counts
            .get(&control_slot_key(&ep.sid, &ep.port_type))
            .copied()
            .map(|slot| slot + ep.port_index.saturating_sub(1))
            .unwrap_or(ep.port_index)
    } else {
        ep.port_index
    };
    crate::egui_app::geometry::port_anchor_pos(
        rect,
        crate::egui_app::geometry::port_side_for(&ep.port_type, mirrored),
        index,
        num_ports,
    )
}

/// Register an endpoint's port in the port-count and connected-ports maps.
pub fn register_endpoint(
    ep: &crate::model::EndpointRef,
    port_counts: &mut std::collections::HashMap<(String, u8), u32>,
    connected_ports: &mut std::collections::HashSet<(String, u32, bool)>,
) {
    let kind = port_kind(&ep.port_type);
    let key = (ep.sid.clone(), kind);
    let idx1 = if ep.port_index == 0 { 1 } else { ep.port_index };
    port_counts
        .entry(key)
        .and_modify(|v| *v = (*v).max(idx1))
        .or_insert(idx1);
    if kind != 2 {
        connected_ports.insert((ep.sid.clone(), ep.port_index, kind == 0));
    }
}

/// Recursively register branch endpoint ports.
pub fn register_branch_endpoints(
    branch: &crate::model::Branch,
    port_counts: &mut std::collections::HashMap<(String, u8), u32>,
    connected_ports: &mut std::collections::HashSet<(String, u32, bool)>,
) {
    if let Some(dst) = &branch.dst {
        register_endpoint(dst, port_counts, connected_ports);
    }
    for sub in &branch.branches {
        register_branch_endpoints(sub, port_counts, connected_ports);
    }
}

/// Compute port counts and connected ports from a set of lines.
#[allow(clippy::type_complexity)]
pub fn compute_port_info(
    lines: &[crate::model::Line],
    blocks: &[crate::model::Block],
) -> (
    std::collections::HashMap<(String, u8), u32>,
    std::collections::HashSet<(String, u32, bool)>,
) {
    let mut port_counts: std::collections::HashMap<(String, u8), u32> =
        std::collections::HashMap::new();
    let mut connected_ports: std::collections::HashSet<(String, u32, bool)> =
        std::collections::HashSet::new();

    for line in lines {
        if let Some(src) = &line.src {
            register_endpoint(src, &mut port_counts, &mut connected_ports);
        }
        if let Some(dst) = &line.dst {
            register_endpoint(dst, &mut port_counts, &mut connected_ports);
        }
        for br in &line.branches {
            register_branch_endpoints(br, &mut port_counts, &mut connected_ports);
        }
    }

    // Pre-populate from block declarations so line and chevron positioning
    // use consistent total port counts.
    for b in blocks {
        if let Some(sid) = &b.sid
            && let Some(pc) = &b.port_counts
        {
            if let Some(ins) = pc.ins {
                let key = (sid.clone(), 0u8);
                port_counts
                    .entry(key)
                    .and_modify(|v| *v = (*v).max(ins))
                    .or_insert(ins);
            }
            if let Some(outs) = pc.outs {
                let key = (sid.clone(), 1u8);
                port_counts
                    .entry(key)
                    .and_modify(|v| *v = (*v).max(outs))
                    .or_insert(outs);
            }
        }

        let Some(sid) = &b.sid else { continue };
        let control_types = crate::simulink_libraries::renderers::subsystem_control_port_types(b);
        if control_types.is_empty() {
            continue;
        }
        let controls = control_types.len() as u32;
        port_counts
            .entry((sid.clone(), 2u8))
            .and_modify(|v| *v = (*v).max(controls))
            .or_insert(controls);
        for (slot, port_type) in control_types.into_iter().enumerate() {
            port_counts
                .entry(control_slot_key(sid, port_type))
                .or_insert(slot as u32 + 1);
        }
    }

    (port_counts, connected_ports)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
