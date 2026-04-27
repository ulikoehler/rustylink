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

/// Register an endpoint's port in the port-count and connected-ports maps.
pub fn register_endpoint(
    ep: &crate::model::EndpointRef,
    port_counts: &mut std::collections::HashMap<(String, u8), u32>,
    connected_ports: &mut std::collections::HashSet<(String, u32, bool)>,
) {
    let key = (ep.sid.clone(), if ep.port_type == "out" { 1 } else { 0 });
    let idx1 = if ep.port_index == 0 { 1 } else { ep.port_index };
    port_counts
        .entry(key)
        .and_modify(|v| *v = (*v).max(idx1))
        .or_insert(idx1);
    let is_input = ep.port_type != "out";
    connected_ports.insert((ep.sid.clone(), ep.port_index, is_input));
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
        if let Some(sid) = &b.sid {
            if let Some(pc) = &b.port_counts {
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
        }
    }

    (port_counts, connected_ports)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Branch, Line, Point};
    use indexmap::IndexMap;

    /// Helper to create a minimal `Line` for tests.
    fn test_line(points: Vec<Point>, branches: Vec<Branch>) -> Line {
        Line {
            src: None,
            dst: None,
            name: None,
            zorder: None,
            labels: None,
            properties: IndexMap::new(),
            points,
            branches,
        }
    }

    /// Helper to create a minimal `Branch` for tests.
    fn test_branch(points: Vec<Point>, branches: Vec<Branch>) -> Branch {
        Branch {
            dst: None,
            name: None,
            zorder: None,
            labels: None,
            properties: IndexMap::new(),
            points,
            branches,
        }
    }

    #[test]
    fn orthogonalize_empty() {
        assert!(orthogonalize_polyline(&[]).is_empty());
    }

    #[test]
    fn orthogonalize_single_point() {
        let pts = vec![Pos2::new(10.0, 20.0)];
        let result = orthogonalize_polyline(&pts);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], Pos2::new(10.0, 20.0));
    }

    #[test]
    fn orthogonalize_horizontal_stays() {
        let pts = vec![Pos2::new(0.0, 5.0), Pos2::new(10.0, 5.0)];
        let result = orthogonalize_polyline(&pts);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn orthogonalize_vertical_stays() {
        let pts = vec![Pos2::new(5.0, 0.0), Pos2::new(5.0, 10.0)];
        let result = orthogonalize_polyline(&pts);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn orthogonalize_diagonal_inserts_corner() {
        let pts = vec![Pos2::new(0.0, 0.0), Pos2::new(10.0, 10.0)];
        let result = orthogonalize_polyline(&pts);
        assert_eq!(result.len(), 3);
        // Corner inserted horizontal-first: (10, 0) then (10, 10)
        assert_eq!(result[1], Pos2::new(10.0, 0.0));
        assert_eq!(result[2], Pos2::new(10.0, 10.0));
    }

    #[test]
    fn orthogonalize_multiple_diagonals() {
        let pts = vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 5.0),
            Pos2::new(20.0, 15.0),
        ];
        let result = orthogonalize_polyline(&pts);
        // Each diagonal adds a corner point
        assert!(result.len() >= 5);
        // All segments should be axis-aligned
        for seg in result.windows(2) {
            let dx = (seg[0].x - seg[1].x).abs();
            let dy = (seg[0].y - seg[1].y).abs();
            assert!(
                dx < f32::EPSILON || dy < f32::EPSILON,
                "Non-orthogonal segment: {:?} -> {:?}",
                seg[0],
                seg[1]
            );
        }
    }

    #[test]
    fn push_segments_from_polyline() {
        let pts = vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
            Pos2::new(10.0, 10.0),
        ];
        let mut segs = Vec::new();
        push_orthogonal_segments(&pts, &mut segs);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0], (Pos2::new(0.0, 0.0), Pos2::new(10.0, 0.0)));
        assert_eq!(segs[1], (Pos2::new(10.0, 0.0), Pos2::new(10.0, 10.0)));
    }

    #[test]
    fn move_line_point_compensates_next() {
        let mut line = test_line(
            vec![
                Point { x: 10, y: 0 },
                Point { x: 20, y: 0 },
                Point { x: 0, y: 30 },
            ],
            vec![],
        );
        move_line_point(&mut line, 1, 5, -3);
        assert_eq!(line.points[1].x, 25);
        assert_eq!(line.points[1].y, -3);
        // Next point compensated
        assert_eq!(line.points[2].x, -5);
        assert_eq!(line.points[2].y, 33);
        // Previous point unchanged
        assert_eq!(line.points[0].x, 10);
        assert_eq!(line.points[0].y, 0);
    }

    #[test]
    fn move_line_point_last_index_no_crash() {
        let mut line = test_line(vec![Point { x: 5, y: 5 }], vec![]);
        // Moving the last (only) point shouldn't panic
        move_line_point(&mut line, 0, 3, 3);
        assert_eq!(line.points[0].x, 8);
        assert_eq!(line.points[0].y, 8);
    }

    #[test]
    fn move_branch_point_basic() {
        let mut branch = test_branch(vec![Point { x: 10, y: 10 }, Point { x: 20, y: 20 }], vec![]);
        move_branch_point(&mut branch, 0, 5, -5);
        assert_eq!(branch.points[0].x, 15);
        assert_eq!(branch.points[0].y, 5);
        assert_eq!(branch.points[1].x, 15);
        assert_eq!(branch.points[1].y, 25);
    }

    #[test]
    fn move_line_layout_shifts_all() {
        let mut line = test_line(
            vec![Point { x: 0, y: 0 }, Point { x: 10, y: 10 }],
            vec![test_branch(vec![Point { x: 5, y: 5 }], vec![])],
        );
        move_line_layout(&mut line, 3, -2);
        assert_eq!(line.points[0], Point { x: 3, y: -2 });
        assert_eq!(line.points[1], Point { x: 13, y: 8 });
        assert_eq!(line.branches[0].points[0], Point { x: 8, y: 3 });
    }

    #[test]
    fn get_branch_mut_navigates_tree() {
        let mut branches = vec![
            test_branch(
                vec![Point { x: 1, y: 1 }],
                vec![test_branch(vec![Point { x: 2, y: 2 }], vec![])],
            ),
            test_branch(vec![Point { x: 3, y: 3 }], vec![]),
        ];

        // Navigate to first branch
        let b = get_branch_mut(&mut branches, &[0]).unwrap();
        assert_eq!(b.points[0], Point { x: 1, y: 1 });

        // Navigate to nested branch
        let b = get_branch_mut(&mut branches, &[0, 0]).unwrap();
        assert_eq!(b.points[0], Point { x: 2, y: 2 });

        // Navigate to second top-level branch
        let b = get_branch_mut(&mut branches, &[1]).unwrap();
        assert_eq!(b.points[0], Point { x: 3, y: 3 });

        // Invalid path returns None
        assert!(get_branch_mut(&mut branches, &[5]).is_none());
        assert!(get_branch_mut(&mut branches, &[0, 5]).is_none());
    }

    #[test]
    fn collect_branch_handles_basic() {
        let identity = |p: Pos2| p;
        let branches = vec![test_branch(
            vec![Point { x: 10, y: 0 }, Point { x: 0, y: 20 }],
            vec![],
        )];
        let mut out = Vec::new();
        collect_branch_handle_positions(
            Pos2::new(0.0, 0.0),
            &branches,
            &identity,
            &mut Vec::new(),
            &mut out,
        );
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].0, vec![0]); // branch path
        assert_eq!(out[0].1, 0); // point index
        assert_eq!(out[0].2, Pos2::new(10.0, 0.0));
        assert_eq!(out[1].2, Pos2::new(10.0, 20.0));
    }
}
