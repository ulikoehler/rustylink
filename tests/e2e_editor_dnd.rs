//! E2E: editor drag-and-drop via synthesized egui input (§10).
//! Covers `DragMode::{Blocks, Connection, SelectionRect, Resize, Pan}`.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use eframe::egui::{self, Pos2};
use harness::{BlockBuilder, EditorHarness, SystemBuilder, gain_chain, input};
use rustylink::editor::state::DragMode;
use rustylink::model::System;

/// Two unconnected blocks for connection-drag tests.
fn two_blocks() -> System {
    SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .value("1")
                .build(),
        )
        .block(
            BlockBuilder::new("Scope", "S", "2")
                .pos(200, 0, 240, 24)
                .ports(1, 0)
                .build(),
        )
        .build()
}

// ────────────────────────────────────────────────────────────────────────────
// DragMode::Blocks
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn ed_001_drag_single_block() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    let before = h.block_position("2");
    let from = h.block_center("2");
    h.drag(from, from + egui::vec2(60.0, 20.0), 8);

    let after = h.block_position("2");
    assert!(after.0 > before.0 && after.1 > before.1);
    assert!(h.state.history.can_undo());
    assert!(h.drag_mode_is_none());
}

#[test]
fn ed_002_drag_snaps_to_grid() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    // grid_size default = 5
    let before = h.block_position("2");
    let from = h.block_center("2");
    h.drag(from, from + egui::vec2(60.0, 20.0), 8);
    let after = h.block_position("2");
    assert_eq!(
        (after.0 - before.0) % 5,
        0,
        "x snapped to grid: {before:?} -> {after:?}"
    );
    assert_eq!((after.1 - before.1) % 5, 0);
}

#[test]
fn ed_003_drag_selected_group_moves_all() {
    let mut h = EditorHarness::new(gain_chain(2));
    h.idle(2);
    // Select both gains via ctrl-clicks.
    h.click(h.block_center("2"));
    h.run(input::ctrl_click(h.block_center("3")));
    assert_eq!(h.selected_blocks().len(), 2);

    let p2 = h.block_position("2");
    let p3 = h.block_position("3");
    let from = h.block_center("2");
    h.drag(from, from + egui::vec2(40.0, 0.0), 6);

    let q2 = h.block_position("2");
    let q3 = h.block_position("3");
    assert!(q2.0 > p2.0, "dragged block moved: {p2:?} -> {q2:?}");
    assert!(q3.0 > p3.0, "group member moved: {p3:?} -> {q3:?}");
}

#[test]
fn ed_005_click_without_drag_selects_only() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.click(h.block_center("2"));
    assert_eq!(h.selected_blocks(), &[1]);
    assert!(!h.state.history.can_undo(), "click emits no command");
    assert!(!h.state.dirty);
}

#[test]
fn ed_046_click_empty_space_clears_selection() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.click(h.block_center("2"));
    assert_eq!(h.selected_blocks().len(), 1);
    h.click(h.empty_canvas_pos());
    assert!(h.selected_blocks().is_empty());
}

#[test]
fn ed_044_ctrl_click_toggles_selection() {
    let mut h = EditorHarness::new(gain_chain(2));
    h.idle(2);
    h.click(h.block_center("2"));
    h.run(input::ctrl_click(h.block_center("3")));
    assert_eq!(h.selected_blocks().len(), 2);
    h.run(input::ctrl_click(h.block_center("3")));
    assert_eq!(h.selected_blocks(), &[1]);
}

// ────────────────────────────────────────────────────────────────────────────
// DragMode::Connection
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn ed_020_draw_line_out_to_in() {
    let mut h = EditorHarness::new(two_blocks());
    h.idle(2);

    let from = h.port_hit_pos("1", "out", 1);
    let to = h.port_hit_pos("2", "in", 1);
    h.drag(from, to, 8);

    let sys = h.sys();
    assert_eq!(sys.lines.len(), 1, "one line created");
    let l = &sys.lines[0];
    assert_eq!(l.src.as_ref().unwrap().sid, "1");
    assert_eq!(l.src.as_ref().unwrap().port_type, "out");
    assert_eq!(l.dst.as_ref().unwrap().sid, "2");
    assert_eq!(l.dst.as_ref().unwrap().port_type, "in");
    assert!(h.state.history.can_undo());
    assert!(h.state.dirty);
    assert!(h.drag_mode_is_none());
}

#[test]
fn ed_021_draw_line_in_to_out_reversed() {
    let mut h = EditorHarness::new(two_blocks());
    h.idle(2);
    // Start on the input port, drag to the output port — src/dst normalize.
    let from = h.port_hit_pos("2", "in", 1);
    let to = h.port_hit_pos("1", "out", 1);
    h.drag(from, to, 8);

    let sys = h.sys();
    assert_eq!(sys.lines.len(), 1);
    let l = &sys.lines[0];
    assert_eq!(l.src.as_ref().unwrap().sid, "1", "out side became src");
    assert_eq!(l.dst.as_ref().unwrap().sid, "2", "in side became dst");
}

#[test]
fn ed_023_drop_on_empty_canvas_no_line() {
    let mut h = EditorHarness::new(two_blocks());
    h.idle(2);
    let from = h.port_hit_pos("1", "out", 1);
    let void = h.empty_canvas_pos();
    h.drag(from, void, 8);
    assert!(h.sys().lines.is_empty(), "no line on void drop");
    assert!(h.drag_mode_is_none());
}

#[test]
fn ed_024_drop_on_invalid_port_kind_rejected() {
    // Two Constant blocks — both only have out ports.
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C1", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Constant", "C2", "2")
                .pos(200, 0, 240, 24)
                .ports(0, 1)
                .build(),
        )
        .build();
    let mut h = EditorHarness::new(sys);
    h.idle(2);
    let from = h.port_hit_pos("1", "out", 1);
    let to = h.port_hit_pos("2", "out", 1);
    h.drag(from, to, 8);
    assert!(h.sys().lines.is_empty(), "out→out rejected");
}

#[test]
fn ed_028_esc_during_connection_drag() {
    let mut h = EditorHarness::new(two_blocks());
    h.idle(2);
    let from = h.port_hit_pos("1", "out", 1);
    h.frame(input::press(from));
    h.frame(input::move_to(from + egui::vec2(30.0, 10.0)));
    assert!(
        matches!(h.state.drag_mode, DragMode::Connection { .. }),
        "connection armed"
    );
    h.frame(input::esc());
    h.frame(input::release(from + egui::vec2(30.0, 10.0)));
    h.idle(1);
    assert!(h.sys().lines.is_empty(), "Esc aborted connection");
}

// ────────────────────────────────────────────────────────────────────────────
// DragMode::SelectionRect (marquee) — Shift + drag on background
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn ed_040_marquee_selects_enclosed_blocks() {
    let mut h = EditorHarness::new(gain_chain(2));
    h.idle(2);
    // Drag a screen rect covering G0 and G1.
    let r2 = h.block_screen_rect("2").unwrap();
    let r3 = h.block_screen_rect("3").unwrap();
    let a = Pos2::new(r2.left() - 10.0, r2.top() - 10.0);
    let b = Pos2::new(r3.right() + 10.0, r3.bottom() + 10.0);
    h.drag_with(a, b, 6, egui::Modifiers::SHIFT);
    assert_eq!(h.selected_blocks(), &[1, 2], "both gains selected");
    assert!(h.drag_mode_is_none());
}

#[test]
fn ed_040b_marquee_excludes_outside_blocks() {
    let mut h = EditorHarness::new(gain_chain(2));
    h.idle(2);
    // Rect covers only G0.
    let r2 = h.block_screen_rect("2").unwrap();
    let a = Pos2::new(r2.left() - 10.0, r2.top() - 10.0);
    let b = Pos2::new(r2.right() + 10.0, r2.bottom() + 10.0);
    h.drag_with(a, b, 6, egui::Modifiers::SHIFT);
    assert_eq!(h.selected_blocks(), &[1]);
}

// ────────────────────────────────────────────────────────────────────────────
// DragMode::Resize
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn ed_050_resize_br_handle_grows() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    // Select the block first — handles only exist when selected.
    h.click(h.block_center("2"));
    let (l, t, r, b) = h.block_position("2");
    let br = h.resize_handle_pos("2", 4); // BR
    h.drag(br, br + egui::vec2(30.0, 20.0), 6);

    let (l2, t2, r2, b2) = h.block_position("2");
    assert_eq!((l2, t2), (l, t), "top-left anchor fixed");
    assert!(
        r2 > r && b2 > b,
        "bottom-right grew: ({l},{t},{r},{b}) -> ({l2},{t2},{r2},{b2})"
    );
    assert!(h.state.history.can_undo());
}

#[test]
fn ed_051_resize_tl_handle_grows_up_left() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.click(h.block_center("2"));
    let (l, t, r, b) = h.block_position("2");
    let tl = h.resize_handle_pos("2", 0);
    h.drag(tl, tl + egui::vec2(-30.0, -20.0), 6);

    let (l2, t2, r2, b2) = h.block_position("2");
    assert_eq!((r2, b2), (r, b), "bottom-right anchor fixed");
    assert!(l2 < l && t2 < t, "top-left grew out");
}

// ────────────────────────────────────────────────────────────────────────────
// DragMode::Pan
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn ed_060_drag_background_pans() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    let pan0 = h.state.app.pan;
    let p = h.empty_canvas_pos();
    h.drag(p, p + egui::vec2(50.0, 30.0), 6);
    assert_ne!(h.state.app.pan, pan0, "pan updated");
    assert!(!h.state.dirty, "pan is not a model edit");
    assert!(h.drag_mode_is_none());
}

// ────────────────────────────────────────────────────────────────────────────
// Undo/redo integrity
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn ed_121_undo_restores_after_drag() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    let before = h.block_position("2");
    let from = h.block_center("2");
    h.drag(from, from + egui::vec2(50.0, 0.0), 6);
    assert_ne!(h.block_position("2"), before);

    h.state.undo();
    assert_eq!(h.block_position("2"), before, "undo restored position");
    h.state.redo();
    assert_ne!(h.block_position("2"), before, "redo re-applied move");
}

#[test]
fn ed_121b_undo_removes_drawn_line() {
    let mut h = EditorHarness::new(two_blocks());
    h.idle(2);
    h.drag(
        h.port_hit_pos("1", "out", 1),
        h.port_hit_pos("2", "in", 1),
        8,
    );
    assert_eq!(h.sys().lines.len(), 1);
    h.state.undo();
    assert!(h.sys().lines.is_empty(), "undo removed line");
}

// ────────────────────────────────────────────────────────────────────────────
// Lines attached to dragged blocks
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn ed_007_dragging_block_keeps_line_attached() {
    // C --line--> Gain --line--> S  (gain_chain(1))
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    let from = h.block_center("2");
    h.drag(from, from + egui::vec2(0.0, 80.0), 8);

    // The Gain moved; its lines' endpoints must still reference it and the
    // routing must stay orthogonal (line point adjustment is geometry-only).
    let sys = h.sys();
    for l in &sys.lines {
        assert!(harness::assert_line_orthogonal(l), "line stays orthogonal");
    }
    assert_eq!(sys.lines[0].dst.as_ref().unwrap().sid, "2");
    assert_eq!(sys.lines[1].src.as_ref().unwrap().sid, "2");
}
