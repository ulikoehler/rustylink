//! E2E: keyboard shortcuts through real input, dangling-endpoint edges.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use eframe::egui;
use harness::{BlockBuilder, EditorHarness, SystemBuilder, ViewerHarness, gain_chain};
use rustylink::connection_targets::ConnectionTargetResolver;

#[test]
fn kb_001_ctrl_z_undoes_move() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    let p0 = h.block_position("1");
    let c = h.block_center("1");
    h.drag(c, c + egui::vec2(60.0, 0.0), 6);
    assert_ne!(h.block_position("1"), p0);
    h.key(egui::Key::Z, egui::Modifiers::CTRL);
    h.idle(1);
    assert_eq!(h.block_position("1"), p0, "Ctrl+Z undid the move");
}

#[test]
fn kb_002_ctrl_y_redoes() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    let c = h.block_center("1");
    h.drag(c, c + egui::vec2(60.0, 0.0), 6);
    let p_after = h.block_position("1");
    h.key(egui::Key::Z, egui::Modifiers::CTRL);
    h.key(egui::Key::Y, egui::Modifiers::CTRL);
    h.idle(1);
    assert_eq!(h.block_position("1"), p_after, "Ctrl+Y redid the move");
}

#[test]
fn kb_003_a_key_opens_block_browser() {
    // Bare 'A' (and Ctrl+A, same key_pressed match) opens the block browser.
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.key(egui::Key::A, egui::Modifiers::NONE);
    h.idle(1);
    assert!(h.state.block_browser.open, "'A' opens block browser");
    h.key(egui::Key::Escape, egui::Modifiers::NONE);
    h.idle(1);
}

#[test]
fn kb_004_shift_arrows_group_move() {
    // Multiple selected + arrow key moves them together.
    let mut h = EditorHarness::new(gain_chain(2));
    h.idle(2);
    h.run(harness::input::ctrl_click(h.block_center("1")));
    h.run(harness::input::ctrl_click(h.block_center("2")));
    assert_eq!(h.selected_blocks().len(), 2);
    let p1 = h.block_position("1");
    let p2 = h.block_position("2");
    h.key(egui::Key::ArrowRight, egui::Modifiers::NONE);
    h.idle(1);
    assert_ne!(h.block_position("1"), p1);
    assert_ne!(h.block_position("2"), p2);
}

#[test]
fn kb_005_dangling_dst_only_line_safe() {
    // Line with only dst set (no src) — resolver + render must not panic.
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Display", "D", "1")
                .pos(0, 0, 40, 24)
                .ports(1, 0)
                .build(),
        )
        .line(
            harness::model_builders::LineBuilder::new()
                .to("1", 1)
                .build(),
        )
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    let _ = res.line_targets_for_line(&[], &sys.lines[0]);
    let mut h = EditorHarness::new(sys);
    h.idle(2);
}

#[test]
fn kb_006_subsystem_flag_empty_sys() {
    // Block with `subsystem=true` but empty inline system — render safe.
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("SubSystem", "Empty", "1")
                .pos(0, 0, 80, 40)
                .ports(1, 1)
                .subsystem(SystemBuilder::new("").build())
                .build(),
        )
        .build();
    let mut h = ViewerHarness::new(sys);
    h.idle(3);
}

#[test]
fn kb_007_arrow_without_selection_no_panic() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    for k in [
        egui::Key::ArrowLeft,
        egui::Key::ArrowUp,
        egui::Key::ArrowDown,
    ] {
        h.key(k, egui::Modifiers::NONE);
        h.idle(1);
    }
}
