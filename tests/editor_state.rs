use indexmap::IndexMap;
use rustylink::editor::state::{
    BlockBrowserState, CodeEditorState, DragMode, EditorState, resolve_subsystem_by_vec_mut,
};
use rustylink::model::System;
use std::collections::BTreeMap;

fn make_empty_system() -> System {
    System {
        properties: IndexMap::new(),
        blocks: Vec::new(),
        lines: Vec::new(),
        annotations: Vec::new(),
        chart: None,
    }
}

#[test]
fn test_editor_state_new() {
    let sys = make_empty_system();
    let state = EditorState::new(sys, vec![], BTreeMap::new(), BTreeMap::new());
    assert!(!state.dirty);
    assert!(state.selection.is_empty());
    assert!(state.history.can_undo() == false);
}

#[test]
fn test_editor_state_snap() {
    let sys = make_empty_system();
    let state = EditorState::new(sys, vec![], BTreeMap::new(), BTreeMap::new());
    // grid_size=5 by default
    assert_eq!(state.snap(12), 10);
    assert_eq!(state.snap(13), 15);
    assert_eq!(state.snap(0), 0);
}

#[test]
fn test_editor_state_snap_disabled() {
    let sys = make_empty_system();
    let mut state = EditorState::new(sys, vec![], BTreeMap::new(), BTreeMap::new());
    state.snap_to_grid = false;
    assert_eq!(state.snap(12), 12);
}

#[test]
fn test_editor_state_dirty() {
    let sys = make_empty_system();
    let mut state = EditorState::new(sys, vec![], BTreeMap::new(), BTreeMap::new());
    assert!(!state.dirty);
    state.mark_dirty();
    assert!(state.dirty);
    state.clear_dirty();
    assert!(!state.dirty);
}

#[test]
fn test_clipboard_copy_paste() {
    let sys = make_empty_system();
    let mut state = EditorState::new(sys, vec![], BTreeMap::new(), BTreeMap::new());
    assert!(!state.clipboard.has_content());

    let block =
        rustylink::editor::operations::create_default_block("Gain", "Gain1", 100, 100, 1, 1);
    state.clipboard.copy_blocks(vec![block]);
    assert!(state.clipboard.has_content());

    state.clipboard.clear();
    assert!(!state.clipboard.has_content());
}

#[test]
fn test_code_editor_state() {
    let mut ce = CodeEditorState::default();
    assert!(!ce.open);
    ce.open_for_block(0, "TestBlock", "function y = f(x)\n  y = x;\nend");
    assert!(ce.open);
    assert!(!ce.is_modified());
    ce.code.push_str("\n// changed");
    assert!(ce.is_modified());
    ce.close();
    assert!(!ce.open);
}

#[test]
fn test_block_browser_state() {
    let mut bb = BlockBrowserState::default();
    assert!(!bb.open);
    bb.open_at(150, 250);
    assert!(bb.open);
    assert_eq!(bb.insert_x, 150);
    assert_eq!(bb.insert_y, 250);
    assert!(!bb.categories.is_empty());
    bb.close();
    assert!(!bb.open);
}

#[test]
fn test_resolve_subsystem_by_vec_mut() {
    let mut root = make_empty_system();
    let child = make_empty_system();
    let mut block =
        rustylink::editor::operations::create_default_block("SubSystem", "Sub1", 100, 100, 1, 1);
    block.subsystem = Some(Box::new(child));
    root.blocks.push(block);

    let resolved = resolve_subsystem_by_vec_mut(&mut root, &["Sub1".to_string()]);
    assert!(resolved.is_some());

    let not_found = resolve_subsystem_by_vec_mut(&mut root, &["NotExist".to_string()]);
    assert!(not_found.is_none());
}

#[test]
fn test_resolve_subsystem_empty_path() {
    let mut root = make_empty_system();
    let resolved = resolve_subsystem_by_vec_mut(&mut root, &[]);
    assert!(resolved.is_some());
}

#[test]
fn test_drag_mode_default() {
    let dm = DragMode::default();
    assert!(matches!(dm, DragMode::None));
}

#[test]
fn test_editor_undo_redo() {
    let mut sys = make_empty_system();
    let block =
        rustylink::editor::operations::create_default_block("Gain", "Gain1", 100, 100, 1, 1);
    sys.blocks.push(block);
    let mut state = EditorState::new(sys, vec![], BTreeMap::new(), BTreeMap::new());

    // Move block and push to history
    {
        let system = state.current_system_mut().unwrap();
        let cmd = rustylink::editor::operations::move_block(system, 0, 200, 200);
        state.history.push(cmd);
    }
    state.mark_dirty();

    // Verify the move happened
    assert_eq!(
        state.current_system().unwrap().blocks[0]
            .position
            .as_deref(),
        Some("[200, 200, 230, 230]")
    );

    // Undo
    state.undo();
    assert_eq!(
        state.current_system().unwrap().blocks[0]
            .position
            .as_deref(),
        Some("[100, 100, 130, 130]")
    );

    // Redo
    state.redo();
    assert_eq!(
        state.current_system().unwrap().blocks[0]
            .position
            .as_deref(),
        Some("[200, 200, 230, 230]")
    );
}

#[test]
fn test_delete_selection() {
    let mut sys = make_empty_system();
    sys.blocks
        .push(rustylink::editor::operations::create_default_block(
            "Gain", "Gain1", 100, 100, 1, 1,
        ));
    sys.blocks
        .push(rustylink::editor::operations::create_default_block(
            "Sum", "Sum1", 200, 100, 2, 1,
        ));
    let mut state = EditorState::new(sys, vec![], BTreeMap::new(), BTreeMap::new());

    state.selection.select_block(0);
    state.delete_selection();

    assert_eq!(state.current_system().unwrap().blocks.len(), 1);
    assert_eq!(state.current_system().unwrap().blocks[0].name, "Sum1");
    assert!(state.selection.is_empty());
    assert!(state.dirty);
}

#[test]
fn test_comment_selection() {
    let mut sys = make_empty_system();
    let block =
        rustylink::editor::operations::create_default_block("Gain", "Gain1", 100, 100, 1, 1);
    sys.blocks.push(block);
    let mut state = EditorState::new(sys, vec![], BTreeMap::new(), BTreeMap::new());

    assert!(!state.current_system().unwrap().blocks[0].commented);
    state.selection.select_block(0);
    state.comment_selection();
    assert!(state.current_system().unwrap().blocks[0].commented);
}

#[test]
fn test_copy_paste() {
    let mut sys = make_empty_system();
    sys.blocks
        .push(rustylink::editor::operations::create_default_block(
            "Gain", "Gain1", 100, 100, 1, 1,
        ));
    let mut state = EditorState::new(sys, vec![], BTreeMap::new(), BTreeMap::new());

    state.selection.select_block(0);
    state.copy_selection();
    assert!(state.clipboard.has_content());

    state.paste();
    assert_eq!(state.current_system().unwrap().blocks.len(), 2);
    let pasted = &state.current_system().unwrap().blocks[1];
    assert_eq!(pasted.name, "Gain1_copy");
    assert!(pasted.sid.is_none());
}
