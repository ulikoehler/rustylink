//! E2E: remaining cases — annotations, port edge cases, editor ops edges.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use eframe::egui;
use harness::{BlockBuilder, EditorHarness, SystemBuilder, ViewerHarness, gain_chain};
use rustylink::connection_targets::ConnectionTargetResolver;

#[test]
fn cp_001_annotation_block_parses_and_renders() {
    // Annotation blocks (no ports) must render + not break the resolver.
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Note", "Note1", "")
                .pos(50, 50, 150, 80)
                .build(),
        )
        .build();
    let mut h = ViewerHarness::new(sys);
    h.idle(2);
    let res = ConnectionTargetResolver::new(&h.app.root);
    let _ = res;
}

#[test]
fn cp_002_block_with_4_in_ports_hit_positions() {
    // 4-in block: all four port hit positions distinct + ordered.
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Gain", "G", "1")
                .pos(0, 0, 40, 120)
                .ports(4, 1)
                .build(),
        )
        .build();
    let mut h = EditorHarness::new(sys);
    h.idle(2);
    let positions: Vec<_> = (1..=4).map(|i| h.port_hit_pos("1", "in", i)).collect();
    for i in 1..4 {
        assert!(
            positions[i - 1].y < positions[i].y,
            "port {i} above port {}",
            i + 1
        );
    }
}

#[test]
fn cp_003_out_port_drawn_before_in_no_panic() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    // Drag starting at an out port to a non-port point — rejected, no line.
    let from = h.port_hit_pos("1", "out", 1);
    let to = h.canvas_rect().center();
    h.drag(from, to, 6);
    h.idle(1);
    assert_eq!(h.sys().lines.len(), 2, "drop on empty canvas adds nothing");
}

#[test]
fn cp_004_line_endpoints_not_changed_by_move() {
    // Moving a block never rewrites line endpoints (SID attachment).
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    let c = h.block_center("2");
    h.drag(c, c + egui::vec2(80.0, 40.0), 6);
    let l = &h.sys().lines[0];
    assert_eq!(l.src.as_ref().unwrap().sid, "1");
    assert_eq!(l.dst.as_ref().unwrap().sid, "2");
}

#[test]
fn cp_005_deep_copy_paste_inside_subsystem() {
    // Copy/paste a block INSIDE a subsystem — stays inside, unique SID.
    let inner = SystemBuilder::new("")
        .block(
            BlockBuilder::new("Gain", "Inner", "10")
                .pos(0, 0, 40, 24)
                .ports(1, 1)
                .build(),
        )
        .build();
    let root = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("SubSystem", "SS", "2")
                .pos(80, 0, 160, 24)
                .ports(1, 1)
                .subsystem(inner)
                .build(),
        )
        .build();
    let mut h = EditorHarness::new(root);
    h.idle(2);
    h.state.app.navigate_to_path(vec!["SS".to_string()]);
    h.idle(2);
    h.click(h.block_center("10"));
    h.key(egui::Key::C, egui::Modifiers::CTRL);
    h.key(egui::Key::V, egui::Modifiers::CTRL);
    let inner_sys = h.sys();
    assert_eq!(inner_sys.blocks.len(), 2, "pasted inside subsystem");
    harness::debug_validate(&h.state.app.root);
}

#[test]
fn cp_006_high_fanout_20_branches() {
    // 1 source → 20 branch sinks — validator + resolver scale.
    let mut b = SystemBuilder::new("model").block(
        BlockBuilder::new("Constant", "S", "1")
            .pos(0, 0, 40, 24)
            .ports(0, 1)
            .build(),
    );
    let mut lb = harness::model_builders::LineBuilder::new().from("1", 1);
    for i in 0..20usize {
        let sid = (i + 2).to_string();
        b = b.block(
            BlockBuilder::new("Display", &format!("D{i}"), &sid)
                .pos(100, i as i32 * 30, 140, i as i32 * 30 + 24)
                .ports(1, 0)
                .build(),
        );
        lb = lb.branch_to(&sid, 1);
    }
    let sys = b.line(lb.build()).build();
    harness::debug_validate(&sys);
    let res = ConnectionTargetResolver::new(&sys);
    let t = res.line_targets_for_line(&[], &sys.lines[0]);
    assert!(t.iter().all(|x| x.path == "model/S"));
}
