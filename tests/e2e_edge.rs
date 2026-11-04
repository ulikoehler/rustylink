//! E2E: edge cases — mirrored blocks, resize clamps, live-value formatting.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use harness::{BlockBuilder, EditorHarness, SystemBuilder, line};
use rustylink::live_values::{LiveValue, LiveValueList};

#[test]
fn edge_001_mirrored_block_out_port_left() {
    // Mirrored block: out ports appear on the LEFT — connection drag must
    // start from the left-edge hit position.
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(200, 0, 240, 24)
                .ports(0, 1)
                .mirrored()
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "D", "2")
                .pos(0, 0, 40, 24)
                .ports(1, 0)
                .build(),
        )
        .build();
    let mut h = EditorHarness::new(sys);
    h.idle(2);
    // Mirrored out port lives on the left edge — port_hit_pos is mirror-aware.
    let from = h.port_hit_pos("1", "out", 1);
    let r = h.block_screen_rect("1").unwrap();
    assert!(
        from.x <= r.left() + 2.0,
        "out hit at left edge when mirrored"
    );
    h.drag(from, h.port_hit_pos("2", "in", 1), 8);
    assert_eq!(
        h.sys().lines.len(),
        1,
        "connection from mirrored port works"
    );
    assert_eq!(h.sys().lines[0].src.as_ref().unwrap().sid, "1");
}

#[test]
fn edge_002_mirror_undo_restores() {
    let mut h = EditorHarness::new(harness::gain_chain(1));
    h.idle(2);
    h.click(h.block_center("2"));
    h.key(eframe::egui::Key::M, eframe::egui::Modifiers::NONE);
    assert_eq!(h.sys().blocks[1].block_mirror, Some(true));
    h.state.undo();
    assert_ne!(h.sys().blocks[1].block_mirror, Some(true), "mirror undone");
}

#[test]
fn edge_003_resize_minimum_clamp() {
    // Dragging a resize handle through the block must clamp to a min size,
    // not produce a negative-width rect.
    let mut h = EditorHarness::new(harness::gain_chain(1));
    h.idle(2);
    h.click(h.block_center("2"));
    let br = h.resize_handle_pos("2", 4);
    h.drag(br, br + eframe::egui::vec2(-300.0, -200.0), 8);
    let (l, _t, r, b) = h.block_position("2");
    assert!(r >= l, "width never negative after over-shrink");
    assert!(b >= h.block_position("2").1, "height never negative");
}

#[test]
fn edge_010_live_value_display_formats() {
    // Verify live value formatting for common types via the entry API.
    let v = LiveValue::new(vec![1], LiveValueList::Float64(vec![1.234]));
    let e = rustylink::live_values::LiveValueEntry::new(v);
    assert!(e.formatted_text().starts_with("1.23"));

    let bv = LiveValue::new(vec![1], LiveValueList::Bool(vec![true]));
    let be = rustylink::live_values::LiveValueEntry::new(bv);
    assert!(!be.formatted_text().is_empty());

    let vv = LiveValue::new(vec![3], LiveValueList::Float64(vec![1.0, 2.0, 3.0]));
    let ve = rustylink::live_values::LiveValueEntry::new(vv);
    let t = ve.formatted_text();
    assert!(t.contains('1') && t.contains('3'));
}

#[test]
fn edge_011_multiport_hit_positions_differ() {
    // Two-in block: in:1 vs in:2 hit positions must be distinct and ordered.
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Gain", "G", "1")
                .pos(0, 0, 40, 60)
                .ports(2, 1)
                .build(),
        )
        .build();
    let mut h = EditorHarness::new(sys);
    h.idle(2);
    let p1 = h.port_hit_pos("1", "in", 1);
    let p2 = h.port_hit_pos("1", "in", 2);
    assert_ne!(p1, p2, "distinct port hit positions");
    assert!(p1.y < p2.y, "port 1 above port 2");
}

#[test]
fn edge_012_line_endpoint_updates_on_block_delete_and_undo() {
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "D", "2")
                .pos(100, 0, 140, 24)
                .ports(1, 0)
                .build(),
        )
        .line(line("1", 1, "2", 1))
        .build();
    let mut h = EditorHarness::new(sys);
    h.idle(2);
    h.click(h.block_center("2"));
    h.key(eframe::egui::Key::Delete, eframe::egui::Modifiers::NONE);
    assert!(h.sys().lines.is_empty(), "cascade removed line");
    h.state.undo();
    h.state.undo();
    harness::debug_validate(h.sys());
    assert_eq!(h.sys().lines.len(), 1, "line restored by cascade undo");
}
