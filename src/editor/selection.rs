//! Selection management for the editor.
//!
//! Provides rectangle-based selection of blocks and signal lines, maintaining
//! the current selection state and providing query methods.
//!
//! # Usage
//!
//! ```rust,ignore
//! use rustylink::editor::selection::{EditorSelection, SelectionRect};
//!
//! let mut sel = EditorSelection::new();
//! sel.start_rect(10.0, 20.0);
//! sel.update_rect(100.0, 120.0);
//! sel.finish_rect(&system, zoom, pan);
//! assert!(!sel.selected_blocks.is_empty());
//! ```

#![cfg(feature = "egui")]

use crate::model::System;

/// A rectangle used for drag-selection in screen coordinates.
#[derive(Debug, Clone, Copy)]
pub struct SelectionRect {
    /// Starting point X (screen coordinates).
    pub start_x: f32,
    /// Starting point Y (screen coordinates).
    pub start_y: f32,
    /// Current end point X (screen coordinates).
    pub end_x: f32,
    /// Current end point Y (screen coordinates).
    pub end_y: f32,
}

impl SelectionRect {
    /// Create a new selection rectangle starting at the given point.
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            start_x: x,
            start_y: y,
            end_x: x,
            end_y: y,
        }
    }

    /// Update the end point of the selection rectangle.
    pub fn update(&mut self, x: f32, y: f32) {
        self.end_x = x;
        self.end_y = y;
    }

    /// Get the normalized (min-max) rectangle bounds.
    pub fn normalized(&self) -> (f32, f32, f32, f32) {
        let min_x = self.start_x.min(self.end_x);
        let min_y = self.start_y.min(self.end_y);
        let max_x = self.start_x.max(self.end_x);
        let max_y = self.start_y.max(self.end_y);
        (min_x, min_y, max_x, max_y)
    }

    /// Check if a point is inside the selection rectangle.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        let (min_x, min_y, max_x, max_y) = self.normalized();
        x >= min_x && x <= max_x && y >= min_y && y <= max_y
    }

    /// Check if a rectangle (l, t, r, b) overlaps with this selection rectangle.
    pub fn overlaps_rect(&self, l: f32, t: f32, r: f32, b: f32) -> bool {
        let (min_x, min_y, max_x, max_y) = self.normalized();
        l < max_x && r > min_x && t < max_y && b > min_y
    }

    /// Width of the selection rectangle (absolute).
    pub fn width(&self) -> f32 {
        (self.end_x - self.start_x).abs()
    }

    /// Height of the selection rectangle (absolute).
    pub fn height(&self) -> f32 {
        (self.end_y - self.start_y).abs()
    }
}

/// Tracks the current selection state in the editor.
///
/// Manages both block and line selections, supporting rectangle-drag selection,
/// individual toggle selection (Ctrl+click), and clearing.
#[derive(Debug, Clone, Default)]
pub struct EditorSelection {
    /// Indices of selected blocks in `system.blocks`.
    pub selected_blocks: Vec<usize>,
    /// Indices of selected lines in `system.lines`.
    pub selected_lines: Vec<usize>,
    /// Active drag-selection rectangle (screen coordinates), if any.
    pub selection_rect: Option<SelectionRect>,
}

impl EditorSelection {
    /// Create an empty selection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all selections.
    pub fn clear(&mut self) {
        self.selected_blocks.clear();
        self.selected_lines.clear();
        self.selection_rect = None;
    }

    /// Returns true if nothing is selected.
    pub fn is_empty(&self) -> bool {
        self.selected_blocks.is_empty() && self.selected_lines.is_empty()
    }

    /// Check if a specific block index is selected.
    pub fn is_block_selected(&self, index: usize) -> bool {
        self.selected_blocks.contains(&index)
    }

    /// Check if a specific line index is selected.
    pub fn is_line_selected(&self, index: usize) -> bool {
        self.selected_lines.contains(&index)
    }

    /// Toggle selection of a block (add if not selected, remove if selected).
    pub fn toggle_block(&mut self, index: usize) {
        if let Some(pos) = self.selected_blocks.iter().position(|&i| i == index) {
            self.selected_blocks.remove(pos);
        } else {
            self.selected_blocks.push(index);
        }
    }

    /// Toggle selection of a line (add if not selected, remove if selected).
    pub fn toggle_line(&mut self, index: usize) {
        if let Some(pos) = self.selected_lines.iter().position(|&i| i == index) {
            self.selected_lines.remove(pos);
        } else {
            self.selected_lines.push(index);
        }
    }

    /// Select a single block, clearing any previous selection.
    pub fn select_block(&mut self, index: usize) {
        self.selected_blocks.clear();
        self.selected_lines.clear();
        self.selected_blocks.push(index);
    }

    /// Select a single line, clearing any previous selection.
    pub fn select_line(&mut self, index: usize) {
        self.selected_blocks.clear();
        self.selected_lines.clear();
        self.selected_lines.push(index);
    }

    /// Start a rectangle selection at the given screen coordinates.
    pub fn start_rect(&mut self, x: f32, y: f32) {
        self.selection_rect = Some(SelectionRect::new(x, y));
    }

    /// Update the current rectangle selection end point.
    pub fn update_rect(&mut self, x: f32, y: f32) {
        if let Some(rect) = &mut self.selection_rect {
            rect.update(x, y);
        }
    }

    /// Finish the rectangle selection, computing which blocks and lines
    /// fall within the rectangle.
    ///
    /// The rectangle is in screen coordinates. We convert block positions
    /// to screen coordinates using the provided zoom and pan.
    pub fn finish_rect(
        &mut self,
        system: &System,
        zoom: f32,
        pan_x: f32,
        pan_y: f32,
        canvas_offset_x: f32,
        canvas_offset_y: f32,
    ) {
        if let Some(rect) = self.selection_rect.take() {
            // Only process if the rectangle has meaningful size (> 3px drag)
            if rect.width() < 3.0 && rect.height() < 3.0 {
                return;
            }

            self.selected_blocks.clear();
            self.selected_lines.clear();

            // Check blocks
            for (i, block) in system.blocks.iter().enumerate() {
                if let Some(pos) = &block.position {
                    if let Some((l, t, r, b)) = super::operations::parse_position(pos) {
                        // Convert block position to screen coordinates
                        let sl = l as f32 * zoom + pan_x + canvas_offset_x;
                        let st = t as f32 * zoom + pan_y + canvas_offset_y;
                        let sr = r as f32 * zoom + pan_x + canvas_offset_x;
                        let sb = b as f32 * zoom + pan_y + canvas_offset_y;

                        if rect.overlaps_rect(sl, st, sr, sb) {
                            self.selected_blocks.push(i);
                        }
                    }
                }
            }

            // Check lines (by checking if any segment point falls within the rect)
            for (i, line) in system.lines.iter().enumerate() {
                if line_intersects_rect(line, &rect, zoom, pan_x, pan_y, canvas_offset_x, canvas_offset_y) {
                    self.selected_lines.push(i);
                }
            }
        }
    }

    /// Get the number of selected items (blocks + lines).
    pub fn count(&self) -> usize {
        self.selected_blocks.len() + self.selected_lines.len()
    }
}

/// Check if any part of a line passes through the selection rectangle.
fn line_intersects_rect(
    line: &crate::model::Line,
    rect: &SelectionRect,
    zoom: f32,
    pan_x: f32,
    pan_y: f32,
    canvas_offset_x: f32,
    canvas_offset_y: f32,
) -> bool {
    // Build absolute points from relative offsets
    if line.points.is_empty() {
        return false;
    }

    let mut abs_points = Vec::new();
    let mut cur_x = 0.0f32;
    let mut cur_y = 0.0f32;

    for pt in &line.points {
        cur_x += pt.x as f32;
        cur_y += pt.y as f32;
        // Convert to screen coordinates
        let sx = cur_x * zoom + pan_x + canvas_offset_x;
        let sy = cur_y * zoom + pan_y + canvas_offset_y;
        abs_points.push((sx, sy));
    }

    // Check if any point is inside the rect
    for &(px, py) in &abs_points {
        if rect.contains(px, py) {
            return true;
        }
    }

    // Check if any segment between consecutive points intersects rect
    for window in abs_points.windows(2) {
        if let [(x1, y1), (x2, y2)] = window {
            if segment_intersects_rect(*x1, *y1, *x2, *y2, rect) {
                return true;
            }
        }
    }

    false
}

/// Check if a line segment from (x1,y1) to (x2,y2) intersects the rect.
fn segment_intersects_rect(
    x1: f32, y1: f32, x2: f32, y2: f32,
    rect: &SelectionRect,
) -> bool {
    let (min_x, min_y, max_x, max_y) = rect.normalized();

    // Quick bounds check
    let seg_min_x = x1.min(x2);
    let seg_max_x = x1.max(x2);
    let seg_min_y = y1.min(y2);
    let seg_max_y = y1.max(y2);

    if seg_max_x < min_x || seg_min_x > max_x || seg_max_y < min_y || seg_min_y > max_y {
        return false;
    }

    // For orthogonal segments (which Simulink typically uses), this simple
    // bounding box check is sufficient
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EndpointRef, Line, Point, System};
    use crate::editor::operations::create_default_block;
    use indexmap::IndexMap;

    fn make_test_system() -> System {
        let mut sys = System {
            properties: IndexMap::new(),
            blocks: Vec::new(),
            lines: Vec::new(),
            annotations: Vec::new(),
            chart: None,
        };

        let mut b1 = create_default_block("Gain", "Gain1", 100, 100, 1, 1);
        b1.sid = Some("1".to_string());

        let mut b2 = create_default_block("Sum", "Sum1", 200, 100, 2, 1);
        b2.sid = Some("2".to_string());

        let mut b3 = create_default_block("Scope", "Scope1", 300, 200, 1, 0);
        b3.sid = Some("3".to_string());

        sys.blocks.push(b1);
        sys.blocks.push(b2);
        sys.blocks.push(b3);

        sys.lines.push(Line {
            name: None,
            zorder: None,
            src: Some(EndpointRef { sid: "1".to_string(), port_type: "out".to_string(), port_index: 1 }),
            dst: Some(EndpointRef { sid: "2".to_string(), port_type: "in".to_string(), port_index: 1 }),
            points: vec![Point { x: 130, y: 115 }, Point { x: 200, y: 115 }],
            labels: None,
            branches: Vec::new(),
            properties: IndexMap::new(),
        });

        sys
    }

    #[test]
    fn test_selection_rect_normalized() {
        let rect = SelectionRect { start_x: 100.0, start_y: 200.0, end_x: 50.0, end_y: 150.0 };
        let (min_x, min_y, max_x, max_y) = rect.normalized();
        assert_eq!(min_x, 50.0);
        assert_eq!(min_y, 150.0);
        assert_eq!(max_x, 100.0);
        assert_eq!(max_y, 200.0);
    }

    #[test]
    fn test_selection_rect_contains() {
        let rect = SelectionRect { start_x: 10.0, start_y: 10.0, end_x: 100.0, end_y: 100.0 };
        assert!(rect.contains(50.0, 50.0));
        assert!(!rect.contains(150.0, 50.0));
    }

    #[test]
    fn test_selection_rect_overlaps() {
        let rect = SelectionRect { start_x: 10.0, start_y: 10.0, end_x: 100.0, end_y: 100.0 };
        assert!(rect.overlaps_rect(50.0, 50.0, 150.0, 150.0));
        assert!(!rect.overlaps_rect(200.0, 200.0, 300.0, 300.0));
    }

    #[test]
    fn test_selection_new_is_empty() {
        let sel = EditorSelection::new();
        assert!(sel.is_empty());
        assert_eq!(sel.count(), 0);
    }

    #[test]
    fn test_selection_toggle_block() {
        let mut sel = EditorSelection::new();
        sel.toggle_block(0);
        assert!(sel.is_block_selected(0));
        assert_eq!(sel.count(), 1);

        sel.toggle_block(0);
        assert!(!sel.is_block_selected(0));
        assert!(sel.is_empty());
    }

    #[test]
    fn test_selection_toggle_line() {
        let mut sel = EditorSelection::new();
        sel.toggle_line(0);
        assert!(sel.is_line_selected(0));

        sel.toggle_line(0);
        assert!(!sel.is_line_selected(0));
    }

    #[test]
    fn test_selection_select_block() {
        let mut sel = EditorSelection::new();
        sel.toggle_block(0);
        sel.toggle_block(1);
        assert_eq!(sel.count(), 2);

        sel.select_block(2);
        assert_eq!(sel.count(), 1);
        assert!(sel.is_block_selected(2));
        assert!(!sel.is_block_selected(0));
    }

    #[test]
    fn test_selection_clear() {
        let mut sel = EditorSelection::new();
        sel.toggle_block(0);
        sel.toggle_line(1);
        sel.clear();
        assert!(sel.is_empty());
    }

    #[test]
    fn test_selection_rect_finish_selects_blocks() {
        let sys = make_test_system();
        let mut sel = EditorSelection::new();

        // Create a selection rect that encompasses blocks at [100,100] and [200,100]
        // With zoom=1.0 and pan=(0,0)  
        sel.start_rect(90.0, 90.0);
        sel.update_rect(240.0, 140.0);
        sel.finish_rect(&sys, 1.0, 0.0, 0.0, 0.0, 0.0);

        assert!(sel.is_block_selected(0), "Block 0 should be selected");
        assert!(sel.is_block_selected(1), "Block 1 should be selected");
        assert!(!sel.is_block_selected(2), "Block 2 should not be selected (at 300,200)");
    }

    #[test]
    fn test_selection_rect_too_small_ignored() {
        let sys = make_test_system();
        let mut sel = EditorSelection::new();

        sel.start_rect(100.0, 100.0);
        sel.update_rect(101.0, 101.0);  // Less than 3px
        sel.finish_rect(&sys, 1.0, 0.0, 0.0, 0.0, 0.0);

        assert!(sel.is_empty(), "Tiny rect should not select anything");
    }

    #[test]
    fn test_selection_width_height() {
        let rect = SelectionRect { start_x: 10.0, start_y: 20.0, end_x: 110.0, end_y: 80.0 };
        assert_eq!(rect.width(), 100.0);
        assert_eq!(rect.height(), 60.0);
    }
}
