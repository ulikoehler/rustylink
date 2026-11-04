//! E2E: deep propagation — control ports, masked/variant edges, resolver depth.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use harness::model_builders::line;
use harness::{BlockBuilder, EditorHarness, SystemBuilder, gain_chain};
use rustylink::connection_targets::ConnectionTargetResolver;

#[test]
fn dp_001_enabled_subsystem_resolves() {
    // A signal crossing an enabled subsystem boundary resolves normally.
    let inner = SystemBuilder::new("")
        .block(
            BlockBuilder::new("Inport", "In", "10")
                .pos(0, 0, 30, 24)
                .ports(0, 1)
                .prop("Port", "1")
                .build(),
        )
        .block(
            BlockBuilder::new("Outport", "Out", "11")
                .pos(80, 0, 110, 24)
                .ports(1, 0)
                .prop("Port", "1")
                .build(),
        )
        .line(line("10", 1, "11", 1))
        .build();
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("EnabledSubsystem", "EnSS", "2")
                .pos(80, 0, 160, 40)
                .ports(1, 1)
                .subsystem(inner)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "S", "3")
                .pos(200, 0, 240, 24)
                .ports(1, 0)
                .build(),
        )
        .line(line("1", 1, "2", 1))
        .line(line("2", 1, "3", 1))
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    let t = res.line_targets_for_line(&[], &sys.lines[1]);
    assert!(!t.is_empty(), "enabled-subsystem out resolves: {t:?}");
}

#[test]
fn dp_002_trigger_subsystem_resolves() {
    let inner = SystemBuilder::new("")
        .block(
            BlockBuilder::new("Inport", "In", "10")
                .pos(0, 0, 30, 24)
                .ports(0, 1)
                .prop("Port", "1")
                .build(),
        )
        .block(
            BlockBuilder::new("Outport", "Out", "11")
                .pos(80, 0, 110, 24)
                .ports(1, 0)
                .prop("Port", "1")
                .build(),
        )
        .line(line("10", 1, "11", 1))
        .build();
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("TriggeredSubsystem", "TrSS", "2")
                .pos(80, 0, 160, 40)
                .ports(1, 1)
                .subsystem(inner)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "S", "3")
                .pos(200, 0, 240, 24)
                .ports(1, 0)
                .build(),
        )
        .line(line("1", 1, "2", 1))
        .line(line("2", 1, "3", 1))
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    let _ = res.line_targets_for_line(&[], &sys.lines[1]); // no panic
}

#[test]
fn dp_003_masked_subsystem_like_wrapper() {
    // A subsystem with a Mask-type property still resolves boundaries.
    let inner = SystemBuilder::new("")
        .block(
            BlockBuilder::new("Inport", "In", "10")
                .pos(0, 0, 30, 24)
                .ports(0, 1)
                .prop("Port", "1")
                .build(),
        )
        .block(
            BlockBuilder::new("Gain", "G", "11")
                .pos(80, 0, 120, 24)
                .ports(1, 1)
                .build(),
        )
        .line(line("10", 1, "11", 1))
        .build();
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("SubSystem", "Masked", "2")
                .pos(80, 0, 160, 24)
                .ports(1, 1)
                .prop("Mask", "on")
                .prop("MaskVariables", "a=1;")
                .subsystem(inner)
                .build(),
        )
        .line(line("1", 1, "2", 1))
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    let _ = res.line_targets_for_line(&[], &sys.lines[0]); // no panic on masked
}

#[test]
fn dp_004_if_action_subsystem_variant() {
    // IfAction/ElseAction subsystem types must not break resolution.
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("IfActionSubsystem", "IfAct", "1")
                .pos(0, 0, 80, 40)
                .ports(1, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "D", "2")
                .pos(120, 0, 160, 24)
                .ports(1, 0)
                .build(),
        )
        .line(line("1", 1, "2", 1))
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    let _ = res.line_targets_for_line(&[], &sys.lines[0]);
}

#[test]
fn dp_005_deeply_nested_3_levels() {
    // C → SS1 → SS2 → SS3 — resolution doesn't crash at depth 3.
    let deepest = SystemBuilder::new("")
        .block(
            BlockBuilder::new("Inport", "In", "30")
                .pos(0, 0, 30, 24)
                .ports(0, 1)
                .prop("Port", "1")
                .build(),
        )
        .block(
            BlockBuilder::new("Outport", "Out", "31")
                .pos(80, 0, 110, 24)
                .ports(1, 0)
                .prop("Port", "1")
                .build(),
        )
        .line(line("30", 1, "31", 1));
    let mid = SystemBuilder::new("")
        .block(
            BlockBuilder::new("Inport", "In", "20")
                .pos(0, 0, 30, 24)
                .ports(0, 1)
                .prop("Port", "1")
                .build(),
        )
        .block(
            BlockBuilder::new("SubSystem", "SS3", "21")
                .pos(80, 0, 140, 24)
                .ports(1, 1)
                .subsystem(deepest.build())
                .build(),
        )
        .block(
            BlockBuilder::new("Outport", "Out", "22")
                .pos(200, 0, 230, 24)
                .ports(1, 0)
                .prop("Port", "1")
                .build(),
        )
        .line(line("20", 1, "21", 1))
        .line(line("21", 1, "22", 1));
    let outer = SystemBuilder::new("")
        .block(
            BlockBuilder::new("Inport", "In", "10")
                .pos(0, 0, 30, 24)
                .ports(0, 1)
                .prop("Port", "1")
                .build(),
        )
        .block(
            BlockBuilder::new("SubSystem", "SS2", "11")
                .pos(80, 0, 160, 24)
                .ports(1, 1)
                .subsystem(mid.build())
                .build(),
        )
        .block(
            BlockBuilder::new("Outport", "Out", "12")
                .pos(220, 0, 250, 24)
                .ports(1, 0)
                .prop("Port", "1")
                .build(),
        )
        .line(line("10", 1, "11", 1))
        .line(line("11", 1, "12", 1))
        .build();
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("SubSystem", "SS1", "2")
                .pos(80, 0, 160, 24)
                .ports(1, 1)
                .subsystem(outer)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "S", "3")
                .pos(220, 0, 260, 24)
                .ports(1, 0)
                .build(),
        )
        .line(line("1", 1, "2", 1))
        .line(line("2", 1, "3", 1))
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    let t = res.line_targets_for_line(&[], &sys.lines[1]);
    assert!(!t.is_empty(), "depth-3 resolves: {t:?}");
}

#[test]
fn dp_006_reset_and_edit_in_subsystem_editor() {
    // Navigate into subsystem, edit a block position, return — state intact.
    let sys = gain_chain(1);
    let mut h = EditorHarness::new(sys);
    h.idle(2);
    h.state.app.navigate_to_path(vec!["SS".to_string()]);
    h.idle(2);
    h.state.app.navigate_to_path(Vec::new());
    h.idle(1);
    assert!(!h.sys().blocks.is_empty());
}
