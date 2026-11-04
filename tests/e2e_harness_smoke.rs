//! Proof-of-life for the E2E harness (M1): build a model, render headless
//! frames, verify the canvas transform exists, and perform one real drag.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use harness::{EditorHarness, gain_chain};

#[test]
fn harness_smoke_renders_and_exposes_transform() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    // The renderer stored the transform it actually used.
    let t = h.transform();
    assert!(t.scale() > 0.0);
    // Both blocks landed inside the canvas viewport.
    let r_in = h.block_screen_rect("1").expect("In1 drawn");
    let r_out = h.block_screen_rect("3").expect("Out1 drawn");
    assert!(t.avail.intersects(r_in));
    assert!(t.avail.intersects(r_out));
}

#[test]
fn harness_smoke_block_drag_moves_block() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    let before = h.block_position("2");
    let from = h.block_center("2");
    // Drag the block 60px right / 20px down on screen.
    h.drag(from, from + eframe::egui::vec2(60.0, 20.0), 8);

    let after = h.block_position("2");
    assert!(
        after.0 > before.0,
        "block moved right: {before:?} -> {after:?}"
    );
    assert!(h.state.history.can_undo(), "drag produced a command");
    assert!(h.state.dirty, "drag marked model dirty");
    assert!(h.drag_mode_is_none(), "drag mode reset to None");
}

#[test]
fn harness_smoke_click_selects_block() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.click(h.block_center("2"));
    assert_eq!(h.selected_blocks(), &[1], "Gain block (index 1) selected");
}
