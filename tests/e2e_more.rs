//! E2E: remaining verified-API cases — invalid drops, viewer toggles, edges.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use eframe::egui;
use harness::{EditorHarness, ViewerHarness, gain_chain, input, live_feed};

#[test]
fn m_001_out_to_out_drop_no_line() {
    // Drawing out→out is invalid — no line created.
    let mut h = EditorHarness::new(gain_chain(2));
    h.idle(2);
    let before = h.sys().lines.len();
    let a = h.port_hit_pos("1", "out", 1);
    let b = h.port_hit_pos("4", "out", 1);
    h.drag(a, b, 6);
    h.idle(1);
    assert_eq!(h.sys().lines.len(), before, "out→out drop rejected");
}

#[test]
fn m_002_connection_esc_aborts() {
    // Esc during connection drag aborts — no line.
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    let a = h.port_hit_pos("1", "out", 1);
    h.frame(input::press(a));
    h.frame(input::move_to(a + egui::vec2(20.0, 20.0)));
    h.key(egui::Key::Escape, egui::Modifiers::NONE);
    h.frame(input::release(a + egui::vec2(20.0, 20.0)));
    h.idle(1);
    assert_eq!(
        h.sys().lines.len(),
        2,
        "esc-aborted connection left no line"
    );
}

#[test]
fn m_003_viewer_move_mode_toggle() {
    let mut h = ViewerHarness::new(gain_chain(1));
    h.app.move_mode_enabled = true;
    h.idle(1);
    assert!(h.app.move_mode_enabled);
    h.app.move_mode_enabled = false;
    h.idle(2);
    assert!(!h.app.move_mode_enabled);
}

#[test]
fn m_004_live_mode_click_no_toggle_without_live() {
    // ManualSwitch click WITHOUT live mode — setting must NOT toggle.
    let sys = harness::SystemBuilder::new("model")
        .block(
            harness::BlockBuilder::new("ManualSwitch", "SW", "1")
                .pos(0, 0, 60, 40)
                .ports(2, 1)
                .build(),
        )
        .build();
    let mut h = ViewerHarness::new(sys);
    h.idle(2);
    h.run(input::click(h.block_screen_rect("1").unwrap().center()));
    h.idle(1);
    assert_eq!(
        h.app.current_system().unwrap().blocks[0]
            .current_setting
            .as_deref(),
        None,
        "no toggle without live mode"
    );
}

#[test]
fn m_005_live_key_inserted_appears_in_map() {
    let mut h = ViewerHarness::new(gain_chain(1));
    h.app
        .live_values
        .insert("2:in:1".into(), live_feed::scalar(7.5));
    h.idle(1);
    assert!(h.app.live_values.contains_key("2:in:1"));
}

#[test]
fn m_006_double_paste_distinct_positions() {
    // Two pastes land at offset positions — not stacked.
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.click(h.block_center("2"));
    h.key(egui::Key::C, egui::Modifiers::CTRL);
    h.key(egui::Key::V, egui::Modifiers::CTRL);
    h.key(egui::Key::V, egui::Modifiers::CTRL);
    let positions: Vec<_> = h.sys().blocks[3..]
        .iter()
        .filter_map(|b| harness::parse_pos(b.position.as_deref().unwrap()))
        .collect();
    assert!(positions.len() >= 2);
    assert_ne!(positions[0], positions[1], "pastes offset, not stacked");
}
