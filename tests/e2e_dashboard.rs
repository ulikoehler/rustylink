//! E2E: dashboard control events and editable constants (§13).

#![cfg(feature = "dashboard")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use harness::{BlockBuilder, SystemBuilder, ViewerHarness, input, line};
use rustylink::egui_app::DashboardControlValue;
use rustylink::model::System;

fn const_model() -> System {
    SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .value("1")
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "D", "2")
                .pos(200, 0, 250, 24)
                .ports(1, 0)
                .build(),
        )
        .line(line("1", 1, "2", 1))
        .build()
}

#[test]
fn db_001_queue_control_emits_event() {
    let mut h = ViewerHarness::new(const_model());
    h.idle(1);
    let blk = h.app.current_system().unwrap().blocks[0].clone();
    h.app
        .queue_dashboard_control(blk, DashboardControlValue::Scalar(5.0));
    let ev = h.drain_dashboard_events().expect("event queued");
    assert!(matches!(ev.value, DashboardControlValue::Scalar(v) if v == 5.0));
    assert_eq!(ev.block.sid.as_deref(), Some("1"));
    assert!(h.drain_dashboard_events().is_none(), "event drained once");
}

#[test]
fn db_002_pulse_high_low() {
    let mut h = ViewerHarness::new(const_model());
    h.idle(1);
    let blk = h.app.current_system().unwrap().blocks[0].clone();
    // pending_dashboard_control is a depth-1 slot — drain between queues.
    h.app
        .queue_dashboard_control(blk.clone(), DashboardControlValue::PulseHigh);
    let e1 = h.drain_dashboard_events().unwrap();
    assert!(matches!(e1.value, DashboardControlValue::PulseHigh));
    h.app
        .queue_dashboard_control(blk, DashboardControlValue::PulseLow);
    let e2 = h.drain_dashboard_events().unwrap();
    assert!(matches!(e2.value, DashboardControlValue::PulseLow));
}

#[test]
fn db_003_bool_value_preserved() {
    let mut h = ViewerHarness::new(const_model());
    h.idle(1);
    let blk = h.app.current_system().unwrap().blocks[0].clone();
    h.app
        .queue_dashboard_control(blk, DashboardControlValue::Bool(true));
    let ev = h.drain_dashboard_events().unwrap();
    assert!(matches!(ev.value, DashboardControlValue::Bool(true)));
}

#[test]
fn db_004_double_click_seeds_constant_edit() {
    // In live mode, double-clicking a Constant opens inline editing:
    // the edit buffer seeds with the current value.
    let mut h = ViewerHarness::new(const_model());
    h.app.live_mode_enabled = true;
    h.idle(2);
    h.run(input::double_click(
        h.block_screen_rect("1").unwrap().center(),
    ));
    h.idle(1);
    assert!(
        h.app.constant_edits.contains_key("1"),
        "constant edit buffer seeded"
    );
}
