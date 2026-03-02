//! Custom egui paint renderers for Simulink Dashboard / UI blocks.
//!
//! Each function draws a Simulink-like widget representation inside the
//! block's screen rectangle, mimicking the look of the Simulink Dashboard
//! library. These renderers are registered in the interior renderer registry
//! so the egui viewer draws proper widget visuals instead of a "?" fallback.

#![cfg(feature = "egui")]

use crate::model::Block;
use eframe::egui::{self, Align2, Color32, Pos2, Rect, Stroke, Vec2};
use std::f32::consts::PI;

// ─── Helpers ────────────────────────────────────────────────────────────

/// Read a block property as a string, falling back to a default.
fn prop<'a>(block: &'a Block, name: &str, default: &'a str) -> &'a str {
    block
        .properties
        .get(name)
        .map(|s| s.as_str())
        .unwrap_or(default)
}

/// Standard widget colours matching Simulink's Dashboard palette.
#[allow(dead_code)]
const BG_LIGHT: Color32 = Color32::from_rgb(245, 245, 245);
const BG_FIELD: Color32 = Color32::from_rgb(255, 255, 255);
const BORDER: Color32 = Color32::from_rgb(180, 180, 180);
const TEXT_DARK: Color32 = Color32::from_rgb(40, 40, 40);
const ACCENT: Color32 = Color32::from_rgb(60, 120, 215);
const ACCENT_DARK: Color32 = Color32::from_rgb(40, 80, 180);
const NEEDLE_RED: Color32 = Color32::from_rgb(200, 40, 40);
const LAMP_ON: Color32 = Color32::from_rgb(60, 200, 60);
const SCOPE_BG: Color32 = Color32::from_rgb(250, 250, 250);
const SCOPE_GRID: Color32 = Color32::from_rgb(220, 220, 220);
const SCOPE_LINE: Color32 = Color32::from_rgb(30, 100, 200);

/// Paint a thin rounded-rect border (the "widget frame").
fn widget_frame(painter: &egui::Painter, rect: Rect, rounding: f32) {
    painter.rect_stroke(rect, rounding, Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);
}

/// A small helper: clamp-shrink the rect and compute a font size that fits.
fn inner_rect(rect: &Rect, frac: f32) -> Rect {
    let inset_x = rect.width() * (1.0 - frac) * 0.5;
    let inset_y = rect.height() * (1.0 - frac) * 0.5;
    Rect::from_min_max(
        Pos2::new(rect.left() + inset_x, rect.top() + inset_y),
        Pos2::new(rect.right() - inset_x, rect.bottom() - inset_y),
    )
}

fn font_for_rect(rect: &Rect, scale: f32) -> f32 {
    (rect.height() * 0.25 * scale).clamp(6.0, 24.0)
}

// ─── PushButton ─────────────────────────────────────────────────────────

/// Draws a push button like Simulink's Dashboard PushButton.
pub fn render_push_button(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
) {
    let inner = inner_rect(rect, 0.85);
    let label = prop(block, "ButtonText", &block.name);
    // Button body
    painter.rect_filled(inner, 4.0, Color32::from_rgb(230, 230, 235));
    widget_frame(painter, inner, 4.0);
    // Label centered
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.45);
    painter.text(
        inner.center(),
        Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(fsz),
        TEXT_DARK,
    );
}

// ─── SliderSwitch ───────────────────────────────────────────────────────

/// Draws a vertical slider switch with Off/On labels.
pub fn render_slider_switch(
    painter: &egui::Painter,
    _block: &Block,
    rect: &Rect,
    font_scale: f32,
) {
    let inner = inner_rect(rect, 0.80);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.15);
    let font = egui::FontId::proportional(fsz);

    // Track (vertical bar)
    let cx = inner.center().x;
    let track_w = (inner.width() * 0.15).clamp(4.0, 14.0);
    let track_top = inner.top() + inner.height() * 0.15;
    let track_bot = inner.bottom() - inner.height() * 0.15;
    let track = Rect::from_min_max(
        Pos2::new(cx - track_w / 2.0, track_top),
        Pos2::new(cx + track_w / 2.0, track_bot),
    );
    painter.rect_filled(track, 3.0, Color32::from_rgb(200, 200, 205));
    painter.rect_stroke(track, 3.0, Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);

    // Thumb (at bottom = Off position)
    let thumb_h = (track_bot - track_top) * 0.25;
    let thumb_y = track_bot - thumb_h; // bottom = off
    let thumb = Rect::from_min_max(
        Pos2::new(cx - track_w * 0.8, thumb_y),
        Pos2::new(cx + track_w * 0.8, thumb_y + thumb_h),
    );
    painter.rect_filled(thumb, 3.0, ACCENT);

    // Labels
    painter.text(
        Pos2::new(cx - track_w - fsz * 0.5, track_top),
        Align2::RIGHT_TOP,
        "On",
        font.clone(),
        TEXT_DARK,
    );
    painter.text(
        Pos2::new(cx - track_w - fsz * 0.5, track_bot),
        Align2::RIGHT_BOTTOM,
        "Off",
        font,
        TEXT_DARK,
    );
}

// ─── RadioButton ────────────────────────────────────────────────────────

/// Draws a radio button group with 3 labelled options.
pub fn render_radio_button(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
) {
    let inner = inner_rect(rect, 0.80);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.15);
    let font = egui::FontId::proportional(fsz);
    let group_name = prop(block, "ButtonGroupName", "Group");

    // Group frame label
    painter.text(
        Pos2::new(inner.left() + 2.0, inner.top()),
        Align2::LEFT_TOP,
        group_name,
        font.clone(),
        TEXT_DARK,
    );

    // Draw 3 radio options
    let labels = ["Label1", "Label2", "Label3"];
    let radio_r = (fsz * 0.4).clamp(2.0, 6.0);
    let y_start = inner.top() + fsz * 1.8;
    let spacing = (inner.height() - fsz * 2.0) / (labels.len() as f32 + 0.5);
    for (i, lbl) in labels.iter().enumerate() {
        let y = y_start + i as f32 * spacing;
        let cx = inner.left() + radio_r + 4.0;
        // Outer circle
        painter.circle_stroke(Pos2::new(cx, y), radio_r, Stroke::new(1.0, BORDER));
        // First one is selected
        if i == 0 {
            painter.circle_filled(Pos2::new(cx, y), radio_r * 0.55, ACCENT);
        }
        painter.text(
            Pos2::new(cx + radio_r + 4.0, y),
            Align2::LEFT_CENTER,
            *lbl,
            font.clone(),
            TEXT_DARK,
        );
    }
}

// ─── ComboBox ───────────────────────────────────────────────────────────

/// Draws a combo box / dropdown with a triangle indicator.
pub fn render_combo_box(
    painter: &egui::Painter,
    _block: &Block,
    rect: &Rect,
    font_scale: f32,
) {
    let inner = inner_rect(rect, 0.80);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.35);
    let font = egui::FontId::proportional(fsz);

    // Dropdown field
    let field_h = (inner.height() * 0.4).clamp(10.0, 30.0);
    let field = Rect::from_min_max(
        Pos2::new(inner.left(), inner.center().y - field_h / 2.0),
        Pos2::new(inner.right(), inner.center().y + field_h / 2.0),
    );
    painter.rect_filled(field, 3.0, BG_FIELD);
    painter.rect_stroke(field, 3.0, Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);

    // Label text
    painter.text(
        Pos2::new(field.left() + 4.0, field.center().y),
        Align2::LEFT_CENTER,
        "Label 1",
        font,
        TEXT_DARK,
    );

    // Dropdown arrow (triangle)
    let arrow_sz = (field_h * 0.3).clamp(3.0, 8.0);
    let arrow_cx = field.right() - arrow_sz * 2.0;
    let arrow_cy = field.center().y;
    let pts = vec![
        Pos2::new(arrow_cx - arrow_sz, arrow_cy - arrow_sz * 0.5),
        Pos2::new(arrow_cx + arrow_sz, arrow_cy - arrow_sz * 0.5),
        Pos2::new(arrow_cx, arrow_cy + arrow_sz * 0.5),
    ];
    painter.add(egui::Shape::convex_polygon(pts, TEXT_DARK, Stroke::NONE));
}

// ─── CheckBox ───────────────────────────────────────────────────────────

/// Draws a checkbox with a label.
pub fn render_checkbox(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
) {
    let inner = inner_rect(rect, 0.80);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.35);
    let font = egui::FontId::proportional(fsz);
    let label = prop(block, "Text", "Label");

    // Checkbox square
    let box_sz = (fsz * 1.1).clamp(6.0, 16.0);
    let cx = inner.left() + box_sz / 2.0 + 2.0;
    let cy = inner.center().y;
    let check_rect = Rect::from_center_size(Pos2::new(cx, cy), Vec2::splat(box_sz));
    painter.rect_filled(check_rect, 2.0, BG_FIELD);
    painter.rect_stroke(check_rect, 2.0, Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);

    // Label
    painter.text(
        Pos2::new(cx + box_sz / 2.0 + 4.0, cy),
        Align2::LEFT_CENTER,
        label,
        font,
        TEXT_DARK,
    );
}

// ─── Slider ─────────────────────────────────────────────────────────────

/// Draws a horizontal slider with tick marks and scale.
pub fn render_slider(
    painter: &egui::Painter,
    _block: &Block,
    rect: &Rect,
    font_scale: f32,
) {
    let inner = inner_rect(rect, 0.80);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.2);
    let font = egui::FontId::proportional(fsz);

    // Track
    let track_h = (inner.height() * 0.06).clamp(2.0, 5.0);
    let cy = inner.center().y;
    let track = Rect::from_min_max(
        Pos2::new(inner.left(), cy - track_h / 2.0),
        Pos2::new(inner.right(), cy + track_h / 2.0),
    );
    painter.rect_filled(track, 2.0, Color32::from_rgb(190, 200, 210));

    // Tick marks
    let n_ticks = 11;
    for i in 0..n_ticks {
        let t = i as f32 / (n_ticks - 1) as f32;
        let x = inner.left() + t * inner.width();
        let tick_h = if i % 2 == 0 { 4.0 } else { 2.5 };
        painter.line_segment(
            [
                Pos2::new(x, cy + track_h / 2.0 + 1.0),
                Pos2::new(x, cy + track_h / 2.0 + 1.0 + tick_h),
            ],
            Stroke::new(1.0, BORDER),
        );
    }

    // Scale labels (0 and 100)
    let label_y = cy + track_h / 2.0 + 8.0;
    painter.text(
        Pos2::new(inner.left(), label_y),
        Align2::LEFT_TOP,
        "0",
        font.clone(),
        TEXT_DARK,
    );
    painter.text(
        Pos2::new(inner.right(), label_y),
        Align2::RIGHT_TOP,
        "100",
        font,
        TEXT_DARK,
    );

    // Thumb
    let thumb_w = (inner.width() * 0.04).clamp(4.0, 10.0);
    let thumb_x = inner.left() + inner.width() * 0.5; // center position
    let thumb = Rect::from_center_size(
        Pos2::new(thumb_x, cy),
        Vec2::new(thumb_w, track_h * 4.0),
    );
    painter.rect_filled(thumb, 2.0, ACCENT);
}

// ─── EditField ──────────────────────────────────────────────────────────

/// Draws a text edit field.
pub fn render_edit_field(
    painter: &egui::Painter,
    _block: &Block,
    rect: &Rect,
    font_scale: f32,
) {
    let inner = inner_rect(rect, 0.80);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.35);

    // Field rectangle
    let field_h = (inner.height() * 0.45).clamp(10.0, 30.0);
    let field = Rect::from_min_max(
        Pos2::new(inner.left(), inner.center().y - field_h / 2.0),
        Pos2::new(inner.right(), inner.center().y + field_h / 2.0),
    );
    painter.rect_filled(field, 3.0, BG_FIELD);
    painter.rect_stroke(field, 3.0, Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);

    // Blinking cursor indicator
    let cursor_x = field.left() + 6.0;
    let cursor_top = field.top() + 3.0;
    let cursor_bot = field.bottom() - 3.0;
    painter.line_segment(
        [Pos2::new(cursor_x, cursor_top), Pos2::new(cursor_x, cursor_bot)],
        Stroke::new(1.0, TEXT_DARK),
    );

    // Placeholder text
    let _ = fsz; // font size computed but placeholder kept minimal
}

// ─── ToggleSwitch ───────────────────────────────────────────────────────

/// Draws a horizontal toggle switch (Off / On).
pub fn render_toggle_switch(
    painter: &egui::Painter,
    _block: &Block,
    rect: &Rect,
    font_scale: f32,
) {
    let inner = inner_rect(rect, 0.80);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.25);
    let font = egui::FontId::proportional(fsz);

    // Switch track (horizontal capsule)
    let track_w = (inner.width() * 0.50).clamp(16.0, 50.0);
    let track_h = (inner.height() * 0.25).clamp(8.0, 20.0);
    let cx = inner.center().x;
    let cy = inner.center().y;
    let track = Rect::from_center_size(Pos2::new(cx, cy), Vec2::new(track_w, track_h));
    painter.rect_filled(track, track_h / 2.0, Color32::from_rgb(190, 195, 200));
    painter.rect_stroke(track, track_h / 2.0, Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);

    // Thumb circle (left = Off position)
    let thumb_r = track_h * 0.4;
    let thumb_x = track.left() + thumb_r + 2.0;
    painter.circle_filled(Pos2::new(thumb_x, cy), thumb_r, Color32::WHITE);
    painter.circle_stroke(Pos2::new(thumb_x, cy), thumb_r, Stroke::new(1.0, BORDER));

    // Labels
    painter.text(
        Pos2::new(track.left() - 4.0, cy),
        Align2::RIGHT_CENTER,
        "Off",
        font.clone(),
        TEXT_DARK,
    );
    painter.text(
        Pos2::new(track.right() + 4.0, cy),
        Align2::LEFT_CENTER,
        "On",
        font,
        TEXT_DARK,
    );
}

// ─── Knob ───────────────────────────────────────────────────────────────

/// Draws a circular knob with tick marks (like Simulink's Knob).
pub fn render_knob(
    painter: &egui::Painter,
    _block: &Block,
    rect: &Rect,
    font_scale: f32,
) {
    let inner = inner_rect(rect, 0.80);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.12);
    let font = egui::FontId::proportional(fsz);

    let cx = inner.center().x;
    let cy = inner.center().y + inner.height() * 0.05;
    let radius = (inner.width().min(inner.height()) * 0.35).max(8.0);

    // Knob body (outer ring)
    painter.circle_filled(Pos2::new(cx, cy), radius, Color32::from_rgb(220, 220, 225));
    painter.circle_stroke(Pos2::new(cx, cy), radius, Stroke::new(1.5, BORDER));
    // Inner circle
    painter.circle_filled(Pos2::new(cx, cy), radius * 0.7, Color32::from_rgb(235, 235, 238));

    // Scale ticks (arc from ~225° to ~315° going clockwise = 225° to -45° in standard)
    let start_angle = 5.0 * PI / 4.0; // 225 degrees
    let end_angle = -PI / 4.0; // -45 degrees
    let n_ticks = 11;
    let tick_r_outer = radius + 4.0;
    let tick_r_inner = radius + 1.0;
    for i in 0..n_ticks {
        let t = i as f32 / (n_ticks - 1) as f32;
        let angle = start_angle + t * (end_angle - start_angle);
        let outer = Pos2::new(cx + tick_r_outer * angle.cos(), cy - tick_r_outer * angle.sin());
        let inner_p = Pos2::new(cx + tick_r_inner * angle.cos(), cy - tick_r_inner * angle.sin());
        painter.line_segment([inner_p, outer], Stroke::new(1.0, BORDER));
    }

    // Needle indicator pointing at ~180° position (left = 0)
    let needle_angle = start_angle; // pointing to "0" at the start
    let needle_end = Pos2::new(
        cx + (radius * 0.6) * needle_angle.cos(),
        cy - (radius * 0.6) * needle_angle.sin(),
    );
    painter.line_segment(
        [Pos2::new(cx, cy), needle_end],
        Stroke::new(2.0, ACCENT_DARK),
    );

    // Scale labels
    let label_r = tick_r_outer + fsz;
    painter.text(
        Pos2::new(
            cx + label_r * start_angle.cos(),
            cy - label_r * start_angle.sin(),
        ),
        Align2::CENTER_CENTER,
        "0",
        font.clone(),
        TEXT_DARK,
    );
    painter.text(
        Pos2::new(
            cx + label_r * end_angle.cos(),
            cy - label_r * end_angle.sin(),
        ),
        Align2::CENTER_CENTER,
        "100",
        font,
        TEXT_DARK,
    );
}

// ─── RockerSwitch ───────────────────────────────────────────────────────

/// Draws a rocker switch (On/Off toggle with a rocker shape).
pub fn render_rocker_switch(
    painter: &egui::Painter,
    _block: &Block,
    rect: &Rect,
    font_scale: f32,
) {
    let inner = inner_rect(rect, 0.80);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.18);
    let font = egui::FontId::proportional(fsz);

    let cx = inner.center().x;
    let cy = inner.center().y;
    let w = (inner.width() * 0.5).clamp(14.0, 50.0);
    let h = (inner.height() * 0.35).clamp(10.0, 30.0);

    // Rocker housing (rounded rect)
    let housing = Rect::from_center_size(Pos2::new(cx, cy), Vec2::new(w, h));
    painter.rect_filled(housing, h * 0.3, Color32::from_rgb(200, 200, 205));
    painter.rect_stroke(housing, h * 0.3, Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);

    // Rocker element (tilted to left = Off)
    let rocker_w = w * 0.55;
    let rocker_h = h * 0.85;
    let rocker = Rect::from_min_max(
        Pos2::new(housing.left() + 1.0, cy - rocker_h / 2.0),
        Pos2::new(housing.left() + 1.0 + rocker_w, cy + rocker_h / 2.0),
    );
    painter.rect_filled(rocker, 3.0, Color32::from_rgb(230, 230, 235));
    painter.rect_stroke(rocker, 3.0, Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);

    // Labels
    painter.text(
        Pos2::new(housing.left() - 4.0, cy),
        Align2::RIGHT_CENTER,
        "Off",
        font.clone(),
        TEXT_DARK,
    );
    painter.text(
        Pos2::new(housing.right() + 4.0, cy),
        Align2::LEFT_CENTER,
        "On",
        font,
        TEXT_DARK,
    );
}

// ─── RotarySwitch ───────────────────────────────────────────────────────

/// Draws a rotary switch with discrete positions (Low / Medium / High).
pub fn render_rotary_switch(
    painter: &egui::Painter,
    _block: &Block,
    rect: &Rect,
    font_scale: f32,
) {
    let inner = inner_rect(rect, 0.80);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.12);
    let font = egui::FontId::proportional(fsz);

    let cx = inner.center().x;
    let cy = inner.center().y + inner.height() * 0.05;
    let radius = (inner.width().min(inner.height()) * 0.30).max(8.0);

    // Body
    painter.circle_filled(Pos2::new(cx, cy), radius, Color32::from_rgb(210, 215, 220));
    painter.circle_stroke(Pos2::new(cx, cy), radius, Stroke::new(1.5, BORDER));

    // Position marks
    let labels = ["Low", "Medium", "High"];
    let angles = [5.0 * PI / 4.0, PI / 2.0, -PI / 4.0]; // left, top, right
    let mark_r = radius + 4.0;
    let label_r = radius + fsz * 1.2 + 4.0;
    for (i, (lbl, angle)) in labels.iter().zip(angles.iter()).enumerate() {
        let mark_end = Pos2::new(cx + mark_r * angle.cos(), cy - mark_r * angle.sin());
        let mark_start = Pos2::new(cx + (mark_r - 3.0) * angle.cos(), cy - (mark_r - 3.0) * angle.sin());
        let col = if i == 0 { ACCENT_DARK } else { BORDER };
        painter.line_segment([mark_start, mark_end], Stroke::new(1.5, col));
        painter.text(
            Pos2::new(cx + label_r * angle.cos(), cy - label_r * angle.sin()),
            Align2::CENTER_CENTER,
            *lbl,
            font.clone(),
            TEXT_DARK,
        );
    }

    // Pointer at position 0 (Low)
    let pointer_angle = angles[0];
    let pointer_end = Pos2::new(
        cx + (radius * 0.7) * pointer_angle.cos(),
        cy - (radius * 0.7) * pointer_angle.sin(),
    );
    painter.line_segment(
        [Pos2::new(cx, cy), pointer_end],
        Stroke::new(2.5, ACCENT_DARK),
    );
    painter.circle_filled(Pos2::new(cx, cy), radius * 0.15, ACCENT_DARK);
}

// ─── Circular Gauge (full 270°) ─────────────────────────────────────────

/// Draws a full circular gauge (≈270° arc) like Simulink's Gauge block.
pub fn render_circular_gauge(
    painter: &egui::Painter,
    _block: &Block,
    rect: &Rect,
    font_scale: f32,
) {
    let inner = inner_rect(rect, 0.80);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.10);
    let font = egui::FontId::proportional(fsz);

    let cx = inner.center().x;
    let cy = inner.center().y + inner.height() * 0.05;
    let radius = (inner.width().min(inner.height()) * 0.40).max(10.0);

    // Arc background
    painter.circle_stroke(Pos2::new(cx, cy), radius, Stroke::new(2.0, BORDER));

    // Scale ticks around the 270° arc (from 225° counter-clockwise to -45°)
    let start_angle = 5.0 * PI / 4.0;
    let end_angle = -PI / 4.0;
    let n_ticks = 11;
    for i in 0..n_ticks {
        let t = i as f32 / (n_ticks - 1) as f32;
        let angle = start_angle + t * (end_angle - start_angle);
        let is_major = i % 2 == 0;
        let r_out = radius;
        let r_in = if is_major { radius - 4.0 } else { radius - 2.5 };
        let p1 = Pos2::new(cx + r_in * angle.cos(), cy - r_in * angle.sin());
        let p2 = Pos2::new(cx + r_out * angle.cos(), cy - r_out * angle.sin());
        painter.line_segment([p1, p2], Stroke::new(if is_major { 1.5 } else { 1.0 }, TEXT_DARK));

        // Scale numbers for major ticks
        if is_major {
            let val = (t * 100.0).round() as i32;
            let lr = radius + fsz * 0.8;
            painter.text(
                Pos2::new(cx + lr * angle.cos(), cy - lr * angle.sin()),
                Align2::CENTER_CENTER,
                format!("{}", val),
                font.clone(),
                TEXT_DARK,
            );
        }
    }

    // Needle (pointing to ~40)
    let needle_t = 0.4;
    let needle_angle = start_angle + needle_t * (end_angle - start_angle);
    let needle_end = Pos2::new(
        cx + (radius * 0.85) * needle_angle.cos(),
        cy - (radius * 0.85) * needle_angle.sin(),
    );
    painter.line_segment(
        [Pos2::new(cx, cy), needle_end],
        Stroke::new(2.0, NEEDLE_RED),
    );
    painter.circle_filled(Pos2::new(cx, cy), radius * 0.08, NEEDLE_RED);
}

// ─── SemiCircular Gauge (half gauge) ────────────────────────────────────

/// Draws a semi-circular (180°) gauge.
pub fn render_semi_circular_gauge(
    painter: &egui::Painter,
    _block: &Block,
    rect: &Rect,
    font_scale: f32,
) {
    let inner = inner_rect(rect, 0.80);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.12);
    let font = egui::FontId::proportional(fsz);

    let cx = inner.center().x;
    let cy = inner.bottom() - inner.height() * 0.15;
    let radius = (inner.width() * 0.40).min(inner.height() * 0.7).max(10.0);

    // Semi-arc from 180° to 0°
    let start_angle = PI;
    let end_angle = 0.0;
    let n_ticks = 11;
    for i in 0..n_ticks {
        let t = i as f32 / (n_ticks - 1) as f32;
        let angle = start_angle + t * (end_angle - start_angle);
        let is_major = i % 2 == 0;
        let r_out = radius;
        let r_in = if is_major { radius - 4.0 } else { radius - 2.5 };
        let p1 = Pos2::new(cx + r_in * angle.cos(), cy - r_in * angle.sin());
        let p2 = Pos2::new(cx + r_out * angle.cos(), cy - r_out * angle.sin());
        painter.line_segment([p1, p2], Stroke::new(if is_major { 1.5 } else { 1.0 }, TEXT_DARK));

        if is_major {
            let val = (t * 100.0).round() as i32;
            let lr = radius + fsz * 0.8;
            painter.text(
                Pos2::new(cx + lr * angle.cos(), cy - lr * angle.sin()),
                Align2::CENTER_CENTER,
                format!("{}", val),
                font.clone(),
                TEXT_DARK,
            );
        }
    }

    // Base line
    painter.line_segment(
        [
            Pos2::new(cx - radius, cy),
            Pos2::new(cx + radius, cy),
        ],
        Stroke::new(1.0, BORDER),
    );

    // Needle
    let needle_t = 0.5;
    let needle_angle = start_angle + needle_t * (end_angle - start_angle);
    let needle_end = Pos2::new(
        cx + (radius * 0.85) * needle_angle.cos(),
        cy - (radius * 0.85) * needle_angle.sin(),
    );
    painter.line_segment(
        [Pos2::new(cx, cy), needle_end],
        Stroke::new(2.0, NEEDLE_RED),
    );
    painter.circle_filled(Pos2::new(cx, cy), radius * 0.08, NEEDLE_RED);
}

// ─── Quarter Gauge ──────────────────────────────────────────────────────

/// Draws a quarter-circle (90°) gauge.
pub fn render_quarter_gauge(
    painter: &egui::Painter,
    _block: &Block,
    rect: &Rect,
    font_scale: f32,
) {
    let inner = inner_rect(rect, 0.80);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.12);
    let font = egui::FontId::proportional(fsz);

    // Origin at bottom-left of inner rect
    let cx = inner.left() + inner.width() * 0.1;
    let cy = inner.bottom() - inner.height() * 0.1;
    let radius = (inner.width() * 0.7).min(inner.height() * 0.7).max(10.0);

    // Quarter arc from 90° to 0°
    let start_angle = PI / 2.0;
    let end_angle = 0.0;
    let n_ticks = 6;
    for i in 0..n_ticks {
        let t = i as f32 / (n_ticks - 1) as f32;
        let angle = start_angle + t * (end_angle - start_angle);
        let r_out = radius;
        let r_in = radius - 3.5;
        let p1 = Pos2::new(cx + r_in * angle.cos(), cy - r_in * angle.sin());
        let p2 = Pos2::new(cx + r_out * angle.cos(), cy - r_out * angle.sin());
        painter.line_segment([p1, p2], Stroke::new(1.5, TEXT_DARK));

        let val = (t * 100.0).round() as i32;
        let lr = radius + fsz * 0.8;
        painter.text(
            Pos2::new(cx + lr * angle.cos(), cy - lr * angle.sin()),
            Align2::CENTER_CENTER,
            format!("{}", val),
            font.clone(),
            TEXT_DARK,
        );
    }

    // Needle
    let needle_t = 0.3;
    let needle_angle = start_angle + needle_t * (end_angle - start_angle);
    let needle_end = Pos2::new(
        cx + (radius * 0.85) * needle_angle.cos(),
        cy - (radius * 0.85) * needle_angle.sin(),
    );
    painter.line_segment(
        [Pos2::new(cx, cy), needle_end],
        Stroke::new(2.0, NEEDLE_RED),
    );
    painter.circle_filled(Pos2::new(cx, cy), radius * 0.06, NEEDLE_RED);
}

// ─── Linear Gauge ───────────────────────────────────────────────────────

/// Draws a horizontal linear gauge (bar-style).
pub fn render_linear_gauge(
    painter: &egui::Painter,
    _block: &Block,
    rect: &Rect,
    font_scale: f32,
) {
    let inner = inner_rect(rect, 0.80);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.18);
    let font = egui::FontId::proportional(fsz);

    // Bar track
    let bar_h = (inner.height() * 0.15).clamp(3.0, 10.0);
    let cy = inner.center().y;
    let bar = Rect::from_min_max(
        Pos2::new(inner.left(), cy - bar_h / 2.0),
        Pos2::new(inner.right(), cy + bar_h / 2.0),
    );
    painter.rect_filled(bar, 2.0, Color32::from_rgb(220, 220, 225));
    painter.rect_stroke(bar, 2.0, Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);

    // Scale ticks below bar
    let n_ticks = 11;
    for i in 0..n_ticks {
        let t = i as f32 / (n_ticks - 1) as f32;
        let x = inner.left() + t * inner.width();
        let tick_len = if i % 5 == 0 { 4.0 } else { 2.5 };
        painter.line_segment(
            [
                Pos2::new(x, bar.bottom() + 1.0),
                Pos2::new(x, bar.bottom() + 1.0 + tick_len),
            ],
            Stroke::new(1.0, TEXT_DARK),
        );
    }

    // Scale labels
    let label_y = bar.bottom() + 7.0;
    painter.text(Pos2::new(inner.left(), label_y), Align2::LEFT_TOP, "0", font.clone(), TEXT_DARK);
    painter.text(Pos2::new(inner.right(), label_y), Align2::RIGHT_TOP, "100", font, TEXT_DARK);

    // Filled portion (indicator at ~50%)
    let fill_frac = 0.5;
    let fill_rect = Rect::from_min_max(
        Pos2::new(inner.left(), cy - bar_h / 2.0),
        Pos2::new(inner.left() + inner.width() * fill_frac, cy + bar_h / 2.0),
    );
    painter.rect_filled(fill_rect, 2.0, ACCENT);

    // Indicator triangle above bar
    let tri_x = inner.left() + inner.width() * fill_frac;
    let tri_sz = bar_h * 0.8;
    let pts = vec![
        Pos2::new(tri_x, bar.top() - 1.0),
        Pos2::new(tri_x - tri_sz, bar.top() - 1.0 - tri_sz),
        Pos2::new(tri_x + tri_sz, bar.top() - 1.0 - tri_sz),
    ];
    painter.add(egui::Shape::convex_polygon(pts, ACCENT, Stroke::NONE));
}

// ─── Dashboard Scope ────────────────────────────────────────────────────

/// Draws a mini oscilloscope / waveform chart.
pub fn render_dashboard_scope(
    painter: &egui::Painter,
    _block: &Block,
    rect: &Rect,
    font_scale: f32,
) {
    let inner = inner_rect(rect, 0.85);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.10);
    let font = egui::FontId::proportional(fsz);

    // Background
    painter.rect_filled(inner, 2.0, SCOPE_BG);
    widget_frame(painter, inner, 2.0);

    // Grid lines
    let n_h = 4; // horizontal grid lines
    let n_v = 5; // vertical grid lines
    for i in 1..n_h {
        let t = i as f32 / n_h as f32;
        let y = inner.top() + t * inner.height();
        painter.line_segment(
            [Pos2::new(inner.left(), y), Pos2::new(inner.right(), y)],
            Stroke::new(0.5, SCOPE_GRID),
        );
    }
    for i in 1..n_v {
        let t = i as f32 / n_v as f32;
        let x = inner.left() + t * inner.width();
        painter.line_segment(
            [Pos2::new(x, inner.top()), Pos2::new(x, inner.bottom())],
            Stroke::new(0.5, SCOPE_GRID),
        );
    }

    // Axes
    painter.line_segment(
        [
            Pos2::new(inner.left(), inner.bottom()),
            Pos2::new(inner.right(), inner.bottom()),
        ],
        Stroke::new(1.0, TEXT_DARK),
    );
    painter.line_segment(
        [
            Pos2::new(inner.left(), inner.top()),
            Pos2::new(inner.left(), inner.bottom()),
        ],
        Stroke::new(1.0, TEXT_DARK),
    );

    // Y-axis labels
    painter.text(
        Pos2::new(inner.left() - 2.0, inner.top()),
        Align2::RIGHT_TOP,
        "1",
        font.clone(),
        TEXT_DARK,
    );
    painter.text(
        Pos2::new(inner.left() - 2.0, inner.bottom()),
        Align2::RIGHT_BOTTOM,
        "0",
        font.clone(),
        TEXT_DARK,
    );

    // Sine wave trace
    let n_pts = 60;
    let mut points: Vec<Pos2> = Vec::with_capacity(n_pts);
    for i in 0..n_pts {
        let t = i as f32 / (n_pts - 1) as f32;
        let x = inner.left() + t * inner.width();
        let y_val = 0.5 + 0.4 * (t * 4.0 * PI).sin();
        let y = inner.bottom() - y_val * inner.height();
        points.push(Pos2::new(x, y));
    }
    for seg in points.windows(2) {
        painter.line_segment([seg[0], seg[1]], Stroke::new(1.5, SCOPE_LINE));
    }

    // X-axis labels
    let x_label_y = inner.bottom() + 2.0;
    painter.text(Pos2::new(inner.left(), x_label_y), Align2::LEFT_TOP, "0", font.clone(), TEXT_DARK);
    let x_max = ((n_pts as f32) * 0.8).round() as i32;
    painter.text(Pos2::new(inner.right(), x_label_y), Align2::RIGHT_TOP, format!("{}", x_max), font, TEXT_DARK);
}

// ─── Display (Dashboard) ────────────────────────────────────────────────

/// Draws a digital display block (value readout).
pub fn render_display_block(
    painter: &egui::Painter,
    _block: &Block,
    rect: &Rect,
    font_scale: f32,
) {
    let inner = inner_rect(rect, 0.85);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.50);

    // Display field (dark background, LCD-like)
    let field_h = (inner.height() * 0.55).clamp(10.0, 40.0);
    let field = Rect::from_min_max(
        Pos2::new(inner.left(), inner.center().y - field_h / 2.0),
        Pos2::new(inner.right(), inner.center().y + field_h / 2.0),
    );
    painter.rect_filled(field, 3.0, Color32::from_rgb(240, 245, 240));
    painter.rect_stroke(field, 3.0, Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);

    // Value text
    painter.text(
        field.center(),
        Align2::CENTER_CENTER,
        "0",
        egui::FontId::monospace(fsz),
        TEXT_DARK,
    );
}

// ─── Lamp ───────────────────────────────────────────────────────────────

/// Draws a circular lamp indicator (green by default).
pub fn render_lamp(
    painter: &egui::Painter,
    _block: &Block,
    rect: &Rect,
    _font_scale: f32,
) {
    let inner = inner_rect(rect, 0.80);
    let radius = (inner.width().min(inner.height()) * 0.35).max(6.0);
    let cx = inner.center().x;
    let cy = inner.center().y;

    // Lamp body (glowing circle)
    painter.circle_filled(Pos2::new(cx, cy), radius, LAMP_ON);
    painter.circle_stroke(Pos2::new(cx, cy), radius, Stroke::new(1.5, BORDER));

    // Highlight (light reflection)
    let highlight_r = radius * 0.3;
    let hx = cx - radius * 0.2;
    let hy = cy - radius * 0.2;
    painter.circle_filled(
        Pos2::new(hx, hy),
        highlight_r,
        Color32::from_rgba_premultiplied(255, 255, 255, 100),
    );
}

// ─── Registry ───────────────────────────────────────────────────────────

/// All dashboard block types and their corresponding custom renderers.
///
/// The key is the canonical `BlockType` string as it appears in the parsed
/// model.  The value is the paint function to call instead of the default
/// icon path.
pub const DASHBOARD_RENDERERS: &[(&str, super::render::InteriorRendererFn)] = &[
    ("PushButtonBlock", render_push_button),
    ("SliderSwitchBlock", render_slider_switch),
    ("RadioButtonGroup", render_radio_button),
    ("ComboBox", render_combo_box),
    ("Checkbox", render_checkbox),
    ("SliderBlock", render_slider),
    ("EditField", render_edit_field),
    ("ToggleSwitchBlock", render_toggle_switch),
    ("KnobBlock", render_knob),
    ("RockerSwitchBlock", render_rocker_switch),
    ("RotarySwitchBlock", render_rotary_switch),
    ("QuarterGaugeBlock", render_quarter_gauge),
    ("SemiCircularGaugeBlock", render_semi_circular_gauge),
    ("LinearGaugeBlock", render_linear_gauge),
    ("DashboardScope", render_dashboard_scope),
    ("DisplayBlock", render_display_block),
    ("CircularGaugeBlock", render_circular_gauge),
    ("LampBlock", render_lamp),
];

/// Check if a block type has a dashboard-specific custom renderer.
pub fn is_dashboard_rendered(block_type: &str) -> bool {
    DASHBOARD_RENDERERS.iter().any(|(k, _)| *k == block_type)
}

/// Get the custom renderer for a dashboard block type, if one exists.
pub fn get_dashboard_renderer(block_type: &str) -> Option<super::render::InteriorRendererFn> {
    DASHBOARD_RENDERERS
        .iter()
        .find(|(k, _)| *k == block_type)
        .map(|(_, f)| *f)
}
