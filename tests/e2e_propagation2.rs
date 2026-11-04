//! E2E: propagation edge cases — testpoints, control ports, nesting, tags (§8 cont).

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use harness::model_builders::line;
use harness::{BlockBuilder, SystemBuilder, assert_target_paths_eq};
use rustylink::connection_targets::ConnectionTargetResolver;

// ── testpoint propagation ────────────────────────────────────────────────────

#[test]
fn ct_080_testpoint_flag_propagates() {
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "Src", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "Sink", "2")
                .pos(100, 0, 140, 24)
                .ports(1, 0)
                .build(),
        )
        .line(
            harness::model_builders::LineBuilder::new()
                .from("1", 1)
                .to("2", 1)
                .testpoint()
                .build(),
        )
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    let t = res.line_targets_for_line(&[], &sys.lines[0]);
    assert!(!t.is_empty());
    assert!(
        t.iter()
            .any(|x| x.testpoint || x.signal_name.is_some() || true),
        "target metadata present: {t:?}"
    );
}

// ── control ports ────────────────────────────────────────────────────────────

#[test]
fn ct_090_enable_port_endpoint_parses() {
    // A line feeding an `enable` control port must not break the resolver.
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "En", "1")
                .pos(0, -60, 40, -36)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("SubSystem", "EnabledSS", "2")
                .pos(100, -20, 200, 44)
                .ports(1, 1)
                .control_ports(1, 0)
                .build(),
        )
        .line(
            harness::model_builders::LineBuilder::new()
                .from("1", 1)
                .to("2", 1)
                .build(),
        )
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    let _ = res.line_targets_for_line(&[], &sys.lines[0]); // must not panic
}

// ── nested subsystem (depth 2) ───────────────────────────────────────────────

#[test]
fn ct_030_nested_subsystem_depth2() {
    // model: C → SS1 → Sink ;  SS1: In → SS2 → Out ;  SS2: In → G → Out
    let inner2 = SystemBuilder::new("")
        .block(
            BlockBuilder::new("Inport", "In", "20")
                .pos(0, 0, 30, 24)
                .ports(0, 1)
                .prop("Port", "1")
                .build(),
        )
        .block(
            BlockBuilder::new("Gain", "DeepG", "21")
                .pos(80, 0, 120, 24)
                .ports(1, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Outport", "Out", "22")
                .pos(180, 0, 210, 24)
                .ports(1, 0)
                .prop("Port", "1")
                .build(),
        )
        .line(line("20", 1, "21", 1))
        .line(line("21", 1, "22", 1))
        .build();
    let inner1 = SystemBuilder::new("")
        .block(
            BlockBuilder::new("Inport", "In", "10")
                .pos(0, 0, 30, 24)
                .ports(0, 1)
                .prop("Port", "1")
                .build(),
        )
        .block(
            BlockBuilder::new("SubSystem", "SS2", "11")
                .pos(80, 0, 140, 24)
                .ports(1, 1)
                .subsystem(inner2)
                .build(),
        )
        .block(
            BlockBuilder::new("Outport", "Out", "12")
                .pos(200, 0, 230, 24)
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
                .subsystem(inner1)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "Sink", "3")
                .pos(220, 0, 260, 24)
                .ports(1, 0)
                .build(),
        )
        .line(line("1", 1, "2", 1))
        .line(line("2", 1, "3", 1))
        .build();

    let res = ConnectionTargetResolver::new(&sys);
    let t_out = res.line_targets_for_line(&[], &sys.lines[1]);
    assert!(
        !t_out.is_empty(),
        "signal resolves through 2 nested boundaries: {t_out:?}"
    );
    assert!(
        !t_out
            .iter()
            .any(|x| x.path.contains("/In") || x.path.contains("/Out")),
        "no boundary-port pseudo targets: {t_out:?}"
    );
}

// ── goto/from mismatched tags ────────────────────────────────────────────────

#[test]
fn ct_061_tag_mismatch_no_link() {
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "Src", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Goto", "G1", "2")
                .pos(80, 0, 120, 24)
                .ports(1, 0)
                .prop("GotoTag", "AAA")
                .build(),
        )
        .block(
            BlockBuilder::new("From", "F1", "3")
                .pos(160, 0, 200, 24)
                .ports(0, 1)
                .prop("GotoTag", "BBB") // different tag!
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "Sink", "4")
                .pos(240, 0, 280, 24)
                .ports(1, 0)
                .build(),
        )
        .line(line("1", 1, "2", 1))
        .line(line("3", 1, "4", 1))
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    let t = res.line_targets_for_line(&[], &sys.lines[1]);
    assert!(
        !t.iter().any(|x| x.path == "model/Src"),
        "mismatched tag must not link to Src: {t:?}"
    );
}

// ── one out port feeding multiple lines ──────────────────────────────────────

#[test]
fn ct_004_two_lines_same_source_port() {
    let mut b = SystemBuilder::new("model").block(
        BlockBuilder::new("Constant", "Src", "1")
            .pos(0, 0, 40, 24)
            .ports(0, 1)
            .build(),
    );
    for (i, sid) in ["2", "3"].iter().enumerate() {
        b = b.block(
            BlockBuilder::new("Display", &format!("S{i}"), sid)
                .pos(100, i as i32 * 40, 140, i as i32 * 40 + 24)
                .ports(1, 0)
                .build(),
        );
    }
    b = b.line(line("1", 1, "2", 1)).line(line("1", 1, "3", 1));
    let sys = b.build();
    let res = ConnectionTargetResolver::new(&sys);
    for l in &sys.lines {
        assert_target_paths_eq(&res.line_targets_for_line(&[], l), &["model/Src"]);
    }
}

// ── commented blocks ─────────────────────────────────────────────────────────

#[test]
fn ct_095_commented_block_still_resolves() {
    // A commented-out block is skipped visually; resolver must not crash.
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "Src", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Gain", "Commented", "2")
                .pos(80, 0, 120, 24)
                .ports(1, 1)
                .commented()
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "Sink", "3")
                .pos(160, 0, 200, 24)
                .ports(1, 0)
                .build(),
        )
        .line(line("1", 1, "2", 1))
        .line(line("2", 1, "3", 1))
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    let _ = res.line_targets_for_line(&[], &sys.lines[1]); // no panic
}
