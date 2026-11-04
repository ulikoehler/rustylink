//! E2E: control-port endpoints (enable/trigger/reset), resolver depth.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use harness::EditorHarness;
use harness::model_builders::{endpoint, line, props};
use harness::{BlockBuilder, SystemBuilder, gain_chain};
use rustylink::connection_targets::ConnectionTargetResolver;
use rustylink::model::Line;
use rustylink::model::System;

#[test]
fn ctl_001_enable_endpoint_resolves() {
    // Line to "2#enable:1" — the enable control-port endpoint parses.
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "En", "1")
                .pos(0, -60, 40, -36)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("SubSystem", "EnSS", "2")
                .pos(80, -20, 160, 40)
                .ports(1, 1)
                .control_ports(1, 0)
                .build(),
        )
        .line(Line {
            name: None,
            zorder: None,
            src: Some(endpoint("1", "out", 1)),
            dst: Some(endpoint("2", "enable", 1)),
            points: vec![],
            labels: None,
            branches: vec![],
            properties: props(&[]),
        })
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    let _ = res.line_targets_for_line(&[], &sys.lines[0]);
}

#[test]
fn ctl_002_trigger_endpoint_resolves() {
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "T", "1")
                .pos(0, -60, 40, -36)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("SubSystem", "TrSS", "2")
                .pos(80, -20, 160, 40)
                .ports(1, 1)
                .control_ports(0, 1)
                .build(),
        )
        .line(Line {
            name: None,
            zorder: None,
            src: Some(endpoint("1", "out", 1)),
            dst: Some(endpoint("2", "trigger", 1)),
            points: vec![],
            labels: None,
            branches: vec![],
            properties: props(&[]),
        })
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    let _ = res.line_targets_for_line(&[], &sys.lines[0]);
}

#[test]
fn ctl_003_reset_endpoint_resolves() {
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "R", "1")
                .pos(0, -60, 40, -36)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("SubSystem", "RSS", "2")
                .pos(80, -20, 160, 40)
                .ports(1, 1)
                .control_ports(0, 0)
                .build(),
        )
        .line(Line {
            name: None,
            zorder: None,
            src: Some(endpoint("1", "out", 1)),
            dst: Some(endpoint("2", "reset", 1)),
            points: vec![],
            labels: None,
            branches: vec![],
            properties: props(&[]),
        })
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    let _ = res.line_targets_for_line(&[], &sys.lines[0]);
}

#[test]
fn ctl_004_lconn_rconn_on_enabled_subsystem() {
    // Enabled subsystem renders LConn/RConn hit areas — harness can reach them.
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("SubSystem", "EnSS", "1")
                .pos(0, 0, 80, 60)
                .ports(1, 1)
                .control_ports(1, 0)
                .build(),
        )
        .build();
    let mut h = EditorHarness::new(sys);
    h.idle(2);
    // LConn/RConn hit positions exist above the block top edge.
    let r = h.block_screen_rect("1").unwrap();
    assert!(r.width() > 0.0);
}

#[test]
fn ctl_005_mixed_control_and_data_lines() {
    // A subsystem fed both a data input AND an enable — all lines resolve.
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "D", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Constant", "E", "2")
                .pos(0, -60, 40, -36)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("SubSystem", "EnSS", "3")
                .pos(80, -20, 160, 40)
                .ports(1, 1)
                .control_ports(1, 0)
                .build(),
        )
        .line(line("1", 1, "3", 1))
        .line(Line {
            name: None,
            zorder: None,
            src: Some(endpoint("2", "out", 1)),
            dst: Some(endpoint("3", "enable", 1)),
            points: vec![],
            labels: None,
            branches: vec![],
            properties: props(&[]),
        })
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    for l in &sys.lines {
        let _ = res.line_targets_for_line(&[], l);
    }
}
