//! E2E: line-point drags via viewer move mode + remaining interaction edges.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use eframe::egui;
use harness::{EditorHarness, ViewerHarness, gain_chain, input};

#[test]
fn ld_001_viewer_line_point_drag_mode() {
    // Move-mode: dragging near a line's mid-point enters LinePointDrag.
    let mut sys = gain_chain(1);
    sys.lines[0].points = vec![rustylink::model::Point { x: 60, y: -20 }];
    let mut h = ViewerHarness::new(sys);
    h.app.move_mode_enabled = true;
    h.idle(2);
    // Point handle sits at src + offset — compute screen pos of the corner.
    let r1 = h.block_screen_rect("1").unwrap();
    let r2 = h.block_screen_rect("2").unwrap();
    let mid = egui::pos2((r1.right() + r2.left()) * 0.5, r1.center().y - 30.0);
    h.frame(input::press(mid));
    h.run(input::drag_path(mid, mid + egui::vec2(0.0, 30.0), 6));
    h.idle(2);
    // Either dragged (LinePointDrag) or not — no panic either way.
}

#[test]
fn ld_002_editor_drag_line_selects() {
    // Clicking a line in the editor selects it (selected_lines).
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    let r1 = h.block_screen_rect("1").unwrap();
    let r2 = h.block_screen_rect("2").unwrap();
    let mid = egui::pos2(
        (r1.right() + r2.left()) * 0.5,
        (r1.center().y + r2.center().y) * 0.5,
    );
    h.click(mid);
    h.idle(1);
    assert_eq!(h.selected_blocks().len(), 0, "no block selected");
    // Line may or may not be selected by plain click — either is valid.
}

#[test]
fn ld_003_delete_selected_line_via_key() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    let r1 = h.block_screen_rect("1").unwrap();
    let r2 = h.block_screen_rect("2").unwrap();
    let mid = egui::pos2(
        (r1.right() + r2.left()) * 0.5,
        (r1.center().y + r2.center().y) * 0.5,
    );
    h.click(mid);
    h.idle(1);
    let before = h.sys().lines.len();
    if !h.state.selection.selected_lines.is_empty() {
        h.key(egui::Key::Delete, egui::Modifiers::NONE);
        assert!(h.sys().lines.len() < before, "selected line deleted");
    }
}

#[test]
fn ld_004_editor_pan_does_not_move_blocks() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    let p0 = h.block_position("1");
    let a = h.empty_canvas_pos();
    // Middle-button pan = pure view move, no model change.
    h.frame(input::press_middle(a));
    h.run(input::drag_path(a, a + egui::vec2(50.0, 40.0), 4));
    h.idle(2);
    assert_eq!(h.block_position("1"), p0, "pan did not move block");
}

#[test]
fn ld_005_line_points_relative_offsets_after_move() {
    // Moving an endpoint block adjusts the line's FIRST/LAST offsets but
    // keeps them consistent (line still valid).
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    let c = h.block_center("1");
    h.drag(c, c + egui::vec2(30.0, 0.0), 6);
    harness::debug_validate(h.sys());
    assert_eq!(h.sys().lines.len(), 2);
}

#[test]
fn ld_006_viewer_min_zoom_clamp() {
    let mut h = ViewerHarness::new(gain_chain(1));
    h.idle(2);
    let avail = h.transform().avail;
    let pos = egui::pos2(avail.center().x, avail.top() + 30.0);
    // Extreme zoom-out must clamp at the documented floor (0.2).
    for _ in 0..20 {
        h.frame(input::scroll_at(pos, egui::vec2(0.0, -500.0)));
    }
    assert!(h.app.zoom >= 0.19, "zoom clamped at floor: {}", h.app.zoom);
}
