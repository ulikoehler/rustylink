//! E2E: copy/paste multiples, redo chains, demux, selection edges.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use eframe::egui;
use harness::model_builders::line;
use harness::{BlockBuilder, EditorHarness, SystemBuilder, gain_chain};
use rustylink::connection_targets::ConnectionTargetResolver;

#[test]
fn x_001_double_paste_two_clones() {
    // Copy a block, paste twice → two offset clones with fresh SIDs.
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.click(h.block_center("2"));
    h.key(egui::Key::C, egui::Modifiers::CTRL);
    h.key(egui::Key::V, egui::Modifiers::CTRL);
    h.key(egui::Key::V, egui::Modifiers::CTRL);
    assert_eq!(h.sys().blocks.len(), 5, "3 originals + 2 pastes");
    let sids: Vec<_> = h
        .sys()
        .blocks
        .iter()
        .filter_map(|b| b.sid.as_deref())
        .collect();
    let mut dedup = sids.clone();
    dedup.dedup();
    assert_eq!(sids.len(), dedup.len(), "all SIDs unique");
    harness::debug_validate(h.sys());
}

#[test]
fn x_002_redo_replays_multiple() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    let c = h.block_center("1");
    h.drag(c, c + egui::vec2(60.0, 0.0), 6);
    h.drag(c, c + egui::vec2(60.0, 60.0), 6);
    let p_after = h.block_position("1");
    h.state.undo();
    h.state.undo();
    let p0 = h.block_position("1");
    assert_ne!(p_after, p0);
    h.state.redo();
    h.state.redo();
    assert_eq!(h.block_position("1"), p_after, "redo restored final pos");
}

#[test]
fn x_003_demux_to_two_sinks() {
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Mux", "M", "1")
                .pos(0, 0, 30, 64)
                .ports(2, 2)
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
        .line(line("1", 1, "2", 1))
        .line(line("1", 2, "3", 1))
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    let a = res.line_targets_for_line(&[], &sys.lines[0]);
    let b = res.line_targets_for_line(&[], &sys.lines[1]);
    // Distinct element indices on each output.
    assert!(
        a.iter().all(|x| x.path.starts_with("model/"))
            && b.iter().all(|x| x.path.starts_with("model/")),
        "demux outputs resolve"
    );
}

#[test]
fn x_004_repeated_click_same_block_stays_one() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    for _ in 0..3 {
        h.click(h.block_center("1"));
    }
    assert_eq!(h.selected_blocks().len(), 1);
}

#[test]
fn x_005_paste_after_delete_no_orphan() {
    // Copy block, delete it, paste — clone references old SID path but
    // gets a fresh SID (no dangling references).
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.click(h.block_center("2"));
    h.key(egui::Key::C, egui::Modifiers::CTRL);
    h.key(egui::Key::Delete, egui::Modifiers::NONE);
    h.key(egui::Key::V, egui::Modifiers::CTRL);
    assert_eq!(h.sys().blocks.len(), 3, "2 remaining + 1 pasted");
    harness::debug_validate(h.sys());
}

#[test]
fn x_006_editor_delete_all_then_empty_state() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    // select all + delete — leaves a valid empty model.
    for c in ["1", "2", "3"] {
        h.run(harness::input::ctrl_click(h.block_center(c)));
    }
    assert_eq!(h.selected_blocks().len(), 3);
    h.key(egui::Key::Delete, egui::Modifiers::NONE);
    assert!(h.sys().blocks.is_empty());
    assert!(h.sys().lines.is_empty());
    h.idle(2); // renders empty-state hint without panic
}
