//! E2E: remaining coverage — empty systems, labels, zorder, resolver on
//! degenerate models.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use harness::model_builders::line;
use harness::{BlockBuilder, EditorHarness, SystemBuilder, ViewerHarness, gain_chain};
use rustylink::connection_targets::ConnectionTargetResolver;

#[test]
fn mx_001_empty_system_renders_no_panic() {
    let sys = SystemBuilder::new("model").build();
    let mut h = ViewerHarness::new(sys);
    h.idle(3); // must not panic — shows "no blocks" message
}

#[test]
fn mx_002_editor_empty_model_add_block_flow() {
    // Editor on empty model renders the "press A" hint without panicking.
    let sys = SystemBuilder::new("model").build();
    let mut h = EditorHarness::new(sys);
    h.idle(3);
    h.key(eframe::egui::Key::A, eframe::egui::Modifiers::NONE);
    // Empty model may return before the shortcut path — just no panic.
    h.idle(1);
}

#[test]
fn mx_003_resolver_empty_system() {
    let sys = SystemBuilder::new("model").build();
    let res = ConnectionTargetResolver::new(&sys);
    let _ = res; // constructed on empty — no lines to query
    assert!(sys.lines.is_empty());
}

#[test]
fn mx_004_self_loop_resolves_safely() {
    // A block wired to itself — resolver must not hang/crash.
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Gain", "G", "1")
                .pos(0, 0, 40, 24)
                .ports(1, 1)
                .build(),
        )
        .line(line("1", 1, "1", 1))
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    let _ = res.line_targets_for_line(&[], &sys.lines[0]); // must not hang
}

#[test]
fn mx_005_named_line_label_persists() {
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
        .line(
            harness::model_builders::LineBuilder::new()
                .name("sig_x")
                .from("1", 1)
                .to("2", 1)
                .build(),
        )
        .build();
    let res = ConnectionTargetResolver::new(&sys);
    let t = res.line_targets_for_line(&[], &sys.lines[0]);
    assert!(t.iter().any(|x| x.signal_name.as_deref() == Some("sig_x")));
}

#[test]
fn mx_006_zorder_sorted_render_no_panic() {
    // Blocks with differing zorder — must render in any order without panic.
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "Back", "1")
                .pos(0, 0, 100, 60)
                .ports(0, 1)
                .prop("ZOrder", "1")
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "Front", "2")
                .pos(10, 5, 50, 40)
                .ports(1, 0)
                .prop("ZOrder", "99")
                .build(),
        )
        .line(line("1", 1, "2", 1))
        .build();
    let mut h = EditorHarness::new(sys);
    h.idle(3);
    harness::debug_validate(h.sys());
}

#[test]
fn mx_007_chain_of_50_blocks_resolves() {
    // Long serial chain — resolver must handle depth without blowup.
    let sys = gain_chain(50);
    harness::debug_validate(&sys);
    let res = ConnectionTargetResolver::new(&sys);
    let t = res.line_targets_for_line(&[], sys.lines.last().unwrap());
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].path, "model/G49");
}

#[test]
fn mx_008_line_rename_via_operation() {
    let mut s = gain_chain(1);
    rustylink::editor::operations::rename_line(&mut s, 0, Some("renamed_sig".into()));
    assert_eq!(s.lines[0].name.as_deref(), Some("renamed_sig"));
}

#[test]
fn mx_009_line_with_many_branches() {
    // One source, 8 branch destinations — resolver + validator hold.
    let mut b = SystemBuilder::new("model").block(
        BlockBuilder::new("Constant", "Src", "1")
            .pos(0, 0, 40, 24)
            .ports(0, 1)
            .build(),
    );
    let mut lb = harness::model_builders::LineBuilder::new().from("1", 1);
    for i in 0..8usize {
        let sid = (i + 2).to_string();
        b = b.block(
            BlockBuilder::new("Display", &format!("S{i}"), &sid)
                .pos(100, i as i32 * 40, 140, i as i32 * 40 + 24)
                .ports(1, 0)
                .build(),
        );
        lb = lb.branch_to(&sid, 1);
    }
    let sys = b.line(lb.build()).build();
    harness::debug_validate(&sys);
    let res = ConnectionTargetResolver::new(&sys);
    let t = res.line_targets_for_line(&[], &sys.lines[0]);
    assert!(t.iter().all(|x| x.path == "model/Src"));
}
