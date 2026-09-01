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
    orthogonalize_polyline_with_dst_side(points, None)
}

/// Like [`orthogonalize_polyline`], but lets the caller choose the corner
/// order for the **last** diagonal segment so the final segment meets the
/// destination block edge orthogonally.
///
/// `last_segment_horizontal`:
/// - `None` → always horizontal-first (same as [`orthogonalize_polyline`]).
/// - `Some(true)` → the last diagonal inserts a "vertical first" corner
///   (`Pos2::new(a.x, b.y)`) so the final segment is horizontal — appropriate
///   for ports on the left or right edge.
/// - `Some(false)` → the last diagonal inserts a "horizontal first" corner
///   so the final segment is vertical — appropriate for ports on the top or
///   bottom edge.
pub fn orthogonalize_polyline_with_dst_side(
    points: &[Pos2],
    last_segment_horizontal: Option<bool>,
) -> Vec<Pos2> {
    if points.len() <= 1 {
        return points.to_vec();
    }
    let n = points.len();
    let mut out = vec![points[0]];
    for (i, pair) in points.windows(2).enumerate() {
        let a = pair[0];
        let b = pair[1];
        if (a.x - b.x).abs() > f32::EPSILON && (a.y - b.y).abs() > f32::EPSILON {
            // Diagonal segment — insert a corner.
            // Default: horizontal first (corner at (b.x, a.y)).
            // For the last segment, choose based on the destination port side.
            let is_last = i == n - 2;
            let corner = if is_last && last_segment_horizontal == Some(true) {
                Pos2::new(a.x, b.y) // vertical first → last segment horizontal
            } else {
                Pos2::new(b.x, a.y) // horizontal first → last segment vertical
            };
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

/// Determine whether the last segment of a line approaching endpoint `ep`
/// should be horizontal (i.e. the port is on the left or right edge of the
/// block).  Returns `Some(true)` for left/right-edge ports, `Some(false)` for
/// top/bottom-edge ports, and `None` if unknown.
pub fn dst_segment_horizontal(
    ep: &crate::model::EndpointRef,
    overrides: &[crate::simulink_libraries::types::PortPositionOverride],
    mirrored: bool,
    port_counts: &std::collections::HashMap<(String, u8), u32>,
) -> Option<bool> {
    use crate::egui_app::geometry::is_control_port_type;
    use crate::simulink_libraries::types::PortPlacement;

    // Control ports (enable/trigger/reset/event) enter from the top edge →
    // the last segment should be vertical.
    if is_control_port_type(&ep.port_type) {
        return Some(false);
    }

    // Check for a port position override (e.g. round Sum bottom port).
    let kind = port_kind(&ep.port_type);
    if kind == 0 || kind == 1 {
        let is_input = kind == 0;
        let count = port_counts
            .get(&(ep.sid.clone(), kind))
            .copied()
            .unwrap_or(ep.port_index);
        if let Some(ovr) = overrides.iter().find(|o| o.matches(is_input, ep.port_index, count)) {
            return match ovr.placement {
                PortPlacement::Left | PortPlacement::Right => Some(true),
                PortPlacement::Top | PortPlacement::Bottom => Some(false),
            };
        }
    }

    // Standard placement: inputs on left (or right if mirrored), outputs on
    // right (or left if mirrored).  Both are horizontal sides.
    let _ = mirrored;
    Some(true)
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

/// Bucket holding how many lifecycle event ports a subsystem carries on its
/// input side.  They occupy the first slots there, so the data inputs of such
/// a block are pushed down by this many.
const EVENT_IN_KIND: u8 = 4;

/// Bucket flagging a subsystem with `ShowSubsystemReinitializePorts = on`.
/// Such a block draws the reinit port in its own top section, a separator
/// line, and the data inputs below – so the data inputs are *not* offset by
/// the event count.
const REINIT_FLAG_KIND: u8 = 5;

/// Vertical fraction (from the top) at which the reinit port sits.
pub const REINIT_PORT_FRAC: f32 = 0.12;

/// Vertical fraction (from the top) at which the separator line is drawn;
/// data inputs are distributed in the region below it.
pub const REINIT_SEP_FRAC: f32 = 0.25;

/// How many event ports precede the data inputs of the block `sid`.
pub fn event_input_offset(
    port_counts: &std::collections::HashMap<(String, u8), u32>,
    sid: &str,
) -> u32 {
    port_counts
        .get(&(sid.to_string(), EVENT_IN_KIND))
        .copied()
        .unwrap_or(0)
}

/// Whether `sid` is a reinit subsystem (`ShowSubsystemReinitializePorts = on`).
pub fn is_reinit_subsystem_counts(
    port_counts: &std::collections::HashMap<(String, u8), u32>,
    sid: &str,
) -> bool {
    port_counts
        .get(&(sid.to_string(), REINIT_FLAG_KIND))
        .copied()
        .unwrap_or(0)
        > 0
}

fn control_slot_key(sid: &str, port_type: &str) -> (String, u8) {
    (
        format!("{sid}\u{1}{}", port_type.to_ascii_lowercase()),
        CONTROL_SLOT_KIND,
    )
}

/// Position of a data input on a reinit subsystem: distributed evenly in the
/// region below the separator line (`REINIT_SEP_FRAC` .. 1.0).
pub fn reinit_data_input_pos(
    rect: eframe::egui::Rect,
    side: crate::egui_app::geometry::PortSide,
    port_index: u32,
    num_ports: u32,
) -> Pos2 {
    let idx1 = if port_index == 0 { 1 } else { port_index };
    let n = num_ports.max(idx1);
    let top = rect.top() + REINIT_SEP_FRAC * rect.height();
    let bottom = rect.bottom();
    // Cell-centered distribution matching Simulink: (i - 0.5) / n.
    let frac = (idx1 as f32 - 0.5) / (n as f32);
    let y = top + frac * (bottom - top);
    match side {
        crate::egui_app::geometry::PortSide::Out => Pos2::new(rect.right(), y),
        _ => Pos2::new(rect.left(), y),
    }
}

/// Where an endpoint attaches to its block.
///
/// Control ports are numbered per type (`enable:1` *and* `trigger:1`), so their
/// index says nothing about where they sit on the top edge; the slot recorded
/// by [`compute_port_info`] does.
///
/// `overrides` carries the block's [`PortPositionOverride`] entries (e.g. a
/// round Sum wraps its last input onto the bottom edge).  When an override
/// matches the endpoint's side and (adjusted) index, its placement is used
/// instead of the standard evenly-distributed anchor.
pub fn endpoint_pos(
    rect: eframe::egui::Rect,
    ep: &crate::model::EndpointRef,
    port_counts: &std::collections::HashMap<(String, u8), u32>,
    mirrored: bool,
    overrides: &[crate::simulink_libraries::types::PortPositionOverride],
) -> Pos2 {
    let is_event = ep.port_type.eq_ignore_ascii_case("event");
    let reinit = is_reinit_subsystem_counts(port_counts, &ep.sid);
    let side = crate::egui_app::geometry::port_side_for(&ep.port_type, mirrored);

    // Reinit subsystems: the reinit port sits at a fixed fraction, and data
    // inputs are distributed below the separator line (no event offset).
    if reinit {
        if is_event {
            let y = rect.top() + REINIT_PORT_FRAC * rect.height();
            return match side {
                crate::egui_app::geometry::PortSide::Out => Pos2::new(rect.right(), y),
                _ => Pos2::new(rect.left(), y),
            };
        }
        let kind = port_kind(&ep.port_type);
        if kind == 0 {
            let num_ports = port_counts.get(&(ep.sid.clone(), 0u8)).copied();
            return reinit_data_input_pos(rect, side, ep.port_index, num_ports.unwrap_or(1));
        }
        // Outputs and control ports use the standard positioning.
    }

    // An event port sits on the input side, so it shares the input count.
    let count_kind = if is_event {
        0
    } else {
        port_kind(&ep.port_type)
    };
    let kind = port_kind(&ep.port_type);
    let num_ports = port_counts.get(&(ep.sid.clone(), count_kind)).copied();
    let index = if is_event {
        ep.port_index
    } else if kind == 2 {
        port_counts
            .get(&control_slot_key(&ep.sid, &ep.port_type))
            .copied()
            .map(|slot| slot + ep.port_index.saturating_sub(1))
            .unwrap_or(ep.port_index)
    } else if kind == 0 {
        ep.port_index + event_input_offset(port_counts, &ep.sid)
    } else {
        ep.port_index
    };

    // Apply a matching port-position override (e.g. round Sum's last input on
    // the bottom edge).  Only data inputs (kind 0) and data outputs (kind 1)
    // are eligible; control ports (kind 2) and events keep their standard
    // placement.
    if kind == 0 || kind == 1 {
        let is_input = kind == 0;
        let count = num_ports.unwrap_or(index.max(1));
        if let Some(ovr) = overrides.iter().find(|o| o.matches(is_input, index, count)) {
            return crate::egui_app::geometry::placement_pos(rect, ovr.placement, ovr.fraction);
        }
    }

    crate::egui_app::geometry::port_anchor_pos(rect, side, index, num_ports)
}

/// Register an endpoint's port in the port-count and connected-ports maps.
pub fn register_endpoint(
    ep: &crate::model::EndpointRef,
    port_counts: &mut std::collections::HashMap<(String, u8), u32>,
    connected_ports: &mut std::collections::HashSet<(String, u32, bool)>,
    connected_control_ports: &mut std::collections::HashSet<(String, String)>,
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
    } else {
        // Control port (enable/trigger/reset/event/…): track by (sid, port_type)
        // so the chevron can be suppressed when a line connects to it.
        connected_control_ports.insert((ep.sid.clone(), ep.port_type.clone()));
    }
}

/// Recursively register branch endpoint ports.
pub fn register_branch_endpoints(
    branch: &crate::model::Branch,
    port_counts: &mut std::collections::HashMap<(String, u8), u32>,
    connected_ports: &mut std::collections::HashSet<(String, u32, bool)>,
    connected_control_ports: &mut std::collections::HashSet<(String, String)>,
) {
    if let Some(dst) = &branch.dst {
        register_endpoint(dst, port_counts, connected_ports, connected_control_ports);
    }
    for sub in &branch.branches {
        register_branch_endpoints(sub, port_counts, connected_ports, connected_control_ports);
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
    std::collections::HashSet<(String, String)>,
) {
    let mut port_counts: std::collections::HashMap<(String, u8), u32> =
        std::collections::HashMap::new();
    let mut connected_ports: std::collections::HashSet<(String, u32, bool)> =
        std::collections::HashSet::new();
    let mut connected_control_ports: std::collections::HashSet<(String, String)> =
        std::collections::HashSet::new();

    for line in lines {
        if let Some(src) = &line.src {
            register_endpoint(src, &mut port_counts, &mut connected_ports, &mut connected_control_ports);
        }
        if let Some(dst) = &line.dst {
            register_endpoint(dst, &mut port_counts, &mut connected_ports, &mut connected_control_ports);
        }
        for br in &line.branches {
            register_branch_endpoints(br, &mut port_counts, &mut connected_ports, &mut connected_control_ports);
        }
    }

    // Pre-populate from block declarations so line and chevron positioning
    // use consistent total port counts.
    for b in blocks {
        if let Some(sid) = &b.sid
            && let Some(pc) = &b.port_counts
        {
            let events = crate::simulink_libraries::renderers::subsystem_event_input_count(b);
            if events > 0 {
                port_counts.insert((sid.clone(), EVENT_IN_KIND), events);
            }
            // A reinit subsystem draws the reinit port in its own top section
            // and the data inputs below a separator line, so the data inputs
            // are NOT offset by the event count.
            let reinit = crate::simulink_libraries::renderers::is_reinit_subsystem(b);
            if reinit {
                port_counts.insert((sid.clone(), REINIT_FLAG_KIND), 1);
            }
            let ins_total = if reinit {
                pc.ins
            } else {
                pc.ins.map(|ins| ins + events)
            };
            if let Some(ins) = ins_total {
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

    (port_counts, connected_ports, connected_control_ports)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
