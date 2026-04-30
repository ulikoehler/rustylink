//! Pure-function corner manipulation utilities for signal lines.
//!
//! These functions operate on the `Vec<Point>` model (cumulative relative
//! offsets from the source endpoint) without any egui dependency, making them
//! straightforward to unit-test.

use crate::model::{Line, Point};

// ---------------------------------------------------------------------------
// Insert / Remove / Merge corners
// ---------------------------------------------------------------------------

/// Insert a new corner point at `index` in the line's point list.
///
/// The inserted point gets the given `offset`. The *next* point (if any)
/// is adjusted so that all downstream geometry remains unchanged.
pub fn insert_corner(points: &mut Vec<Point>, index: usize, offset: Point) {
    if index > points.len() {
        return;
    }
    // If inserting before an existing point, subtract our offset from it
    // so the cumulative position after index stays the same.
    if index < points.len() {
        let next = &mut points[index];
        next.x -= offset.x;
        next.y -= offset.y;
    }
    points.insert(index, offset);
}

/// Remove the corner point at `index`, merging its offset into the next
/// point so downstream geometry is preserved.
///
/// Returns the removed point (for undo support), or `None` if the index
/// was out of range.
pub fn remove_corner(points: &mut Vec<Point>, index: usize) -> Option<Point> {
    if index >= points.len() {
        return None;
    }
    let removed = points.remove(index);
    // Compensate the next point so the cumulative position doesn't change.
    if index < points.len() {
        points[index].x += removed.x;
        points[index].y += removed.y;
    }
    Some(removed)
}

/// Merge adjacent corner points that are closer than `threshold` model
/// units apart (Manhattan distance). The second point is absorbed into
/// the first, preserving downstream positions.
#[cfg(test)]
pub fn merge_adjacent_corners(points: &mut Vec<Point>, threshold: i32) {
    let mut i = 0;
    while i + 1 < points.len() {
        let dist = points[i].x.abs() + points[i].y.abs();
        if dist <= threshold {
            // Merge point[i] into point[i+1]
            points[i + 1].x += points[i].x;
            points[i + 1].y += points[i].y;
            points.remove(i);
            // Don't advance i — check the merged result against the next
        } else {
            i += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Auto-adjust on block move
// ---------------------------------------------------------------------------

/// Adjust a line's corners when a connected block moves by `(dx, dy)`.
///
/// * `is_source = true`  → the *source* block moved; adjust/insert the
///   **first** point so the line tracks the block.
/// * `is_source = false` → the *destination* block moved; adjust/insert the
///   **last** point so the line tracks the destination.
///
/// When the line already has points, the first (or last) point is adjusted.
/// When the line has *no* points, a new point is created to absorb the delta.
pub fn auto_adjust_on_block_move(line: &mut Line, is_source: bool, dx: i32, dy: i32) {
    if dx == 0 && dy == 0 {
        return;
    }
    if is_source {
        // Source block moved — the source anchor moves with the block.
        // Offset of first point relative to the (now-moved) source must
        // be *reduced* by the block's delta to keep the absolute position
        // of the first corner unchanged.
        if let Some(first) = line.points.first_mut() {
            first.x -= dx;
            first.y -= dy;
        } else {
            // No corners: insert one to absorb the discrepancy.
            line.points.push(Point { x: -dx, y: -dy });
        }
    } else {
        // Destination block moved — the destination anchor moves with the
        // block. We need to extend/adjust the last point to reach the new
        // destination position.
        if let Some(last) = line.points.last_mut() {
            last.x += dx;
            last.y += dy;
        } else {
            line.points.push(Point { x: dx, y: dy });
        }
    }
}

/// Adjust all branches whose destination matches the moved block.
pub fn auto_adjust_branches_on_block_move(
    branches: &mut [crate::model::Branch],
    moved_sid: &str,
    dx: i32,
    dy: i32,
) {
    for branch in branches.iter_mut() {
        if let Some(dst) = &branch.dst {
            if dst.sid == moved_sid {
                if let Some(last) = branch.points.last_mut() {
                    last.x += dx;
                    last.y += dy;
                } else {
                    branch.points.push(Point { x: dx, y: dy });
                }
            }
        }
        auto_adjust_branches_on_block_move(&mut branch.branches, moved_sid, dx, dy);
    }
}

// ---------------------------------------------------------------------------
// Orthogonal enforcement (model-level)
// ---------------------------------------------------------------------------

/// Snap each point's offset so the segment from its predecessor is either
/// purely horizontal or purely vertical (horizontal-first convention).
///
/// This operates on the *relative offsets* (the `Point` values), not
/// absolute screen positions.
#[cfg(test)]
pub fn enforce_orthogonal(points: &mut Vec<Point>) {
    for point in points.iter_mut() {
        if point.x != 0 && point.y != 0 {
            // Diagonal offset — split into horizontal-only.
            // The vertical component will be absorbed by the next
            // orthogonalization pass (or the final segment to the
            // destination). For now, zero out the smaller axis.
            if point.x.abs() >= point.y.abs() {
                point.y = 0;
            } else {
                point.x = 0;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Branch, EndpointRef, Line, Point};
    use indexmap::IndexMap;

    fn test_line(points: Vec<Point>) -> Line {
        Line {
            src: None,
            dst: None,
            name: None,
            zorder: None,
            labels: None,
            properties: IndexMap::new(),
            points,
            branches: vec![],
        }
    }

    // ── insert_corner ──

    #[test]
    fn insert_corner_at_beginning() {
        let mut pts = vec![Point { x: 20, y: 0 }, Point { x: 0, y: 30 }];
        insert_corner(&mut pts, 0, Point { x: 10, y: 0 });
        assert_eq!(pts.len(), 3);
        assert_eq!(pts[0], Point { x: 10, y: 0 });
        assert_eq!(pts[1], Point { x: 10, y: 0 }); // 20 - 10
        assert_eq!(pts[2], Point { x: 0, y: 30 });
        // Cumulative: 10, 20, 20+30=50 — same as original 20, 20+30=50 plus new 10
    }

    #[test]
    fn insert_corner_at_middle() {
        let mut pts = vec![Point { x: 30, y: 0 }, Point { x: 0, y: 20 }];
        insert_corner(&mut pts, 1, Point { x: 10, y: 5 });
        assert_eq!(pts.len(), 3);
        assert_eq!(pts[0], Point { x: 30, y: 0 });
        assert_eq!(pts[1], Point { x: 10, y: 5 });
        assert_eq!(pts[2], Point { x: -10, y: 15 }); // 0-10, 20-5
    }

    #[test]
    fn insert_corner_at_end() {
        let mut pts = vec![Point { x: 10, y: 0 }];
        insert_corner(&mut pts, 1, Point { x: 5, y: 5 });
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0], Point { x: 10, y: 0 });
        assert_eq!(pts[1], Point { x: 5, y: 5 });
    }

    #[test]
    fn insert_corner_into_empty() {
        let mut pts = Vec::new();
        insert_corner(&mut pts, 0, Point { x: 10, y: 0 });
        assert_eq!(pts.len(), 1);
        assert_eq!(pts[0], Point { x: 10, y: 0 });
    }

    // ── remove_corner ──

    #[test]
    fn remove_corner_merges_offset() {
        let mut pts = vec![
            Point { x: 10, y: 0 },
            Point { x: 5, y: 5 },
            Point { x: 0, y: 20 },
        ];
        let removed = remove_corner(&mut pts, 1);
        assert_eq!(removed, Some(Point { x: 5, y: 5 }));
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0], Point { x: 10, y: 0 });
        assert_eq!(pts[1], Point { x: 5, y: 25 }); // 0+5, 20+5
    }

    #[test]
    fn remove_corner_last_point() {
        let mut pts = vec![Point { x: 10, y: 0 }, Point { x: 5, y: 5 }];
        let removed = remove_corner(&mut pts, 1);
        assert_eq!(removed, Some(Point { x: 5, y: 5 }));
        assert_eq!(pts.len(), 1);
        assert_eq!(pts[0], Point { x: 10, y: 0 });
    }

    #[test]
    fn remove_corner_only_point() {
        let mut pts = vec![Point { x: 10, y: 5 }];
        let removed = remove_corner(&mut pts, 0);
        assert_eq!(removed, Some(Point { x: 10, y: 5 }));
        assert!(pts.is_empty());
    }

    #[test]
    fn remove_corner_out_of_range() {
        let mut pts = vec![Point { x: 1, y: 2 }];
        assert_eq!(remove_corner(&mut pts, 5), None);
        assert_eq!(pts.len(), 1);
    }

    // ── merge_adjacent_corners ──

    #[test]
    fn merge_adjacent_within_threshold() {
        let mut pts = vec![
            Point { x: 2, y: 0 },  // distance 2 — within threshold 5
            Point { x: 20, y: 0 }, // distance 20 — outside
            Point { x: 1, y: 1 },  // distance 2 — within threshold 5
            Point { x: 0, y: 30 },
        ];
        merge_adjacent_corners(&mut pts, 5);
        // First point (dist=2) merged into second: second becomes (22, 0)
        // Then third (now index 1) point (dist=2) merged into fourth: fourth becomes (1, 31)
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0], Point { x: 22, y: 0 });
        assert_eq!(pts[1], Point { x: 1, y: 31 });
    }

    #[test]
    fn merge_adjacent_nothing_to_merge() {
        let mut pts = vec![Point { x: 10, y: 0 }, Point { x: 0, y: 20 }];
        merge_adjacent_corners(&mut pts, 5);
        assert_eq!(pts.len(), 2);
    }

    // ── auto_adjust_on_block_move ──

    #[test]
    fn auto_adjust_source_with_corners() {
        let mut line = test_line(vec![Point { x: 20, y: 0 }, Point { x: 0, y: 30 }]);
        auto_adjust_on_block_move(&mut line, true, 5, 3);
        assert_eq!(line.points[0], Point { x: 15, y: -3 });
        assert_eq!(line.points[1], Point { x: 0, y: 30 });
    }

    #[test]
    fn auto_adjust_source_no_corners() {
        let mut line = test_line(vec![]);
        auto_adjust_on_block_move(&mut line, true, 5, 3);
        assert_eq!(line.points.len(), 1);
        assert_eq!(line.points[0], Point { x: -5, y: -3 });
    }

    #[test]
    fn auto_adjust_dest_with_corners() {
        let mut line = test_line(vec![Point { x: 20, y: 0 }, Point { x: 0, y: 30 }]);
        auto_adjust_on_block_move(&mut line, false, 5, 3);
        assert_eq!(line.points[0], Point { x: 20, y: 0 });
        assert_eq!(line.points[1], Point { x: 5, y: 33 });
    }

    #[test]
    fn auto_adjust_dest_no_corners() {
        let mut line = test_line(vec![]);
        auto_adjust_on_block_move(&mut line, false, 5, 3);
        assert_eq!(line.points.len(), 1);
        assert_eq!(line.points[0], Point { x: 5, y: 3 });
    }

    #[test]
    fn auto_adjust_zero_delta_noop() {
        let mut line = test_line(vec![Point { x: 10, y: 0 }]);
        auto_adjust_on_block_move(&mut line, true, 0, 0);
        assert_eq!(line.points[0], Point { x: 10, y: 0 });
    }

    // ── auto_adjust_branches_on_block_move ──

    #[test]
    fn auto_adjust_branch_dest() {
        let mut branches = vec![Branch {
            dst: Some(EndpointRef {
                sid: "42".to_string(),
                port_type: "in".to_string(),
                port_index: 1,
            }),
            name: None,
            zorder: None,
            labels: None,
            properties: IndexMap::new(),
            points: vec![Point { x: 10, y: 0 }, Point { x: 0, y: 20 }],
            branches: vec![],
        }];
        auto_adjust_branches_on_block_move(&mut branches, "42", 3, -2);
        assert_eq!(branches[0].points[1], Point { x: 3, y: 18 });
    }

    #[test]
    fn auto_adjust_branch_no_match() {
        let mut branches = vec![Branch {
            dst: Some(EndpointRef {
                sid: "99".to_string(),
                port_type: "in".to_string(),
                port_index: 1,
            }),
            name: None,
            zorder: None,
            labels: None,
            properties: IndexMap::new(),
            points: vec![Point { x: 10, y: 0 }],
            branches: vec![],
        }];
        auto_adjust_branches_on_block_move(&mut branches, "42", 3, -2);
        assert_eq!(branches[0].points[0], Point { x: 10, y: 0 });
    }

    // ── enforce_orthogonal ──

    #[test]
    fn enforce_orthogonal_horizontal_dominant() {
        let mut pts = vec![Point { x: 10, y: 3 }];
        enforce_orthogonal(&mut pts);
        assert_eq!(pts[0], Point { x: 10, y: 0 });
    }

    #[test]
    fn enforce_orthogonal_vertical_dominant() {
        let mut pts = vec![Point { x: 2, y: 15 }];
        enforce_orthogonal(&mut pts);
        assert_eq!(pts[0], Point { x: 0, y: 15 });
    }

    #[test]
    fn enforce_orthogonal_already_axis_aligned() {
        let mut pts = vec![Point { x: 10, y: 0 }, Point { x: 0, y: 20 }];
        enforce_orthogonal(&mut pts);
        assert_eq!(pts[0], Point { x: 10, y: 0 });
        assert_eq!(pts[1], Point { x: 0, y: 20 });
    }

    #[test]
    fn enforce_orthogonal_empty() {
        let mut pts: Vec<Point> = vec![];
        enforce_orthogonal(&mut pts);
        assert!(pts.is_empty());
    }

    // ── insert + remove round-trip ──

    #[test]
    fn insert_then_remove_roundtrip() {
        let original = vec![Point { x: 20, y: 0 }, Point { x: 0, y: 30 }];
        let mut pts = original.clone();
        insert_corner(&mut pts, 1, Point { x: 10, y: 5 });
        assert_eq!(pts.len(), 3);
        let removed = remove_corner(&mut pts, 1);
        assert_eq!(removed, Some(Point { x: 10, y: 5 }));
        assert_eq!(pts, original);
    }
}
