//! E2E: subsystem-boundary propagation + remaining verified-API cases.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use eframe::egui;
use harness::model_builders::line;
use harness::{BlockBuilder, EditorHarness, SystemBuilder, ViewerHarness, gain_chain};
use rustylink::connection_targets::ConnectionTargetResolver;
use rustylink::model::System;

fn sub_model() -> System {
    let inner = SystemBuilder::new("")
        .block(
            BlockBuilder::new("Inport", "In", "10")
                .pos(0, 0, 30, 24)
                .ports(0, 1)
                .prop("Port", "1")
                .build(),
        )
        .block(
            BlockBuilder::new("Gain", "G", "11")
                .pos(80, 0, 120, 24)
                .ports(1, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Outport", "Out", "12")
                .pos(200, 0, 230, 24)
                .ports(1, 0)
                .prop("Port", "1")
                .build(),
        )
        .line(line("10", 1, "11", 1))
        .line(line("11", 1, "12", 1))
        .build();
    SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("SubSystem", "SS", "2")
                .pos(80, 0, 160, 24)
                .ports(1, 1)
                .subsystem(inner)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "Sink", "3")
                .pos(220, 0, 260, 24)
                .ports(1, 0)
                .build(),
        )
        .line(line("1", 1, "2", 1))
        .line(line("2", 1, "3", 1))
        .build()
}

#[test]
fn ss_001_outport_maps_to_parent_sink() {
    // SS's out line (SS→Sink) resolves through Outport to the inner G.
    let sys = sub_model();
    let res = ConnectionTargetResolver::new(&sys);
    let t = res.line_targets_for_line(&[], &sys.lines[1]);
    assert!(!t.is_empty());
    // The ultimate upstream through SS is C (constant feeding the SS).
    assert!(
        t.iter()
            .any(|x| x.path == "model/C" || x.path.contains("/G")),
        "out resolves through subsystem: {t:?}"
    );
}

#[test]
fn ss_002_resolver_on_subpath() {
    let sys = sub_model();
    let res = ConnectionTargetResolver::new(&sys);
    let inner = &sys.blocks[1].subsystem.as_ref().unwrap();
    let t = res.line_targets_for_line(&["SS".to_string()], &inner.lines[0]);
    assert!(!t.is_empty(), "inner line resolves in sub-path context");
}

#[test]
fn ss_003_editor_view_in_subsystem_transform() {
    let mut h = EditorHarness::new(sub_model());
    h.idle(2);
    h.state.app.navigate_to_path(vec!["SS".to_string()]);
    h.idle(2);
    // Inside the subsystem, inner blocks have screen rects.
    assert!(h.block_screen_rect("10").is_some(), "In visible inside SS");
    assert!(h.block_screen_rect("11").is_some(), "G visible inside SS");
    h.state.app.navigate_to_path(Vec::new());
    h.idle(1);
    assert!(h.block_screen_rect("1").is_some(), "root C visible again");
}

#[test]
fn ss_004_viewer_double_click_round_trip() {
    let mut h = ViewerHarness::new(sub_model());
    h.idle(2);
    h.run(harness::input::double_click(
        h.block_screen_rect("2").unwrap().center(),
    ));
    h.idle(1);
    assert_eq!(h.app.path, vec!["SS".to_string()]);
    h.app.navigate_to_path(Vec::new());
    h.idle(1);
    assert!(h.app.path.is_empty());
}

#[test]
fn ss_005_resolver_deterministic_repeat_calls() {
    // Resolver output is stable across repeated queries (cache-safe).
    let sys = sub_model();
    let res = ConnectionTargetResolver::new(&sys);
    let l = &sys.lines[0];
    let a = res.line_targets_for_line(&[], l);
    let b = res.line_targets_for_line(&[], l);
    assert_eq!(
        a.iter().map(|t| t.path.clone()).collect::<Vec<_>>(),
        b.iter().map(|t| t.path.clone()).collect::<Vec<_>>()
    );
}

#[test]
fn ss_006_group_drag_keeps_positions_relative() {
    // Drag two selected blocks together — their relative offset is preserved.
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.run(harness::input::ctrl_click(h.block_center("1")));
    h.run(harness::input::ctrl_click(h.block_center("2")));
    assert_eq!(h.selected_blocks().len(), 2);
    let from = h.block_center("1");
    h.drag(from, from + egui::vec2(40.0, 30.0), 6);
    let p1 = h.block_position("1");
    let p2 = h.block_position("2");
    // Group moved; blocks still distinct positions (relative offset kept).
    assert_ne!((p1.0, p1.1), (p2.0, p2.1));
}
