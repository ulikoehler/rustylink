//! Interactive egui controls backing the live widgets.

#![cfg(feature = "egui")]

#[cfg(feature = "dashboard")]
use super::live::{
    live_push_button, live_radial_gauge, live_radio_button_group, live_rocker_switch,
    live_slider_or_linear_gauge, live_slider_switch, live_toggle_switch,
};
#[cfg(feature = "dashboard")]
use super::painters::paint_dashboard_widget_icon;
#[cfg(feature = "dashboard")]
use super::style::{
    apply_dashboard_widget_style, apply_dashboard_widget_style_with_body_size,
    dashboard_arc_fraction, dashboard_knob_geometry, dashboard_rotary_geometry, inner_rect,
    paint_dashboard_widget_background, radio_group_metrics, should_render_dashboard_icon,
    widget_palette,
};
#[cfg(feature = "dashboard")]
use super::values::{
    checkbox_label, dashboard_control_storage_key, dashboard_input_control_kind,
    discrete_option_items, discrete_selected_index, gauge_range, option_label_value_pairs,
    option_labels,
};
#[cfg(feature = "dashboard")]
use crate::egui_app::{DashboardControlValue, state::SubsystemApp};
use crate::model::Block;
use eframe::egui::{self, Rect};
#[cfg(feature = "dashboard")]
use eframe::egui::{Pos2, Stroke};

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
#[doc(hidden)]
pub fn dashboard_scalar_value_from_pointer(
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
        .unwrap_or_else(|| crate::egui_app::ui::block_text::format_live_scalar_csv(live_value));
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
                    .align(egui::Align2::CENTER_CENTER)
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
