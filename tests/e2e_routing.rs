//! E2E: signal routing & line geometry (§9).

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use eframe::egui::{Pos2, pos2};
use harness::{BlockBuilder, SystemBuilder, line};
use rustylink::editor::operations::{auto_route, find_snap_port};
use rustylink::egui_app::ui::corner_ops;
use rustylink::egui_app::ui::line_coloring::{assign_line_colors, compute_line_adjacency};
use rustylink::egui_app::ui::signal_routing::{
    get_branch_mut, move_line_layout, move_line_point, orthogonalize_polyline,
};
use rustylink::model::Point;

// ── orthogonalization ────────────────────────────────────────────────────────

#[test]
fn rt_001_horizontal_stays() {
    let pts = vec![pos2(0.0, 0.0), pos2(10.0, 0.0)];
    assert_eq!(orthogonalize_polyline(&pts), pts);
}

#[test]
fn rt_003_diagonal_gets_corner() {
    let out = orthogonalize_polyline(&[pos2(0.0, 0.0), pos2(10.0, 10.0)]);
    assert_eq!(out.len(), 3, "corner inserted");
    for w in out.windows(2) {
        assert!(w[0].x == w[1].x || w[0].y == w[1].y, "axis aligned");
    }
    assert_eq!(out[0], pos2(0.0, 0.0));
    assert_eq!(*out.last().unwrap(), pos2(10.0, 10.0));
}

#[test]
fn rt_004_multi_diagonal_all_orthogonal() {
    let pts = vec![
        pos2(0.0, 0.0),
        pos2(10.0, 10.0),
        pos2(20.0, 5.0),
        pos2(30.0, 30.0),
    ];
    let out = orthogonalize_polyline(&pts);
    assert!(out.len() > pts.len());
    for w in out.windows(2) {
        assert!(w[0].x == w[1].x || w[0].y == w[1].y);
    }
}

#[test]
fn rt_006_orthogonalize_idempotent() {
    let pts = vec![pos2(0.0, 0.0), pos2(7.0, 13.0), pos2(29.0, 2.0)];
    let once = orthogonalize_polyline(&pts);
    let twice = orthogonalize_polyline(&once);
    assert_eq!(once, twice);
}

// ── corner ops ───────────────────────────────────────────────────────────────

#[test]
fn rt_040_insert_then_remove_corner() {
    let mut pts = vec![Point { x: 0, y: 0 }, Point { x: 100, y: 0 }];
    corner_ops::insert_corner(&mut pts, 0, Point { x: 50, y: 30 });
    assert_eq!(pts.len(), 3);
    let removed = corner_ops::remove_corner(&mut pts, 1);
    assert!(removed.is_some());
    assert_eq!(pts.len(), 2);
}

#[test]
fn rt_042_merge_adjacent_corners_threshold() {
    // Points are RELATIVE offsets; a segment shorter than `threshold` is
    // folded into the next one.
    let mut pts = vec![
        Point { x: 2, y: 0 },
        Point { x: 5, y: 0 },
        Point { x: 100, y: 0 },
    ];
    corner_ops::merge_adjacent_corners(&mut pts, 4);
    assert_eq!(pts, vec![Point { x: 7, y: 0 }, Point { x: 100, y: 0 }]);
    // Chained merges accumulate until a segment exceeds the threshold.
    let mut pts = vec![
        Point { x: 1, y: 0 },
        Point { x: 1, y: 0 },
        Point { x: 50, y: 0 },
    ];
    corner_ops::merge_adjacent_corners(&mut pts, 4);
    assert_eq!(pts, vec![Point { x: 52, y: 0 }]);
}

#[test]
fn rt_043_enforce_orthogonal_repairs() {
    let mut pts = vec![
        Point { x: 0, y: 0 },
        Point { x: 50, y: 50 },
        Point { x: 100, y: 50 },
    ];
    corner_ops::enforce_orthogonal(&mut pts);
    for w in pts.windows(2) {
        assert!(w[0].x == w[1].x || w[0].y == w[1].y);
    }
}

// ── point/branch mutation ────────────────────────────────────────────────────

#[test]
fn rt_030_move_line_point_compensates() {
    // Line.points are RELATIVE segment offsets — moving point i compensates
    // point i+1 so the line's absolute end stays put.
    let mut l = harness::model_builders::LineBuilder::new()
        .from("1", 1)
        .to("2", 1)
        .points(&[(10, 0), (10, 20), (40, 20)])
        .build();
    move_line_point(&mut l, 0, 15, 0);
    assert_eq!(l.points[0], Point { x: 25, y: 0 }, "moved point +15");
    assert_eq!(l.points[1], Point { x: -5, y: 20 }, "next compensated -15");
    assert_eq!(l.points[2], Point { x: 40, y: 20 });
    // Move the last point — nothing to compensate.
    move_line_point(&mut l, 2, 0, 5);
    assert_eq!(l.points[2], Point { x: 40, y: 25 });
}

#[test]
fn rt_033_move_line_layout_shifts_all() {
    let mut l = harness::model_builders::LineBuilder::new()
        .from("1", 1)
        .to("2", 1)
        .points(&[(10, 0), (10, 20)])
        .branch_to("3", 1)
        .build();
    l.branches[0].points = vec![Point { x: 20, y: 0 }, Point { x: 20, y: 9 }];
    let orig_branch = l.branches[0].points.clone();
    move_line_layout(&mut l, 5, 7);
    assert!(l.points.iter().all(|p| p.x != 10 || p.y != 20));
    for (i, p) in l.branches[0].points.iter().enumerate() {
        assert_eq!(p.x, orig_branch[i].x + 5);
        assert_eq!(p.y, orig_branch[i].y + 7);
    }
}

#[test]
fn rt_035_get_branch_mut_navigates() {
    let mut l = harness::model_builders::LineBuilder::new()
        .from("1", 1)
        .branch_to("2", 1)
        .build();
    l.branches[0].branches.push(rustylink::model::Branch {
        name: None,
        zorder: None,
        dst: Some(harness::model_builders::endpoint("3", "in", 1)),
        points: vec![],
        labels: None,
        branches: vec![],
        properties: indexmap::IndexMap::new(),
    });
    let b = get_branch_mut(&mut l.branches, &[0, 0]).expect("nested branch");
    assert_eq!(b.dst.as_ref().unwrap().sid, "3");
    assert!(get_branch_mut(&mut l.branches, &[5]).is_none());
}

// ── auto_route & snapping ────────────────────────────────────────────────────

#[test]
fn rt_050_auto_route_straight_horizontal() {
    let pts = auto_route(0.0, 10.0, 100.0, 10.0, "out", "in");
    assert!(
        pts.is_empty() || pts.iter().all(|p| p.y == 0),
        "no dogleg needed"
    );
}

#[test]
fn rt_051_auto_route_dogleg_is_orthogonal() {
    // Reconstruct absolute path: src + cumulative relative points.
    let pts = auto_route(0.0, 0.0, 100.0, 50.0, "out", "in");
    let mut cur = Pos2::new(0.0, 0.0);
    let mut abs = vec![cur];
    for p in &pts {
        cur += eframe::egui::vec2(p.x as f32, p.y as f32);
        abs.push(cur);
    }
    abs.push(Pos2::new(100.0, 50.0));
    for w in abs.windows(2) {
        assert!(w[0].x == w[1].x || w[0].y == w[1].y, "segment axis-aligned");
    }
}

#[test]
fn rt_052_auto_route_backward_is_orthogonal() {
    // Backward (dst left of src) with a vertical offset → dogleg waypoints.
    let pts = auto_route(100.0, 0.0, 0.0, 50.0, "out", "in");
    assert!(!pts.is_empty(), "backward route needs waypoints");
    let mut cur = Pos2::new(100.0, 0.0);
    let mut abs = vec![cur];
    for p in &pts {
        cur += eframe::egui::vec2(p.x as f32, p.y as f32);
        abs.push(cur);
    }
    abs.push(Pos2::new(0.0, 50.0));
    for w in abs.windows(2) {
        assert!(w[0].x == w[1].x || w[0].y == w[1].y, "segment axis-aligned");
    }
}

#[test]
fn rt_054_find_snap_port_hits_and_misses() {
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "S", "2")
                .pos(200, 0, 240, 24)
                .ports(1, 0)
                .build(),
        )
        .build();
    // Snap pos for S.in:1 → x=200 (left), y = 0 + (2*1-0.5)*24/3 = 12.
    let hit = find_snap_port(&sys, 200.0, 12.0, 20.0, None);
    assert!(hit.is_some());
    let (idx, kind, port, _px, _py) = hit.unwrap();
    assert_eq!(
        (sys.blocks[idx].name.as_str(), kind.as_str(), port),
        ("S", "in", 1)
    );

    let miss = find_snap_port(&sys, 120.0, 120.0, 5.0, None);
    assert!(miss.is_none(), "far from any port");
}

// ── line coloring ────────────────────────────────────────────────────────────

#[test]
fn rt_060_adjacency_and_colors() {
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Gain", "G", "2")
                .pos(100, 0, 140, 24)
                .ports(1, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "S", "3")
                .pos(200, 0, 240, 24)
                .ports(1, 0)
                .build(),
        )
        .line(line("1", 1, "2", 1))
        .line(line("2", 1, "3", 1))
        .build();
    let adj = compute_line_adjacency(&sys.lines);
    assert_eq!(adj.len(), 2);
    let colors = assign_line_colors(&adj, 0.9);
    assert_eq!(colors.len(), 2);
}
