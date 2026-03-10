//! Coordinate-transform helpers for the viewer canvas.
//!
//! `ViewTransform` encapsulates the mapping between model (world) coordinates
//! and screen (pixel) coordinates so that the transform logic is defined once
//! and can be tested independently.

use eframe::egui::{Pos2, Rect, Vec2};

/// Immutable snapshot of the viewer's coordinate transform for a single frame.
#[derive(Clone, Copy, Debug)]
pub struct ViewTransform {
    /// Bounding-box in model space that is being fitted into the viewport.
    pub bb: Rect,
    /// Available screen-space rectangle (the egui central panel area).
    pub avail: Rect,
    /// Margin in screen pixels between `avail` edges and the fitted content.
    pub margin: f32,
    /// Scale factor that fits `bb` into `avail` at zoom = 1.
    pub base_scale: f32,
    /// User-controlled zoom factor (1.0 = fit).
    pub zoom: f32,
    /// User-controlled pan offset in screen pixels.
    pub pan: Vec2,
}

impl ViewTransform {
    /// Compute a new `ViewTransform` from the given content bounds and viewport.
    pub fn new(bb: Rect, avail: Rect, margin: f32, zoom: f32, pan: Vec2) -> Self {
        let width = bb.width().max(1.0);
        let height = bb.height().max(1.0);
        let avail_size = avail.size();
        let sx = (avail_size.x - 2.0 * margin) / width;
        let sy = (avail_size.y - 2.0 * margin) / height;
        let base_scale = sx.min(sy).max(0.1);
        Self {
            bb,
            avail,
            margin,
            base_scale,
            zoom,
            pan,
        }
    }

    /// Combined scale factor (base * zoom).
    #[inline]
    pub fn scale(&self) -> f32 {
        self.base_scale * self.zoom
    }

    /// Convert a model-space position to a screen-space position.
    #[inline]
    pub fn to_screen(&self, p: Pos2) -> Pos2 {
        let s = self.scale();
        let x = (p.x - self.bb.left()) * s + self.avail.left() + self.margin + self.pan.x;
        let y = (p.y - self.bb.top()) * s + self.avail.top() + self.margin + self.pan.y;
        Pos2::new(x, y)
    }

    /// Convert a screen-space position back to model-space.
    #[inline]
    pub fn from_screen(&self, p: Pos2) -> Pos2 {
        let s = self.scale();
        let x = (p.x - self.avail.left() - self.margin - self.pan.x) / s + self.bb.left();
        let y = (p.y - self.avail.top() - self.margin - self.pan.y) / s + self.bb.top();
        Pos2::new(x, y)
    }

    /// Font scaling factor for in-canvas text.
    ///
    /// The baseline is 400% zoom → scale = zoom / 4.0.  The user requested
    /// double font size, so we use zoom / 2.0 instead.
    #[inline]
    pub fn font_scale(&self) -> f32 {
        (self.zoom / 2.0).max(0.01)
    }

    /// Compute the new zoom and pan values when zooming at `cursor` by `factor`.
    pub fn zoom_at(&self, cursor: Pos2, factor: f32) -> (f32, Vec2) {
        let old_zoom = self.zoom;
        let new_zoom = (old_zoom * factor).clamp(0.2, 10.0);
        let s_old = self.base_scale * old_zoom;
        let s_new = self.base_scale * new_zoom;
        let origin = Pos2::new(self.avail.left() + self.margin, self.avail.top() + self.margin);
        let world_x = (cursor.x - origin.x - self.pan.x) / s_old + self.bb.left();
        let world_y = (cursor.y - origin.y - self.pan.y) / s_old + self.bb.top();
        let new_pan_x = cursor.x - ((world_x - self.bb.left()) * s_new + origin.x);
        let new_pan_y = cursor.y - ((world_y - self.bb.top()) * s_new + origin.y);
        (new_zoom, Vec2::new(new_pan_x, new_pan_y))
    }
}

/// Compute a `preview_block_rect` during drag — offsets the block's model
/// rect by the current drag delta if the block is selected.
pub fn preview_block_rect(
    drag_state: &super::super::state::ViewerDragState,
    selected_sids: &std::collections::BTreeSet<String>,
    block_sid: Option<&str>,
    rect: Rect,
) -> Rect {
    use super::super::state::ViewerDragState;
    match drag_state {
        ViewerDragState::Blocks { current_dx, current_dy } => {
            if block_sid.map_or(false, |sid| selected_sids.contains(sid)) {
                rect.translate(Vec2::new(*current_dx as f32, *current_dy as f32))
            } else {
                rect
            }
        }
        ViewerDragState::Resize {
            sid,
            handle,
            original_l,
            original_t,
            original_r,
            original_b,
            current_dx,
            current_dy,
        } => {
            if block_sid == Some(sid.as_str()) {
                let (nl, nt, nr, nb) = compute_resized_rect(
                    *original_l as f32,
                    *original_t as f32,
                    *original_r as f32,
                    *original_b as f32,
                    *handle,
                    *current_dx as f32,
                    *current_dy as f32,
                );
                Rect::from_min_max(
                    Pos2::new(nl as f32, nt as f32),
                    Pos2::new(nr as f32, nb as f32),
                )
            } else {
                rect
            }
        }
        _ => rect,
    }
}

/// Positions of the 8 resize handles around a screen-space rect.
///
/// Returns `[(position, handle_id); 8]` where handle_id encodes the corner/edge.
pub fn resize_handle_positions(r: &Rect) -> [(Pos2, u8); 8] {
    let cx = r.center().x;
    let cy = r.center().y;
    [
        (r.left_top(), 0),
        (Pos2::new(cx, r.top()), 1),
        (r.right_top(), 2),
        (Pos2::new(r.right(), cy), 3),
        (r.right_bottom(), 4),
        (Pos2::new(cx, r.bottom()), 5),
        (r.left_bottom(), 6),
        (Pos2::new(r.left(), cy), 7),
    ]
}

/// Compute the new (l, t, r, b) after dragging a resize handle by `(dx, dy)`.
///
/// Enforces a minimum size of 10 model-units.
pub fn compute_resized_rect(
    l: f32, t: f32, r: f32, b: f32,
    handle: u8,
    dx: f32, dy: f32,
) -> (i32, i32, i32, i32) {
    let min_size = 10.0;
    let (mut nl, mut nt, mut nr, mut nb) = (l, t, r, b);

    match handle {
        0 => { nl = l + dx; nt = t + dy; }
        1 => { nt = t + dy; }
        2 => { nr = r + dx; nt = t + dy; }
        3 => { nr = r + dx; }
        4 => { nr = r + dx; nb = b + dy; }
        5 => { nb = b + dy; }
        6 => { nl = l + dx; nb = b + dy; }
        7 => { nl = l + dx; }
        _ => {}
    }

    if nr - nl < min_size {
        if matches!(handle, 0 | 6 | 7) {
            nl = nr - min_size;
        } else {
            nr = nl + min_size;
        }
    }
    if nb - nt < min_size {
        if matches!(handle, 0 | 1 | 2) {
            nt = nb - min_size;
        } else {
            nb = nt + min_size;
        }
    }

    (nl.round() as i32, nt.round() as i32, nr.round() as i32, nb.round() as i32)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_transform() -> ViewTransform {
        let bb = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 100.0));
        let avail = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(500.0, 500.0));
        ViewTransform::new(bb, avail, 20.0, 1.0, Vec2::ZERO)
    }

    #[test]
    fn to_screen_from_screen_roundtrip() {
        let vt = make_transform();
        let model_pt = Pos2::new(50.0, 25.0);
        let screen_pt = vt.to_screen(model_pt);
        let back = vt.from_screen(screen_pt);
        assert!((back.x - model_pt.x).abs() < 0.01, "x: {} vs {}", back.x, model_pt.x);
        assert!((back.y - model_pt.y).abs() < 0.01, "y: {} vs {}", back.y, model_pt.y);
    }

    #[test]
    fn origin_maps_to_margin() {
        let vt = make_transform();
        let screen = vt.to_screen(Pos2::new(0.0, 0.0));
        assert!((screen.x - 20.0).abs() < 0.01);
        assert!((screen.y - 20.0).abs() < 0.01);
    }

    #[test]
    fn zoom_at_preserves_cursor_position() {
        let vt = make_transform();
        let cursor = Pos2::new(250.0, 250.0);
        let world_before = vt.from_screen(cursor);
        let (new_zoom, new_pan) = vt.zoom_at(cursor, 1.5);
        let vt2 = ViewTransform { zoom: new_zoom, pan: new_pan, ..vt };
        let world_after = vt2.from_screen(cursor);
        assert!((world_before.x - world_after.x).abs() < 0.5);
        assert!((world_before.y - world_after.y).abs() < 0.5);
    }

    #[test]
    fn font_scale_positive_at_min_zoom() {
        let vt = ViewTransform::new(
            Rect::from_min_max(Pos2::ZERO, Pos2::new(100.0, 100.0)),
            Rect::from_min_max(Pos2::ZERO, Pos2::new(500.0, 500.0)),
            20.0,
            0.2,
            Vec2::ZERO,
        );
        assert!(vt.font_scale() > 0.0);
    }

    #[test]
    fn preview_block_rect_no_drag() {
        use super::super::super::state::ViewerDragState;
        let r = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(50.0, 30.0));
        let sids = std::collections::BTreeSet::new();
        let result = preview_block_rect(&ViewerDragState::None, &sids, Some("1"), r);
        assert_eq!(result, r);
    }

    #[test]
    fn preview_block_rect_blocks_drag() {
        use super::super::super::state::ViewerDragState;
        let r = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(50.0, 30.0));
        let mut sids = std::collections::BTreeSet::new();
        sids.insert("1".to_string());
        let state = ViewerDragState::Blocks { current_dx: 10, current_dy: -5 };
        let result = preview_block_rect(&state, &sids, Some("1"), r);
        assert!((result.left() - 10.0).abs() < 0.01);
        assert!((result.top() - (-5.0)).abs() < 0.01);
    }

    #[test]
    fn preview_block_rect_unselected_not_moved() {
        use super::super::super::state::ViewerDragState;
        let r = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(50.0, 30.0));
        let mut sids = std::collections::BTreeSet::new();
        sids.insert("1".to_string());
        let state = ViewerDragState::Blocks { current_dx: 10, current_dy: -5 };
        let result = preview_block_rect(&state, &sids, Some("2"), r);
        assert_eq!(result, r);
    }

    #[test]
    fn resize_handle_positions_count() {
        let r = Rect::from_min_max(Pos2::new(10.0, 20.0), Pos2::new(110.0, 120.0));
        let handles = resize_handle_positions(&r);
        assert_eq!(handles.len(), 8);
        // Handle 0 is top-left
        assert_eq!(handles[0].0, r.left_top());
        assert_eq!(handles[0].1, 0);
        // Handle 4 is bottom-right
        assert_eq!(handles[4].0, r.right_bottom());
        assert_eq!(handles[4].1, 4);
    }

    #[test]
    fn compute_resized_rect_right_edge() {
        let (nl, nt, nr, nb) = compute_resized_rect(
            0.0, 0.0, 100.0, 100.0,
            3, // right edge
            20.0, 0.0,
        );
        assert_eq!(nl, 0);
        assert_eq!(nt, 0);
        assert_eq!(nr, 120);
        assert_eq!(nb, 100);
    }

    #[test]
    fn compute_resized_rect_min_size_enforced() {
        let (nl, _nt, nr, _nb) = compute_resized_rect(
            0.0, 0.0, 20.0, 20.0,
            3, // right edge
            -15.0, 0.0, // shrinking right to 5, below min_size 10
        );
        assert!(nr - nl >= 10);
    }

    #[test]
    fn compute_resized_rect_all_handles() {
        // Each handle should only move the expected edges.
        let base = (0.0, 0.0, 100.0, 100.0);
        let dx = 5.0;
        let dy = 7.0;

        // Handle 0: top-left
        let (nl, nt, nr, nb) = compute_resized_rect(base.0, base.1, base.2, base.3, 0, dx, dy);
        assert_eq!(nl, 5); assert_eq!(nt, 7); assert_eq!(nr, 100); assert_eq!(nb, 100);

        // Handle 1: top-center
        let (nl, nt, nr, nb) = compute_resized_rect(base.0, base.1, base.2, base.3, 1, dx, dy);
        assert_eq!(nl, 0); assert_eq!(nt, 7); assert_eq!(nr, 100); assert_eq!(nb, 100);

        // Handle 2: top-right
        let (nl, nt, nr, nb) = compute_resized_rect(base.0, base.1, base.2, base.3, 2, dx, dy);
        assert_eq!(nl, 0); assert_eq!(nt, 7); assert_eq!(nr, 105); assert_eq!(nb, 100);

        // Handle 3: right-center
        let (nl, nt, nr, nb) = compute_resized_rect(base.0, base.1, base.2, base.3, 3, dx, dy);
        assert_eq!(nl, 0); assert_eq!(nt, 0); assert_eq!(nr, 105); assert_eq!(nb, 100);

        // Handle 4: bottom-right
        let (nl, nt, nr, nb) = compute_resized_rect(base.0, base.1, base.2, base.3, 4, dx, dy);
        assert_eq!(nl, 0); assert_eq!(nt, 0); assert_eq!(nr, 105); assert_eq!(nb, 107);

        // Handle 5: bottom-center
        let (nl, nt, nr, nb) = compute_resized_rect(base.0, base.1, base.2, base.3, 5, dx, dy);
        assert_eq!(nl, 0); assert_eq!(nt, 0); assert_eq!(nr, 100); assert_eq!(nb, 107);

        // Handle 6: bottom-left
        let (nl, nt, nr, nb) = compute_resized_rect(base.0, base.1, base.2, base.3, 6, dx, dy);
        assert_eq!(nl, 5); assert_eq!(nt, 0); assert_eq!(nr, 100); assert_eq!(nb, 107);

        // Handle 7: left-center
        let (nl, nt, nr, nb) = compute_resized_rect(base.0, base.1, base.2, base.3, 7, dx, dy);
        assert_eq!(nl, 5); assert_eq!(nt, 0); assert_eq!(nr, 100); assert_eq!(nb, 100);
    }

    #[test]
    fn preview_block_rect_resize_drag() {
        use super::super::super::state::ViewerDragState;
        let r = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(50.0, 30.0));
        let sids = std::collections::BTreeSet::new();
        let state = ViewerDragState::Resize {
            sid: "1".to_string(),
            handle: 4, // bottom-right
            original_l: 0,
            original_t: 0,
            original_r: 50,
            original_b: 30,
            current_dx: 10,
            current_dy: 5,
        };
        let result = preview_block_rect(&state, &sids, Some("1"), r);
        // Bottom-right moved by (10, 5)
        assert!((result.right() - 60.0).abs() < 0.01);
        assert!((result.bottom() - 35.0).abs() < 0.01);
    }

    #[test]
    fn compute_resized_rect_min_height_enforced() {
        let (_nl, nt, _nr, nb) = compute_resized_rect(
            0.0, 0.0, 50.0, 20.0,
            5, // bottom edge
            0.0, -15.0, // shrinking height to 5, below min_size 10
        );
        assert!(nb - nt >= 10);
    }
}
