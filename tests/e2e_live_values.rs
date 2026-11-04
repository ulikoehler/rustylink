//! E2E: live values in the viewer — block text, scope feeds, high-rate pushes (§12/§14).

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use harness::live_feed;
use harness::{BlockBuilder, SystemBuilder, ViewerHarness, line};
use rustylink::model::System;

fn display_model() -> System {
    SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
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
fn lv_001_live_value_reaches_display_block() {
    let mut h = ViewerHarness::new(display_model());
    h.app.live_mode_enabled = true;
    h.idle(1);
    h.app
        .live_block_values
        .insert("2".to_string(), live_feed::scalar(42.5));
    h.idle(2);
    // The Display block's live text should reflect the pushed value.
    let text = h.app.live_block_values.get("2").map(|e| e.formatted_text());
    assert_eq!(text.as_deref(), Some("42.50"));
}

#[test]
fn lv_010_high_rate_pushes_no_leak() {
    let mut h = ViewerHarness::new(display_model());
    h.app.live_mode_enabled = true;
    h.idle(1);
    let sine = live_feed::sine(200.0, 1.0);
    // 10k synchronous updates — no sleeps; the map must stay bounded.
    for i in 0..10_000usize {
        h.app
            .live_block_values
            .insert("2".to_string(), live_feed::scalar(sine(i)));
        if i % 1000 == 0 {
            h.idle(1);
        }
    }
    h.idle(2);
    assert_eq!(h.app.live_block_values.len(), 1, "map stays bounded");
    let last = h
        .app
        .live_block_values
        .get("2")
        .and_then(|e| e.first_f64())
        .unwrap();
    assert!((last - sine(9999)).abs() < 1e-9);
}

#[test]
fn lv_011_multi_key_updates() {
    let mut h = ViewerHarness::new(display_model());
    h.app.live_mode_enabled = true;
    for k in 0..200usize {
        h.app
            .live_block_values
            .insert(format!("key_{k}"), live_feed::scalar(k as f64));
    }
    h.idle(3);
    assert_eq!(h.app.live_block_values.len(), 200);
    assert_eq!(
        h.app
            .live_block_values
            .get("key_199")
            .and_then(|e| e.first_f64()),
        Some(199.0)
    );
}
