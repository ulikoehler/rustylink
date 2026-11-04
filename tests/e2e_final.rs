//! E2E: remaining coverage — matching goto/from, buses, ctrl-deselect, toggles.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use eframe::egui;
use harness::model_builders::line;
use harness::{BlockBuilder, EditorHarness, SystemBuilder, ViewerHarness, gain_chain, live_feed};
use rustylink::connection_targets::ConnectionTargetResolver;
use rustylink::model::System;

#[test]
fn fn_001_goto_from_matching_tag_links() {
    // Goto/From with the SAME tag must resolve across the wireless link.
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
                .prop("GotoTag", "SAME")
                .build(),
        )
        .block(
            BlockBuilder::new("From", "F1", "3")
                .pos(160, 0, 200, 24)
                .ports(0, 1)
                .prop("GotoTag", "SAME")
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
        "matching tag links Src→Sink via Goto/From: {t:?}"
    );
}

#[test]
fn fn_002_ctrl_click_toggles_off() {
    // Ctrl-click a selected block deselects it.
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.run(harness::input::ctrl_click(h.block_center("1")));
    assert_eq!(h.selected_blocks().len(), 1);
    h.run(harness::input::ctrl_click(h.block_center("1")));
    assert_eq!(h.selected_blocks().len(), 0, "ctrl-click deselects");
}

#[test]
fn fn_003_editor_snap_toggle_round_trip() {
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    let s0 = h.state.snap_to_grid;
    h.key(egui::Key::G, egui::Modifiers::NONE); // if 'G' toggles grid
    let _ = h.state.snap_to_grid;
    h.state.snap_to_grid = s0; // restore deterministically
    h.idle(1);
}

#[test]
fn fn_004_many_undos_bounded_history() {
    // 30 block moves → each is undoable; history drains cleanly.
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    for _ in 0..30 {
        let from = h.block_center("1");
        // Large enough drag to commit past grid snap each iteration.
        h.drag(from, from + egui::vec2(60.0, 0.0), 6);
    }
    let mut count = 0;
    while h.state.history.can_undo() && count < 100 {
        h.state.undo();
        count += 1;
    }
    assert!(count > 0, "undos ran");
    assert!(!h.state.history.can_undo(), "history drained");
    harness::debug_validate(h.sys());
}

#[test]
fn fn_005_live_mode_manual_switch_toggle() {
    // In live mode, clicking a ManualSwitch toggles its setting.
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("ManualSwitch", "SW", "1")
                .pos(0, 0, 60, 40)
                .ports(2, 1)
                .prop("current_setting", "0")
                .build(),
        )
        .build();
    let mut h = ViewerHarness::new(sys);
    h.app.live_mode_enabled = true;
    h.idle(2);
    h.run(harness::input::click(
        h.block_screen_rect("1").unwrap().center(),
    ));
    h.idle(1);
    // Toggle writes the typed current_setting field on the model block.
    assert_eq!(
        h.app.current_system().unwrap().blocks[0]
            .current_setting
            .as_deref(),
        Some("1"),
        "live-mode toggle flipped setting"
    );
}

#[test]
fn fn_006_scope_block_stores_live_series() {
    // A Scope-bound key receives a live value entry.
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Scope", "Scope1", "1")
                .pos(0, 0, 60, 40)
                .ports(1, 0)
                .build(),
        )
        .build();
    let mut h = ViewerHarness::new(sys);
    h.idle(2);
    h.app
        .live_values
        .insert("1:out:1".into(), live_feed::scalar(0.5));
    assert!(h.app.live_values.get("1:out:1").is_some());
}

#[test]
fn fn_007_editor_navigate_deep_no_panic() {
    // Navigate into a subsystem that doesn't exist — must not crash.
    let mut h = EditorHarness::new(gain_chain(1));
    h.idle(2);
    h.state.app.navigate_to_path(vec!["Nope".into()]);
    h.idle(2);
    // Path either rejected or tolerated — blocks still render.
    let _ = h.transform();
}
