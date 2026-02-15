//! Comprehensive Simulink model editor (feature = "egui").
//!
//! This module provides a full-featured graphical editor for Simulink models,
//! building upon the existing viewer infrastructure. It supports:
//!
//! - **Block manipulation**: Moving, adding, deleting, rotating, mirroring blocks
//! - **Connection editing**: Drawing, dragging, branching, and snapping signal lines
//! - **Selection**: Rectangle selection of blocks and lines, multi-select operations
//! - **Block browser**: 750+ block types organized by category (hotkey "A")
//! - **Code editing**: Inline code editor for MATLAB Function and CFunction blocks
//! - **Subsystem creation**: Group selected blocks into a new subsystem
//! - **Commenting**: Toggle commented state on blocks
//! - **Labels**: Add/edit names on signal lines
//! - **Context menus**: Rich context menus for blocks, lines, and canvas
//! - **ID management**: Automatic SID assignment and reassignment
//! - **Undo/Redo**: Full undo/redo stack for all editing operations

#![cfg(feature = "egui")]

pub mod block_catalog;
pub mod operations;
pub mod selection;
pub mod state;
pub mod ui;

pub use block_catalog::{BlockCatalogCategory, BlockCatalogEntry, get_block_catalog};
pub use operations::{
    EditorCommand, EditorHistory, add_block, add_line, assign_sids, branch_line,
    comment_blocks, create_subsystem_from_selection, delete_blocks, delete_lines,
    mirror_blocks, move_block, move_blocks, rename_line, rotate_blocks,
};
pub use selection::{EditorSelection, SelectionRect};
pub use state::EditorState;
pub use ui::{editor_update, editor_update_with_info};
