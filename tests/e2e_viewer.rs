//! E2E: viewer interactions — selection, zoom, pan, move mode, navigation (§11).

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use eframe::egui::{self, Pos2};
use harness::{BlockBuilder, SystemBuilder, ViewerHarness, gain_chain, input};
use rustylink::model::System;

fn nested() -> System {
    let inner = SystemBuilder::new("")
        .block(
            BlockBuilder::new("Gain", "InnerG", "10")
                .pos(80, 0, 120, 24)
                .ports(1, 1)
                .build(),
        )
        .build();
    SystemBuilder::new("model")
        .block(
            BlockBuilder::new("SubSystem", "SS", "2")
                .pos(100, -10, 200, 34)
                .ports(1, 1)
                .subsystem(inner)
                .build(),
        )
        .build()
}

#[test]
fn vw_001_click_selects_block() {
    let mut h = ViewerHarness::new(gain_chain(1));
    h.idle(2);
    h.run(input::click(h.block_screen_rect("2").unwrap().center()));
    assert!(
        h.app.selected_block_sids.contains("2"),
        "block sid selected"
    );
}

#[test]
fn vw_002_click_selects_line() {
    let mut h = ViewerHarness::new(gain_chain(1));
    h.idle(2);
    // Line In1→G0 runs horizontally between the two blocks — click its midpoint.
    let r_in = h.block_screen_rect("1").unwrap();
    let r_g = h.block_screen_rect("2").unwrap();
    let mid = Pos2::new(
        (r_in.right() + r_g.left()) * 0.5,
        (r_in.center().y + r_g.center().y) * 0.5,
    );
    h.run(input::click(mid));
    // Non-move-mode line click applies a Signal response → signal dialog.
    assert!(
        h.app.signal_view.is_some(),
        "signal dialog opened on line click"
    );
}

#[test]
fn vw_003_scroll_zooms_at_pointer() {
    let mut h = ViewerHarness::new(gain_chain(1));
    h.idle(2);
    let z0 = h.app.zoom;
    // Scroll over empty canvas (top area) — blocks sit at canvas center.
    let avail = h.transform().avail;
    let pos = egui::pos2(avail.center().x, avail.top() + 30.0);
    h.frame(input::scroll_at(pos, egui::vec2(0.0, 100.0)));
    assert!(
        h.app.zoom > z0,
        "scroll up zooms in: {z0} -> {}",
        h.app.zoom
    );
    let z1 = h.app.zoom;
    h.frame(input::scroll_at(pos, egui::vec2(0.0, -100.0)));
    assert!(h.app.zoom < z1, "scroll down zooms out");
}

#[test]
fn vw_004_canvas_drag_pans() {
    let mut h = ViewerHarness::new(gain_chain(1));
    h.idle(2);
    let p0 = h.app.pan;
    let a = h.transform().avail.center();
    h.frame(input::press(a));
    h.run(input::drag_path(a, a + egui::vec2(60.0, 40.0), 5));
    assert_ne!(h.app.pan, p0, "canvas drag pans view");
}

#[test]
fn vw_005_move_mode_drag_moves_block() {
    let mut h = ViewerHarness::new(gain_chain(1));
    h.app.move_mode_enabled = true;
    h.idle(2);
    let before = harness::parse_pos(
        h.app.current_system().unwrap().blocks[1]
            .position
            .as_deref()
            .unwrap(),
    )
    .unwrap();
    let from = h.block_screen_rect("2").unwrap().center();
    h.frame(input::press(from));
    h.run(input::drag_path(from, from + egui::vec2(50.0, 20.0), 6));
    h.idle(2);
    let after = harness::parse_pos(
        h.app.current_system().unwrap().blocks[1]
            .position
            .as_deref()
            .unwrap(),
    )
    .unwrap();
    assert!(
        after.0 != before.0 || after.1 != before.1,
        "move-mode drag moved block: {before:?} -> {after:?}"
    );
}

#[test]
fn vw_006_double_click_enters_subsystem() {
    let mut h = ViewerHarness::new(nested());
    h.idle(2);
    h.run(input::double_click(
        h.block_screen_rect("2").unwrap().center(),
    ));
    h.idle(1);
    assert_eq!(h.app.path, vec!["SS".to_string()], "entered subsystem");
}

#[test]
fn vw_007_breadcrumb_back_to_root() {
    let mut h = ViewerHarness::new(nested());
    h.app.navigate_to_path(vec!["SS".to_string()]);
    h.idle(2);
    h.app.navigate_to_path(Vec::new());
    h.idle(1);
    assert!(h.app.path.is_empty());
    assert!(h.block_screen_rect("2").is_some());
}

#[test]
fn vw_008_reset_view_recenters() {
    let mut h = ViewerHarness::new(gain_chain(3));
    h.idle(2);
    h.app.pan = egui::vec2(500.0, 500.0);
    h.app.reset_view = true;
    h.idle(2);
    assert!(h.app.pan.x < 500.0, "reset recenters");
    assert!(!h.app.reset_view);
}
