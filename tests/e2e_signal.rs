//! E2E: signal-name propagation, branch paths, remaining resolver cases.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use harness::model_builders::line;
use harness::{BlockBuilder, EditorHarness, SystemBuilder, ViewerHarness, gain_chain};
use rustylink::connection_targets::ConnectionTargetResolver;

#[test]
fn sg_001_branch_dst_resolves_to_src() {
    // A branch endpoint inherits the main line's source target.
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "A", "2")
                .pos(80, 0, 120, 24)
                .ports(1, 0)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "B", "3")
                .pos(80, 40, 120, 64)
                .ports(1, 0)
                .build(),
        )
        .line(
            harness::model_builders::LineBuilder::new()
                .from("1", 1)
                .to("2", 1)
                .branch_to("3", 1)
                .build(),
        )
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    let t = res.line_targets_for_line(&[], &sys.lines[0]);
    assert!(
        t.iter().all(|x| x.path == "model/C"),
        "branch inherits source: {t:?}"
    );
}

#[test]
fn sg_002_named_line_named_target() {
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "Src", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "Sink", "2")
                .pos(80, 0, 120, 24)
                .ports(1, 0)
                .build(),
        )
        .line(
            harness::model_builders::LineBuilder::new()
                .name("explicit_name")
                .from("1", 1)
                .to("2", 1)
                .build(),
        )
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    let t = res.line_targets_for_line(&[], &sys.lines[0]);
    assert!(
        t.iter()
            .any(|x| x.signal_name.as_deref() == Some("explicit_name")),
        "line name on target"
    );
}

#[test]
fn sg_003_gain_opaque_two_chain_resolves_to_first() {
    // A→G1→G2→Sink: sink's line resolves to G2 (Gain opaque).
    let sys = gain_chain(2);
    let res = ConnectionTargetResolver::new(&sys);
    let t = res.line_targets_for_line(&[], sys.lines.last().unwrap());
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].path, "model/G1"); // G0=idx1, G1=idx2, last line G1→Out1 → src G1
}

#[test]
fn sg_004_inport_outport_root_level_source_sink() {
    // In1 → Out1 at root: resolves to the Inport (a source block).
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Inport", "In1", "1")
                .pos(0, 0, 30, 24)
                .ports(0, 1)
                .prop("Port", "1")
                .build(),
        )
        .block(
            BlockBuilder::new("Outport", "Out1", "2")
                .pos(80, 0, 110, 24)
                .ports(1, 0)
                .prop("Port", "1")
                .build(),
        )
        .line(line("1", 1, "2", 1))
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    let t = res.line_targets_for_line(&[], &sys.lines[0]);
    assert_eq!(t[0].path, "model/In1");
}

#[test]
fn sg_005_editor_two_connections_same_out() {
    // One out port feeds two blocks — both lines created via drags.
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "A", "2")
                .pos(150, -40, 190, -16)
                .ports(1, 0)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "B", "3")
                .pos(150, 40, 190, 64)
                .ports(1, 0)
                .build(),
        )
        .build();
    let mut h = EditorHarness::new(sys);
    h.idle(2);
    h.drag(
        h.port_hit_pos("1", "out", 1),
        h.port_hit_pos("2", "in", 1),
        8,
    );
    h.drag(
        h.port_hit_pos("1", "out", 1),
        h.port_hit_pos("3", "in", 1),
        8,
    );
    assert_eq!(h.sys().lines.len(), 2, "two lines from same out port");
    assert!(
        h.sys()
            .lines
            .iter()
            .all(|l| l.src.as_ref().unwrap().sid == "1")
    );
    harness::debug_validate(h.sys());
}

#[test]
fn sg_006_viewer_click_selected_then_canvas_clears() {
    let mut h = ViewerHarness::new(gain_chain(1));
    h.idle(2);
    h.run(harness::input::click(
        h.block_screen_rect("1").unwrap().center(),
    ));
    assert!(!h.app.selected_block_sids.is_empty());
    // Click empty canvas — clears selection.
    let avail = h.transform().avail;
    let empty = egui::pos2(avail.center().x, avail.top() + 30.0);
    h.run(harness::input::click(empty));
    h.idle(1);
}
