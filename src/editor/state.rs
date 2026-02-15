//! Editor state management.
//!
//! [`EditorState`] wraps the existing [`SubsystemApp`] with additional editing
//! state: selection, undo/redo history, drag state, connection drawing,
//! block browser state, code editor, and clipboard.

#![cfg(feature = "egui")]

use std::collections::BTreeMap;

use crate::model::{Block, Chart, System};

use super::block_catalog::{BlockCatalogCategory, get_block_catalog_by_category};
use super::operations::EditorHistory;
use super::selection::EditorSelection;
use crate::egui_app::SubsystemApp;
use crate::egui_app::resolve_subsystem_by_vec;

// ────────────────────────────────────────────────────────────────────────────
// Drag state
// ────────────────────────────────────────────────────────────────────────────

/// What the user is currently dragging.
#[derive(Debug, Clone)]
pub enum DragMode {
    /// Not dragging anything.
    None,
    /// Dragging selected blocks by an accumulated pixel delta.
    Blocks {
        /// Accumulated drag delta in model coordinates.
        dx: f32,
        dy: f32,
    },
    /// Drawing a new connection from a port.
    Connection {
        /// Source block SID.
        src_sid: String,
        /// Source port type ("out" / "in").
        src_port_type: String,
        /// Source port index (1-based).
        src_port_index: u32,
        /// Current endpoint (model coordinates).
        current_x: f32,
        current_y: f32,
    },
    /// Drawing the selection rectangle.
    SelectionRect,
    /// Panning the canvas (same as viewer).
    Pan,
}

impl Default for DragMode {
    fn default() -> Self {
        Self::None
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Block browser state
// ────────────────────────────────────────────────────────────────────────────

/// State for the "Add Block" browser popup.
#[derive(Debug, Clone)]
pub struct BlockBrowserState {
    /// Whether the browser window is open.
    pub open: bool,
    /// Search/filter query text.
    pub query: String,
    /// The catalog categories (lazily loaded once).
    pub categories: Vec<BlockCatalogCategory>,
    /// Currently expanded category index (None = all collapsed).
    pub expanded_category: Option<usize>,
    /// Position where the new block should be placed (model coordinates).
    pub insert_x: i32,
    pub insert_y: i32,
}

impl Default for BlockBrowserState {
    fn default() -> Self {
        Self {
            open: false,
            query: String::new(),
            categories: Vec::new(),
            expanded_category: None,
            insert_x: 200,
            insert_y: 200,
        }
    }
}

impl BlockBrowserState {
    /// Open the browser at the given model position.
    pub fn open_at(&mut self, x: i32, y: i32) {
        self.open = true;
        self.query.clear();
        self.expanded_category = None;
        self.insert_x = x;
        self.insert_y = y;
        if self.categories.is_empty() {
            self.categories = get_block_catalog_by_category().to_vec();
        }
    }

    /// Close the browser.
    pub fn close(&mut self) {
        self.open = false;
        self.query.clear();
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Code editor state
// ────────────────────────────────────────────────────────────────────────────

/// State for the inline code editor window.
#[derive(Debug, Clone)]
pub struct CodeEditorState {
    /// Whether the editor is visible.
    pub open: bool,
    /// Index of the block being edited in the current system.
    pub block_index: usize,
    /// The block name (for display in the title bar).
    pub block_name: String,
    /// Current contents of the code editor.
    pub code: String,
    /// The original code (to detect changes).
    pub original_code: String,
}

impl Default for CodeEditorState {
    fn default() -> Self {
        Self {
            open: false,
            block_index: 0,
            block_name: String::new(),
            code: String::new(),
            original_code: String::new(),
        }
    }
}

impl CodeEditorState {
    /// Open the code editor for a block.
    pub fn open_for_block(&mut self, index: usize, name: &str, code: &str) {
        self.open = true;
        self.block_index = index;
        self.block_name = name.to_string();
        self.code = code.to_string();
        self.original_code = code.to_string();
    }

    /// Returns true if the code has been modified.
    pub fn is_modified(&self) -> bool {
        self.code != self.original_code
    }

    /// Close the editor without saving.
    pub fn close(&mut self) {
        self.open = false;
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Clipboard
// ────────────────────────────────────────────────────────────────────────────

/// Clipboard contents for copy/paste operations.
#[derive(Debug, Clone, Default)]
pub struct EditorClipboard {
    /// Copied blocks.
    pub blocks: Vec<Block>,
    /// Offset for paste positioning.
    pub paste_offset: i32,
}

impl EditorClipboard {
    /// Copy the given blocks to the clipboard.
    pub fn copy_blocks(&mut self, blocks: Vec<Block>) {
        self.blocks = blocks;
        self.paste_offset = 20;
    }

    /// Returns true if the clipboard has content.
    pub fn has_content(&self) -> bool {
        !self.blocks.is_empty()
    }

    /// Clear the clipboard.
    pub fn clear(&mut self) {
        self.blocks.clear();
    }
}

// ────────────────────────────────────────────────────────────────────────────
// EditorState — the top-level state for the editor
// ────────────────────────────────────────────────────────────────────────────

/// The complete state of the Simulink model editor.
///
/// This wraps [`SubsystemApp`] (which manages the viewer aspects: navigation,
/// zoom, pan, chart views) and adds editing-specific state.
///
/// # Example
///
/// ```rust,ignore
/// use rustylink::editor::EditorState;
/// use rustylink::model::System;
/// use std::collections::BTreeMap;
///
/// let system = System { ..Default::default() };
/// let state = EditorState::new(system, vec![], BTreeMap::new(), BTreeMap::new());
/// ```
#[derive(Clone)]
pub struct EditorState {
    /// The underlying viewer application state.
    pub app: SubsystemApp,
    /// Block/line selection.
    pub selection: EditorSelection,
    /// Undo/redo history.
    pub history: EditorHistory,
    /// Current drag operation.
    pub drag_mode: DragMode,
    /// Block browser state.
    pub block_browser: BlockBrowserState,
    /// Code editor state.
    pub code_editor: CodeEditorState,
    /// Clipboard.
    pub clipboard: EditorClipboard,
    /// Whether the model has been modified since last save.
    pub dirty: bool,
    /// Grid snapping enabled.
    pub snap_to_grid: bool,
    /// Grid size for snapping (in model coordinates).
    pub grid_size: i32,
    /// Show grid lines.
    pub show_grid: bool,
}

impl EditorState {
    /// Create a new editor state.
    pub fn new(
        root: System,
        initial_path: Vec<String>,
        charts: BTreeMap<u32, Chart>,
        chart_map: BTreeMap<String, u32>,
    ) -> Self {
        Self {
            app: SubsystemApp::new(root, initial_path, charts, chart_map),
            selection: EditorSelection::new(),
            history: EditorHistory::new(200),
            drag_mode: DragMode::None,
            block_browser: BlockBrowserState::default(),
            code_editor: CodeEditorState::default(),
            clipboard: EditorClipboard::default(),
            dirty: false,
            snap_to_grid: true,
            grid_size: 5,
            show_grid: false,
        }
    }

    /// Convenience: get a reference to the current system.
    pub fn current_system(&self) -> Option<&System> {
        self.app.current_system()
    }

    /// Get a mutable reference to the current system by navigating the path.
    pub fn current_system_mut(&mut self) -> Option<&mut System> {
        resolve_subsystem_by_vec_mut(&mut self.app.root, &self.app.path)
    }

    /// Snap a coordinate to the grid if snapping is enabled.
    pub fn snap(&self, value: i32) -> i32 {
        if self.snap_to_grid && self.grid_size > 0 {
            ((value as f64 / self.grid_size as f64).round() as i32) * self.grid_size
        } else {
            value
        }
    }

    /// Mark the model as dirty (modified).
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Clear the dirty flag (e.g., after saving).
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Copy selected blocks to the clipboard.
    pub fn copy_selection(&mut self) {
        let indices = self.selection.selected_blocks.clone();
        if let Some(system) = resolve_subsystem_by_vec(&self.app.root, &self.app.path) {
            let blocks: Vec<Block> = indices
                .iter()
                .filter_map(|&i| system.blocks.get(i).cloned())
                .collect();
            self.clipboard.copy_blocks(blocks);
        }
    }

    /// Paste clipboard contents into the current system.
    pub fn paste(&mut self) {
        if !self.clipboard.has_content() {
            return;
        }
        let blocks_to_paste = self.clipboard.blocks.clone();
        let offset = self.clipboard.paste_offset;

        if let Some(system) = resolve_subsystem_by_vec_mut(&mut self.app.root, &self.app.path) {
            let mut new_indices = Vec::new();
            for block in blocks_to_paste {
                let mut pasted = block.clone();
                // Offset position
                super::operations::apply_position_delta(&mut pasted, offset, offset);
                // Append suffix to name
                pasted.name = format!("{}_copy", pasted.name);
                // Clear SID (will be reassigned)
                pasted.sid = None;
                let idx = system.blocks.len();
                system.blocks.push(pasted);
                new_indices.push(idx);
            }
        }
        // Update selection outside the borrow
        self.selection.selected_blocks = Vec::new();
        self.selection.selected_lines.clear();
        self.clipboard.paste_offset += 20;
        self.dirty = true;
    }

    /// Delete selected items.
    pub fn delete_selection(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        let line_indices = self.selection.selected_lines.clone();
        let block_indices = self.selection.selected_blocks.clone();
        if let Some(system) = resolve_subsystem_by_vec_mut(&mut self.app.root, &self.app.path) {
            // Delete lines first (higher indices first)
            if !line_indices.is_empty() {
                let cmd = super::operations::delete_lines(system, &line_indices);
                self.history.push(cmd);
            }
            // Then blocks
            if !block_indices.is_empty() {
                let cmd = super::operations::delete_blocks(system, &block_indices);
                self.history.push(cmd);
            }
        }
        self.selection.clear();
        self.dirty = true;
    }

    /// Comment/uncomment selected blocks.
    pub fn comment_selection(&mut self) {
        if self.selection.selected_blocks.is_empty() {
            return;
        }
        let indices = self.selection.selected_blocks.clone();
        if let Some(system) = resolve_subsystem_by_vec_mut(&mut self.app.root, &self.app.path) {
            let cmd = super::operations::comment_blocks(system, &indices);
            self.history.push(cmd);
        }
        self.dirty = true;
    }

    /// Rotate selected blocks.
    pub fn rotate_selection(&mut self) {
        if self.selection.selected_blocks.is_empty() {
            return;
        }
        let indices = self.selection.selected_blocks.clone();
        if let Some(system) = resolve_subsystem_by_vec_mut(&mut self.app.root, &self.app.path) {
            let cmd = super::operations::rotate_blocks(system, &indices);
            self.history.push(cmd);
        }
        self.dirty = true;
    }

    /// Mirror selected blocks.
    pub fn mirror_selection(&mut self) {
        if self.selection.selected_blocks.is_empty() {
            return;
        }
        let indices = self.selection.selected_blocks.clone();
        if let Some(system) = resolve_subsystem_by_vec_mut(&mut self.app.root, &self.app.path) {
            let cmd = super::operations::mirror_blocks(system, &indices);
            self.history.push(cmd);
        }
        self.dirty = true;
    }

    /// Create a subsystem from selected blocks.
    pub fn create_subsystem_from_selection(&mut self, name: &str) {
        if self.selection.selected_blocks.is_empty() {
            return;
        }
        let indices = self.selection.selected_blocks.clone();
        if let Some(system) = resolve_subsystem_by_vec_mut(&mut self.app.root, &self.app.path) {
            let cmd = super::operations::create_subsystem_from_selection(
                system, &indices, name,
            );
            self.history.push(cmd);
        }
        self.selection.clear();
        self.dirty = true;
    }

    /// Undo the last operation.
    pub fn undo(&mut self) {
        if let Some(system) = resolve_subsystem_by_vec_mut(&mut self.app.root, &self.app.path) {
            if self.history.undo(system) {
                self.dirty = true;
            }
        }
    }

    /// Redo the last undone operation.
    pub fn redo(&mut self) {
        if let Some(system) = resolve_subsystem_by_vec_mut(&mut self.app.root, &self.app.path) {
            if self.history.redo(system) {
                self.dirty = true;
            }
        }
    }
}

impl eframe::App for EditorState {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            super::ui::editor_update_with_info(self, ui);
        });
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Mutable subsystem resolution
// ────────────────────────────────────────────────────────────────────────────

/// Resolve a mutable reference to a subsystem by path.
pub fn resolve_subsystem_by_vec_mut<'a>(
    root: &'a mut System,
    path: &[String],
) -> Option<&'a mut System> {
    if path.is_empty() {
        return Some(root);
    }

    let mut current = root;
    for name in path {
        let block = current
            .blocks
            .iter_mut()
            .find(|b| b.name == *name && b.subsystem.is_some())?;
        current = block.subsystem.as_mut()?;
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::System;
    use indexmap::IndexMap;
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

        let block = super::super::operations::create_default_block("Gain", "Gain1", 100, 100, 1, 1);
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
        let mut block = super::super::operations::create_default_block("SubSystem", "Sub1", 100, 100, 1, 1);
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
        let block = super::super::operations::create_default_block("Gain", "Gain1", 100, 100, 1, 1);
        sys.blocks.push(block);
        let mut state = EditorState::new(sys, vec![], BTreeMap::new(), BTreeMap::new());

        // Move block and push to history
        {
            let system = state.current_system_mut().unwrap();
            let cmd = super::super::operations::move_block(system, 0, 200, 200);
            state.history.push(cmd);
        }
        state.mark_dirty();

        // Verify the move happened
        assert_eq!(
            state.current_system().unwrap().blocks[0].position.as_deref(),
            Some("[200, 200, 230, 230]")
        );

        // Undo
        state.undo();
        assert_eq!(
            state.current_system().unwrap().blocks[0].position.as_deref(),
            Some("[100, 100, 130, 130]")
        );

        // Redo
        state.redo();
        assert_eq!(
            state.current_system().unwrap().blocks[0].position.as_deref(),
            Some("[200, 200, 230, 230]")
        );
    }

    #[test]
    fn test_delete_selection() {
        let mut sys = make_empty_system();
        sys.blocks.push(super::super::operations::create_default_block("Gain", "Gain1", 100, 100, 1, 1));
        sys.blocks.push(super::super::operations::create_default_block("Sum", "Sum1", 200, 100, 2, 1));
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
        let block = super::super::operations::create_default_block("Gain", "Gain1", 100, 100, 1, 1);
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
        sys.blocks.push(super::super::operations::create_default_block("Gain", "Gain1", 100, 100, 1, 1));
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
}
