//! Coordinate-transform helpers for the viewer canvas.
//!
//! `ViewTransform` encapsulates the mapping between model (world) coordinates
//! and screen (pixel) coordinates so that the transform logic is defined once
//! and can be tested independently.

use eframe::egui::{Pos2, Rect, Vec2};

#[inline]
pub fn shared_canvas_text_font_px(font_scale: f32, font_factor: f32) -> f32 {
    ((10.0 + 12.0 * font_scale.max(0.0)) * font_factor).max(1.0)
}

/// Immutable snapshot of the viewer's coordinate transform for a single frame.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
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

#[allow(dead_code)]
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
    /// Coupled to the model's measurement unit: `base_scale * zoom` is the
    /// screen-pixels-per-model-unit scale used for block geometry, so text and
    /// icons scale exactly with the on-screen block size.
    #[inline]
    pub fn font_scale(&self) -> f32 {
        (self.base_scale * self.zoom / 2.0).max(0.01)
    }

    /// Compute the new zoom and pan values when zooming at `cursor` by `factor`.
    pub fn zoom_at(&self, cursor: Pos2, factor: f32) -> (f32, Vec2) {
        let old_zoom = self.zoom;
        let new_zoom = (old_zoom * factor).clamp(0.2, 30.0);
        let s_old = self.base_scale * old_zoom;
        let s_new = self.base_scale * new_zoom;
        let origin = Pos2::new(
            self.avail.left() + self.margin,
            self.avail.top() + self.margin,
        );
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
        ViewerDragState::Blocks {
            current_dx,
            current_dy,
        } => {
            if block_sid.is_some_and(|sid| selected_sids.contains(sid)) {
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
    l: f32,
    t: f32,
    r: f32,
    b: f32,
    handle: u8,
    dx: f32,
    dy: f32,
) -> (i32, i32, i32, i32) {
    let min_size = 10.0;
    let (mut nl, mut nt, mut nr, mut nb) = (l, t, r, b);

    match handle {
        0 => {
            nl = l + dx;
            nt = t + dy;
        }
        1 => {
            nt = t + dy;
        }
        2 => {
            nr = r + dx;
            nt = t + dy;
        }
        3 => {
            nr = r + dx;
        }
        4 => {
            nr = r + dx;
            nb = b + dy;
        }
        5 => {
            nb = b + dy;
        }
        6 => {
            nl = l + dx;
            nb = b + dy;
        }
        7 => {
            nl = l + dx;
        }
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
        if matches!(handle, 0..=2) {
            nt = nb - min_size;
        } else {
            nb = nt + min_size;
        }
    }

    (
        nl.round() as i32,
        nt.round() as i32,
        nr.round() as i32,
        nb.round() as i32,
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
