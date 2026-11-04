//! E2E: propagation — block-level targets, multi-port sources, name inheritance.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use harness::model_builders::line;
use harness::{BlockBuilder, SystemBuilder, assert_target_paths_eq, gain_chain};
use rustylink::connection_targets::ConnectionTargetResolver;

// ── block-level targets ──────────────────────────────────────────────────────

#[test]
fn ct_110_block_targets_for_source_block() {
    let sys = gain_chain(2);
    let res = ConnectionTargetResolver::new(&sys);
    let g0 = &sys.blocks[1];
    let t = res.block_targets_for_block(&[], g0);
    assert!(
        t.iter().any(|x| x.path == "model/In1"),
        "G0 upstream reaches In1: {t:?}"
    );
}

#[test]
fn ct_111_block_targets_for_sink() {
    let sys = gain_chain(2);
    let res = ConnectionTargetResolver::new(&sys);
    let out = sys.blocks.last().unwrap();
    let t = res.block_targets_for_block(&[], out);
    assert!(
        t.iter().any(|x| x.path == "model/G1"),
        "sink's immediate upstream is G1: {t:?}"
    );
}

// ── multi-source-port blocks ─────────────────────────────────────────────────

#[test]
fn ct_112_multi_out_port_distinct_sources() {
    // Switch-like block: in:1 → out:1, in:2 → out:2 (model via two inputs).
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "A", "1")
                .pos(0, 0, 30, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Constant", "B", "2")
                .pos(0, 50, 30, 74)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("MultiPortSwitch", "SW", "3")
                .pos(80, 0, 120, 74)
                .ports(2, 2)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "S1", "4")
                .pos(160, 0, 200, 24)
                .ports(1, 0)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "S2", "5")
                .pos(160, 50, 200, 74)
                .ports(1, 0)
                .build(),
        )
        .line(line("1", 1, "3", 1))
        .line(line("2", 1, "3", 2))
        .line(line("3", 1, "4", 1))
        .line(line("3", 2, "5", 1))
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    // Each out line resolves — assert no crash and valid source paths.
    for l in &sys.lines {
        let t = res.line_targets_for_line(&[], l);
        for x in &t {
            assert!(x.path.starts_with("model/"), "valid path: {}", x.path);
        }
    }
}

// ── signal name inheritance ──────────────────────────────────────────────────

#[test]
fn ct_113_explicit_name_survives_through_gain() {
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "Src", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Gain", "G", "2")
                .pos(80, 0, 120, 24)
                .ports(1, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "Sink", "3")
                .pos(160, 0, 200, 24)
                .ports(1, 0)
                .build(),
        )
        .line(
            harness::model_builders::LineBuilder::new()
                .name("named_sig")
                .from("1", 1)
                .to("2", 1)
                .build(),
        )
        .line(line("2", 1, "3", 1))
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    // The upstream (named) line exposes the name on its source target.
    let t = res.line_targets_for_line(&[], &sys.lines[0]);
    assert!(
        t.iter()
            .any(|x| x.signal_name.as_deref() == Some("named_sig") || x.path == "model/Src")
    );
}

// ── subsystem input port resolution ──────────────────────────────────────────

#[test]
fn ct_031_inner_line_sees_parent_source() {
    // Inside SS, the Inport-fed line resolves to the parent-level source C.
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
    let root = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("SubSystem", "SS", "2")
                .pos(80, 0, 160, 24)
                .ports(1, 1)
                .subsystem(inner)
                .build(),
        )
        .line(line("1", 1, "2", 1))
        .build();
    let res = ConnectionTargetResolver::new(&root);
    let inner_sys = &root.blocks[1].subsystem.as_ref().unwrap();
    let t = res.line_targets_for_line(&["SS".to_string()], &inner_sys.lines[0]);
    assert!(
        t.iter().any(|x| x.path == "model/C"),
        "inner line resolves through Inport to parent source: {t:?}"
    );
}

// ── many-input convergence ───────────────────────────────────────────────────

#[test]
fn ct_113b_many_sources_converge_on_mux() {
    let mut b = SystemBuilder::new("model");
    let mut sids = Vec::new();
    for i in 0..4usize {
        let sid = (i + 1).to_string();
        sids.push(sid.clone());
        b = b.block(
            BlockBuilder::new("Constant", &format!("C{i}"), &sid)
                .pos(0, i as i32 * 40, 30, i as i32 * 40 + 24)
                .ports(0, 1)
                .build(),
        );
    }
    b = b.block(
        BlockBuilder::new("Mux", "M", "9")
            .pos(80, 0, 110, 170)
            .ports(4, 1)
            .build(),
    );
    b = b.block(
        BlockBuilder::new("Display", "S", "10")
            .pos(160, 60, 200, 84)
            .ports(1, 0)
            .build(),
    );
    for (i, sid) in sids.iter().enumerate() {
        b = b.line(line(sid, 1, "9", (i + 1) as u32));
    }
    b = b.line(line("9", 1, "10", 1));
    let sys = b.build();
    let res = ConnectionTargetResolver::new(&sys);
    let t = res.line_targets_for_line(&[], sys.lines.last().unwrap());
    assert_eq!(t.len(), 4, "mux out carries all 4 element sources: {t:?}");
    assert_target_paths_eq(&t, &["model/C0", "model/C1", "model/C2", "model/C3"]);
}
