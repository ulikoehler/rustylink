//! E2E: full user journeys — model → edit → connect → undo → save → reparse (§15).

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use eframe::egui;
use harness::{BlockBuilder, EditorHarness, SystemBuilder, slx_factory};
use rustylink::connection_targets::ConnectionTargetResolver;
use rustylink::model::{SlxArchive, SlxContent};
use tempfile::TempDir;

#[test]
fn jrn_001_build_wire_undo_save_reparse() {
    // 1) start with two disconnected blocks
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .value("2")
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "D", "2")
                .pos(200, 0, 250, 24)
                .ports(1, 0)
                .build(),
        )
        .build();
    let mut h = EditorHarness::new(sys);
    h.idle(2);

    // 2) draw a connection via port drag
    h.drag(
        h.port_hit_pos("1", "out", 1),
        h.port_hit_pos("2", "in", 1),
        8,
    );
    assert_eq!(h.sys().lines.len(), 1);

    // 3) move the display block — line must stay attached
    let from = h.block_center("2");
    h.drag(from, from + egui::vec2(0.0, 60.0), 6);
    assert_eq!(h.sys().lines[0].dst.as_ref().unwrap().sid, "2");

    // 4) undo the move — block position restored, line kept
    h.state.undo();
    assert_eq!(h.sys().lines.len(), 1);

    // 5) save via real writer, reparse via real archive reader
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("journey.slx");
    slx_factory::write_model_slx(&path, h.sys()).unwrap();
    let arch = SlxArchive::from_file(&path).unwrap();
    let parsed = arch
        .entries
        .iter()
        .find_map(|e| match &e.content {
            SlxContent::SystemXml(s) => Some(s.as_ref()),
            _ => None,
        })
        .unwrap();
    assert_eq!(parsed.blocks.len(), 2);
    assert_eq!(parsed.lines.len(), 1);
    assert_eq!(parsed.lines[0].src.as_ref().unwrap().sid, "1");
    assert_eq!(parsed.lines[0].dst.as_ref().unwrap().sid, "2");

    // 6) resolver sees the wired chain
    let res = ConnectionTargetResolver::new(parsed);
    let t = res.line_targets_for_line(&[], &parsed.lines[0]);
    assert_eq!(t[0].path, "model/C");
}

#[test]
fn jrn_002_edit_in_subsystem_round_trip() {
    // Enter a subsystem, wire its internals, save, reparse.
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
        .block(
            BlockBuilder::new("Outport", "Out", "12")
                .pos(200, 0, 230, 24)
                .ports(1, 0)
                .prop("Port", "1")
                .build(),
        )
        .build();
    let root = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("SubSystem", "SS", "2")
                .pos(100, -10, 200, 34)
                .ports(1, 1)
                .subsystem(inner)
                .build(),
        )
        .build();
    let mut h = EditorHarness::new(root);
    h.idle(2);
    h.state.app.navigate_to_path(vec!["SS".to_string()]);
    h.idle(2);

    // Wire In → G → Out inside the subsystem.
    h.drag(
        h.port_hit_pos("10", "out", 1),
        h.port_hit_pos("11", "in", 1),
        6,
    );
    h.drag(
        h.port_hit_pos("11", "out", 1),
        h.port_hit_pos("12", "in", 1),
        6,
    );
    assert_eq!(h.sys().lines.len(), 2, "two inner lines");

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nested.slx");
    slx_factory::write_model_slx(&path, &h.state.app.root).unwrap();
    let arch = SlxArchive::from_file(&path).unwrap();
    let parsed = arch
        .entries
        .iter()
        .find_map(|e| match &e.content {
            SlxContent::SystemXml(s) => Some(s.as_ref()),
            _ => None,
        })
        .unwrap();
    let inner_parsed = &parsed.blocks[0].subsystem.as_ref().unwrap();
    assert_eq!(inner_parsed.lines.len(), 2, "inner lines persisted");
}

#[test]
fn jrn_003_select_all_delete_rebuild() {
    // Marquee-select everything, delete, verify empty, undo all.
    let mut h = EditorHarness::new(harness::gain_chain(2));
    h.idle(2);
    let avail = h.canvas_rect();
    let a = egui::pos2(avail.left() + 4.0, avail.top() + 4.0);
    let b = egui::pos2(avail.right() - 4.0, avail.bottom() - 4.0);
    h.drag_with(a, b, 6, egui::Modifiers::SHIFT);
    assert_eq!(h.selected_blocks().len(), 4, "all blocks selected");

    h.key(egui::Key::Delete, egui::Modifiers::NONE);
    h.idle(1);
    assert_eq!(h.sys().blocks.len(), 0, "all blocks deleted");
    assert!(h.sys().lines.is_empty(), "lines cascaded");

    // Undo: blocks back, then lines back (per-command order).
    while h.state.history.can_undo() {
        h.state.undo();
    }
    assert_eq!(h.sys().blocks.len(), 4, "everything restored");
    assert_eq!(h.sys().lines.len(), 3);
    harness::debug_validate(h.sys());
}
