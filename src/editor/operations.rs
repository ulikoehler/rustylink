//! Editing operations for Simulink models.
//!
//! This module provides all low-level model-mutation operations used by the
//! editor UI. Each operation works directly on [`System`] / [`Block`] / [`Line`]
//! structures and can be recorded for undo/redo via [`EditorCommand`].
//!
//! # Design
//!
//! Operations are pure functions that mutate the model in-place. The
//! [`EditorHistory`] struct wraps them with undo/redo support by storing
//! inverse commands.

#![cfg(feature = "egui")]

use crate::model::{
    Block, BlockChildKind, Branch, EndpointRef, Line, NameLocation, Point, Port,
    PortCounts, System,
};
use indexmap::IndexMap;
use std::collections::BTreeSet;

// ────────────────────────────────────────────────────────────────────────────
// Editor Command (undo/redo unit)
// ────────────────────────────────────────────────────────────────────────────

/// A single undoable editor operation.
///
/// Each variant captures enough state to reverse the operation.
#[derive(Debug, Clone)]
pub enum EditorCommand {
    /// Move a block to a new position.
    MoveBlock {
        block_index: usize,
        old_position: String,
        new_position: String,
    },
    /// Move multiple blocks by a delta offset.
    MoveBlocks {
        block_indices: Vec<usize>,
        dx: i32,
        dy: i32,
    },
    /// Add a new block at a given index.
    AddBlock {
        block_index: usize,
        block: Box<Block>,
    },
    /// Delete blocks at given indices (sorted descending for safe removal).
    DeleteBlocks {
        /// (original_index, block) pairs sorted by descending index.
        removed: Vec<(usize, Block)>,
    },
    /// Add a new line.
    AddLine {
        line_index: usize,
        line: Box<Line>,
    },
    /// Delete lines at given indices (sorted descending).
    DeleteLines {
        removed: Vec<(usize, Line)>,
    },
    /// Toggle the commented state of blocks.
    CommentBlocks {
        block_indices: Vec<usize>,
    },
    /// Rotate blocks 90° clockwise (changes port layout by swapping width/height).
    RotateBlocks {
        block_indices: Vec<usize>,
        /// Old positions before rotation, matching `block_indices` order.
        old_positions: Vec<String>,
    },
    /// Mirror blocks (flip input/output sides).
    MirrorBlocks {
        block_indices: Vec<usize>,
    },
    /// Rename a signal line.
    RenameLine {
        line_index: usize,
        old_name: Option<String>,
        new_name: Option<String>,
    },
    /// Create a subsystem from selected blocks: stores the removed blocks and
    /// lines plus the new subsystem block and its rewired external lines.
    CreateSubsystem {
        /// Blocks that were removed from the parent system (descending index order).
        removed_blocks: Vec<(usize, Block)>,
        /// Lines that were removed from the parent system (descending index order).
        removed_lines: Vec<(usize, Line)>,
        /// The new SubSystem block that was inserted.
        subsystem_block_index: usize,
        subsystem_block: Box<Block>,
        /// Lines that were added to replace removed connections.
        added_lines: Vec<(usize, Line)>,
    },
    /// Add a branch to an existing line.
    BranchLine {
        line_index: usize,
        branch: Branch,
    },
    /// Reassign all SIDs in the system.
    ReassignSids {
        /// (block_index, old_sid) pairs for reversal.
        old_sids: Vec<(usize, Option<String>)>,
    },
    /// Batch command combining multiple sub-commands.
    Batch(Vec<EditorCommand>),
}

// ────────────────────────────────────────────────────────────────────────────
// Editor History (undo / redo stack)
// ────────────────────────────────────────────────────────────────────────────

/// Undo/redo history for the editor.
///
/// # Example
///
/// ```rust,ignore
/// let mut history = EditorHistory::new(100);
/// let cmd = move_block(&mut system, 0, 100, 200);
/// history.push(cmd);
/// history.undo(&mut system); // reverts the move
/// history.redo(&mut system); // re-applies the move
/// ```
#[derive(Debug, Clone)]
pub struct EditorHistory {
    undo_stack: Vec<EditorCommand>,
    redo_stack: Vec<EditorCommand>,
    max_size: usize,
}

impl EditorHistory {
    /// Create a new history with the given maximum undo depth.
    pub fn new(max_size: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_size,
        }
    }

    /// Push a command onto the undo stack and clear the redo stack.
    pub fn push(&mut self, cmd: EditorCommand) {
        self.undo_stack.push(cmd);
        self.redo_stack.clear();
        if self.undo_stack.len() > self.max_size {
            self.undo_stack.remove(0);
        }
    }

    /// Undo the last command, returning true if an undo was performed.
    pub fn undo(&mut self, system: &mut System) -> bool {
        if let Some(cmd) = self.undo_stack.pop() {
            let inverse = apply_inverse(system, &cmd);
            self.redo_stack.push(inverse);
            true
        } else {
            false
        }
    }

    /// Redo the last undone command, returning true if a redo was performed.
    pub fn redo(&mut self, system: &mut System) -> bool {
        if let Some(cmd) = self.redo_stack.pop() {
            let inverse = apply_inverse(system, &cmd);
            self.undo_stack.push(inverse);
            true
        } else {
            false
        }
    }

    /// Returns true if there are commands to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns true if there are commands to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

/// Apply the inverse of a command to the system, returning the forward command
/// for the redo stack.
fn apply_inverse(system: &mut System, cmd: &EditorCommand) -> EditorCommand {
    match cmd {
        EditorCommand::MoveBlock {
            block_index,
            old_position,
            new_position,
        } => {
            if let Some(block) = system.blocks.get_mut(*block_index) {
                block.position = Some(old_position.clone());
                if let Some(v) = block.properties.get_mut("Position") {
                    *v = old_position.clone();
                }
            }
            EditorCommand::MoveBlock {
                block_index: *block_index,
                old_position: new_position.clone(),
                new_position: old_position.clone(),
            }
        }
        EditorCommand::MoveBlocks {
            block_indices,
            dx,
            dy,
        } => {
            for &idx in block_indices {
                if let Some(block) = system.blocks.get_mut(idx) {
                    apply_position_delta(block, -dx, -dy);
                }
            }
            EditorCommand::MoveBlocks {
                block_indices: block_indices.clone(),
                dx: -dx,
                dy: -dy,
            }
        }
        EditorCommand::AddBlock {
            block_index,
            block: _,
        } => {
            let removed = if *block_index < system.blocks.len() {
                system.blocks.remove(*block_index)
            } else {
                return EditorCommand::AddBlock {
                    block_index: *block_index,
                    block: Box::new(Block {
                        block_type: String::new(),
                        name: String::new(),
                        sid: None,
                        tag_name: "Block".to_string(),
                        position: None,
                        zorder: None,
                        commented: false,
                        name_location: NameLocation::Bottom,
                        is_matlab_function: false,
                        value: None,
                        value_kind: crate::model::ValueKind::Unknown,
                        value_rows: None,
                        value_cols: None,
                        properties: IndexMap::new(),
                        ref_properties: BTreeSet::new(),
                        port_counts: None,
                        ports: Vec::new(),
                        subsystem: None,
                        system_ref: None,
                        c_function: None,
                        instance_data: None,
                        link_data: None,
                        mask: None,
                        annotations: Vec::new(),
                        background_color: None,
                        show_name: None,
                        font_size: None,
                        font_weight: None,
                        mask_display_text: None,
                        current_setting: None,
                        block_mirror: None,
                        library_source: None,
                        library_block_path: None,
                        child_order: Vec::new(),
                    }),
                };
            };
            EditorCommand::DeleteBlocks {
                removed: vec![(*block_index, removed)],
            }
        }
        EditorCommand::DeleteBlocks { removed } => {
            // Re-insert in ascending index order
            let mut sorted: Vec<_> = removed.clone();
            sorted.sort_by_key(|(i, _)| *i);
            for (idx, block) in &sorted {
                if *idx <= system.blocks.len() {
                    system.blocks.insert(*idx, block.clone());
                } else {
                    system.blocks.push(block.clone());
                }
            }
            // Build the inverse: delete those same indices (descending order)
            let mut indices: Vec<usize> = sorted.iter().map(|(i, _)| *i).collect();
            indices.reverse();
            EditorCommand::AddBlock {
                block_index: sorted.first().map_or(0, |(i, _)| *i),
                block: Box::new(sorted.first().map_or_else(
                    || Block {
                        block_type: String::new(),
                        name: String::new(),
                        sid: None,
                        tag_name: "Block".to_string(),
                        position: None,
                        zorder: None,
                        commented: false,
                        name_location: NameLocation::Bottom,
                        is_matlab_function: false,
                        value: None,
                        value_kind: crate::model::ValueKind::Unknown,
                        value_rows: None,
                        value_cols: None,
                        properties: IndexMap::new(),
                        ref_properties: BTreeSet::new(),
                        port_counts: None,
                        ports: Vec::new(),
                        subsystem: None,
                        system_ref: None,
                        c_function: None,
                        instance_data: None,
                        link_data: None,
                        mask: None,
                        annotations: Vec::new(),
                        background_color: None,
                        show_name: None,
                        font_size: None,
                        font_weight: None,
                        mask_display_text: None,
                        current_setting: None,
                        block_mirror: None,
                        library_source: None,
                        library_block_path: None,
                        child_order: Vec::new(),
                    },
                    |(_, b)| b.clone(),
                )),
            }
        }
        EditorCommand::AddLine { line_index, line: _ } => {
            let removed = if *line_index < system.lines.len() {
                system.lines.remove(*line_index)
            } else {
                return cmd.clone();
            };
            EditorCommand::DeleteLines {
                removed: vec![(*line_index, removed)],
            }
        }
        EditorCommand::DeleteLines { removed } => {
            let mut sorted: Vec<_> = removed.clone();
            sorted.sort_by_key(|(i, _)| *i);
            for (idx, line) in &sorted {
                if *idx <= system.lines.len() {
                    system.lines.insert(*idx, line.clone());
                } else {
                    system.lines.push(line.clone());
                }
            }
            let first_idx = sorted.first().map_or(0, |(i, _)| *i);
            let first_line = sorted
                .first()
                .map_or_else(Line::default, |(_, l)| l.clone());
            EditorCommand::AddLine {
                line_index: first_idx,
                line: Box::new(first_line),
            }
        }
        EditorCommand::CommentBlocks { block_indices } => {
            for &idx in block_indices {
                if let Some(block) = system.blocks.get_mut(idx) {
                    block.commented = !block.commented;
                    if block.commented {
                        block
                            .properties
                            .insert("Commented".to_string(), "on".to_string());
                    } else {
                        block.properties.swap_remove("Commented");
                    }
                }
            }
            EditorCommand::CommentBlocks {
                block_indices: block_indices.clone(),
            }
        }
        EditorCommand::RotateBlocks {
            block_indices,
            old_positions,
        } => {
            let mut current_positions = Vec::new();
            for (i, &idx) in block_indices.iter().enumerate() {
                if let Some(block) = system.blocks.get_mut(idx) {
                    current_positions.push(
                        block
                            .position
                            .clone()
                            .unwrap_or_else(|| "[0, 0, 30, 30]".to_string()),
                    );
                    if let Some(old_pos) = old_positions.get(i) {
                        block.position = Some(old_pos.clone());
                        if let Some(v) = block.properties.get_mut("Position") {
                            *v = old_pos.clone();
                        }
                    }
                }
            }
            EditorCommand::RotateBlocks {
                block_indices: block_indices.clone(),
                old_positions: current_positions,
            }
        }
        EditorCommand::MirrorBlocks { block_indices } => {
            for &idx in block_indices {
                if let Some(block) = system.blocks.get_mut(idx) {
                    let mirrored = block.block_mirror.unwrap_or(false);
                    block.block_mirror = Some(!mirrored);
                    block.properties.insert(
                        "BlockMirror".to_string(),
                        if !mirrored { "on" } else { "off" }.to_string(),
                    );
                }
            }
            EditorCommand::MirrorBlocks {
                block_indices: block_indices.clone(),
            }
        }
        EditorCommand::RenameLine {
            line_index,
            old_name,
            new_name,
        } => {
            if let Some(line) = system.lines.get_mut(*line_index) {
                line.name.clone_from(old_name);
                if let Some(n) = old_name {
                    line.properties
                        .insert("Name".to_string(), n.clone());
                } else {
                    line.properties.swap_remove("Name");
                }
            }
            EditorCommand::RenameLine {
                line_index: *line_index,
                old_name: new_name.clone(),
                new_name: old_name.clone(),
            }
        }
        EditorCommand::CreateSubsystem {
            removed_blocks,
            removed_lines,
            subsystem_block_index,
            subsystem_block: _,
            added_lines,
        } => {
            // Undo: remove added lines, remove subsystem block, re-insert removed blocks/lines
            // Remove added lines (descending)
            let mut added_sorted: Vec<_> = added_lines.clone();
            added_sorted.sort_by_key(|(i, _)| std::cmp::Reverse(*i));
            for (idx, _) in &added_sorted {
                if *idx < system.lines.len() {
                    system.lines.remove(*idx);
                }
            }
            // Remove subsystem block
            let sub_block = if *subsystem_block_index < system.blocks.len() {
                system.blocks.remove(*subsystem_block_index)
            } else {
                return cmd.clone();
            };
            // Re-insert original blocks (ascending)
            let mut blocks_sorted: Vec<_> = removed_blocks.clone();
            blocks_sorted.sort_by_key(|(i, _)| *i);
            for (idx, block) in &blocks_sorted {
                if *idx <= system.blocks.len() {
                    system.blocks.insert(*idx, block.clone());
                } else {
                    system.blocks.push(block.clone());
                }
            }
            // Re-insert original lines (ascending)
            let mut lines_sorted: Vec<_> = removed_lines.clone();
            lines_sorted.sort_by_key(|(i, _)| *i);
            for (idx, line) in &lines_sorted {
                if *idx <= system.lines.len() {
                    system.lines.insert(*idx, line.clone());
                } else {
                    system.lines.push(line.clone());
                }
            }
            EditorCommand::CreateSubsystem {
                removed_blocks: removed_blocks.clone(),
                removed_lines: removed_lines.clone(),
                subsystem_block_index: *subsystem_block_index,
                subsystem_block: Box::new(sub_block),
                added_lines: added_lines.clone(),
            }
        }
        EditorCommand::BranchLine {
            line_index,
            branch: _,
        } => {
            if let Some(line) = system.lines.get_mut(*line_index) {
                line.branches.pop();
            }
            cmd.clone()
        }
        EditorCommand::ReassignSids { old_sids } => {
            let mut current_sids = Vec::new();
            for (idx, old_sid) in old_sids {
                if let Some(block) = system.blocks.get_mut(*idx) {
                    current_sids.push((*idx, block.sid.clone()));
                    block.sid = old_sid.clone();
                    if let Some(s) = old_sid {
                        block.properties.insert("SID".to_string(), s.clone());
                    } else {
                        block.properties.swap_remove("SID");
                    }
                }
            }
            EditorCommand::ReassignSids {
                old_sids: current_sids,
            }
        }
        EditorCommand::Batch(cmds) => {
            let mut inverses = Vec::new();
            for c in cmds.iter().rev() {
                inverses.push(apply_inverse(system, c));
            }
            inverses.reverse();
            EditorCommand::Batch(inverses)
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Helper functions
// ────────────────────────────────────────────────────────────────────────────

/// Parse a position string `"[l, t, r, b]"` into `(l, t, r, b)`.
pub fn parse_position(pos: &str) -> Option<(i32, i32, i32, i32)> {
    let inner = pos.trim().trim_start_matches('[').trim_end_matches(']');
    let nums: Vec<i32> = inner
        .split(',')
        .map(|s| s.trim())
        .filter_map(|s| s.parse().ok())
        .collect();
    if nums.len() == 4 {
        Some((nums[0], nums[1], nums[2], nums[3]))
    } else {
        None
    }
}

/// Format a position tuple as `"[l, t, r, b]"`.
pub fn format_position(l: i32, t: i32, r: i32, b: i32) -> String {
    format!("[{}, {}, {}, {}]", l, t, r, b)
}

/// Apply a delta to a block's position.
pub(crate) fn apply_position_delta(block: &mut Block, dx: i32, dy: i32) {
    if let Some(pos) = &block.position {
        if let Some((l, t, r, b)) = parse_position(pos) {
            let new_pos = format_position(l + dx, t + dy, r + dx, b + dy);
            block.position = Some(new_pos.clone());
            block.properties.insert("Position".to_string(), new_pos);
        }
    }
}

/// Create a default block from a catalog entry specification.
///
/// # Arguments
///
/// * `block_type` - The block type name (e.g., `"Gain"`)
/// * `name` - The block display name
/// * `x` - X position (left edge)
/// * `y` - Y position (top edge)
/// * `inputs` - Number of input ports
/// * `outputs` - Number of output ports
pub fn create_default_block(
    block_type: &str,
    name: &str,
    x: i32,
    y: i32,
    inputs: u32,
    outputs: u32,
) -> Block {
    let width = 30;
    let height = 30;
    let pos = format_position(x, y, x + width, y + height);
    let mut properties = IndexMap::new();
    properties.insert("Position".to_string(), pos.clone());
    properties.insert("BlockType".to_string(), block_type.to_string());

    let port_counts = if inputs > 0 || outputs > 0 {
        Some(PortCounts {
            ins: if inputs > 0 { Some(inputs) } else { None },
            outs: if outputs > 0 { Some(outputs) } else { None },
        })
    } else {
        None
    };

    let mut ports = Vec::new();
    for i in 1..=inputs {
        ports.push(Port {
            port_type: "in".to_string(),
            index: Some(i),
            properties: IndexMap::new(),
        });
    }
    for i in 1..=outputs {
        ports.push(Port {
            port_type: "out".to_string(),
            index: Some(i),
            properties: IndexMap::new(),
        });
    }

    let mut child_order = Vec::new();
    if port_counts.is_some() {
        child_order.push(BlockChildKind::PortCounts);
    }
    child_order.push(BlockChildKind::P("Position".to_string()));
    child_order.push(BlockChildKind::P("BlockType".to_string()));
    if !ports.is_empty() {
        child_order.push(BlockChildKind::PortProperties);
    }

    let is_subsystem = block_type == "SubSystem"
        || block_type == "AtomicSubSystem"
        || block_type == "EnabledSubSystem"
        || block_type == "TriggeredSubSystem"
        || block_type == "ForEachSubSystem"
        || block_type == "ForIterator"
        || block_type == "WhileIterator"
        || block_type == "MaskedSubSystem"
        || block_type == "ConfigSubSystem"
        || block_type == "VariantSubSystem";

    let subsystem = if is_subsystem {
        Some(Box::new(System {
            properties: IndexMap::new(),
            blocks: Vec::new(),
            lines: Vec::new(),
            annotations: Vec::new(),
            chart: None,
        }))
    } else {
        None
    };

    Block {
        block_type: block_type.to_string(),
        name: name.to_string(),
        sid: None,
        tag_name: "Block".to_string(),
        position: Some(pos),
        zorder: None,
        commented: false,
        name_location: NameLocation::Bottom,
        is_matlab_function: block_type == "MATLAB Function",
        value: None,
        value_kind: crate::model::ValueKind::Unknown,
        value_rows: None,
        value_cols: None,
        properties,
        ref_properties: BTreeSet::new(),
        port_counts,
        ports,
        subsystem,
        system_ref: None,
        c_function: if block_type == "CFunction" {
            Some(crate::model::CFunctionCode::default())
        } else {
            None
        },
        instance_data: None,
        link_data: None,
        mask: None,
        annotations: Vec::new(),
        background_color: None,
        show_name: None,
        font_size: None,
        font_weight: None,
        mask_display_text: None,
        current_setting: None,
        block_mirror: None,
        library_source: None,
        library_block_path: None,
        child_order,
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Public operation functions
// ────────────────────────────────────────────────────────────────────────────

/// Move a single block to a new absolute position, returning the command for undo.
///
/// # Arguments
///
/// * `system` - The system containing the block
/// * `block_index` - Index of the block in `system.blocks`
/// * `new_x` - New left-edge X coordinate
/// * `new_y` - New top-edge Y coordinate
pub fn move_block(system: &mut System, block_index: usize, new_x: i32, new_y: i32) -> EditorCommand {
    let block = &system.blocks[block_index];
    let old_position = block
        .position
        .clone()
        .unwrap_or_else(|| "[0, 0, 30, 30]".to_string());

    let (l, t, r, b) = parse_position(&old_position).unwrap_or((0, 0, 30, 30));
    let w = r - l;
    let h = b - t;
    let new_position = format_position(new_x, new_y, new_x + w, new_y + h);

    let block = &mut system.blocks[block_index];
    block.position = Some(new_position.clone());
    block
        .properties
        .insert("Position".to_string(), new_position.clone());

    EditorCommand::MoveBlock {
        block_index,
        old_position,
        new_position,
    }
}

/// Move multiple blocks by a delta offset, returning the command for undo.
///
/// Also moves the endpoints and intermediate points of all lines connected
/// to the moved blocks so that connections track correctly.
pub fn move_blocks(
    system: &mut System,
    block_indices: &[usize],
    dx: i32,
    dy: i32,
) -> EditorCommand {
    // Collect SIDs of moved blocks for line adjustment
    let moved_sids: std::collections::HashSet<String> = block_indices
        .iter()
        .filter_map(|&i| system.blocks.get(i))
        .filter_map(|b| b.sid.clone())
        .collect();

    for &idx in block_indices {
        if let Some(block) = system.blocks.get_mut(idx) {
            apply_position_delta(block, dx, dy);
        }
    }

    // Adjust line points for lines whose source or destination is one of the moved blocks
    for line in &mut system.lines {
        let src_moved = line
            .src
            .as_ref()
            .map_or(false, |ep| moved_sids.contains(&ep.sid));
        let dst_moved = line
            .dst
            .as_ref()
            .map_or(false, |ep| moved_sids.contains(&ep.sid));

        if src_moved && dst_moved {
            // Both ends moved by the same delta – shift all points
            for pt in &mut line.points {
                pt.x += dx;
                pt.y += dy;
            }
            adjust_branches_delta(&mut line.branches, dx, dy, &moved_sids, true);
        } else if src_moved && !line.points.is_empty() {
            // Only source moved – adjust first point offset
            line.points[0].x += dx;
            line.points[0].y += dy;
        } else if dst_moved && !line.points.is_empty() {
            let last = line.points.len() - 1;
            line.points[last].x += dx;
            line.points[last].y += dy;
        }
    }

    EditorCommand::MoveBlocks {
        block_indices: block_indices.to_vec(),
        dx,
        dy,
    }
}

fn adjust_branches_delta(
    branches: &mut [Branch],
    dx: i32,
    dy: i32,
    moved_sids: &std::collections::HashSet<String>,
    all_moved: bool,
) {
    for branch in branches.iter_mut() {
        let dst_moved = branch
            .dst
            .as_ref()
            .map_or(false, |ep| moved_sids.contains(&ep.sid));

        if all_moved || dst_moved {
            for pt in &mut branch.points {
                pt.x += dx;
                pt.y += dy;
            }
            adjust_branches_delta(&mut branch.branches, dx, dy, moved_sids, all_moved);
        }
    }
}

/// Add a block to the system, returning the command for undo.
///
/// The block is appended at the end of `system.blocks`.
pub fn add_block(system: &mut System, block: Block) -> EditorCommand {
    let idx = system.blocks.len();
    let cmd = EditorCommand::AddBlock {
        block_index: idx,
        block: Box::new(block.clone()),
    };
    system.blocks.push(block);
    cmd
}

/// Delete blocks at the given indices, returning the command for undo.
///
/// Indices are processed in descending order to preserve correctness.
pub fn delete_blocks(system: &mut System, indices: &[usize]) -> EditorCommand {
    let mut sorted: Vec<usize> = indices.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    sorted.reverse();

    let mut removed = Vec::new();
    for idx in sorted {
        if idx < system.blocks.len() {
            let block = system.blocks.remove(idx);
            removed.push((idx, block));
        }
    }

    EditorCommand::DeleteBlocks { removed }
}

/// Add a signal line connecting two endpoints.
///
/// # Arguments
///
/// * `system` - The system to add the line to
/// * `src_sid` - Source block SID
/// * `src_port` - Source port index (1-based)
/// * `dst_sid` - Destination block SID
/// * `dst_port` - Destination port index (1-based)
/// * `points` - Intermediate routing points (relative offsets)
pub fn add_line(
    system: &mut System,
    src_sid: &str,
    src_port: u32,
    dst_sid: &str,
    dst_port: u32,
    points: Vec<Point>,
) -> EditorCommand {
    let line = Line {
        name: None,
        zorder: None,
        src: Some(EndpointRef {
            sid: src_sid.to_string(),
            port_type: "out".to_string(),
            port_index: src_port,
        }),
        dst: Some(EndpointRef {
            sid: dst_sid.to_string(),
            port_type: "in".to_string(),
            port_index: dst_port,
        }),
        points,
        labels: None,
        branches: Vec::new(),
        properties: {
            let mut p = IndexMap::new();
            p.insert(
                "Src".to_string(),
                format!("{}#out:{}", src_sid, src_port),
            );
            p.insert(
                "Dst".to_string(),
                format!("{}#in:{}", dst_sid, dst_port),
            );
            p
        },
    };

    let idx = system.lines.len();
    let cmd = EditorCommand::AddLine {
        line_index: idx,
        line: Box::new(line.clone()),
    };
    system.lines.push(line);
    cmd
}

/// Delete lines at the given indices, returning the command for undo.
pub fn delete_lines(system: &mut System, indices: &[usize]) -> EditorCommand {
    let mut sorted: Vec<usize> = indices.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    sorted.reverse();

    let mut removed = Vec::new();
    for idx in sorted {
        if idx < system.lines.len() {
            let line = system.lines.remove(idx);
            removed.push((idx, line));
        }
    }

    EditorCommand::DeleteLines { removed }
}

/// Toggle the commented state of the given blocks.
pub fn comment_blocks(system: &mut System, indices: &[usize]) -> EditorCommand {
    for &idx in indices {
        if let Some(block) = system.blocks.get_mut(idx) {
            block.commented = !block.commented;
            if block.commented {
                block
                    .properties
                    .insert("Commented".to_string(), "on".to_string());
            } else {
                block.properties.swap_remove("Commented");
            }
        }
    }
    EditorCommand::CommentBlocks {
        block_indices: indices.to_vec(),
    }
}

/// Rotate blocks 90° clockwise by swapping width and height around the center.
pub fn rotate_blocks(system: &mut System, indices: &[usize]) -> EditorCommand {
    let mut old_positions = Vec::new();
    for &idx in indices {
        if let Some(block) = system.blocks.get_mut(idx) {
            let pos = block
                .position
                .clone()
                .unwrap_or_else(|| "[0, 0, 30, 30]".to_string());
            old_positions.push(pos.clone());

            if let Some((l, t, r, b)) = parse_position(&pos) {
                let cx = (l + r) / 2;
                let cy = (t + b) / 2;
                let w = r - l;
                let h = b - t;
                // Swap width and height around center
                let new_l = cx - h / 2;
                let new_t = cy - w / 2;
                let new_r = new_l + h;
                let new_b = new_t + w;
                let new_pos = format_position(new_l, new_t, new_r, new_b);
                block.position = Some(new_pos.clone());
                block.properties.insert("Position".to_string(), new_pos);
            }
        }
    }
    EditorCommand::RotateBlocks {
        block_indices: indices.to_vec(),
        old_positions,
    }
}

/// Toggle the mirrored state of the given blocks.
pub fn mirror_blocks(system: &mut System, indices: &[usize]) -> EditorCommand {
    for &idx in indices {
        if let Some(block) = system.blocks.get_mut(idx) {
            let mirrored = block.block_mirror.unwrap_or(false);
            block.block_mirror = Some(!mirrored);
            block.properties.insert(
                "BlockMirror".to_string(),
                if !mirrored { "on" } else { "off" }.to_string(),
            );
        }
    }
    EditorCommand::MirrorBlocks {
        block_indices: indices.to_vec(),
    }
}

/// Rename a signal line (set or clear its name label).
pub fn rename_line(
    system: &mut System,
    line_index: usize,
    new_name: Option<String>,
) -> EditorCommand {
    let old_name = system.lines[line_index].name.clone();
    system.lines[line_index].name = new_name.clone();
    if let Some(n) = &new_name {
        system.lines[line_index]
            .properties
            .insert("Name".to_string(), n.clone());
    } else {
        system.lines[line_index].properties.swap_remove("Name");
    }
    EditorCommand::RenameLine {
        line_index,
        old_name,
        new_name,
    }
}

/// Add a branch to an existing line, connecting to a new destination.
///
/// # Arguments
///
/// * `system` - The system containing the line
/// * `line_index` - Index of the line to branch from
/// * `dst_sid` - Destination block SID for the branch
/// * `dst_port` - Destination port index (1-based)
/// * `points` - Intermediate routing points for the branch
pub fn branch_line(
    system: &mut System,
    line_index: usize,
    dst_sid: &str,
    dst_port: u32,
    points: Vec<Point>,
) -> EditorCommand {
    let branch = Branch {
        name: None,
        zorder: None,
        dst: Some(EndpointRef {
            sid: dst_sid.to_string(),
            port_type: "in".to_string(),
            port_index: dst_port,
        }),
        points,
        labels: None,
        branches: Vec::new(),
        properties: {
            let mut p = IndexMap::new();
            p.insert(
                "Dst".to_string(),
                format!("{}#in:{}", dst_sid, dst_port),
            );
            p
        },
    };

    system.lines[line_index].branches.push(branch.clone());

    EditorCommand::BranchLine {
        line_index,
        branch,
    }
}

/// Create a subsystem from a set of selected blocks and their interconnecting lines.
///
/// Returns the command for undo. Blocks are moved into a new `SubSystem` block
/// at the centroid of the selected blocks. External connections are rewired
/// through Inport/Outport blocks.
pub fn create_subsystem_from_selection(
    system: &mut System,
    block_indices: &[usize],
    subsystem_name: &str,
) -> EditorCommand {
    if block_indices.is_empty() {
        return EditorCommand::Batch(Vec::new());
    }

    // Gather selected blocks' SIDs
    let selected_sids: std::collections::HashSet<String> = block_indices
        .iter()
        .filter_map(|&i| system.blocks.get(i))
        .filter_map(|b| b.sid.clone())
        .collect();

    // Compute centroid of selected blocks for subsystem placement
    let mut cx = 0i32;
    let mut cy = 0i32;
    let mut count = 0;
    for &idx in block_indices {
        if let Some(block) = system.blocks.get(idx) {
            if let Some(pos) = &block.position {
                if let Some((l, t, r, b)) = parse_position(pos) {
                    cx += (l + r) / 2;
                    cy += (t + b) / 2;
                    count += 1;
                }
            }
        }
    }
    if count > 0 {
        cx /= count;
        cy /= count;
    }

    // Classify lines: internal (both endpoints in selection), external-in, external-out
    let mut internal_line_indices = Vec::new();
    let mut _external_in = Vec::new(); // lines going INTO selected blocks from outside
    let mut _external_out = Vec::new(); // lines going OUT from selected blocks to outside

    for (i, line) in system.lines.iter().enumerate() {
        let src_in = line
            .src
            .as_ref()
            .map_or(false, |ep| selected_sids.contains(&ep.sid));
        let dst_in = line
            .dst
            .as_ref()
            .map_or(false, |ep| selected_sids.contains(&ep.sid));

        if src_in && dst_in {
            internal_line_indices.push(i);
        } else if !src_in && dst_in {
            _external_in.push(i);
        } else if src_in && !dst_in {
            _external_out.push(i);
        }
    }

    // Build the subsystem's inner content
    let mut sub_blocks: Vec<Block> = Vec::new();
    let mut sub_lines: Vec<Line> = Vec::new();

    // Add selected blocks to subsystem (with position offsets)
    for &idx in block_indices {
        let mut block = system.blocks[idx].clone();
        // Adjust position relative to centroid
        if let Some(pos) = &block.position {
            if let Some((l, t, r, b)) = parse_position(pos) {
                let new_pos =
                    format_position(l - cx + 200, t - cy + 200, r - cx + 200, b - cy + 200);
                block.position = Some(new_pos.clone());
                block.properties.insert("Position".to_string(), new_pos);
            }
        }
        sub_blocks.push(block);
    }

    // Add internal lines to subsystem
    for &idx in &internal_line_indices {
        sub_lines.push(system.lines[idx].clone());
    }

    // Create inport/outport blocks for external connections
    let mut next_inport = 1u32;
    let mut next_outport = 1u32;
    let new_external_lines: Vec<(usize, Line)> = Vec::new();

    // Handle external inputs
    for &line_idx in &_external_in {
        let _line = &system.lines[line_idx];
        // Add Inport inside subsystem wired to the destination
        let inport = create_default_block(
            "Inport",
            &format!("In{}", next_inport),
            50,
            50 + (next_inport as i32 - 1) * 60,
            0,
            1,
        );
        sub_blocks.push(inport);
        next_inport += 1;
    }

    // Handle external outputs
    for &line_idx in &_external_out {
        let _line = &system.lines[line_idx];
        let outport = create_default_block(
            "Outport",
            &format!("Out{}", next_outport),
            400,
            50 + (next_outport as i32 - 1) * 60,
            1,
            0,
        );
        sub_blocks.push(outport);
        next_outport += 1;
    }

    // Create the subsystem block
    let sub_system = System {
        properties: IndexMap::new(),
        blocks: sub_blocks,
        lines: sub_lines,
        annotations: Vec::new(),
        chart: None,
    };

    let total_inports = next_inport - 1;
    let total_outports = next_outport - 1;
    let mut subsystem_block =
        create_default_block(subsystem_name, subsystem_name, cx - 25, cy - 25, total_inports, total_outports);
    subsystem_block.subsystem = Some(Box::new(sub_system));
    subsystem_block.block_type = "SubSystem".to_string();

    // Remove internal lines (descending order)
    let mut all_removed_line_indices: Vec<usize> = internal_line_indices.clone();
    all_removed_line_indices.extend(&_external_in);
    all_removed_line_indices.extend(&_external_out);
    all_removed_line_indices.sort_unstable();
    all_removed_line_indices.dedup();
    all_removed_line_indices.reverse();

    let mut removed_lines = Vec::new();
    for idx in &all_removed_line_indices {
        if *idx < system.lines.len() {
            removed_lines.push((*idx, system.lines.remove(*idx)));
        }
    }

    // Remove selected blocks (descending order)
    let mut sorted_block_indices: Vec<usize> = block_indices.to_vec();
    sorted_block_indices.sort_unstable();
    sorted_block_indices.reverse();

    let mut removed_blocks = Vec::new();
    for idx in &sorted_block_indices {
        if *idx < system.blocks.len() {
            removed_blocks.push((*idx, system.blocks.remove(*idx)));
        }
    }

    // Insert subsystem block
    let subsystem_block_index = system.blocks.len();
    system.blocks.push(subsystem_block.clone());

    // Add new external lines
    for (idx, line) in &new_external_lines {
        if *idx <= system.lines.len() {
            system.lines.insert(*idx, line.clone());
        } else {
            system.lines.push(line.clone());
        }
    }

    EditorCommand::CreateSubsystem {
        removed_blocks,
        removed_lines,
        subsystem_block_index,
        subsystem_block: Box::new(subsystem_block),
        added_lines: new_external_lines,
    }
}

/// Assign sequential SIDs to all blocks that lack them.
///
/// Finds the maximum existing numeric SID and assigns new ones starting
/// from `max + 1`. Returns the command for undo.
pub fn assign_sids(system: &mut System) -> EditorCommand {
    let mut max_sid: u32 = 0;

    // Find maximum existing numeric SID
    for block in &system.blocks {
        if let Some(sid) = &block.sid {
            if let Ok(n) = sid.parse::<u32>() {
                max_sid = max_sid.max(n);
            }
        }
    }

    let mut old_sids = Vec::new();
    let mut next = max_sid + 1;

    for (i, block) in system.blocks.iter_mut().enumerate() {
        if block.sid.is_none() {
            old_sids.push((i, None));
            let new_sid = next.to_string();
            block.sid = Some(new_sid.clone());
            block.properties.insert("SID".to_string(), new_sid);
            next += 1;
        }
    }

    EditorCommand::ReassignSids { old_sids }
}

/// Find a snap target port near the given screen position.
///
/// Returns `(block_index, port_type, port_index, snap_position)` if a port
/// is within `snap_radius` of `pos`.
///
/// This function works in model coordinates (not screen coordinates).
pub fn find_snap_port(
    system: &System,
    pos_x: f32,
    pos_y: f32,
    snap_radius: f32,
    exclude_block_idx: Option<usize>,
) -> Option<(usize, String, u32, f32, f32)> {
    let mut best: Option<(usize, String, u32, f32, f32, f32)> = None;

    for (block_idx, block) in system.blocks.iter().enumerate() {
        if Some(block_idx) == exclude_block_idx {
            continue;
        }
        if let Some(pos) = &block.position {
            if let Some((l, t, r, b)) = parse_position(pos) {
                let rect_l = l as f32;
                let rect_t = t as f32;
                let rect_r = r as f32;
                let rect_b = b as f32;

                // Count ports
                let n_in = block
                    .port_counts
                    .as_ref()
                    .and_then(|pc| pc.ins)
                    .unwrap_or_else(|| {
                        block
                            .ports
                            .iter()
                            .filter(|p| p.port_type == "in")
                            .count() as u32
                    });
                let n_out = block
                    .port_counts
                    .as_ref()
                    .and_then(|pc| pc.outs)
                    .unwrap_or_else(|| {
                        block
                            .ports
                            .iter()
                            .filter(|p| p.port_type == "out")
                            .count() as u32
                    });

                let mirrored = block.block_mirror.unwrap_or(false);

                // Check input ports
                for i in 1..=n_in {
                    let (px, py) = port_model_pos(rect_l, rect_t, rect_r, rect_b, "in", i, n_in, mirrored);
                    let dist = ((pos_x - px).powi(2) + (pos_y - py).powi(2)).sqrt();
                    if dist < snap_radius {
                        if best.as_ref().map_or(true, |b| dist < b.5) {
                            best = Some((block_idx, "in".to_string(), i, px, py, dist));
                        }
                    }
                }

                // Check output ports
                for i in 1..=n_out {
                    let (px, py) = port_model_pos(rect_l, rect_t, rect_r, rect_b, "out", i, n_out, mirrored);
                    let dist = ((pos_x - px).powi(2) + (pos_y - py).powi(2)).sqrt();
                    if dist < snap_radius {
                        if best.as_ref().map_or(true, |b| dist < b.5) {
                            best = Some((block_idx, "out".to_string(), i, px, py, dist));
                        }
                    }
                }
            }
        }
    }

    best.map(|(idx, pt, pi, px, py, _)| (idx, pt, pi, px, py))
}

/// Compute port position in model coordinates.
fn port_model_pos(
    l: f32, t: f32, r: f32, b: f32,
    port_type: &str, port_index: u32, num_ports: u32,
    mirrored: bool,
) -> (f32, f32) {
    let n = num_ports.max(port_index);
    let total_segments = n * 2 + 1;
    let dy = (b - t) / (total_segments as f32);
    let y = t + ((2 * port_index) as f32 - 0.5) * dy;

    let is_left = match (port_type, mirrored) {
        ("in", false) | ("out", true) => true,
        _ => false,
    };

    if is_left { (l, y) } else { (r, y) }
}

/// Compute an auto-routing path between two points using orthogonal segments.
///
/// Returns a list of relative-offset points for the line's `points` field.
/// The routing avoids diagonal lines and creates clean right-angle paths.
pub fn auto_route(
    src_x: f32, src_y: f32,
    dst_x: f32, dst_y: f32,
    src_port_type: &str,
    dst_port_type: &str,
) -> Vec<Point> {
    let mut points = Vec::new();

    let going_right = src_port_type == "out" && dst_port_type == "in";

    if going_right {
        let mid_x = ((src_x + dst_x) / 2.0) as i32;
        if (src_y - dst_y).abs() < 1.0 {
            // Straight horizontal line – no intermediate points needed
        } else {
            // L-shaped or Z-shaped routing
            let halfway = mid_x - src_x as i32;
            points.push(Point { x: halfway, y: 0 });
            points.push(Point {
                x: 0,
                y: (dst_y - src_y) as i32,
            });
        }
    } else {
        // Going opposite direction or same-side connection
        let offset = 30;
        let exit_x = if src_port_type == "out" { offset } else { -offset };
        let enter_x = if dst_port_type == "in" { -offset } else { offset };

        points.push(Point { x: exit_x, y: 0 });
        let mid_y = ((src_y + dst_y) / 2.0) as i32 - src_y as i32;
        points.push(Point { x: 0, y: mid_y });
        points.push(Point {
            x: dst_x as i32 + enter_x - src_x as i32 - exit_x,
            y: 0,
        });
        points.push(Point {
            x: 0,
            y: dst_y as i32 - src_y as i32 - mid_y,
        });
        points.push(Point { x: -enter_x, y: 0 });
    }

    points
}

impl Line {
    fn default() -> Self {
        Line {
            name: None,
            zorder: None,
            src: None,
            dst: None,
            points: Vec::new(),
            labels: None,
            branches: Vec::new(),
            properties: IndexMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_system() -> System {
        let mut sys = System {
            properties: IndexMap::new(),
            blocks: Vec::new(),
            lines: Vec::new(),
            annotations: Vec::new(),
            chart: None,
        };

        // Add two blocks with SIDs
        let mut b1 = create_default_block("Gain", "Gain1", 100, 100, 1, 1);
        b1.sid = Some("1".to_string());
        b1.properties.insert("SID".to_string(), "1".to_string());

        let mut b2 = create_default_block("Sum", "Sum1", 200, 100, 2, 1);
        b2.sid = Some("2".to_string());
        b2.properties.insert("SID".to_string(), "2".to_string());

        sys.blocks.push(b1);
        sys.blocks.push(b2);

        // Add a line connecting them
        sys.lines.push(Line {
            name: Some("signal1".to_string()),
            zorder: None,
            src: Some(EndpointRef {
                sid: "1".to_string(),
                port_type: "out".to_string(),
                port_index: 1,
            }),
            dst: Some(EndpointRef {
                sid: "2".to_string(),
                port_type: "in".to_string(),
                port_index: 1,
            }),
            points: vec![Point { x: 50, y: 0 }],
            labels: None,
            branches: Vec::new(),
            properties: IndexMap::new(),
        });

        sys
    }

    #[test]
    fn test_parse_position() {
        assert_eq!(parse_position("[10, 20, 40, 50]"), Some((10, 20, 40, 50)));
        assert_eq!(parse_position("[0, 0, 30, 30]"), Some((0, 0, 30, 30)));
        assert_eq!(parse_position("invalid"), None);
    }

    #[test]
    fn test_format_position() {
        assert_eq!(format_position(10, 20, 40, 50), "[10, 20, 40, 50]");
    }

    #[test]
    fn test_move_block() {
        let mut sys = make_test_system();
        let cmd = move_block(&mut sys, 0, 150, 200);
        assert_eq!(sys.blocks[0].position.as_deref(), Some("[150, 200, 180, 230]"));

        match cmd {
            EditorCommand::MoveBlock {
                old_position,
                new_position,
                ..
            } => {
                assert_eq!(old_position, "[100, 100, 130, 130]");
                assert_eq!(new_position, "[150, 200, 180, 230]");
            }
            _ => panic!("Expected MoveBlock command"),
        }
    }

    #[test]
    fn test_move_blocks_delta() {
        let mut sys = make_test_system();
        let cmd = move_blocks(&mut sys, &[0, 1], 10, 20);
        assert_eq!(
            sys.blocks[0].position.as_deref(),
            Some("[110, 120, 140, 150]")
        );
        assert_eq!(
            sys.blocks[1].position.as_deref(),
            Some("[210, 120, 240, 150]")
        );

        match cmd {
            EditorCommand::MoveBlocks { dx, dy, .. } => {
                assert_eq!(dx, 10);
                assert_eq!(dy, 20);
            }
            _ => panic!("Expected MoveBlocks command"),
        }
    }

    #[test]
    fn test_add_and_delete_block() {
        let mut sys = make_test_system();
        let initial_count = sys.blocks.len();

        let new_block = create_default_block("Constant", "Const1", 50, 50, 0, 1);
        let _cmd = add_block(&mut sys, new_block);
        assert_eq!(sys.blocks.len(), initial_count + 1);
        assert_eq!(sys.blocks.last().unwrap().name, "Const1");

        let _cmd = delete_blocks(&mut sys, &[initial_count]);
        assert_eq!(sys.blocks.len(), initial_count);
    }

    #[test]
    fn test_add_and_delete_line() {
        let mut sys = make_test_system();
        let initial_count = sys.lines.len();

        let _cmd = add_line(&mut sys, "1", 1, "2", 2, vec![]);
        assert_eq!(sys.lines.len(), initial_count + 1);

        let _cmd = delete_lines(&mut sys, &[initial_count]);
        assert_eq!(sys.lines.len(), initial_count);
    }

    #[test]
    fn test_comment_blocks() {
        let mut sys = make_test_system();
        assert!(!sys.blocks[0].commented);

        let _cmd = comment_blocks(&mut sys, &[0]);
        assert!(sys.blocks[0].commented);
        assert_eq!(
            sys.blocks[0].properties.get("Commented"),
            Some(&"on".to_string())
        );

        // Toggle back
        let _cmd = comment_blocks(&mut sys, &[0]);
        assert!(!sys.blocks[0].commented);
        assert!(sys.blocks[0].properties.get("Commented").is_none());
    }

    #[test]
    fn test_rotate_blocks() {
        let mut sys = make_test_system();
        // Block 0 is at [100, 100, 130, 130] (30x30 square – rotation is identity)
        let _cmd = rotate_blocks(&mut sys, &[0]);
        // For a square, position should be the same
        assert_eq!(
            sys.blocks[0].position.as_deref(),
            Some("[100, 100, 130, 130]")
        );
    }

    #[test]
    fn test_mirror_blocks() {
        let mut sys = make_test_system();
        assert_eq!(sys.blocks[0].block_mirror, None);

        let _cmd = mirror_blocks(&mut sys, &[0]);
        assert_eq!(sys.blocks[0].block_mirror, Some(true));

        let _cmd = mirror_blocks(&mut sys, &[0]);
        assert_eq!(sys.blocks[0].block_mirror, Some(false));
    }

    #[test]
    fn test_rename_line() {
        let mut sys = make_test_system();
        assert_eq!(sys.lines[0].name, Some("signal1".to_string()));

        let _cmd = rename_line(&mut sys, 0, Some("new_name".to_string()));
        assert_eq!(sys.lines[0].name, Some("new_name".to_string()));

        let _cmd = rename_line(&mut sys, 0, None);
        assert_eq!(sys.lines[0].name, None);
    }

    #[test]
    fn test_branch_line() {
        let mut sys = make_test_system();
        assert!(sys.lines[0].branches.is_empty());

        let _cmd = branch_line(&mut sys, 0, "2", 2, vec![Point { x: 0, y: 30 }]);
        assert_eq!(sys.lines[0].branches.len(), 1);
        assert_eq!(
            sys.lines[0].branches[0].dst.as_ref().unwrap().port_index,
            2
        );
    }

    #[test]
    fn test_assign_sids() {
        let mut sys = make_test_system();
        // Add a block without SID
        let new_block = create_default_block("Constant", "Const1", 50, 50, 0, 1);
        sys.blocks.push(new_block);
        assert!(sys.blocks[2].sid.is_none());

        let _cmd = assign_sids(&mut sys);
        assert!(sys.blocks[2].sid.is_some());
        // Should be "3" since max existing is "2"
        assert_eq!(sys.blocks[2].sid.as_deref(), Some("3"));
    }

    #[test]
    fn test_undo_redo_move() {
        let mut sys = make_test_system();
        let mut history = EditorHistory::new(100);

        let cmd = move_block(&mut sys, 0, 300, 300);
        history.push(cmd);
        assert_eq!(
            sys.blocks[0].position.as_deref(),
            Some("[300, 300, 330, 330]")
        );

        // Undo
        assert!(history.undo(&mut sys));
        assert_eq!(
            sys.blocks[0].position.as_deref(),
            Some("[100, 100, 130, 130]")
        );

        // Redo
        assert!(history.redo(&mut sys));
        assert_eq!(
            sys.blocks[0].position.as_deref(),
            Some("[300, 300, 330, 330]")
        );
    }

    #[test]
    fn test_undo_redo_comment() {
        let mut sys = make_test_system();
        let mut history = EditorHistory::new(100);

        let cmd = comment_blocks(&mut sys, &[0]);
        history.push(cmd);
        assert!(sys.blocks[0].commented);

        history.undo(&mut sys);
        assert!(!sys.blocks[0].commented);

        history.redo(&mut sys);
        assert!(sys.blocks[0].commented);
    }

    #[test]
    fn test_create_default_block() {
        let block = create_default_block("Gain", "MyGain", 100, 200, 1, 1);
        assert_eq!(block.block_type, "Gain");
        assert_eq!(block.name, "MyGain");
        assert_eq!(block.position.as_deref(), Some("[100, 200, 130, 230]"));
        assert_eq!(block.ports.len(), 2);
        assert!(block.sid.is_none());
    }

    #[test]
    fn test_create_subsystem_from_selection() {
        let mut sys = make_test_system();
        // Add a third block
        let mut b3 = create_default_block("Scope", "Scope1", 300, 100, 1, 0);
        b3.sid = Some("3".to_string());
        sys.blocks.push(b3);

        let initial_blocks = sys.blocks.len();
        let _cmd = create_subsystem_from_selection(&mut sys, &[0, 1], "NewSubsystem");

        // Should have removed 2 blocks and added 1 subsystem
        assert_eq!(sys.blocks.len(), initial_blocks - 2 + 1);
        // The last block should be the subsystem
        let sub = sys.blocks.last().unwrap();
        assert_eq!(sub.block_type, "SubSystem");
        assert_eq!(sub.name, "NewSubsystem");
        assert!(sub.subsystem.is_some());
    }

    #[test]
    fn test_find_snap_port() {
        let sys = make_test_system();
        // Block 0 (Gain1) at [100, 100, 130, 130], has 1 in, 1 out
        // Output port should be at right edge (130, ~115)
        let result = find_snap_port(&sys, 130.0, 115.0, 10.0, None);
        assert!(result.is_some());
        let (idx, pt, pi, _px, _py) = result.unwrap();
        assert_eq!(idx, 0);
        assert_eq!(pt, "out");
        assert_eq!(pi, 1);
    }

    #[test]
    fn test_auto_route_straight() {
        let points = auto_route(130.0, 115.0, 200.0, 115.0, "out", "in");
        // Should be empty for straight horizontal (same Y)
        assert!(points.is_empty());
    }

    #[test]
    fn test_auto_route_l_shape() {
        let points = auto_route(130.0, 100.0, 200.0, 200.0, "out", "in");
        // Should have intermediate routing points
        assert!(!points.is_empty());
    }

    #[test]
    fn test_history_max_size() {
        let mut sys = make_test_system();
        let mut history = EditorHistory::new(3);

        for i in 0..5 {
            let cmd = move_block(&mut sys, 0, 100 + i * 10, 100);
            history.push(cmd);
        }

        // Should only be able to undo 3 times
        let mut undo_count = 0;
        while history.undo(&mut sys) {
            undo_count += 1;
        }
        assert_eq!(undo_count, 3);
    }

    #[test]
    fn test_port_model_pos() {
        // Block [100, 100, 130, 130], 1 input, not mirrored
        let (x, y) = port_model_pos(100.0, 100.0, 130.0, 130.0, "in", 1, 1, false);
        assert_eq!(x, 100.0); // left side
        assert!(y > 100.0 && y < 130.0); // within block height
    }
}
