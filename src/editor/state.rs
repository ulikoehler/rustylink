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
    /// Resizing a block via a handle.
    Resize {
        /// Index of the block being resized.
        block_index: usize,
        /// Which resize handle: 0=TL, 1=T, 2=TR, 3=R, 4=BR, 5=B, 6=BL, 7=L
        handle: u8,
        /// Original block position before resize started.
        original_l: i32,
        original_t: i32,
        original_r: i32,
        original_b: i32,
        /// Accumulated drag delta in model coordinates.
        dx: f32,
        dy: f32,
    },
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
            let cmd = super::operations::create_subsystem_from_selection(system, &indices, name);
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
