//! E2E: line/arrow endpoint positions must be correct at ALL zoom levels.
//!
//! The drawn port chevron, its interaction hit-area, the snap target used
//! while drawing a connection, and the line's endpoint anchor must ALL resolve
//! to the same position — otherwise the line visibly misses the port and the
//! gap grows with zoom.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use eframe::egui;
use harness::{BlockBuilder, EditorHarness, SystemBuilder, ViewerHarness, gain_chain, input};
use rustylink::model::System;

fn multi_in(n: u32) -> System {
    SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Mux", "M", "1")
                .pos(0, 0, 40, 120)
                .ports(n, 0)
                .build(),
        )
        .build()
}

/// Where port `i` of `n` side-ports sits (fraction of block height) per the
/// verified anchor formula `(i - 0.5) / n` used by `endpoint_pos`.
fn anchor_frac(i: u32, n: u32) -> f32 {
    (i as f32 - 0.5) / n as f32
}

#[test]
fn ep_001_hit_pos_equals_anchor_multiport() {
    // For a 3-in block, the editor's port hit positions must land exactly on
    // the anchor fractions 1/6, 3/6, 5/6 — not on the evenly-spaced 1/4,2/4,3/4.
    let mut h = EditorHarness::new(multi_in(3));
    h.idle(2);
    let r = h.block_screen_rect("1").unwrap();
    for (i, want) in [(1u32, 1.0 / 6.0), (2, 3.0 / 6.0), (3, 5.0 / 6.0)] {
        let p = h.port_hit_pos("1", "in", i);
        let frac = (p.y - r.top()) / r.height();
        assert!(
            (frac - want as f32).abs() < 0.02,
            "in:{i} hit at frac {frac:.3}, want anchor {want:.3}"
        );
    }
}

#[test]
fn ep_002_hit_pos_equals_anchor_two_ports() {
    let mut h = EditorHarness::new(multi_in(2));
    h.idle(2);
    let r = h.block_screen_rect("1").unwrap();
    for (i, want) in [(1u32, 0.25), (2, 0.75)] {
        let p = h.port_hit_pos("1", "in", i);
        let frac = (p.y - r.top()) / r.height();
        assert!(
            (frac - want as f32).abs() < 0.02,
            "in:{i} hit at {frac:.3}, want {want:.3}"
        );
    }
}

#[test]
fn ep_003_snap_pos_equals_anchor_multiport() {
    // find_snap_port must return the same position as the drawn anchor —
    // otherwise a dropped connection lands off the port.
    let sys = multi_in(3);
    for i in 1..=3u32 {
        let model_y = 0.0 + 120.0 * anchor_frac(i, 3);
        // Probe just below/above anchor: snapped pos must be the anchor.
        let (_, _, idx, px, py) =
            rustylink::editor::operations::find_snap_port(&sys, 0.0, model_y, 10.0, None)
                .expect("snap found near anchor");
        assert_eq!(idx, i, "snap picks port {i}");
        assert!((py - model_y).abs() < 0.5, "snap y {py} ≈ anchor {model_y}");
        let _ = px;
    }
}

#[test]
fn ep_004_endpoint_matches_anchor_at_zooms() {
    // End-to-end: a drawn line's endpoint (screen pos via anchor) must equal
    // the port anchor at zoom 0.5 / 1.0 / 2.0 / 4.0.
    for zoom in [0.5f32, 1.0, 2.0, 4.0] {
        let mut h = EditorHarness::new(gain_chain(1));
        h.state.app.zoom = zoom;
        h.idle(3);
        let r2 = h.block_screen_rect("2").unwrap();
        let hit = h.port_hit_pos("2", "in", 1);
        // The in:1 anchor for a 1-port block is the left-edge centre.
        let want_y = r2.top() + 0.5 * r2.height();
        assert!(
            (hit.y - want_y).abs() <= 2.0,
            "zoom {zoom}: hit y {} ≈ anchor y {want_y}",
            hit.y
        );
    }
}

#[test]
fn ep_005_viewer_anchor_hit_consistency() {
    // Viewer: the drawn endpoint (anchor) for a 2-port block must sit at
    // 0.25/0.75 of height — checking via endpoint_pos on the screen rect.
    let mut h = ViewerHarness::new(multi_in(2));
    h.idle(2);
    let r = h.block_screen_rect("1").unwrap();
    let ep = rustylink::model::EndpointRef {
        sid: "1".into(),
        port_type: "in".into(),
        port_index: 1,
    };
    let mut counts = std::collections::HashMap::new();
    counts.insert(("1".to_string(), 0u8), 2u32); // 2 input ports
    let pos = rustylink::egui_app::ui::signal_routing::endpoint_pos(r, &ep, &counts, false, &[]);
    let frac = (pos.y - r.top()) / r.height();
    assert!(
        (frac - 0.25).abs() < 0.02,
        "endpoint_pos in:1 frac {frac:.3}, want 0.25"
    );
}

#[test]
fn ep_006_endpoint_matches_chevron_visual() {
    // The chevron is DRAWN at the hit position; the line's anchor must land
    // at the same spot — compare port_hit_pos (editor overlay) against the
    // endpoint anchor computed by the renderer for n=2.
    let mut h = EditorHarness::new(multi_in(2));
    h.idle(2);
    let r = h.block_screen_rect("1").unwrap();
    let mut counts = std::collections::HashMap::new();
    counts.insert(("1".to_string(), 0u8), 2u32);
    for i in 1..=2u32 {
        let ep = rustylink::model::EndpointRef {
            sid: "1".into(),
            port_type: "in".into(),
            port_index: i,
        };
        let anchor =
            rustylink::egui_app::ui::signal_routing::endpoint_pos(r, &ep, &counts, false, &[]);
        let hit = h.port_hit_pos("1", "in", i);
        assert!(
            (hit.y - anchor.y).abs() <= 2.0,
            "in:{i}: hit {} == anchor {}",
            hit.y,
            anchor.y
        );
    }
}

// ── out-port equivalents ─────────────────────────────────────────────────────

fn multi_out(n: u32) -> System {
    SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Mux", "M", "1")
                .pos(0, 0, 40, 120)
                .ports(0, n)
                .build(),
        )
        .build()
}

#[test]
fn ep_007_out_hit_pos_equals_anchor() {
    // Same check as inputs but on the right edge (out ports).
    let mut h = EditorHarness::new(multi_out(3));
    h.idle(2);
    let r = h.block_screen_rect("1").unwrap();
    for (i, want) in [(1u32, 1.0 / 6.0), (2, 3.0 / 6.0), (3, 5.0 / 6.0)] {
        let p = h.port_hit_pos("1", "out", i);
        let frac = (p.y - r.top()) / r.height();
        assert!(
            (frac - want as f32).abs() < 0.02,
            "out:{i} hit at frac {frac:.3}, want anchor {want:.3}"
        );
        assert!((p.x - r.right()).abs() <= 1.0, "out:{i} on right edge");
    }
}

#[test]
fn ep_008_mirrored_multiport_anchor() {
    // Mirrored 2-in block: in ports move to the RIGHT edge but keep the
    // same (i-0.5)/n fractions — mirrored does not flip port ORDER.
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Mux", "M", "1")
                .pos(0, 0, 40, 120)
                .ports(2, 1)
                .mirrored()
                .build(),
        )
        .build();
    let mut h = EditorHarness::new(sys);
    h.idle(2);
    let r = h.block_screen_rect("1").unwrap();
    for (i, want) in [(1u32, 0.25), (2, 0.75)] {
        let p = h.port_hit_pos("1", "in", i);
        let frac = (p.y - r.top()) / r.height();
        assert!(
            (frac - want as f32).abs() < 0.02,
            "mirrored in:{i} frac {frac:.3}"
        );
        assert!(p.x > r.center().x, "mirrored in:{i} on right side");
    }
}

#[test]
fn ep_009_drawn_endpoint_matches_anchor_zooms() {
    // End-to-end at multiple zooms: draw a connection, then verify the
    // renderer's endpoint anchor on the dst block's screen rect equals the
    // cell-centred fraction — at every zoom the geometry is identical.
    let mut sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Mux", "M", "2")
                .pos(200, 0, 240, 120)
                .ports(3, 0)
                .build(),
        )
        .build();
    sys.lines.push(
        harness::model_builders::LineBuilder::new()
            .from("1", 1)
            .to("2", 2)
            .build(),
    );
    for zoom in [0.3f32, 0.7, 1.0, 2.5, 4.0] {
        let mut h = EditorHarness::new(sys.clone());
        h.state.app.zoom = zoom;
        h.idle(3);
        let r = h.block_screen_rect("2").unwrap();
        // Endpoint anchor for in:2 of 3 = fraction 0.5 (3/6).
        let ep = rustylink::model::EndpointRef {
            sid: "2".into(),
            port_type: "in".into(),
            port_index: 2,
        };
        let mut counts = std::collections::HashMap::new();
        counts.insert(("2".to_string(), 0u8), 3u32);
        let anchor =
            rustylink::egui_app::ui::signal_routing::endpoint_pos(r, &ep, &counts, false, &[]);
        let frac = (anchor.y - r.top()) / r.height();
        assert!(
            (frac - 0.5).abs() < 0.02,
            "zoom {zoom}: endpoint frac {frac:.3}, want 0.5"
        );
    }
}

#[test]
fn ep_010_pending_connection_starts_at_anchor() {
    // The pending-connection preview must start at the out anchor — the same
    // spot the finished line's endpoint lands.
    let mut h = EditorHarness::new(multi_out(3));
    h.idle(2);
    let from = h.port_hit_pos("1", "out", 2); // anchor: out:2 of 3 → frac 0.5
    h.frame(input::press(from));
    h.frame(input::move_to(from + egui::vec2(30.0, 20.0)));
    match &h.state.drag_mode {
        rustylink::editor::state::DragMode::Connection {
            src_sid,
            src_port_index,
            current_y,
            ..
        } => {
            assert_eq!(src_sid.as_str(), "1");
            assert_eq!(*src_port_index, 2, "drag sourced from out:2");
            // current_y is MODEL-space; out:2 of 3 anchor = y 60 (0.5*120),
            // pointer moved +20px (~+3 model) → expect ~60-80.
            assert!(
                (*current_y - 60.0).abs() < 30.0,
                "tip tracks near out:2 anchor, got {current_y}"
            );
        }
        other => panic!("expected Connection drag mode, got {other:?}"),
    }
    h.frame(input::release(from + egui::vec2(30.0, 20.0)));
    h.idle(1);
}
