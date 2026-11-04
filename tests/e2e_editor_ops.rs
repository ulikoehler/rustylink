//! E2E: editor keyboard ops, clipboard, navigation (§10 continued).

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use eframe::egui::{self, Key, Modifiers};
use harness::{BlockBuilder, EditorHarness, SystemBuilder, gain_chain, input};
use rustylink::model::System;

fn sys_two() -> System {
    SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .value("3.14")
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "D", "2")
                .pos(200, 0, 240, 24)
                .ports(1, 0)
                .build(),
        )
        .build()
}

fn nested() -> System {
    let inner = SystemBuilder::new("")
        .block(
            BlockBuilder::new("Gain", "InnerG", "10")
                .pos(80, 0, 120, 24)
                .ports(1, 1)
                .build(),
        )
        .build();
    SystemBuilder::new("model")
        .block(
            BlockBuilder::new("SubSystem", "SS", "2")
                .pos(100, -10, 200, 34)
                .ports(1, 1)
                .subsystem(inner)
                .build(),
        )
        .build()
}

// ── arrow-key moves ──────────────────────────────────────────────────────────

#[test]
fn ed_070_arrow_keys_move_selected() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.click(h.block_center("2"));
    let before = h.block_position("2");
    h.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(h.block_position("2").0, before.0 + 5, "arrow = +5 grid");
    h.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(h.block_position("2").1, before.1 + 5);
    assert!(h.state.history.can_undo());
}

#[test]
fn ed_071_ctrl_arrow_fine_move() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.click(h.block_center("2"));
    let before = h.block_position("2");
    h.key(Key::ArrowRight, Modifiers::CTRL);
    assert_eq!(h.block_position("2").0, before.0 + 1, "ctrl+arrow = +1");
}

// ── delete ───────────────────────────────────────────────────────────────────

#[test]
fn ed_080_delete_removes_block_and_lines() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.click(h.block_center("2"));
    h.key(Key::Delete, Modifiers::NONE);
    let sys = h.sys();
    assert!(
        sys.blocks.iter().all(|b| b.sid.as_deref() != Some("2")),
        "Gain deleted"
    );
    assert!(
        sys.lines
            .iter()
            .all(|l| l.src.as_ref().is_none_or(|e| e.sid != "2")
                && l.dst.as_ref().is_none_or(|e| e.sid != "2")),
        "attached lines deleted"
    );
    // Undo is per-command: blocks restored first, then the cascaded lines.
    h.state.undo();
    assert_eq!(h.sys().blocks.len(), 3, "undo restores block");
    h.state.undo();
    assert_eq!(h.sys().lines.len(), 2, "undo restores cascaded lines");
}

// ── copy/paste ───────────────────────────────────────────────────────────────

#[test]
fn ed_082_copy_paste_creates_offset_clone() {
    let mut h = EditorHarness::new(sys_two());
    h.idle(2);
    h.click(h.block_center("1"));
    h.key(Key::C, Modifiers::CTRL);
    assert!(h.state.clipboard.has_content());
    let n0 = h.sys().blocks.len();
    let p1 = h.block_position("1");
    h.key(Key::V, Modifiers::CTRL);
    assert_eq!(h.sys().blocks.len(), n0 + 1, "paste adds block");
    let new = h.sys().blocks.last().unwrap();
    assert_eq!(new.name, "C_copy");
    assert!(new.sid.is_none(), "pasted block gets fresh sid later");
    let np = harness::parse_pos(new.position.as_deref().unwrap()).unwrap();
    assert_eq!(np.0, p1.0 + 20, "offset +20");
    h.key(Key::V, Modifiers::CTRL);
    let np2 =
        harness::parse_pos(h.sys().blocks.last().unwrap().position.as_deref().unwrap()).unwrap();
    assert_eq!(np2.0, p1.0 + 40, "repeated paste offsets again");
}

// ── rotate / mirror ──────────────────────────────────────────────────────────

#[test]
fn ed_084_rotate_swaps_rect_axes() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.click(h.block_center("2"));
    let (l, t, r, b) = h.block_position("2");
    h.key(Key::R, Modifiers::NONE);
    let (l2, t2, r2, b2) = h.block_position("2");
    assert_eq!(r2 - l2, b - t, "width/height swapped");
    assert_eq!(b2 - t2, r - l);
    assert!(h.state.history.can_undo());
}

#[test]
fn ed_085_mirror_sets_flag() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.click(h.block_center("2"));
    h.key(Key::M, Modifiers::NONE);
    let b = &h.sys().blocks[1];
    assert_eq!(b.block_mirror, Some(true), "mirror flag set");
    h.key(Key::M, Modifiers::NONE);
    assert_eq!(h.sys().blocks[1].block_mirror, Some(false), "toggle back");
}

// ── escape ───────────────────────────────────────────────────────────────────

#[test]
fn ed_089_esc_clears_selection() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.click(h.block_center("2"));
    assert!(!h.selected_blocks().is_empty());
    h.frame(input::esc());
    assert!(h.selected_blocks().is_empty());
}

// ── block browser ────────────────────────────────────────────────────────────

#[test]
fn ed_090_a_opens_browser_esc_closes() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.key(Key::A, Modifiers::NONE);
    assert!(h.state.block_browser.open, "A opens browser");
    h.frame(input::esc());
    assert!(!h.state.block_browser.open, "Esc closes browser");
}

// ── subsystem navigation ─────────────────────────────────────────────────────

#[test]
fn ed_100_double_click_enters_subsystem() {
    let mut h = EditorHarness::new(nested());
    h.idle(2);
    h.run(input::double_click(h.block_center("2")));
    h.idle(1);
    assert_eq!(
        h.state.app.path,
        vec!["SS".to_string()],
        "entered subsystem"
    );
    // Inner blocks now visible
    assert!(h.block_screen_rect("10").is_some());
}

#[test]
fn ed_101_navigate_back_to_root() {
    let mut h = EditorHarness::new(nested());
    h.idle(2);
    h.state.app.navigate_to_path(vec!["SS".to_string()]);
    assert_eq!(h.state.app.path, vec!["SS".to_string()]);
    h.state.app.navigate_to_path(Vec::new());
    assert!(h.state.app.path.is_empty());
    assert!(h.block_screen_rect("2").is_some(), "SS visible again");
}

// ── create subsystem from selection ──────────────────────────────────────────

#[test]
fn ed_110_create_subsystem_wraps_selection() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.click(h.block_center("2"));
    h.state.create_subsystem_from_selection("NewSS");
    let sys = h.sys();
    let ss = sys
        .blocks
        .iter()
        .find(|b| b.name == "NewSS")
        .expect("subsystem block created");
    assert!(ss.subsystem.is_some());
    // Inner system = moved block + auto-created boundary port blocks.
    let inner = ss.subsystem.as_ref().unwrap();
    assert!(inner.blocks.iter().any(|b| b.name == "G0"));
    assert!(
        inner
            .blocks
            .iter()
            .filter(|b| b.block_type == "Inport" || b.block_type == "Outport")
            .count()
            >= 2,
        "boundary ports created"
    );
    assert!(h.state.history.can_undo());
}

// ── undo chain across op types ────────────────────────────────────────────────

#[test]
fn ed_122_undo_chain_mixed_ops() {
    let mut h = EditorHarness::new(sys_two());
    h.idle(2);
    // 1) draw line   2) move block   3) delete block
    h.drag(
        h.port_hit_pos("1", "out", 1),
        h.port_hit_pos("2", "in", 1),
        6,
    );
    let from = h.block_center("2");
    h.drag(from, from + egui::vec2(40.0, 0.0), 6);
    h.click(h.block_center("1"));
    h.key(Key::Delete, Modifiers::NONE);
    assert_eq!(h.sys().blocks.len(), 1);

    h.state.undo(); // restore block
    assert_eq!(h.sys().blocks.len(), 2);
    h.state.undo(); // restore cascaded line delete (block's attached line)
    assert_eq!(h.sys().lines.len(), 1);
    h.state.undo(); // revert move
    h.state.undo(); // remove drawn line
    assert!(h.sys().lines.is_empty());
    assert!(!h.state.history.can_undo(), "history drained");
    h.state.redo();
    assert_eq!(h.sys().lines.len(), 1, "redo replays line add");
}
