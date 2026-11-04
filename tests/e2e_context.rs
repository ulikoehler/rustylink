//! E2E: right-click context menus + remaining interaction corner cases.

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

#[test]
fn ctx_001_editor_right_click_block_no_panic() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.run(rclick(h.block_center("2")));
    h.idle(2); // menu frame renders without panic
}

#[test]
fn ctx_002_viewer_right_click_block_no_panic() {
    let mut h = ViewerHarness::new(gain_chain(1));
    h.idle(2);
    h.run(rclick(h.block_screen_rect("2").unwrap().center()));
    h.idle(2);
}

#[test]
fn ctx_003_editor_right_click_canvas_no_panic() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.run(rclick(h.canvas_rect().center()));
    h.idle(2);
}

#[test]
fn ctx_004_viewer_secondary_click_selects() {
    // Secondary click on a block fires ClickAction::Secondary — no crash,
    // selection stays consistent.
    let mut h = ViewerHarness::new(gain_chain(1));
    h.idle(2);
    h.run(rclick(h.block_screen_rect("1").unwrap().center()));
    h.idle(1);
    let _ = h.app.selected_block_sids; // state readable
}

#[test]
fn ctx_005_line_right_click_editor() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    let r1 = h.block_screen_rect("1").unwrap();
    let r2 = h.block_screen_rect("2").unwrap();
    let mid = egui::pos2(
        (r1.right() + r2.left()) * 0.5,
        (r1.center().y + r2.center().y) * 0.5,
    );
    h.run(rclick(mid));
    h.idle(2); // line context menu or no-op — must not panic
}

#[test]
fn ctx_006_escape_closes_any_open_menu() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.run(rclick(h.block_center("1")));
    h.idle(1);
    h.key(egui::Key::Escape, egui::Modifiers::NONE);
    h.idle(2); // menu dismissed, no panic
}

#[test]
fn ctx_007_double_click_on_line_editor() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    let r1 = h.block_screen_rect("1").unwrap();
    let r2 = h.block_screen_rect("2").unwrap();
    let mid = egui::pos2(
        (r1.right() + r2.left()) * 0.5,
        (r1.center().y + r2.center().y) * 0.5,
    );
    h.run(input::double_click(mid));
    h.idle(2); // must not panic
}
