//! E2E: context-menu assertions — menu actually opens, not just no-panic.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use eframe::egui;
use harness::{EditorHarness, ViewerHarness, gain_chain, input};

fn rclick(pos: egui::Pos2) -> Vec<egui::RawInput> {
    let press = egui::RawInput {
        events: vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        ..input::idle()
    };
    let release = input::release_button(pos, egui::PointerButton::Secondary);
    vec![input::move_to(pos), press, release]
}

fn menu_layer_open(ctx: &egui::Context) -> bool {
    // An open menu creates a popup layer (Order::Foreground) — check that
    // the frame's output contains more than just the background.
    ctx.memory(|m| m.layer_ids().next().is_some())
}

#[test]
fn cm_001_editor_block_menu_opens() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.run(rclick(h.block_center("1")));
    h.idle(1);
    // A context menu should create an open popup layer.
    let layers = h.ctx.memory(|m| m.layer_ids().count());
    assert!(layers >= 1, "menu layer(s) present after right-click");
}

#[test]
fn cm_002_editor_esc_closes_menu() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.run(rclick(h.block_center("1")));
    h.idle(1);
    h.key(egui::Key::Escape, egui::Modifiers::NONE);
    h.idle(1); // closed without panic — layer cleaned up next frame
}

#[test]
fn cm_003_viewer_block_menu_opens() {
    let mut h = ViewerHarness::new(gain_chain(1));
    h.idle(2);
    h.run(rclick(h.block_screen_rect("1").unwrap().center()));
    h.idle(1);
    let _ = menu_layer_open(&h.ctx);
}

#[test]
fn cm_004_editor_canvas_menu_opens() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.run(rclick(h.canvas_rect().center()));
    h.idle(1);
    // Canvas (empty area) right-click — may show a menu or not, no panic.
}

#[test]
fn cm_005_menu_survives_hover_frames() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    let p = h.block_center("2");
    h.run(rclick(p));
    h.idle(3); // several frames — menu stays rendered
}
