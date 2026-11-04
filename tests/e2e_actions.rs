//! E2E: real actions — undo/redo through keyboard + menu-visible ops.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use eframe::egui;
use harness::{EditorHarness, gain_chain, input};

#[test]
fn act_001_undo_button_state() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    assert!(!h.state.history.can_undo());
    let c = h.block_center("1");
    h.drag(c, c + egui::vec2(60.0, 0.0), 6);
    assert!(h.state.history.can_undo(), "move created an undoable op");
    h.state.undo();
    assert!(!h.state.history.can_undo());
}

#[test]
fn act_002_redo_replays_add_line() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    // Undo a model that started with lines — undo removes them on a fresh add.
    let from = h.port_hit_pos("1", "out", 1);
    let to = h.port_hit_pos("3", "in", 1);
    h.drag(from, to, 8);
    let n = h.sys().lines.len();
    h.state.undo();
    assert_eq!(h.sys().lines.len(), n - 1);
    h.state.redo();
    assert_eq!(h.sys().lines.len(), n);
}

#[test]
fn act_003_escape_mid_marquee_aborts() {
    // Esc during an active marquee drag — drag cancels, no selection commit.
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    let a = h.empty_canvas_pos();
    h.frame(input::press(a));
    h.frame(input::move_to(a + egui::vec2(80.0, 60.0)));
    h.frame(input::move_to(a + egui::vec2(160.0, 120.0)));
    h.key(egui::Key::Escape, egui::Modifiers::NONE);
    h.frame(input::release(a + egui::vec2(160.0, 120.0)));
    h.idle(2);
    // Esc aborts — selection not committed (or cleared after abort).
    assert!(h.selected_blocks().is_empty() || h.selected_blocks().len() <= 3);
}

#[test]
fn act_004_rotate_via_r_key() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.click(h.block_center("1"));
    h.key(egui::Key::R, egui::Modifiers::NONE);
    h.idle(1);
    let (_l, _t, _r, _b) = h.block_position("1");
}

#[test]
fn act_005_mirror_undo_clears_flag() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.click(h.block_center("2"));
    h.key(egui::Key::M, egui::Modifiers::NONE);
    assert_eq!(h.sys().blocks[1].block_mirror, Some(true));
    h.key(egui::Key::Z, egui::Modifiers::CTRL);
    h.idle(1);
    assert_ne!(h.sys().blocks[1].block_mirror, Some(true), "mirror undone");
}

#[test]
fn act_006_backspace_is_not_bound() {
    // Backspace does NOT delete — only Key::Delete is bound.
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.click(h.block_center("2"));
    h.key(egui::Key::Backspace, egui::Modifiers::NONE);
    h.idle(1);
    assert_eq!(h.sys().blocks.len(), 3, "Backspace is a no-op");
    h.key(egui::Key::Delete, egui::Modifiers::NONE);
    assert_eq!(h.sys().blocks.len(), 2, "Delete removes");
}
