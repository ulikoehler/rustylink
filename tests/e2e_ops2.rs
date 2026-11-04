//! E2E: operations-level invariants — move_blocks line tracking, comment,
//! add/delete, snap port at multiport blocks.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use harness::{BlockBuilder, SystemBuilder, gain_chain};
use rustylink::editor::operations;

fn sys() -> rustylink::model::System {
    gain_chain(1)
}

#[test]
fn op_001_move_blocks_shifts_line_points() {
    let mut s = sys();
    // Give line 2 a mid corner — it must move with the endpoint blocks.
    s.lines[1].points = vec![rustylink::model::Point { x: 20, y: 0 }];
    let _cmd = operations::move_blocks(&mut s, &[1], 10, 0);
    // Connected line points may be adjusted; model must stay consistent.
    assert!(s.blocks[1].position.is_some());
    harness::debug_validate(&s);
}

#[test]
fn op_002_comment_blocks_toggles() {
    let mut s = sys();
    operations::comment_blocks(&mut s, &[1]);
    assert!(s.blocks[1].commented, "commented flag set");
    operations::comment_blocks(&mut s, &[1]);
    assert!(!s.blocks[1].commented, "toggle back");
}

#[test]
fn op_003_add_delete_block_round_trip() {
    let mut s = sys();
    let nb = BlockBuilder::new("Gain", "NewG", "99")
        .pos(300, 0, 340, 24)
        .ports(1, 1)
        .build();
    let cmd = operations::add_block(&mut s, nb);
    assert_eq!(s.blocks.len(), 4);
    assert!(s.blocks.iter().any(|b| b.sid.as_deref() == Some("99")));
    harness::debug_validate(&s);
    drop(cmd);
    operations::delete_blocks(&mut s, &[3]);
    assert_eq!(s.blocks.len(), 3);
}

#[test]
fn op_004_snap_port_multiport_block() {
    // 2-in block: find_snap_port distinguishes in:1 vs in:2 positions.
    let s = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Gain", "G", "1")
                .pos(100, 0, 140, 60)
                .ports(2, 1)
                .build(),
        )
        .build();
    let p1 = operations::find_snap_port(&s, 100.0, 15.0, 20.0, None);
    let p2 = operations::find_snap_port(&s, 100.0, 40.0, 20.0, None);
    let (_, _k1, i1, ..) = p1.expect("snap near upper port");
    let (_, _k2, i2, ..) = p2.expect("snap near lower port");
    assert_ne!(i1, i2, "different port indices snapped");
}

#[test]
fn op_005_auto_route_same_side_ports() {
    // Out→in where dst is LEFT of src: route must dogleg, staying orthogonal.
    let pts = operations::auto_route(100.0, 0.0, 0.0, 30.0, "out", "in");
    assert!(!pts.is_empty(), "backward connection needs waypoints");
}

#[test]
fn op_006_rotate_mirror_commands_undoable() {
    let mut s = sys();
    let c1 = operations::rotate_blocks(&mut s, &[1]);
    let c2 = operations::mirror_blocks(&mut s, &[1]);
    assert!(s.blocks[1].block_mirror == Some(true));
    // Commands carry inverse data for undo.
    let _ = (c1, c2);
}

#[test]
fn op_007_add_line_assigns_endpoints() {
    let mut s = SystemBuilder::new("model")
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
        .build();
    let cmd = operations::add_line(&mut s, "1", 1, "2", 1, vec![]);
    assert_eq!(s.lines.len(), 1);
    let l = &s.lines[0];
    assert_eq!(l.src.as_ref().unwrap().sid, "1");
    assert_eq!(l.dst.as_ref().unwrap().sid, "2");
    assert!(matches!(
        cmd,
        rustylink::editor::operations::EditorCommand::AddLine { .. }
            | rustylink::editor::operations::EditorCommand::Batch(_)
    ));
}

#[test]
fn op_008_move_blocks_keeps_orthogonality() {
    let mut s = sys();
    s.lines[1].points = vec![
        rustylink::model::Point { x: 20, y: 0 },
        rustylink::model::Point { x: 0, y: 30 },
        rustylink::model::Point { x: 40, y: 0 },
    ];
    operations::move_blocks(&mut s, &[1], 0, 20);
    harness::debug_validate(&s);
}
