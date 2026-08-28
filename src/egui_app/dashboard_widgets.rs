//! Custom egui paint renderers for Simulink Dashboard / UI blocks.
//!
//! Each function draws a Simulink-like widget representation inside the
//! block's screen rectangle, mimicking the look of the Simulink Dashboard
//! library. These renderers are registered in the interior renderer registry
//! so the egui viewer draws proper widget visuals instead of a "?" fallback.

#![cfg(feature = "egui")]

#[cfg(feature = "dashboard")]
use crate::egui_app::{DashboardControlValue, state::SubsystemApp};
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
const BG_FIELD: Color32 = Color32::from_rgb(255, 255, 255);
const BORDER: Color32 = Color32::from_rgb(180, 180, 180);
const TEXT_DARK: Color32 = Color32::from_rgb(40, 40, 40);
const ACCENT: Color32 = Color32::from_rgb(60, 120, 215);
const ACCENT_DARK: Color32 = Color32::from_rgb(40, 80, 180);
const NEEDLE_RED: Color32 = Color32::from_rgb(200, 40, 40);
const SCOPE_BG: Color32 = Color32::from_rgb(250, 250, 250);
const SCOPE_GRID: Color32 = Color32::from_rgb(220, 220, 220);
const SCOPE_LINE: Color32 = Color32::from_rgb(30, 100, 200);

#[derive(Clone, Copy)]
struct WidgetPalette {
    bg_field: Color32,
    border: Color32,
    text: Color32,
    accent: Color32,
    accent_dark: Color32,
}

fn clamp_u8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

fn color_mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let inv = 1.0 - t;
    Color32::from_rgb(
        clamp_u8(a.r() as f32 * inv + b.r() as f32 * t),
        clamp_u8(a.g() as f32 * inv + b.g() as f32 * t),
        clamp_u8(a.b() as f32 * inv + b.b() as f32 * t),
    )
}

fn parse_color_string(raw: &str) -> Option<Color32> {
    let cleaned = raw
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .replace(';', ",");
    let parts: Vec<f32> = cleaned
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<f32>().ok())
        .collect();
    if parts.len() != 3 {
        return None;
    }
    let scale = if parts.iter().all(|v| *v <= 1.0) {
        255.0
    } else {
        1.0
    };
    Some(Color32::from_rgb(
        clamp_u8(parts[0] * scale),
        clamp_u8(parts[1] * scale),
        clamp_u8(parts[2] * scale),
    ))
}

fn parse_block_background_color(block: &Block) -> Option<Color32> {
    block
        .background_color
        .as_deref()
        .and_then(parse_color_string)
}

fn parse_color_property(block: &Block, keys: &[&str]) -> Option<Color32> {
    keys.iter().find_map(|key| {
        block
            .properties
            .get(*key)
            .and_then(|raw| parse_color_string(raw))
    })
}

fn widget_palette(block: &Block) -> WidgetPalette {
    let bg = parse_color_property(block, &["BackgroundColor", "Background"])
        .or_else(|| parse_block_background_color(block))
        .unwrap_or(BG_FIELD);
    let fg = parse_color_property(block, &["ForegroundColor", "Foreground", "TextColor"])
        .unwrap_or(TEXT_DARK);
    let border = color_mix(fg, bg, 0.45);
    let accent = color_mix(ACCENT, fg, 0.35);
    let accent_dark = color_mix(ACCENT_DARK, fg, 0.45);
    WidgetPalette {
        bg_field: bg,
        border,
        text: fg,
        accent,
        accent_dark,
    }
}

/// Paint a thin rounded-rect border (the "widget frame").
fn widget_frame(painter: &egui::Painter, rect: Rect, rounding: f32) {
    painter.rect_stroke(
        rect,
        rounding,
        Stroke::new(1.0_f32, BORDER),
        egui::StrokeKind::Inside,
    );
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
    (rect.height() * 0.25 * scale).max(4.0)
}
fn safe_clamp_f32(value: f32, min: f32, max: f32) -> f32 {
    let low = min.min(max);
    let high = min.max(max);
    if !low.is_finite() || !high.is_finite() {
        return if value.is_finite() { value } else { 0.0 };
    }
    if !value.is_finite() {
        return low;
    }
    value.clamp(low, high)
}

fn switch_labels_visible(rect: &Rect, vertical: bool) -> bool {
    if vertical {
        rect.height() >= 56.0 && rect.width() >= 20.0
    } else {
        rect.width() >= 62.0 && rect.height() >= 18.0
    }
}

fn should_render_dashboard_icon(rect: &Rect) -> bool {
    rect.width() < 34.0 || rect.height() < 22.0 || rect.area() < 950.0
}

fn paint_dashboard_widget_icon(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    palette: WidgetPalette,
) {
    // Draw the block's catalog icon (the single source of truth for its
    // small-size representation).  When the definition carries no icon, fall
    // back to a neutral field box.
    let def = crate::simulink_libraries::resolve_definition(block);
    if let Some(icon) = def.icon {
        let spec = crate::simulink_libraries::config::icon_to_spec(icon);
        // Simulink draws the Toggle and Rocker switches vertically; rotate their
        // minimized fallback glyph 90°.  The Slider Switch shares the same glyph
        // but stays upright.
        if matches!(
            block.block_type.as_str(),
            "ToggleSwitchBlock" | "RockerSwitchBlock"
        ) {
            crate::egui_app::render::draw_icon_spec_rotated_quarter(
                painter,
                rect,
                &spec,
                palette.text,
            );
        } else {
            crate::egui_app::render::draw_icon_spec(painter, rect, 1.0, &spec, palette.text, None);
        }
        return;
    }
    let inner = inner_rect(rect, 0.70);
    let body = Rect::from_center_size(
        inner.center(),
        Vec2::new(inner.width() * 0.8, inner.height() * 0.5),
    );
    painter.rect_stroke(
        body,
        3.0,
        Stroke::new(1.1_f32, palette.border),
        egui::StrokeKind::Inside,
    );
}

fn radio_group_metrics(rect: &Rect, font_scale: f32, option_count: usize) -> (f32, f32, f32) {
    let inner = inner_rect(rect, 0.80);
    let rows = option_count.max(1) as f32;
    let row_h = safe_clamp_f32((inner.height() - 6.0) / (rows + 1.05), 6.0, inner.height());
    let width_limited = (inner.width() / 8.5).max(5.0);
    let font_size = safe_clamp_f32(
        (row_h * 0.62).min(width_limited),
        5.0,
        18.0 * font_scale.max(0.7),
    );
    let header_h = safe_clamp_f32(row_h * 0.95, 8.0, row_h + 4.0);
    (font_size, row_h, header_h)
}

fn paint_slider_visual(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    palette: WidgetPalette,
    font_scale: f32,
    fraction: f32,
) {
    let inner = inner_rect(rect, 0.94);
    let label_space = if inner.height() > 28.0 {
        inner.height() * 0.22
    } else {
        0.0
    };
    let track_center_y = inner.center().y - label_space * 0.12;
    let track_h = safe_clamp_f32(inner.height() * 0.12, 4.0, 10.0);
    let handle_r = safe_clamp_f32(track_h * 0.85, 4.5, inner.height() * 0.20);
    let track = Rect::from_min_max(
        Pos2::new(
            inner.left() + handle_r * 0.75,
            track_center_y - track_h * 0.5,
        ),
        Pos2::new(
            inner.right() - handle_r * 0.75,
            track_center_y + track_h * 0.5,
        ),
    );
    let track_fill = color_mix(palette.border, palette.bg_field, 0.55);
    painter.rect_filled(track, track_h * 0.5, track_fill);
    painter.rect_stroke(
        track,
        track_h * 0.5,
        Stroke::new(1.0_f32, palette.border),
        egui::StrokeKind::Inside,
    );

    let handle_x = egui::lerp(track.left()..=track.right(), fraction.clamp(0.0, 1.0));
    let handle_center = Pos2::new(handle_x, track.center().y);
    painter.circle_filled(handle_center, handle_r, Color32::WHITE);
    painter.circle_stroke(
        handle_center,
        handle_r,
        Stroke::new(1.2_f32, palette.border),
    );

    let tick_top = track.bottom() + 2.0;
    let tick_bottom = tick_top + safe_clamp_f32(inner.height() * 0.10, 2.0, 6.0);
    for index in 0..11 {
        let t = index as f32 / 10.0;
        let x = egui::lerp(track.left()..=track.right(), t);
        let tick_len = if index % 5 == 0 {
            tick_bottom - tick_top
        } else {
            (tick_bottom - tick_top) * 0.55
        };
        painter.line_segment(
            [Pos2::new(x, tick_top), Pos2::new(x, tick_top + tick_len)],
            Stroke::new(1.0_f32, palette.border),
        );
    }

    if label_space > 0.0 {
        let font =
            egui::FontId::proportional((font_for_rect(rect, font_scale) * 0.72).clamp(4.0, 14.0));
        let (min, max) = gauge_range(block);
        let label_y = tick_bottom + 1.5;
        painter.text(
            Pos2::new(track.left(), label_y),
            Align2::LEFT_TOP,
            format_scale_value(min),
            font.clone(),
            palette.text,
        );
        painter.text(
            Pos2::new(track.right(), label_y),
            Align2::RIGHT_TOP,
            format_scale_value(max),
            font,
            palette.text,
        );
    }
}

fn paint_rocker_switch_visual(
    painter: &egui::Painter,
    rect: &Rect,
    palette: WidgetPalette,
    is_on: bool,
    font_scale: f32,
) {
    let inner = inner_rect(rect, 0.80);
    let show_labels = switch_labels_visible(rect, true);
    let fsz = font_for_rect(rect, font_scale).min(inner.width().min(inner.height()) * 0.18);
    let font = egui::FontId::proportional(fsz);
    let label_pad = if show_labels { fsz + 4.0 } else { 2.0 };
    let housing_area = Rect::from_min_max(
        Pos2::new(inner.left(), inner.top() + label_pad),
        Pos2::new(inner.right(), inner.bottom() - label_pad),
    );
    let cx = housing_area.center().x;
    let cy = housing_area.center().y;
    let w = (housing_area.width() * 0.30).clamp(10.0, 28.0);
    let h = safe_clamp_f32(housing_area.height() * 0.82, 18.0, housing_area.height());
    let housing = Rect::from_center_size(Pos2::new(cx, cy), Vec2::new(w, h));
    let housing_fill = if is_on {
        palette.accent.linear_multiply(0.16)
    } else {
        palette.bg_field.linear_multiply(0.92)
    };
    painter.rect_filled(housing, w * 0.4, housing_fill);
    painter.rect_stroke(
        housing,
        w * 0.4,
        Stroke::new(1.0_f32, palette.border),
        egui::StrokeKind::Inside,
    );

    let rocker_w = w * 0.85;
    let rocker_h = h * 0.55;
    let rocker = if is_on {
        Rect::from_min_max(
            Pos2::new(cx - rocker_w / 2.0, housing.top() + 1.0),
            Pos2::new(cx + rocker_w / 2.0, housing.top() + rocker_h + 1.0),
        )
    } else {
        Rect::from_min_max(
            Pos2::new(cx - rocker_w / 2.0, housing.bottom() - rocker_h - 1.0),
            Pos2::new(cx + rocker_w / 2.0, housing.bottom() - 1.0),
        )
    };
    let rocker_fill = if is_on {
        palette.accent.linear_multiply(0.42)
    } else {
        Color32::from_rgb(230, 230, 235)
    };
    painter.rect_filled(rocker, 3.0, rocker_fill);
    painter.rect_stroke(
        rocker,
        3.0,
        Stroke::new(1.0_f32, palette.border),
        egui::StrokeKind::Inside,
    );

    if show_labels {
        painter.text(
            Pos2::new(inner.center().x, inner.top() + 1.0),
            Align2::CENTER_TOP,
            "On",
            font.clone(),
            palette.text,
        );
        painter.text(
            Pos2::new(inner.center().x, inner.bottom() - 1.0),
            Align2::CENTER_BOTTOM,
            "Off",
            font,
            palette.text,
        );
    }
}

fn paint_switch_visual(
    painter: &egui::Painter,
    rect: &Rect,
    palette: WidgetPalette,
    is_on: bool,
    vertical: bool,
    font_scale: f32,
) {
    let inner = inner_rect(rect, 0.80);
    let show_labels = switch_labels_visible(rect, vertical);
    let fsz = font_for_rect(rect, font_scale).min(inner.width().min(inner.height()) * 0.18);
    let font = egui::FontId::proportional(fsz);
    let track_size = if vertical {
        Vec2::new(
            (inner.width() * 0.30).clamp(14.0, 28.0),
            (inner.height() * if show_labels { 0.40 } else { 0.62 })
                .clamp(18.0, (inner.height() - 4.0).max(18.0)),
        )
    } else {
        Vec2::new(
            (inner.width() * if show_labels { 0.32 } else { 0.62 })
                .clamp(18.0, (inner.width() - 6.0).max(18.0)),
            (inner.height() * 0.28).clamp(10.0, 22.0),
        )
    };
    let track = Rect::from_center_size(inner.center(), track_size);
    let rounding = if vertical {
        track.width() * 0.5
    } else {
        track.height() * 0.5
    };
    let track_fill = if is_on {
        palette.accent.linear_multiply(0.32)
    } else {
        palette.bg_field.linear_multiply(0.94)
    };
    painter.rect_filled(track, rounding, track_fill);
    painter.rect_stroke(
        track,
        rounding,
        Stroke::new(1.0_f32, palette.border),
        egui::StrokeKind::Inside,
    );

    let thumb_r = if vertical {
        safe_clamp_f32(track.width() * 0.32, 4.0, track.height() * 0.2)
    } else {
        safe_clamp_f32(track.height() * 0.40, 4.0, track.width() * 0.2)
    };
    let thumb_center = if vertical {
        let y = if is_on {
            track.top() + rounding
        } else {
            track.bottom() - rounding
        };
        Pos2::new(track.center().x, y)
    } else {
        let x = if is_on {
            track.right() - rounding
        } else {
            track.left() + rounding
        };
        Pos2::new(x, track.center().y)
    };
    painter.circle_filled(thumb_center, thumb_r, Color32::WHITE);
    painter.circle_stroke(thumb_center, thumb_r, Stroke::new(1.0_f32, palette.border));

    if vertical && show_labels {
        painter.text(
            Pos2::new(inner.center().x, inner.top() + 1.0),
            Align2::CENTER_TOP,
            "On",
            font.clone(),
            palette.text,
        );
        painter.text(
            Pos2::new(inner.center().x, inner.bottom() - 1.0),
            Align2::CENTER_BOTTOM,
            "Off",
            font,
            palette.text,
        );
    } else if !vertical && show_labels {
        painter.text(
            Pos2::new(inner.left() + 1.0, inner.center().y),
            Align2::LEFT_CENTER,
            "Off",
            font.clone(),
            palette.text,
        );
        painter.text(
            Pos2::new(inner.right() - 1.0, inner.center().y),
            Align2::RIGHT_CENTER,
            "On",
            font,
            palette.text,
        );
    }
}

fn checkbox_label(block: &Block) -> String {
    block
        .properties
        .get("Label")
        .or_else(|| block.properties.get("Text"))
        .cloned()
        .unwrap_or_else(|| "Label".to_string())
}

fn parse_range_property(block: &Block, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|key| {
        block
            .properties
            .get(*key)
            .and_then(|value| value.trim().parse::<f64>().ok())
    })
}

fn normalized_live_value(block: &Block, value: f64) -> f32 {
    let min =
        parse_range_property(block, &["Minimum", "ScaleMin", "LowerLimit", "Min"]).unwrap_or(0.0);
    let max =
        parse_range_property(block, &["Maximum", "ScaleMax", "UpperLimit", "Max"]).unwrap_or(100.0);
    if max <= min {
        return 0.0;
    }
    ((value - min) / (max - min)).clamp(0.0, 1.0) as f32
}

fn discrete_live_index(value: f64, option_count: usize) -> usize {
    if option_count == 0 {
        return 0;
    }
    value
        .round()
        .clamp(0.0, (option_count.saturating_sub(1)) as f64) as usize
}

fn combo_box_label(block: &Block, index: usize) -> String {
    option_labels(block)
        .get(index)
        .cloned()
        .unwrap_or_else(|| format!("Label {}", index + 1))
}

fn option_label_value_pairs(block: &Block) -> Vec<(String, Option<f64>)> {
    block
        .properties
        .get("Values")
        .map(|values| {
            values
                .split(['\n', ',', ';'])
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(|entry| {
                    if let Some((label, value)) = entry.rsplit_once(':') {
                        let parsed = value.trim().parse::<f64>().ok();
                        (label.trim().to_string(), parsed)
                    } else {
                        (entry.to_string(), None)
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn checkbox_state_from_value(block: &Block, live_value: f64) -> bool {
    let pairs = option_label_value_pairs(block);
    if pairs.len() >= 2 {
        let off = pairs[0].1.unwrap_or(0.0);
        let on = pairs[1].1.unwrap_or(1.0);
        return (live_value - on).abs() <= (live_value - off).abs();
    }
    live_value >= 0.5
}

fn gauge_range(block: &Block) -> (f64, f64) {
    let min =
        parse_range_property(block, &["Minimum", "ScaleMin", "LowerLimit", "Min"]).unwrap_or(0.0);
    let max =
        parse_range_property(block, &["Maximum", "ScaleMax", "UpperLimit", "Max"]).unwrap_or(100.0);
    if min < max { (min, max) } else { (0.0, 100.0) }
}

fn format_scale_value(value: f64) -> String {
    if (value.fract()).abs() < 1e-9 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}

fn option_labels(block: &Block) -> Vec<String> {
    let labels = option_label_value_pairs(block)
        .into_iter()
        .map(|(label, _)| label)
        .collect::<Vec<_>>();
    if labels.is_empty() {
        vec![
            "Label 1".to_string(),
            "Label 2".to_string(),
            "Label 3".to_string(),
        ]
    } else {
        labels
    }
}

#[allow(dead_code)]
fn discrete_option_items(block: &Block) -> Vec<(String, f64)> {
    let pairs = option_label_value_pairs(block);
    if pairs.is_empty() {
        return option_labels(block)
            .into_iter()
            .enumerate()
            .map(|(index, label)| (label, index as f64))
            .collect();
    }
    pairs
        .into_iter()
        .enumerate()
        .map(|(index, (label, value))| (label, value.unwrap_or(index as f64)))
        .collect()
}

#[allow(dead_code)]
fn discrete_selected_index(block: &Block, live_value: f64) -> usize {
    let options = discrete_option_items(block);
    if options.is_empty() {
        return 0;
    }
    options
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            (left.1 - live_value)
                .abs()
                .partial_cmp(&(right.1 - live_value).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn push_button_visuals(block: &Block, live_value: Option<f64>) -> (Color32, f64) {
    let on_color = parse_color_property(block, &["IconOnColor"]).unwrap_or(ACCENT);
    let off_color = parse_color_property(block, &["IconOffColor"]).unwrap_or(ACCENT_DARK);
    let off_value = block
        .properties
        .get("OffValue")
        .and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(0.0);
    let color = match live_value {
        Some(value) if (value - off_value).abs() > f64::EPSILON => on_color,
        Some(_) => off_color,
        None => off_color,
    };
    (color, off_value)
}

fn configured_dashboard_value(block: &Block) -> Option<f64> {
    block
        .current_setting
        .as_ref()
        .and_then(|value| value.trim().parse::<f64>().ok())
        .or_else(|| {
            block
                .value
                .as_ref()
                .and_then(|value| value.trim().parse::<f64>().ok())
        })
        .or_else(|| {
            block
                .properties
                .get("Value")
                .and_then(|value| value.trim().parse::<f64>().ok())
        })
}

fn parse_bracketed_numbers(raw: &str) -> Vec<f64> {
    let trimmed = raw.trim();
    let values = if let Some(idx) = trimmed.find('[') {
        &trimmed[idx..]
    } else {
        trimmed
    };
    values
        .replace(['[', ']'], " ")
        .replace(';', ",")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<f64>().ok())
        .collect()
}

fn lamp_default_color(block: &Block) -> Color32 {
    parse_color_property(block, &["DefaultColor"])
        .or_else(|| parse_block_background_color(block))
        .unwrap_or(Color32::from_rgb(192, 192, 192))
}

fn lamp_state_colors(block: &Block) -> Option<Vec<(f64, Color32)>> {
    let raw = block.properties.get("States")?;
    let mut cells = raw.split('|');
    let values = parse_bracketed_numbers(cells.next()?);
    let colors = parse_bracketed_numbers(cells.next()?);
    if values.is_empty() || colors.len() < 3 {
        return None;
    }
    let color_values = colors
        .chunks(3)
        .filter(|chunk| chunk.len() == 3)
        .map(|chunk| {
            Color32::from_rgb(
                clamp_u8(chunk[0] as f32),
                clamp_u8(chunk[1] as f32),
                clamp_u8(chunk[2] as f32),
            )
        })
        .collect::<Vec<_>>();
    if color_values.is_empty() {
        return None;
    }
    Some(values.into_iter().zip(color_values).collect())
}

fn lamp_color_for_value(block: &Block, value: Option<f64>) -> Color32 {
    let default = lamp_default_color(block);
    let Some(value) = value else {
        return default;
    };
    lamp_state_colors(block)
        .and_then(|states| {
            states.into_iter().find_map(|(state_value, color)| {
                if (value - state_value).abs() <= 1e-9 {
                    Some(color)
                } else {
                    None
                }
            })
        })
        .unwrap_or(default)
}

// ─── PushButton ─────────────────────────────────────────────────────────

/// Draws a push button like Simulink's Dashboard PushButton.
pub fn render_push_button(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    _name_font_factor: f32,
) {
    if should_render_dashboard_icon(rect) {
        paint_dashboard_widget_icon(painter, block, rect, widget_palette(block));
        return;
    }
    let palette = widget_palette(block);
    let inner = inner_rect(rect, 0.85);
    let label = prop(block, "ButtonText", &block.name);
    painter.rect_filled(inner, 4.0, palette.bg_field);
    painter.rect_stroke(
        inner,
        4.0,
        Stroke::new(1.0_f32, palette.border),
        egui::StrokeKind::Inside,
    );
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.45);
    painter.text(
        inner.center(),
        Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(fsz),
        palette.text,
    );
}

// ─── SliderSwitch ───────────────────────────────────────────────────────

/// Draws a vertical slider switch with Off/On labels.
pub fn render_slider_switch(
    painter: &egui::Painter,
    _block: &Block,
    rect: &Rect,
    font_scale: f32,
    _name_font_factor: f32,
) {
    if should_render_dashboard_icon(rect) {
        paint_dashboard_widget_icon(painter, _block, rect, widget_palette(_block));
        return;
    }
    paint_switch_visual(
        painter,
        rect,
        widget_palette(_block),
        false,
        false,
        font_scale,
    );
}

// ─── RadioButton ────────────────────────────────────────────────────────

/// Draws a radio button group with 3 labelled options.
pub fn render_radio_button(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    _name_font_factor: f32,
) {
    if should_render_dashboard_icon(rect) {
        paint_dashboard_widget_icon(painter, block, rect, widget_palette(block));
        return;
    }
    let inner = inner_rect(rect, 0.80);
    let palette = widget_palette(block);
    let labels = option_labels(block);
    let (fsz, row_h, header_h) = radio_group_metrics(rect, font_scale, labels.len());
    let font = egui::FontId::proportional(fsz);
    let group_name = prop(block, "ButtonGroupName", "Group");
    painter.text(
        Pos2::new(inner.left() + 4.0, inner.top() + 2.0),
        Align2::LEFT_TOP,
        group_name,
        font.clone(),
        palette.text,
    );

    let radio_r = safe_clamp_f32(fsz * 0.32, 3.0, row_h * 0.28);
    let y_start = inner.top() + header_h + 4.0;
    for (i, lbl) in labels.iter().enumerate() {
        let y = y_start + i as f32 * row_h + row_h * 0.4;
        if y + radio_r > inner.bottom() {
            break; // Don't overflow the rect
        }
        let cx = inner.left() + radio_r + 4.0;
        painter.circle_stroke(
            Pos2::new(cx, y),
            radio_r,
            Stroke::new(1.0_f32, palette.border),
        );
        if i == 0 {
            painter.circle_filled(Pos2::new(cx, y), radio_r * 0.55, palette.accent);
        }
        painter.text(
            Pos2::new(cx + radio_r + 6.0, y),
            Align2::LEFT_CENTER,
            lbl,
            font.clone(),
            palette.text,
        );
    }
}

// ─── ComboBox ───────────────────────────────────────────────────────────

/// Draws a combo box / dropdown with a triangle indicator.
pub fn render_combo_box(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    _name_font_factor: f32,
) {
    if should_render_dashboard_icon(rect) {
        paint_dashboard_widget_icon(painter, block, rect, widget_palette(block));
        return;
    }
    let palette = widget_palette(block);
    let inner = inner_rect(rect, 0.80);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.35);
    let font = egui::FontId::proportional(fsz);

    // Dropdown field
    let field_h = (inner.height() * 0.4).max(8.0);
    let field = Rect::from_min_max(
        Pos2::new(inner.left(), inner.center().y - field_h / 2.0),
        Pos2::new(inner.right(), inner.center().y + field_h / 2.0),
    );
    painter.rect_filled(field, 3.0, palette.bg_field);
    painter.rect_stroke(
        field,
        3.0,
        Stroke::new(1.0_f32, palette.border),
        egui::StrokeKind::Inside,
    );

    // Label text
    painter.text(
        Pos2::new(field.left() + 4.0, field.center().y),
        Align2::LEFT_CENTER,
        combo_box_label(block, 0),
        font,
        palette.text,
    );

    // Dropdown arrow (triangle)
    let arrow_sz = (field_h * 0.3).max(3.0);
    let arrow_cx = field.right() - arrow_sz * 2.0;
    let arrow_cy = field.center().y;
    let pts = vec![
        Pos2::new(arrow_cx - arrow_sz, arrow_cy - arrow_sz * 0.5),
        Pos2::new(arrow_cx + arrow_sz, arrow_cy - arrow_sz * 0.5),
        Pos2::new(arrow_cx, arrow_cy + arrow_sz * 0.5),
    ];
    painter.add(egui::Shape::convex_polygon(pts, palette.text, Stroke::NONE));
}

// ─── CheckBox ───────────────────────────────────────────────────────────

/// Draws a checkbox with a label.
pub fn render_checkbox(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    _name_font_factor: f32,
) {
    if should_render_dashboard_icon(rect) {
        paint_dashboard_widget_icon(painter, block, rect, widget_palette(block));
        return;
    }
    let inner = inner_rect(rect, 0.80);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.35);
    let font = egui::FontId::proportional(fsz);
    let label = checkbox_label(block);

    // Checkbox square
    let box_sz = (fsz * 1.1).max(6.0);
    let cx = inner.left() + box_sz / 2.0 + 2.0;
    let cy = inner.center().y;
    let check_rect = Rect::from_center_size(Pos2::new(cx, cy), Vec2::splat(box_sz));
    painter.rect_filled(check_rect, 2.0, BG_FIELD);
    painter.rect_stroke(
        check_rect,
        2.0,
        Stroke::new(1.0_f32, BORDER),
        egui::StrokeKind::Inside,
    );

    // Label
    painter.text(
        Pos2::new(cx + box_sz / 2.0 + 4.0, cy),
        Align2::LEFT_CENTER,
        &label,
        font,
        TEXT_DARK,
    );
}

// ─── Slider ─────────────────────────────────────────────────────────────

/// Draws a horizontal slider with tick marks and scale.
pub fn render_slider(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    _name_font_factor: f32,
) {
    if should_render_dashboard_icon(rect) {
        paint_dashboard_widget_icon(painter, block, rect, widget_palette(block));
        return;
    }
    paint_slider_visual(painter, block, rect, widget_palette(block), font_scale, 0.5);
}

// ─── EditField ──────────────────────────────────────────────────────────

/// Draws a text edit field.
pub fn render_edit_field(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    _name_font_factor: f32,
) {
    if should_render_dashboard_icon(rect) {
        paint_dashboard_widget_icon(painter, block, rect, widget_palette(block));
        return;
    }
    let palette = widget_palette(block);
    let inner = inner_rect(rect, 0.80);
    let fsz = safe_clamp_f32(
        (inner.height() * 0.48 * font_scale).min(inner.width() * 0.24),
        5.0,
        inner.height() * 0.62,
    );

    // Field rectangle
    let field_h = (inner.height() * 0.45).max(8.0);
    let field = Rect::from_min_max(
        Pos2::new(inner.left(), inner.center().y - field_h / 2.0),
        Pos2::new(inner.right(), inner.center().y + field_h / 2.0),
    );
    painter.rect_stroke(
        field,
        3.0,
        Stroke::new(1.0_f32, palette.border),
        egui::StrokeKind::Inside,
    );

    // Blinking cursor indicator
    let cursor_x = field.left() + 6.0;
    let cursor_top = field.top() + 3.0;
    let cursor_bot = field.bottom() - 3.0;
    painter.line_segment(
        [
            Pos2::new(cursor_x, cursor_top),
            Pos2::new(cursor_x, cursor_bot),
        ],
        Stroke::new(1.0_f32, palette.text),
    );

    if block
        .properties
        .get("ShowInitialText")
        .is_some_and(|value| value.eq_ignore_ascii_case("on"))
    {
        painter.text(
            Pos2::new(field.center().x, field.center().y),
            match block
                .properties
                .get("Alignment")
                .map(|value| value.to_ascii_lowercase())
                .as_deref()
            {
                Some("left") => Align2::LEFT_CENTER,
                Some("right") => Align2::RIGHT_CENTER,
                _ => Align2::CENTER_CENTER,
            },
            "0",
            egui::FontId::proportional(fsz),
            color_mix(palette.text, palette.bg_field, 0.45),
        );
    }
}

// ─── ToggleSwitch ───────────────────────────────────────────────────────

/// Draws a horizontal toggle switch (Off / On).
pub fn render_toggle_switch(
    painter: &egui::Painter,
    _block: &Block,
    rect: &Rect,
    font_scale: f32,
    _name_font_factor: f32,
) {
    if should_render_dashboard_icon(rect) {
        paint_dashboard_widget_icon(painter, _block, rect, widget_palette(_block));
        return;
    }
    paint_switch_visual(
        painter,
        rect,
        widget_palette(_block),
        false,
        true,
        font_scale,
    );
}

// ─── Knob ───────────────────────────────────────────────────────────────

/// Draws a circular knob with tick marks (like Simulink's Knob).
pub fn render_knob(
    painter: &egui::Painter,
    _block: &Block,
    rect: &Rect,
    font_scale: f32,
    _name_font_factor: f32,
) {
    if should_render_dashboard_icon(rect) {
        paint_dashboard_widget_icon(painter, _block, rect, widget_palette(_block));
        return;
    }
    let inner = inner_rect(rect, 0.80);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.12);
    let font = egui::FontId::proportional(fsz);

    let cx = inner.center().x;
    let cy = inner.center().y + inner.height() * 0.05;
    let radius = (inner.width().min(inner.height()) * 0.35).max(8.0);

    // Knob body (outer ring)
    painter.circle_filled(Pos2::new(cx, cy), radius, Color32::from_rgb(220, 220, 225));
    painter.circle_stroke(Pos2::new(cx, cy), radius, Stroke::new(1.5_f32, BORDER));
    // Inner circle
    painter.circle_filled(
        Pos2::new(cx, cy),
        radius * 0.7,
        Color32::from_rgb(235, 235, 238),
    );

    // Scale ticks (arc from ~225° to ~315° going clockwise = 225° to -45° in standard)
    let start_angle = 5.0 * PI / 4.0; // 225 degrees
    let end_angle = -PI / 4.0; // -45 degrees
    let n_ticks = 11;
    let tick_r_outer = radius + 4.0;
    let tick_r_inner = radius + 1.0;
    for i in 0..n_ticks {
        let t = i as f32 / (n_ticks - 1) as f32;
        let angle = start_angle + t * (end_angle - start_angle);
        let outer = Pos2::new(
            cx + tick_r_outer * angle.cos(),
            cy - tick_r_outer * angle.sin(),
        );
        let inner_p = Pos2::new(
            cx + tick_r_inner * angle.cos(),
            cy - tick_r_inner * angle.sin(),
        );
        painter.line_segment([inner_p, outer], Stroke::new(1.0_f32, BORDER));
    }

    // Needle indicator pointing at ~180° position (left = 0)
    let needle_angle = start_angle; // pointing to "0" at the start
    let needle_end = Pos2::new(
        cx + (radius * 0.6) * needle_angle.cos(),
        cy - (radius * 0.6) * needle_angle.sin(),
    );
    painter.line_segment(
        [Pos2::new(cx, cy), needle_end],
        Stroke::new(2.0_f32, ACCENT_DARK),
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
    _name_font_factor: f32,
) {
    if should_render_dashboard_icon(rect) {
        paint_dashboard_widget_icon(painter, _block, rect, widget_palette(_block));
        return;
    }
    paint_rocker_switch_visual(painter, rect, widget_palette(_block), false, font_scale);
}

// ─── RotarySwitch ───────────────────────────────────────────────────────

/// Draws a rotary switch with discrete positions.
pub fn render_rotary_switch(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    _name_font_factor: f32,
) {
    if should_render_dashboard_icon(rect) {
        paint_dashboard_widget_icon(painter, block, rect, widget_palette(block));
        return;
    }
    let inner = inner_rect(rect, 0.80);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.12);
    let font = egui::FontId::proportional(fsz);

    let cx = inner.center().x;
    let cy = inner.center().y + inner.height() * 0.05;
    let radius = (inner.width().min(inner.height()) * 0.30).max(8.0);

    // Body
    painter.circle_filled(Pos2::new(cx, cy), radius, Color32::from_rgb(210, 215, 220));
    painter.circle_stroke(Pos2::new(cx, cy), radius, Stroke::new(1.5_f32, BORDER));

    // Position marks
    let labels = option_labels(block);
    let mark_r = radius + 4.0;
    let label_r = radius + fsz * 1.2 + 4.0;
    let steps = labels.len().saturating_sub(1).max(1) as f32;
    for (i, lbl) in labels.iter().enumerate() {
        let tick_t = i as f32 / steps;
        let angle = 5.0 * PI / 4.0 + tick_t * (-3.0 * PI / 2.0);
        let mark_end = Pos2::new(cx + mark_r * angle.cos(), cy - mark_r * angle.sin());
        let mark_start = Pos2::new(
            cx + (mark_r - 3.0) * angle.cos(),
            cy - (mark_r - 3.0) * angle.sin(),
        );
        let col = if i == 0 { ACCENT_DARK } else { BORDER };
        painter.line_segment([mark_start, mark_end], Stroke::new(1.5_f32, col));
        painter.text(
            Pos2::new(cx + label_r * angle.cos(), cy - label_r * angle.sin()),
            Align2::CENTER_CENTER,
            lbl,
            font.clone(),
            TEXT_DARK,
        );
    }

    // Pointer at position 0 (Low)
    let pointer_angle = 5.0 * PI / 4.0;
    let pointer_end = Pos2::new(
        cx + (radius * 0.7) * pointer_angle.cos(),
        cy - (radius * 0.7) * pointer_angle.sin(),
    );
    painter.line_segment(
        [Pos2::new(cx, cy), pointer_end],
        Stroke::new(2.5_f32, ACCENT_DARK),
    );
    painter.circle_filled(Pos2::new(cx, cy), radius * 0.15, ACCENT_DARK);
}

// ─── Circular Gauge (full 270°) ─────────────────────────────────────────

/// Draws a full circular gauge (≈270° arc) like Simulink's Gauge block.
pub fn render_circular_gauge(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    _name_font_factor: f32,
) {
    if should_render_dashboard_icon(rect) {
        paint_dashboard_widget_icon(painter, block, rect, widget_palette(block));
        return;
    }
    let (min, max) = gauge_range(block);
    let inner = inner_rect(rect, 0.80);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.10);
    let font = egui::FontId::proportional(fsz);

    let cx = inner.center().x;
    let cy = inner.center().y + inner.height() * 0.05;
    let radius = (inner.width().min(inner.height()) * 0.40).max(10.0);

    // Arc background
    painter.circle_stroke(Pos2::new(cx, cy), radius, Stroke::new(2.0_f32, BORDER));

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
        painter.line_segment(
            [p1, p2],
            Stroke::new(if is_major { 1.5_f32 } else { 1.0_f32 }, TEXT_DARK),
        );

        // Scale numbers for major ticks
        if is_major {
            let val = min + (max - min) * t as f64;
            let lr = radius + fsz * 0.8;
            painter.text(
                Pos2::new(cx + lr * angle.cos(), cy - lr * angle.sin()),
                Align2::CENTER_CENTER,
                format_scale_value(val),
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
        Stroke::new(2.0_f32, NEEDLE_RED),
    );
    painter.circle_filled(Pos2::new(cx, cy), radius * 0.08, NEEDLE_RED);
}

// ─── SemiCircular Gauge (half gauge) ────────────────────────────────────

/// Draws a semi-circular (180°) gauge.
pub fn render_semi_circular_gauge(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    _name_font_factor: f32,
) {
    if should_render_dashboard_icon(rect) {
        paint_dashboard_widget_icon(painter, block, rect, widget_palette(block));
        return;
    }
    let (min, max) = gauge_range(block);
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
        painter.line_segment(
            [p1, p2],
            Stroke::new(if is_major { 1.5_f32 } else { 1.0_f32 }, TEXT_DARK),
        );

        if is_major {
            let val = min + (max - min) * t as f64;
            let lr = radius + fsz * 0.8;
            painter.text(
                Pos2::new(cx + lr * angle.cos(), cy - lr * angle.sin()),
                Align2::CENTER_CENTER,
                format_scale_value(val),
                font.clone(),
                TEXT_DARK,
            );
        }
    }

    // Base line
    painter.line_segment(
        [Pos2::new(cx - radius, cy), Pos2::new(cx + radius, cy)],
        Stroke::new(1.0_f32, BORDER),
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
        Stroke::new(2.0_f32, NEEDLE_RED),
    );
    painter.circle_filled(Pos2::new(cx, cy), radius * 0.08, NEEDLE_RED);
}

// ─── Quarter Gauge ──────────────────────────────────────────────────────

/// Draws a quarter-circle (90°) gauge.
pub fn render_quarter_gauge(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    _name_font_factor: f32,
) {
    if should_render_dashboard_icon(rect) {
        paint_dashboard_widget_icon(painter, block, rect, widget_palette(block));
        return;
    }
    let (min, max) = gauge_range(block);
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
        painter.line_segment([p1, p2], Stroke::new(1.5_f32, TEXT_DARK));

        let val = min + (max - min) * t as f64;
        let lr = radius + fsz * 0.8;
        painter.text(
            Pos2::new(cx + lr * angle.cos(), cy - lr * angle.sin()),
            Align2::CENTER_CENTER,
            format_scale_value(val),
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
        Stroke::new(2.0_f32, NEEDLE_RED),
    );
    painter.circle_filled(Pos2::new(cx, cy), radius * 0.06, NEEDLE_RED);
}

// ─── Linear Gauge ───────────────────────────────────────────────────────

/// Draws a horizontal linear gauge (bar-style).
pub fn render_linear_gauge(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    _name_font_factor: f32,
) {
    if should_render_dashboard_icon(rect) {
        paint_dashboard_widget_icon(painter, block, rect, widget_palette(block));
        return;
    }
    let (min, max) = gauge_range(block);
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
    painter.rect_stroke(
        bar,
        2.0,
        Stroke::new(1.0_f32, BORDER),
        egui::StrokeKind::Inside,
    );

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
            Stroke::new(1.0_f32, TEXT_DARK),
        );
    }

    // Scale labels
    let label_y = bar.bottom() + 7.0;
    painter.text(
        Pos2::new(inner.left(), label_y),
        Align2::LEFT_TOP,
        format_scale_value(min),
        font.clone(),
        TEXT_DARK,
    );
    painter.text(
        Pos2::new(inner.right(), label_y),
        Align2::RIGHT_TOP,
        format_scale_value(max),
        font,
        TEXT_DARK,
    );

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
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    _name_font_factor: f32,
) {
    if should_render_dashboard_icon(rect) {
        paint_dashboard_widget_icon(painter, block, rect, widget_palette(block));
        return;
    }
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
            Stroke::new(0.5_f32, SCOPE_GRID),
        );
    }
    for i in 1..n_v {
        let t = i as f32 / n_v as f32;
        let x = inner.left() + t * inner.width();
        painter.line_segment(
            [Pos2::new(x, inner.top()), Pos2::new(x, inner.bottom())],
            Stroke::new(0.5_f32, SCOPE_GRID),
        );
    }

    // Axes
    painter.line_segment(
        [
            Pos2::new(inner.left(), inner.bottom()),
            Pos2::new(inner.right(), inner.bottom()),
        ],
        Stroke::new(1.0_f32, TEXT_DARK),
    );
    painter.line_segment(
        [
            Pos2::new(inner.left(), inner.top()),
            Pos2::new(inner.left(), inner.bottom()),
        ],
        Stroke::new(1.0_f32, TEXT_DARK),
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
        painter.line_segment([seg[0], seg[1]], Stroke::new(1.5_f32, SCOPE_LINE));
    }

    // X-axis labels
    let x_label_y = inner.bottom() + 2.0;
    painter.text(
        Pos2::new(inner.left(), x_label_y),
        Align2::LEFT_TOP,
        "0",
        font.clone(),
        TEXT_DARK,
    );
    let x_max = ((n_pts as f32) * 0.8).round() as i32;
    painter.text(
        Pos2::new(inner.right(), x_label_y),
        Align2::RIGHT_TOP,
        format!("{}", x_max),
        font,
        TEXT_DARK,
    );
}

// ─── Display (Dashboard) ────────────────────────────────────────────────

/// Draws a digital display block (value readout).
pub fn render_display_block(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    _name_font_factor: f32,
) {
    if should_render_dashboard_icon(rect) {
        paint_dashboard_widget_icon(painter, block, rect, widget_palette(block));
        return;
    }
    let inner = inner_rect(rect, 0.85);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.50);

    // Display field (dark background, LCD-like)
    let field_h = (inner.height() * 0.55).clamp(10.0, 40.0);
    let field = Rect::from_min_max(
        Pos2::new(inner.left(), inner.center().y - field_h / 2.0),
        Pos2::new(inner.right(), inner.center().y + field_h / 2.0),
    );
    painter.rect_filled(field, 3.0, Color32::from_rgb(240, 245, 240));
    painter.rect_stroke(
        field,
        3.0,
        Stroke::new(1.0_f32, BORDER),
        egui::StrokeKind::Inside,
    );

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
    block: &Block,
    rect: &Rect,
    _font_scale: f32,
    _name_font_factor: f32,
) {
    if should_render_dashboard_icon(rect) {
        paint_dashboard_widget_icon(painter, block, rect, widget_palette(block));
        return;
    }
    let inner = inner_rect(rect, 0.80);
    let radius = (inner.width().min(inner.height()) * 0.35).max(6.0);
    let cx = inner.center().x;
    let cy = inner.center().y;

    // Lamp body (glowing circle)
    painter.circle_filled(
        Pos2::new(cx, cy),
        radius,
        lamp_color_for_value(block, configured_dashboard_value(block)),
    );
    painter.circle_stroke(Pos2::new(cx, cy), radius, Stroke::new(1.5_f32, BORDER));

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

#[cfg(feature = "dashboard")]
fn dashboard_input_control_kind(block: &Block) -> Option<&'static str> {
    if !matches!(
        block.dashboard_binding,
        Some(crate::model::DashboardBinding::ParamSource { .. })
    ) {
        return None;
    }

    // The control kind is data on the block's definition, not a block-type match.
    crate::simulink_libraries::resolve_definition(block)
        .dashboard_control
        .map(|kind| kind.as_str())
}

#[cfg(feature = "dashboard")]
fn dashboard_control_storage_key(block: &Block) -> String {
    block.sid.clone().unwrap_or_else(|| block.name.clone())
}

#[cfg(feature = "dashboard")]
fn apply_dashboard_widget_style_with_body_size(
    ui: &mut egui::Ui,
    body_size: f32,
    palette: WidgetPalette,
) {
    let mut style: egui::Style = ui.style().as_ref().clone();
    style.visuals.override_text_color = Some(palette.text);
    style.visuals.widgets.noninteractive.fg_stroke.color = palette.text;
    style.visuals.widgets.inactive.fg_stroke.color = palette.text;
    style.visuals.widgets.hovered.fg_stroke.color = palette.text;
    style.visuals.widgets.active.fg_stroke.color = palette.text;
    style.visuals.widgets.inactive.bg_fill = palette.bg_field;
    style.visuals.widgets.hovered.bg_fill = palette.bg_field;
    style.visuals.widgets.active.bg_fill = palette.bg_field;
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(body_size));
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::proportional(body_size),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::proportional((body_size * 0.85).max(7.0)),
    );
    ui.set_style(style);
}

#[cfg(feature = "dashboard")]
fn apply_dashboard_widget_style(
    ui: &mut egui::Ui,
    rect: Rect,
    font_scale: f32,
    palette: WidgetPalette,
) {
    let body_size = (rect.height() * 0.35 * font_scale).clamp(8.0, 40.0);
    apply_dashboard_widget_style_with_body_size(ui, body_size, palette);
}

#[cfg(feature = "dashboard")]
fn paint_dashboard_widget_background(ui: &mut egui::Ui, rect: Rect, palette: WidgetPalette) {
    if palette.bg_field != BG_FIELD {
        ui.painter()
            .rect_filled(rect.shrink(2.0), 4.0, palette.bg_field);
    }
}

#[cfg(feature = "dashboard")]
fn dashboard_knob_geometry(rect: Rect) -> (Pos2, f32) {
    let inner = rect.shrink2(Vec2::new(rect.width() * 0.1, rect.height() * 0.1));
    let center = Pos2::new(inner.center().x, inner.center().y + inner.height() * 0.05);
    let radius = (inner.width().min(inner.height()) * 0.35).max(8.0);
    (center, radius)
}

#[cfg(feature = "dashboard")]
fn dashboard_rotary_geometry(rect: Rect) -> (Pos2, f32) {
    let inner = rect.shrink2(Vec2::new(rect.width() * 0.1, rect.height() * 0.1));
    let center = Pos2::new(inner.center().x, inner.center().y + inner.height() * 0.05);
    let radius = (inner.width().min(inner.height()) * 0.30).max(8.0);
    (center, radius)
}

#[cfg(feature = "dashboard")]
fn dashboard_arc_fraction(pointer: Pos2, center: Pos2) -> f64 {
    let dx = pointer.x - center.x;
    let dy = center.y - pointer.y;
    let mut angle_deg = dy.atan2(dx).to_degrees();
    if angle_deg < 0.0 {
        angle_deg += 360.0;
    }
    let start_deg = 225.0;
    let clockwise = (start_deg - angle_deg).rem_euclid(360.0);
    if clockwise <= 270.0 {
        (clockwise / 270.0).clamp(0.0, 1.0) as f64
    } else if clockwise < 315.0 {
        1.0
    } else {
        0.0
    }
}

#[cfg(feature = "dashboard")]
fn dashboard_discrete_value_from_pointer(
    block: &Block,
    rect: Rect,
    pointer: Pos2,
    fallback: f64,
) -> f64 {
    match block.block_type.as_str() {
        "RadioButtonGroup" => {
            let labels = option_labels(block);
            if labels.is_empty() {
                return fallback;
            }
            let inner = inner_rect(&rect, 0.80);
            let (_, row_h, header_h) = radio_group_metrics(&rect, 1.0, labels.len());
            let y_start = inner.top() + header_h + 4.0;
            let index = ((pointer.y - y_start) / row_h).floor() as isize;
            index.clamp(0, labels.len().saturating_sub(1) as isize) as f64
        }
        "RotarySwitchBlock" => {
            let (center, radius) = dashboard_rotary_geometry(rect);
            if pointer.distance(center) > radius * 2.5 {
                return fallback;
            }
            let labels = option_labels(block);
            let steps = labels.len().saturating_sub(1).max(1) as f64;
            (dashboard_arc_fraction(pointer, center) * steps).round()
        }
        _ => {
            let labels = option_labels(block);
            let steps = labels.len().saturating_sub(1).max(1) as f64;
            let t = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
            (t * steps).round()
        }
    }
}

#[cfg(feature = "dashboard")]
pub(crate) fn dashboard_scalar_value_from_pointer(
    block: &Block,
    rect: Rect,
    pointer: Pos2,
    fallback: f64,
) -> f64 {
    let (min, max) = gauge_range(block);
    let fraction = match block.block_type.as_str() {
        "KnobBlock" => {
            let (center, radius) = dashboard_knob_geometry(rect);
            if pointer.distance(center) > radius * 3.0 {
                return fallback;
            }
            dashboard_arc_fraction(pointer, center) as f32
        }
        "RotarySwitchBlock" => {
            let (center, radius) = dashboard_rotary_geometry(rect);
            if pointer.distance(center) > radius * 2.5 {
                return fallback;
            }
            let normalized = dashboard_arc_fraction(pointer, center) as f32;
            let labels = option_labels(block);
            let steps = labels.len().saturating_sub(1).max(1) as f32;
            return (normalized * steps).round() as f64;
        }
        _ if rect.height() > rect.width() => {
            ((rect.bottom() - pointer.y) / rect.height()).clamp(0.0, 1.0)
        }
        _ => ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0),
    };

    if fraction.is_finite() {
        min + (max - min) * fraction as f64
    } else {
        fallback
    }
}

#[cfg(feature = "dashboard")]
fn render_checkbox_control_widget(
    app: &mut SubsystemApp,
    ui: &mut egui::Ui,
    block: &Block,
    rect: Rect,
    font_scale: f32,
    live_value: f64,
) -> bool {
    if should_render_dashboard_icon(&rect) {
        paint_dashboard_widget_icon(ui.painter(), block, &rect, widget_palette(block));
        return false;
    }
    let palette = widget_palette(block);
    let (off_value, on_value) = option_label_value_pairs(block)
        .get(0..2)
        .map(|pairs| (pairs[0].1.unwrap_or(0.0), pairs[1].1.unwrap_or(1.0)))
        .unwrap_or((0.0, 1.0));
    let mut current = (live_value - on_value).abs() <= (live_value - off_value).abs();
    let mut changed = false;
    let label = checkbox_label(block);
    paint_dashboard_widget_background(ui, rect, palette);
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(rect.shrink(6.0)),
        |child_ui| {
            apply_dashboard_widget_style(child_ui, rect, font_scale, palette);
            child_ui.add_enabled_ui(app.live_mode_enabled, |child_ui| {
                child_ui.centered_and_justified(|child_ui| {
                    if child_ui
                        .add(egui::Checkbox::new(&mut current, label))
                        .changed()
                    {
                        changed = true;
                    }
                });
            });
        },
    );
    if app.live_mode_enabled && changed {
        let value = if current { on_value } else { off_value };
        app.queue_dashboard_control(block.clone(), DashboardControlValue::Scalar(value));
    }
    // The live visual has been drawn above; return `true` so the caller does
    // not also draw the static renderer on top of it.
    true
}

#[cfg(feature = "dashboard")]
fn render_radio_button_group_control_widget(
    app: &mut SubsystemApp,
    ui: &mut egui::Ui,
    block: &Block,
    rect: Rect,
    font_scale: f32,
    live_value: f64,
) -> bool {
    if should_render_dashboard_icon(&rect) {
        paint_dashboard_widget_icon(ui.painter(), block, &rect, widget_palette(block));
        return false;
    }
    render_painted_control_widget(
        app,
        ui,
        block,
        rect,
        font_scale,
        live_value,
        live_radio_button_group,
    )
}

#[cfg(feature = "dashboard")]
fn render_combo_box_control_widget(
    app: &mut SubsystemApp,
    ui: &mut egui::Ui,
    block: &Block,
    rect: Rect,
    font_scale: f32,
    live_value: f64,
) -> bool {
    if should_render_dashboard_icon(&rect) {
        paint_dashboard_widget_icon(ui.painter(), block, &rect, widget_palette(block));
        return false;
    }
    let storage_key = dashboard_control_storage_key(block);
    let interact_id = app.egui_id(("dashboard_combo_box", storage_key.as_str()));
    let options = discrete_option_items(block);
    let selected_index =
        discrete_selected_index(block, live_value).min(options.len().saturating_sub(1));
    let palette = widget_palette(block);
    let default_visuals = ui.style().visuals.clone();
    let mut selected_value = None;
    paint_dashboard_widget_background(ui, rect, palette);
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(rect.shrink(6.0)),
        |child_ui| {
            apply_dashboard_widget_style(child_ui, rect, font_scale, palette);
            child_ui.add_enabled_ui(app.live_mode_enabled, |child_ui| {
                let selected_label = options
                    .get(selected_index)
                    .map(|(label, _)| label.as_str())
                    .unwrap_or("—");
                child_ui.scope(|combo_ui| {
                    let mut combo_style: egui::Style = combo_ui.style().as_ref().clone();
                    combo_style.visuals.override_text_color = default_visuals.override_text_color;
                    combo_style.visuals.widgets.noninteractive.fg_stroke.color =
                        default_visuals.widgets.noninteractive.fg_stroke.color;
                    combo_style.visuals.widgets.inactive.fg_stroke.color =
                        default_visuals.widgets.inactive.fg_stroke.color;
                    combo_style.visuals.widgets.hovered.fg_stroke.color =
                        default_visuals.widgets.hovered.fg_stroke.color;
                    combo_style.visuals.widgets.active.fg_stroke.color =
                        default_visuals.widgets.active.fg_stroke.color;
                    *combo_ui.style_mut() = combo_style;

                    egui::ComboBox::from_id_salt(interact_id)
                        .selected_text(egui::RichText::new(selected_label))
                        .width(rect.shrink(12.0).width().max(80.0))
                        .wrap_mode(egui::TextWrapMode::Truncate)
                        .show_ui(combo_ui, |ui| {
                            ui.set_min_width(rect.width().max(120.0));
                            for (index, (label, value)) in options.iter().enumerate() {
                                if ui
                                    .selectable_label(index == selected_index, label)
                                    .clicked()
                                {
                                    selected_value = Some(*value);
                                    ui.close();
                                }
                            }
                        });
                });
            });
        },
    );

    if let Some(value) = selected_value {
        app.queue_dashboard_control(block.clone(), DashboardControlValue::Scalar(value));
    }
    // The live visual has been drawn above; return `true` so the caller does
    // not also draw the static renderer on top of it.
    true
}

#[cfg(feature = "dashboard")]
fn render_edit_field_control_widget(
    app: &mut SubsystemApp,
    ui: &mut egui::Ui,
    block: &Block,
    rect: Rect,
    font_scale: f32,
    live_value: f64,
    live_text: Option<&str>,
) -> bool {
    let palette = widget_palette(block);
    let initial = live_text
        .map(str::to_string)
        .unwrap_or_else(|| crate::egui_app::ui::update::format_live_scalar_csv(live_value));
    let storage_key = dashboard_control_storage_key(block);
    let edit_id = ui.make_persistent_id(("dashboard_edit_field", storage_key.as_str()));
    let buffer = app
        .dashboard_edit_buffers
        .entry(storage_key.clone())
        .or_insert_with(|| initial.clone());
    if !ui.memory(|memory| memory.has_focus(edit_id)) && *buffer != initial {
        *buffer = initial.clone();
    }
    let mut submitted = None;
    paint_dashboard_widget_background(ui, rect, palette);
    let content_margin = 6.0_f32.min(rect.width() * 0.5).min(rect.height() * 0.5);
    let content_rect = rect.shrink(content_margin);
    ui.scope_builder(egui::UiBuilder::new().max_rect(content_rect), |child_ui| {
        let max_body_size = (rect.height() * 0.58).max(7.0);
        let edit_body_size = (rect.height() * 0.44 * font_scale)
            .min(rect.width() * 0.22)
            .clamp(7.0, max_body_size);
        apply_dashboard_widget_style_with_body_size(child_ui, edit_body_size, palette);
        child_ui.add_enabled_ui(app.live_mode_enabled, |child_ui| {
            let response = child_ui.add_sized(
                content_rect.size(),
                egui::TextEdit::singleline(buffer)
                    .id(edit_id)
                    .horizontal_align(egui::Align::Center)
                    .frame(egui::Frame::NONE),
            );
            child_ui.painter().rect_stroke(
                response.rect,
                3.0,
                Stroke::new(1.0_f32, palette.border),
                egui::StrokeKind::Inside,
            );
            if response.lost_focus() && child_ui.input(|input| input.key_pressed(egui::Key::Enter))
            {
                submitted = buffer.trim().parse::<f64>().ok();
            }
        });
    });
    if app.live_mode_enabled
        && let Some(value) = submitted
    {
        app.queue_dashboard_control(block.clone(), DashboardControlValue::Scalar(value));
    }
    // The live visual has been drawn above; return `true` so the caller does
    // not also draw the static renderer on top of it.
    true
}

#[cfg(feature = "dashboard")]
fn render_push_button_control_widget(
    app: &mut SubsystemApp,
    ui: &mut egui::Ui,
    block: &Block,
    rect: Rect,
    font_scale: f32,
) -> bool {
    if should_render_dashboard_icon(&rect) {
        paint_dashboard_widget_icon(ui.painter(), block, &rect, widget_palette(block));
        return false;
    }
    let storage_key = dashboard_control_storage_key(block);
    let preview_value = if app.dashboard_active_pulses.contains(&storage_key) {
        1.0
    } else {
        0.0
    };
    live_push_button(
        &ui.painter().with_clip_rect(rect),
        block,
        &rect,
        font_scale,
        preview_value,
        Some(&app.live_display_defaults),
    );
    let interact_id = app.egui_id(("dashboard_live_overlay", storage_key.as_str()));
    let response = ui.interact(rect.shrink(4.0), interact_id, egui::Sense::click());
    if app.live_mode_enabled {
        let is_down = response.is_pointer_button_down_on();
        let was_down = app.dashboard_active_pulses.contains(&storage_key);
        if is_down && !was_down {
            app.dashboard_active_pulses.insert(storage_key.clone());
            app.queue_dashboard_control(block.clone(), DashboardControlValue::PulseHigh);
        } else if was_down && !ui.input(|input| input.pointer.primary_down()) {
            app.dashboard_active_pulses.remove(&storage_key);
            app.queue_dashboard_control(block.clone(), DashboardControlValue::PulseLow);
        }
    }
    // The live visual has been drawn above; return `true` so the caller does
    // not also draw the static renderer on top of it.
    true
}

/// A painter-only per-widget live visual: the single concern of "draw this
/// dashboard widget at `value`".  Interactive controls compose one of these
/// with their interaction handling; non-interactive widgets use it directly.
#[cfg(feature = "dashboard")]
type PainterLiveDrawFn = fn(
    &egui::Painter,
    &Block,
    &Rect,
    f32,
    f64,
    Option<&crate::live_values::LiveValueDisplayOptions>,
);

#[cfg(feature = "dashboard")]
fn render_painted_control_widget(
    app: &mut SubsystemApp,
    ui: &mut egui::Ui,
    block: &Block,
    rect: Rect,
    font_scale: f32,
    live_value: f64,
    draw: PainterLiveDrawFn,
) -> bool {
    let Some(kind) = dashboard_input_control_kind(block) else {
        return false;
    };
    let interact_id = app.egui_id((
        "dashboard_live_overlay",
        dashboard_control_storage_key(block),
    ));
    let sense = if app.live_mode_enabled {
        match kind {
            "scalar" => egui::Sense::click_and_drag(),
            _ => egui::Sense::click(),
        }
    } else {
        egui::Sense::hover()
    };
    let interact_rect = rect.shrink(4.0);
    let response = ui.interact(interact_rect, interact_id, sense);
    let preview_value = if app.live_mode_enabled {
        match kind {
            "bool" if response.clicked() => Some(if live_value < 0.5 { 1.0 } else { 0.0 }),
            "discrete" | "scalar"
                if response.interact_pointer_pos().is_some()
                    && (response.clicked() || response.dragged()) =>
            {
                response.interact_pointer_pos().map(|pointer_pos| {
                    if kind == "discrete" {
                        dashboard_discrete_value_from_pointer(
                            block,
                            interact_rect,
                            pointer_pos,
                            live_value,
                        )
                    } else {
                        dashboard_scalar_value_from_pointer(
                            block,
                            interact_rect,
                            pointer_pos,
                            live_value,
                        )
                    }
                })
            }
            _ => None,
        }
    } else {
        None
    };
    draw(
        &ui.painter().with_clip_rect(rect),
        block,
        &rect,
        font_scale,
        preview_value.unwrap_or(live_value),
        Some(&app.live_display_defaults),
    );
    if !app.live_mode_enabled {
        return false;
    }

    // The live visual has been drawn above; queue any interaction side effect
    // but always return `true` so the caller does not also draw the static
    // renderer on top of the live visual.
    match kind {
        "bool" => {
            if let Some(preview_value) = preview_value {
                app.queue_dashboard_control(
                    block.clone(),
                    DashboardControlValue::Bool(preview_value >= 0.5),
                );
            }
        }
        "discrete" | "scalar" => {
            if let Some(value) = preview_value {
                app.queue_dashboard_control(block.clone(), DashboardControlValue::Scalar(value));
            }
        }
        _ => {}
    }
    true
}

// ─── Per-block interactive control entry points ─────────────────────────────
//
// Each interactive dashboard control composes its painter-only live visual with
// its interaction handling.  The catalog wires one of these as the block's
// (unified) `LiveRendererFn`, so the live UI dispatches purely through the
// resolved definition — there is no separate control-renderer type and no
// `block_type` match.  Every entry point shares one signature so the catalog
// adapter can call them uniformly.

/// A painted control = a painter-only live visual + pointer interaction handled
/// generically by [`render_painted_control_widget`] (per `dashboard_control`).
#[cfg(feature = "dashboard")]
macro_rules! painted_control {
    ($name:ident => $draw:ident) => {
        pub fn $name(
            app: &mut SubsystemApp,
            ui: &mut egui::Ui,
            block: &Block,
            rect: Rect,
            font_scale: f32,
            live_value: f64,
            _live_text: Option<&str>,
        ) -> bool {
            render_painted_control_widget(app, ui, block, rect, font_scale, live_value, $draw)
        }
    };
}

/// A control that owns its own egui widgets (checkbox/combo/radio) and ignores
/// the live-text representation.
#[cfg(feature = "dashboard")]
macro_rules! simple_control {
    ($name:ident => $inner:ident) => {
        pub fn $name(
            app: &mut SubsystemApp,
            ui: &mut egui::Ui,
            block: &Block,
            rect: Rect,
            font_scale: f32,
            live_value: f64,
            _live_text: Option<&str>,
        ) -> bool {
            $inner(app, ui, block, rect, font_scale, live_value)
        }
    };
}

#[cfg(feature = "dashboard")]
simple_control!(control_checkbox => render_checkbox_control_widget);
#[cfg(feature = "dashboard")]
simple_control!(control_combo_box => render_combo_box_control_widget);
#[cfg(feature = "dashboard")]
simple_control!(control_radio_button_group => render_radio_button_group_control_widget);

#[cfg(feature = "dashboard")]
painted_control!(control_slider => live_slider_or_linear_gauge);
#[cfg(feature = "dashboard")]
painted_control!(control_slider_switch => live_slider_switch);
#[cfg(feature = "dashboard")]
painted_control!(control_toggle_switch => live_toggle_switch);
#[cfg(feature = "dashboard")]
painted_control!(control_rocker_switch => live_rocker_switch);
#[cfg(feature = "dashboard")]
painted_control!(control_rotary_switch => live_radial_gauge);
#[cfg(feature = "dashboard")]
painted_control!(control_knob => live_radial_gauge);

/// Push button has no live value to forward.
#[cfg(feature = "dashboard")]
pub fn control_push_button(
    app: &mut SubsystemApp,
    ui: &mut egui::Ui,
    block: &Block,
    rect: Rect,
    font_scale: f32,
    _live_value: f64,
    _live_text: Option<&str>,
) -> bool {
    render_push_button_control_widget(app, ui, block, rect, font_scale)
}

/// Edit field also consumes the live text representation.
#[cfg(feature = "dashboard")]
pub fn control_edit_field(
    app: &mut SubsystemApp,
    ui: &mut egui::Ui,
    block: &Block,
    rect: Rect,
    font_scale: f32,
    live_value: f64,
    live_text: Option<&str>,
) -> bool {
    render_edit_field_control_widget(app, ui, block, rect, font_scale, live_value, live_text)
}

// Without the `dashboard` feature the catalog still references these entry
// points (it is `egui`-gated), so provide inert stubs that never claim to draw.
#[cfg(not(feature = "dashboard"))]
macro_rules! control_stub {
    ($name:ident) => {
        pub fn $name(
            _app: &mut crate::egui_app::state::SubsystemApp,
            _ui: &mut egui::Ui,
            _block: &Block,
            _rect: Rect,
            _font_scale: f32,
            _live_value: f64,
            _live_text: Option<&str>,
        ) -> bool {
            false
        }
    };
}

#[cfg(not(feature = "dashboard"))]
control_stub!(control_checkbox);
#[cfg(not(feature = "dashboard"))]
control_stub!(control_radio_button_group);
#[cfg(not(feature = "dashboard"))]
control_stub!(control_combo_box);
#[cfg(not(feature = "dashboard"))]
control_stub!(control_slider);
#[cfg(not(feature = "dashboard"))]
control_stub!(control_slider_switch);
#[cfg(not(feature = "dashboard"))]
control_stub!(control_toggle_switch);
#[cfg(not(feature = "dashboard"))]
control_stub!(control_rocker_switch);
#[cfg(not(feature = "dashboard"))]
control_stub!(control_rotary_switch);
#[cfg(not(feature = "dashboard"))]
control_stub!(control_knob);
#[cfg(not(feature = "dashboard"))]
control_stub!(control_push_button);
#[cfg(not(feature = "dashboard"))]
control_stub!(control_edit_field);

/// Draw the small-size icon fallback for dashboard widgets; returns true
/// when it handled drawing (block too small for a full live widget).
fn dashboard_icon_fallback(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    _font_scale: f32,
    _live_value: f64,
) -> bool {
    if !should_render_dashboard_icon(rect) {
        return false;
    }
    // No block-type branching: the catalog icon is the single small-size
    // representation for every dashboard widget.
    paint_dashboard_widget_icon(painter, block, rect, widget_palette(block));
    true
}

pub fn live_push_button(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    live_value: f64,
    _display_options: Option<&crate::live_values::LiveValueDisplayOptions>,
) {
    if dashboard_icon_fallback(painter, block, rect, font_scale, live_value) {
        return;
    }
    let palette = widget_palette(block);
    let inner = inner_rect(rect, 0.85);
    let label = prop(block, "ButtonText", &block.name);
    let (icon_color, off_value) = push_button_visuals(block, Some(live_value));
    let is_on = (live_value - off_value).abs() > f64::EPSILON;
    let fill = if is_on {
        icon_color.linear_multiply(0.2)
    } else {
        palette.bg_field
    };
    painter.rect_filled(inner, 4.0, fill);
    painter.rect_stroke(
        inner,
        4.0,
        Stroke::new(1.0_f32, palette.border),
        egui::StrokeKind::Inside,
    );
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.45);
    painter.text(
        inner.center(),
        Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(fsz),
        palette.text,
    );
}

pub fn live_slider_switch(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    live_value: f64,
    _display_options: Option<&crate::live_values::LiveValueDisplayOptions>,
) {
    if dashboard_icon_fallback(painter, block, rect, font_scale, live_value) {
        return;
    }
    let palette = widget_palette(block);
    paint_switch_visual(painter, rect, palette, live_value >= 0.5, false, font_scale);
}

pub fn live_radio_button_group(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    live_value: f64,
    _display_options: Option<&crate::live_values::LiveValueDisplayOptions>,
) {
    if dashboard_icon_fallback(painter, block, rect, font_scale, live_value) {
        return;
    }
    let palette = widget_palette(block);
    let inner = inner_rect(rect, 0.80);
    let labels = option_labels(block);
    let (fsz, row_h, header_h) = radio_group_metrics(rect, font_scale, labels.len());
    let font = egui::FontId::proportional(fsz);
    let group_name = prop(block, "ButtonGroupName", "Group");
    painter.text(
        Pos2::new(inner.left() + 4.0, inner.top() + 2.0),
        Align2::LEFT_TOP,
        group_name,
        font.clone(),
        palette.text,
    );
    let radio_r = safe_clamp_f32(fsz * 0.32, 3.0, row_h * 0.28);
    let y_start = inner.top() + header_h + 4.0;
    let selected = discrete_live_index(live_value, labels.len());
    for (index, label) in labels.iter().enumerate() {
        let y = y_start + index as f32 * row_h + row_h * 0.4;
        if y + radio_r > inner.bottom() {
            break; // Don't overflow
        }
        let cx = inner.left() + radio_r + 4.0;
        painter.circle_stroke(
            Pos2::new(cx, y),
            radio_r,
            Stroke::new(1.0_f32, palette.border),
        );
        if index == selected {
            painter.circle_filled(Pos2::new(cx, y), radio_r * 0.55, palette.accent);
        }
        painter.text(
            Pos2::new(cx + radio_r + 4.0, y),
            Align2::LEFT_CENTER,
            label,
            font.clone(),
            palette.text,
        );
    }
}

pub fn live_combo_box(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    live_value: f64,
    _display_options: Option<&crate::live_values::LiveValueDisplayOptions>,
) {
    if dashboard_icon_fallback(painter, block, rect, font_scale, live_value) {
        return;
    }
    let palette = widget_palette(block);
    let inner = inner_rect(rect, 0.80);
    let field_h = (inner.height() * 0.4).max(8.0);
    let field = Rect::from_min_max(
        Pos2::new(inner.left(), inner.center().y - field_h / 2.0),
        Pos2::new(inner.right(), inner.center().y + field_h / 2.0),
    );
    painter.rect_filled(field, 3.0, palette.bg_field);
    painter.rect_stroke(
        field,
        3.0,
        Stroke::new(1.0_f32, palette.border),
        egui::StrokeKind::Inside,
    );
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.35);
    let labels = option_labels(block);
    let label_count = labels.len().max(1);
    let label = combo_box_label(block, discrete_live_index(live_value, label_count));
    painter.text(
        Pos2::new(field.left() + 4.0, field.center().y),
        Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(fsz),
        palette.text,
    );
    let arrow_sz = (field_h * 0.3).max(3.0);
    let arrow_cx = field.right() - arrow_sz * 2.0;
    let arrow_cy = field.center().y;
    let pts = vec![
        Pos2::new(arrow_cx - arrow_sz, arrow_cy - arrow_sz * 0.5),
        Pos2::new(arrow_cx + arrow_sz, arrow_cy - arrow_sz * 0.5),
        Pos2::new(arrow_cx, arrow_cy + arrow_sz * 0.5),
    ];
    painter.add(egui::Shape::convex_polygon(pts, palette.text, Stroke::NONE));
}

pub fn live_checkbox(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    live_value: f64,
    _display_options: Option<&crate::live_values::LiveValueDisplayOptions>,
) {
    if dashboard_icon_fallback(painter, block, rect, font_scale, live_value) {
        return;
    }
    let palette = widget_palette(block);
    let inner = inner_rect(rect, 0.80);
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.35);
    let font = egui::FontId::proportional(fsz);
    let label = checkbox_label(block);
    let box_sz = (fsz * 1.1).max(6.0);
    let cx = inner.left() + box_sz / 2.0 + 2.0;
    let cy = inner.center().y;
    let check_rect = Rect::from_center_size(Pos2::new(cx, cy), Vec2::splat(box_sz));
    painter.rect_filled(check_rect, 2.0, palette.bg_field);
    painter.rect_stroke(
        check_rect,
        2.0,
        Stroke::new(1.0_f32, palette.border),
        egui::StrokeKind::Inside,
    );
    painter.text(
        Pos2::new(cx + box_sz / 2.0 + 4.0, cy),
        Align2::LEFT_CENTER,
        &label,
        font,
        palette.text,
    );
    if checkbox_state_from_value(block, live_value) {
        let left = cx - box_sz * 0.28;
        let mid = cx - box_sz * 0.05;
        let right = cx + box_sz * 0.30;
        painter.line_segment(
            [Pos2::new(left, cy), Pos2::new(mid, cy + box_sz * 0.22)],
            Stroke::new(1.5_f32, palette.accent_dark),
        );
        painter.line_segment(
            [
                Pos2::new(mid, cy + box_sz * 0.22),
                Pos2::new(right, cy - box_sz * 0.25),
            ],
            Stroke::new(1.5_f32, palette.accent_dark),
        );
    }
}

pub fn live_slider_or_linear_gauge(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    live_value: f64,
    _display_options: Option<&crate::live_values::LiveValueDisplayOptions>,
) {
    if dashboard_icon_fallback(painter, block, rect, font_scale, live_value) {
        return;
    }
    let palette = widget_palette(block);
    let inner = inner_rect(rect, 0.80);
    let cy = inner.center().y;
    let (scale_min, scale_max) = gauge_range(block);
    let track_h = if block.block_type == "SliderBlock" {
        paint_slider_visual(
            painter,
            block,
            rect,
            palette,
            font_scale,
            normalized_live_value(block, live_value),
        );
        0.0
    } else {
        let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.18);
        let font = egui::FontId::proportional(fsz);
        let bar_h = (inner.height() * 0.15).clamp(3.0, 10.0);
        let bar = Rect::from_min_max(
            Pos2::new(inner.left(), cy - bar_h / 2.0),
            Pos2::new(inner.right(), cy + bar_h / 2.0),
        );
        painter.rect_filled(bar, 2.0, Color32::from_rgb(220, 220, 225));
        painter.rect_stroke(
            bar,
            2.0,
            Stroke::new(1.0_f32, BORDER),
            egui::StrokeKind::Inside,
        );
        let n_ticks = 11;
        for index in 0..n_ticks {
            let tick_t = index as f32 / (n_ticks - 1) as f32;
            let x = inner.left() + tick_t * inner.width();
            let tick_len = if index % 5 == 0 { 4.0 } else { 2.5 };
            painter.line_segment(
                [
                    Pos2::new(x, bar.bottom() + 1.0),
                    Pos2::new(x, bar.bottom() + 1.0 + tick_len),
                ],
                Stroke::new(1.0_f32, TEXT_DARK),
            );
        }
        let label_y = bar.bottom() + 7.0;
        painter.text(
            Pos2::new(inner.left(), label_y),
            Align2::LEFT_TOP,
            format_scale_value(scale_min),
            font.clone(),
            TEXT_DARK,
        );
        painter.text(
            Pos2::new(inner.right(), label_y),
            Align2::RIGHT_TOP,
            format_scale_value(scale_max),
            font,
            TEXT_DARK,
        );
        bar_h
    };
    if block.block_type == "SliderBlock" {
        return;
    }
    let fraction = normalized_live_value(block, live_value);
    let fill_rect = Rect::from_min_max(
        Pos2::new(inner.left(), cy - track_h / 2.0),
        Pos2::new(inner.left() + inner.width() * fraction, cy + track_h / 2.0),
    );
    painter.rect_filled(fill_rect, 2.0, ACCENT);
    let thumb_x = inner.left() + inner.width() * fraction;
    if block.block_type == "SliderBlock" {
        let thumb_w = (inner.width() * 0.04).clamp(4.0, 10.0);
        let thumb =
            Rect::from_center_size(Pos2::new(thumb_x, cy), Vec2::new(thumb_w, track_h * 4.0));
        painter.rect_filled(thumb, 2.0, ACCENT_DARK);
    } else {
        let tri_sz = track_h * 0.8;
        let pts = vec![
            Pos2::new(thumb_x, cy - track_h / 2.0 - 1.0),
            Pos2::new(thumb_x - tri_sz, cy - track_h / 2.0 - 1.0 - tri_sz),
            Pos2::new(thumb_x + tri_sz, cy - track_h / 2.0 - 1.0 - tri_sz),
        ];
        painter.add(egui::Shape::convex_polygon(pts, ACCENT, Stroke::NONE));
    }
}

pub fn live_field_or_display(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    live_value: f64,
    display_options: Option<&crate::live_values::LiveValueDisplayOptions>,
) {
    if dashboard_icon_fallback(painter, block, rect, font_scale, live_value) {
        return;
    }
    let inner = inner_rect(
        rect,
        if block.block_type == "DisplayBlock" {
            0.85
        } else {
            0.80
        },
    );
    let field_h = if block.block_type == "DisplayBlock" {
        (inner.height() * 0.55).clamp(10.0, 40.0)
    } else {
        (inner.height() * 0.45).clamp(10.0, 30.0)
    };
    let field = Rect::from_min_max(
        Pos2::new(inner.left(), inner.center().y - field_h / 2.0),
        Pos2::new(inner.right(), inner.center().y + field_h / 2.0),
    );
    let fill = if block.block_type == "DisplayBlock" {
        Color32::from_rgb(240, 245, 240)
    } else {
        Color32::TRANSPARENT
    };
    if fill != Color32::TRANSPARENT {
        painter.rect_filled(field, 3.0, fill);
    }
    painter.rect_stroke(
        field,
        3.0,
        Stroke::new(1.0_f32, BORDER),
        egui::StrokeKind::Inside,
    );
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.42);
    painter.text(
        field.center(),
        Align2::CENTER_CENTER,
        format_dashboard_scalar_with_options(live_value, display_options),
        if block.block_type == "DisplayBlock" {
            egui::FontId::monospace(fsz)
        } else {
            egui::FontId::proportional(fsz)
        },
        TEXT_DARK,
    );
}

pub fn live_toggle_switch(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    live_value: f64,
    _display_options: Option<&crate::live_values::LiveValueDisplayOptions>,
) {
    if dashboard_icon_fallback(painter, block, rect, font_scale, live_value) {
        return;
    }
    let palette = widget_palette(block);
    paint_switch_visual(painter, rect, palette, live_value >= 0.5, true, font_scale);
}

pub fn live_radial_gauge(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    live_value: f64,
    _display_options: Option<&crate::live_values::LiveValueDisplayOptions>,
) {
    if dashboard_icon_fallback(painter, block, rect, font_scale, live_value) {
        return;
    }
    let inner = inner_rect(rect, 0.80);
    let fraction = if block.block_type == "RotarySwitchBlock" {
        let labels = option_labels(block);
        let idx = discrete_live_index(live_value, labels.len());
        let denom = labels.len().saturating_sub(1).max(1) as f32;
        idx as f32 / denom
    } else {
        normalized_live_value(block, live_value)
    };
    let (cx, cy, radius, start_angle, end_angle, stroke, color) = match block.block_type.as_str() {
        "KnobBlock" | "RotarySwitchBlock" => (
            inner.center().x,
            inner.center().y + inner.height() * 0.05,
            (inner.width().min(inner.height()) * 0.35).max(8.0),
            5.0 * PI / 4.0,
            -PI / 4.0,
            2.5_f32,
            ACCENT_DARK,
        ),
        "CircularGaugeBlock" => (
            inner.center().x,
            inner.center().y + inner.height() * 0.05,
            (inner.width().min(inner.height()) * 0.40).max(10.0),
            5.0 * PI / 4.0,
            -PI / 4.0,
            2.0_f32,
            NEEDLE_RED,
        ),
        "SemiCircularGaugeBlock" => (
            inner.center().x,
            inner.bottom() - inner.height() * 0.15,
            (inner.width() * 0.40).min(inner.height() * 0.7).max(10.0),
            PI,
            0.0,
            2.0_f32,
            NEEDLE_RED,
        ),
        _ => (
            inner.left() + inner.width() * 0.1,
            inner.bottom() - inner.height() * 0.1,
            (inner.width() * 0.7).min(inner.height() * 0.7).max(10.0),
            PI / 2.0,
            0.0,
            2.0_f32,
            NEEDLE_RED,
        ),
    };
    let fsz = font_for_rect(rect, font_scale).min(inner.height() * 0.12);
    let font = egui::FontId::proportional(fsz);
    let (scale_min, scale_max) = gauge_range(block);
    match block.block_type.as_str() {
        "KnobBlock" => {
            painter.circle_filled(Pos2::new(cx, cy), radius, Color32::from_rgb(220, 220, 225));
            painter.circle_stroke(Pos2::new(cx, cy), radius, Stroke::new(1.5_f32, BORDER));
            painter.circle_filled(
                Pos2::new(cx, cy),
                radius * 0.7,
                Color32::from_rgb(235, 235, 238),
            );
            let tick_r_outer = radius + 4.0;
            let tick_r_inner = radius + 1.0;
            let n_ticks = 11;
            for index in 0..n_ticks {
                let tick_t = index as f32 / (n_ticks - 1) as f32;
                let tick_angle = start_angle + tick_t * (end_angle - start_angle);
                let outer = Pos2::new(
                    cx + tick_r_outer * tick_angle.cos(),
                    cy - tick_r_outer * tick_angle.sin(),
                );
                let inner_p = Pos2::new(
                    cx + tick_r_inner * tick_angle.cos(),
                    cy - tick_r_inner * tick_angle.sin(),
                );
                painter.line_segment([inner_p, outer], Stroke::new(1.0_f32, BORDER));
            }
            let label_r = tick_r_outer + fsz;
            painter.text(
                Pos2::new(
                    cx + label_r * start_angle.cos(),
                    cy - label_r * start_angle.sin(),
                ),
                Align2::CENTER_CENTER,
                format_scale_value(scale_min),
                font.clone(),
                TEXT_DARK,
            );
            painter.text(
                Pos2::new(
                    cx + label_r * end_angle.cos(),
                    cy - label_r * end_angle.sin(),
                ),
                Align2::CENTER_CENTER,
                format_scale_value(scale_max),
                font.clone(),
                TEXT_DARK,
            );
        }
        "RotarySwitchBlock" => {
            painter.circle_filled(Pos2::new(cx, cy), radius, Color32::from_rgb(210, 215, 220));
            painter.circle_stroke(Pos2::new(cx, cy), radius, Stroke::new(1.5_f32, BORDER));
            let labels = option_labels(block);
            let selected = discrete_live_index(live_value, labels.len());
            let mark_r = radius + 4.0;
            let label_r = radius + fsz * 1.2 + 4.0;
            let steps = labels.len().saturating_sub(1).max(1) as f32;
            for (index, label) in labels.iter().enumerate() {
                let tick_t = index as f32 / steps;
                let angle = start_angle + tick_t * (end_angle - start_angle);
                let mark_end = Pos2::new(cx + mark_r * angle.cos(), cy - mark_r * angle.sin());
                let mark_start = Pos2::new(
                    cx + (mark_r - 3.0) * angle.cos(),
                    cy - (mark_r - 3.0) * angle.sin(),
                );
                let mark_color = if index == selected {
                    ACCENT_DARK
                } else {
                    BORDER
                };
                painter.line_segment([mark_start, mark_end], Stroke::new(1.5_f32, mark_color));
                painter.text(
                    Pos2::new(cx + label_r * angle.cos(), cy - label_r * angle.sin()),
                    Align2::CENTER_CENTER,
                    label,
                    font.clone(),
                    TEXT_DARK,
                );
            }
        }
        "CircularGaugeBlock" => {
            painter.circle_stroke(Pos2::new(cx, cy), radius, Stroke::new(2.0_f32, BORDER));
            let n_ticks = 11;
            for index in 0..n_ticks {
                let tick_t = index as f32 / (n_ticks - 1) as f32;
                let tick_angle = start_angle + tick_t * (end_angle - start_angle);
                let is_major = index % 2 == 0;
                let r_in = if is_major { radius - 4.0 } else { radius - 2.5 };
                let p1 = Pos2::new(cx + r_in * tick_angle.cos(), cy - r_in * tick_angle.sin());
                let p2 = Pos2::new(
                    cx + radius * tick_angle.cos(),
                    cy - radius * tick_angle.sin(),
                );
                painter.line_segment(
                    [p1, p2],
                    Stroke::new(if is_major { 1.5_f32 } else { 1.0_f32 }, TEXT_DARK),
                );
                if is_major {
                    let val = scale_min + (scale_max - scale_min) * tick_t as f64;
                    let label_r = radius + fsz * 0.8;
                    painter.text(
                        Pos2::new(
                            cx + label_r * tick_angle.cos(),
                            cy - label_r * tick_angle.sin(),
                        ),
                        Align2::CENTER_CENTER,
                        format_scale_value(val),
                        font.clone(),
                        TEXT_DARK,
                    );
                }
            }
        }
        "SemiCircularGaugeBlock" => {
            let n_ticks = 11;
            for index in 0..n_ticks {
                let tick_t = index as f32 / (n_ticks - 1) as f32;
                let tick_angle = start_angle + tick_t * (end_angle - start_angle);
                let is_major = index % 2 == 0;
                let r_in = if is_major { radius - 4.0 } else { radius - 2.5 };
                let p1 = Pos2::new(cx + r_in * tick_angle.cos(), cy - r_in * tick_angle.sin());
                let p2 = Pos2::new(
                    cx + radius * tick_angle.cos(),
                    cy - radius * tick_angle.sin(),
                );
                painter.line_segment(
                    [p1, p2],
                    Stroke::new(if is_major { 1.5_f32 } else { 1.0_f32 }, TEXT_DARK),
                );
                if is_major {
                    let val = scale_min + (scale_max - scale_min) * tick_t as f64;
                    let label_r = radius + fsz * 0.8;
                    painter.text(
                        Pos2::new(
                            cx + label_r * tick_angle.cos(),
                            cy - label_r * tick_angle.sin(),
                        ),
                        Align2::CENTER_CENTER,
                        format_scale_value(val),
                        font.clone(),
                        TEXT_DARK,
                    );
                }
            }
            painter.line_segment(
                [Pos2::new(cx - radius, cy), Pos2::new(cx + radius, cy)],
                Stroke::new(1.0_f32, BORDER),
            );
        }
        "QuarterGaugeBlock" => {
            let n_ticks = 6;
            for index in 0..n_ticks {
                let tick_t = index as f32 / (n_ticks - 1) as f32;
                let tick_angle = start_angle + tick_t * (end_angle - start_angle);
                let p1 = Pos2::new(
                    cx + (radius - 3.5) * tick_angle.cos(),
                    cy - (radius - 3.5) * tick_angle.sin(),
                );
                let p2 = Pos2::new(
                    cx + radius * tick_angle.cos(),
                    cy - radius * tick_angle.sin(),
                );
                painter.line_segment([p1, p2], Stroke::new(1.5_f32, TEXT_DARK));
                let label_r = radius + fsz * 0.8;
                let val = scale_min + (scale_max - scale_min) * tick_t as f64;
                painter.text(
                    Pos2::new(
                        cx + label_r * tick_angle.cos(),
                        cy - label_r * tick_angle.sin(),
                    ),
                    Align2::CENTER_CENTER,
                    format_scale_value(val),
                    font.clone(),
                    TEXT_DARK,
                );
            }
        }
        _ => {}
    }
    let angle = start_angle + fraction * (end_angle - start_angle);
    let needle_end = Pos2::new(
        cx + (radius * 0.85) * angle.cos(),
        cy - (radius * 0.85) * angle.sin(),
    );
    painter.line_segment([Pos2::new(cx, cy), needle_end], Stroke::new(stroke, color));
    painter.circle_filled(Pos2::new(cx, cy), radius * 0.08, color);
}

pub fn live_rocker_switch(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    live_value: f64,
    _display_options: Option<&crate::live_values::LiveValueDisplayOptions>,
) {
    if dashboard_icon_fallback(painter, block, rect, font_scale, live_value) {
        return;
    }
    let palette = widget_palette(block);
    paint_rocker_switch_visual(painter, rect, palette, live_value >= 0.5, font_scale);
}

pub fn live_lamp(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    live_value: f64,
    _display_options: Option<&crate::live_values::LiveValueDisplayOptions>,
) {
    if dashboard_icon_fallback(painter, block, rect, font_scale, live_value) {
        return;
    }
    let inner = inner_rect(rect, 0.80);
    let radius = (inner.width().min(inner.height()) * 0.35).max(6.0);
    let center = inner.center();
    let color = lamp_color_for_value(block, Some(live_value));
    painter.circle_filled(center, radius, color);
    painter.circle_stroke(center, radius, Stroke::new(1.5_f32, BORDER));
}

fn format_dashboard_scalar_with_options(
    value: f64,
    display_options: Option<&crate::live_values::LiveValueDisplayOptions>,
) -> String {
    let mut entry = crate::live_values::LiveValueEntry::new(crate::live_values::LiveValue::new(
        vec![1],
        crate::live_values::LiveValueList::Float64(vec![value]),
    ));
    if let Some(options) = display_options {
        entry = entry.with_display(options.clone());
    }
    entry.formatted_text()
}
