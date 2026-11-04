//! E2E: connection-target resolution / signal propagation (§8).

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use harness::model_builders::line;
use harness::{
    BlockBuilder, SystemBuilder, assert_target_paths_eq, assert_targets_eq, gen_wide_net,
};
use rustylink::connection_targets::{ConnectionTargetOrigin, ConnectionTargetResolver};
use rustylink::model::System;

fn resolve_targets(sys: &System, line_idx: usize) -> Vec<String> {
    let res = ConnectionTargetResolver::new(sys);
    res.line_targets_for_line(&[], &sys.lines[line_idx])
        .iter()
        .map(|t| t.path.clone())
        .collect()
}

// ── flat propagation ────────────────────────────────────────────────────────

#[test]
fn ct_001_direct_line_resolves_to_source() {
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
        .line(line("1", 1, "2", 1))
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    assert_target_paths_eq(
        &res.line_targets_for_line(&[], &sys.lines[0]),
        &["model/Src"],
    );
}

#[test]
fn ct_002_fanout_branches_share_source() {
    // One source feeding three branch destinations.
    let mut b = SystemBuilder::new("model").block(
        BlockBuilder::new("Constant", "Src", "1")
            .pos(0, 0, 40, 24)
            .ports(0, 1)
            .build(),
    );
    for (i, sid) in ["2", "3", "4"].iter().enumerate() {
        b = b.block(
            BlockBuilder::new("Display", &format!("S{i}"), sid)
                .pos(100, i as i32 * 40, 140, i as i32 * 40 + 24)
                .ports(1, 0)
                .build(),
        );
    }
    b = b.line(
        harness::model_builders::LineBuilder::new()
            .from("1", 1)
            .branch_to("2", 1)
            .branch_to("3", 1)
            .branch_to("4", 1)
            .build(),
    );
    let sys = b.build();
    let res = ConnectionTargetResolver::new(&sys);
    let targets = res.line_targets_for_line(&[], &sys.lines[0]);
    assert!(!targets.is_empty());
    assert!(
        targets.iter().all(|t| t.path == "model/Src"),
        "all branches trace to source: {targets:?}"
    );
}

#[test]
fn ct_003_chain_resolves_immediate_source() {
    let sys = harness::gain_chain(3);
    // Each line resolves to the block that produces its signal — Gain is a
    // producer, not a pass-through like mux/demux/bus blocks.
    let expected = ["model/In1", "model/G0", "model/G1", "model/G2"];
    for (i, want) in expected.iter().enumerate() {
        let targets = resolve_targets(&sys, i);
        assert_eq!(targets, vec![want.to_string()], "line {i} source");
    }
}

#[test]
fn ct_006_dangling_line_empty_targets() {
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .line(
            harness::model_builders::LineBuilder::new()
                .from("1", 1)
                .build(), // no dst
        )
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    // Resolver must not panic on an unconnected line.
    let _ = res.line_targets_for_line(&[], &sys.lines[0]);
}

// ── subsystem boundary propagation ──────────────────────────────────────────

fn subsystem_model() -> System {
    // Inner: In1(Port=1) → G(ain) → Out1(Port=1)
    let inner = SystemBuilder::new("")
        .block(
            BlockBuilder::new("Inport", "In1", "10")
                .pos(0, 0, 30, 24)
                .ports(0, 1)
                .prop("Port", "1")
                .build(),
        )
        .block(
            BlockBuilder::new("Gain", "InnerG", "11")
                .pos(80, 0, 120, 24)
                .ports(1, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Outport", "Out1", "12")
                .pos(180, 0, 210, 24)
                .ports(1, 0)
                .prop("Port", "1")
                .build(),
        )
        .line(line("10", 1, "11", 1))
        .line(line("11", 1, "12", 1))
        .build();

    SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("SubSystem", "SS", "2")
                .pos(100, -10, 200, 34)
                .ports(1, 1)
                .subsystem(inner)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "Sink", "3")
                .pos(260, 0, 300, 24)
                .ports(1, 0)
                .build(),
        )
        .line(line("1", 1, "2", 1))
        .line(line("2", 1, "3", 1))
        .build()
}

#[test]
fn ct_020_in_port_passthrough() {
    let sys = subsystem_model();
    let res = ConnectionTargetResolver::new(&sys);
    // Line C→SS: the signal's source is C regardless of the boundary.
    let t = res.line_targets_for_line(&[], &sys.lines[0]);
    assert!(!t.is_empty(), "resolved across in-boundary: {t:?}");
    assert!(
        !t.iter().any(|x| x.path.contains("SS/In1")),
        "no pseudo boundary targets: {t:?}"
    );
}

#[test]
fn ct_021_out_port_passthrough() {
    let sys = subsystem_model();
    let res = ConnectionTargetResolver::new(&sys);
    let t = res.line_targets_for_line(&[], &sys.lines[1]);
    assert!(
        !t.is_empty(),
        "subsystem output resolves to a real source: {t:?}"
    );
    assert!(
        !t.iter().any(|x| x.path.contains("SS/Out1")),
        "no pseudo boundary targets: {t:?}"
    );
}

// ── bus propagation ──────────────────────────────────────────────────────────

fn bus_model() -> System {
    SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "A", "1")
                .pos(0, 0, 30, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Constant", "B", "2")
                .pos(0, 40, 30, 64)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("BusCreator", "Bus", "3")
                .pos(80, 0, 110, 64)
                .ports(2, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("BusSelector", "Sel", "4")
                .pos(160, 0, 190, 64)
                .ports(1, 1)
                .prop("OutputSignals", "beta")
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "Sink", "5")
                .pos(240, 0, 280, 24)
                .ports(1, 0)
                .build(),
        )
        .line(
            harness::model_builders::LineBuilder::new()
                .name("alpha")
                .from("1", 1)
                .to("3", 1)
                .build(),
        )
        .line(
            harness::model_builders::LineBuilder::new()
                .name("beta")
                .from("2", 1)
                .to("3", 2)
                .build(),
        )
        .line(line("3", 1, "4", 1))
        .line(line("4", 1, "5", 1))
        .build()
}

#[test]
fn ct_041_bus_selector_extracts_element() {
    // Selector output ports carry OutputSignal names via port properties;
    // with only one out port it selects by index/name — assert the resolved
    // target is one of the two feeding constants.
    let sys = bus_model();
    let res = ConnectionTargetResolver::new(&sys);
    let t = res.line_targets_for_line(&[], &sys.lines[3]);
    assert!(!t.is_empty(), "selector resolved: {t:?}");
    assert!(
        t.iter().all(|x| x.path == "model/B"),
        "selector 'beta' resolves to its element source: {t:?}"
    );
}

#[test]
fn ct_047_bus_creator_collects_inputs() {
    let sys = bus_model();
    let res = ConnectionTargetResolver::new(&sys);
    // The bus line (creator→selector) carries both element sources.
    let t = res.line_targets_for_line(&[], &sys.lines[2]);
    assert_eq!(t.len(), 2, "bus carries both elements: {t:?}");
    assert_target_paths_eq(&t, &["model/A", "model/B"]);
}

// ── goto/from propagation ────────────────────────────────────────────────────

#[test]
fn ct_060_goto_from_virtual_wire() {
    // Src → Goto(tag T); From(tag T) → Sink
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
                .prop("GotoTag", "T")
                .build(),
        )
        .block(
            BlockBuilder::new("From", "F1", "3")
                .pos(160, 0, 200, 24)
                .ports(0, 1)
                .prop("GotoTag", "T")
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
        t.iter().any(|x| x.path == "model/Src"),
        "from-line resolves through the tag to Src: {t:?}"
    );
}

// ── mux/demux ────────────────────────────────────────────────────────────────

#[test]
fn ct_071_demux_lane_index() {
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "A", "1")
                .pos(0, 0, 30, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Constant", "B", "2")
                .pos(0, 40, 30, 64)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Mux", "M", "3")
                .pos(80, 0, 110, 64)
                .ports(2, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Demux", "D", "4")
                .pos(160, 0, 190, 64)
                .ports(1, 2)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "S1", "5")
                .pos(240, 0, 280, 24)
                .ports(1, 0)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "S2", "6")
                .pos(240, 40, 280, 64)
                .ports(1, 0)
                .build(),
        )
        .line(line("1", 1, "3", 1))
        .line(line("2", 1, "3", 2))
        .line(line("3", 1, "4", 1))
        .line(line("4", 1, "5", 1))
        .line(line("4", 2, "6", 1))
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    let t1 = res.line_targets_for_line(&[], &sys.lines[3]);
    let t2 = res.line_targets_for_line(&[], &sys.lines[4]);
    // Each demux lane resolves to the matching mux input element.
    assert_eq!(t1.len(), 1);
    assert_eq!(t2.len(), 1);
    assert_ne!(
        t1[0].path, t2[0].path,
        "lanes map to different inputs: {t1:?} {t2:?}"
    );
}

// ── resolver lifecycle / progress ────────────────────────────────────────────

#[test]
fn ct_102_deterministic_across_runs() {
    let (sys, _o) = gen_wide_net(50, 3);
    let r1 = ConnectionTargetResolver::new(&sys);
    let r2 = ConnectionTargetResolver::new(&sys);
    for l in &sys.lines {
        assert_eq!(
            r1.line_targets_for_line(&[], l),
            r2.line_targets_for_line(&[], l)
        );
    }
}

#[test]
fn ct_103_500_signal_oracle() {
    let (sys, oracle) = gen_wide_net(200, 2);
    let res = ConnectionTargetResolver::new(&sys);
    for (idx, expected) in &oracle.line_targets {
        let t = res.line_targets_for_line(&[], &sys.lines[*idx]);
        let paths: Vec<String> = t.iter().map(|x| x.path.clone()).collect();
        let mut got = paths.clone();
        got.sort();
        let mut want = expected.clone();
        want.sort();
        assert_eq!(got, want, "line {idx} targets");
    }
}

#[test]
fn ct_targets_with_signal_name() {
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
                .name("mysig")
                .from("1", 1)
                .to("2", 1)
                .build(),
        )
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    let t = res.line_targets_for_line(&[], &sys.lines[0]);
    assert_targets_eq(&t, &[("model/Src", Some("mysig"))]);
    assert_eq!(t[0].origin, ConnectionTargetOrigin::SourceBlock);
}
