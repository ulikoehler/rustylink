use super::colors::{
    area_annotation_border, area_annotation_fill, block_fill_color, block_has_model_color,
    contrast_color, monochrome_block_border, monochrome_line_color,
};
use super::corner_ops;
use super::helpers::record_interaction;
use super::line_coloring;
use super::signal_routing;
use super::types::{ClickAction, UpdateResponse};
use super::view_transform;
use super::zoom_controls::show_zoom_controls;
use crate::editor::operations;
#[cfg(feature = "dashboard")]
use crate::egui_app::DashboardControlValue;
use crate::egui_app::geometry::{parse_block_rect, parse_rect_str};
use crate::egui_app::navigation::resolve_subsystem_by_vec;
use crate::egui_app::render::wrap_text_to_max_width;
use crate::egui_app::render::{ComputedPortYCoordinates, PortLabelMaxWidths};
use crate::egui_app::state::ViewerDragState;
use crate::egui_app::state::{
    LiveTooltipEntry, LiveTooltipKind, SubsystemApp, resolve_subsystem_by_vec_mut,
};
use crate::egui_app::text::highlight_query_job;
use crate::egui_app::{get_block_type_cfg, port_label_defined_name, port_label_display_name};
use eframe::egui::{self, Align2, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use egui_phosphor_icons::icons::{ARROW_UP, FOLDER_OPEN};
use std::collections::{BTreeSet, HashMap};

/// Shallow clone of a block that prunes the expensive subsystem tree.
///
/// Rendering a subsystem needs its *direct* children – Simulink previews a
/// subsystem from the boundary ports and conditional-execution port blocks it
/// contains – but never the nested systems below them, which are what make a
/// deep clone expensive.
fn shallow_clone_block(b: &crate::model::Block) -> crate::model::Block {
    let mut clone = b.clone();
    if let Some(system) = clone.subsystem.as_deref_mut() {
        system.lines.clear();
        system.annotations.clear();
        for child in &mut system.blocks {
            child.subsystem = None;
        }
    }
    clone
}

pub(crate) fn format_live_scalar_csv(value: f64) -> String {
    let mut text = format!("{value:.6}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    if text == "-0" { "0".to_string() } else { text }
}

fn format_constant_value_for_display(raw: &str) -> String {
    let trimmed = raw.trim();
    let Ok(value) = trimmed.parse::<f64>() else {
        return trimmed.to_string();
    };
    if value != 0.0 && value.abs() < 0.1 {
        format!("{value:.2e}")
    } else {
        format!("{value:.2}")
    }
}

const MIN_BLOCK_VALUE_FONT_FACTOR: f32 = 0.45;

fn ellipsize_text_to_width(
    painter: &egui::Painter,
    text: &str,
    font_id: &egui::FontId,
    max_width: f32,
    color: Color32,
) -> String {
    let measure = |value: &str| {
        painter
            .layout_no_wrap(value.to_string(), font_id.clone(), color)
            .size()
            .x
    };

    if measure(text) <= max_width {
        return text.to_string();
    }

    let ellipsis = "...";
    let mut trimmed = text.to_string();
    while !trimmed.is_empty() {
        trimmed.pop();
        let candidate = format!("{}{}", trimmed.trim_end(), ellipsis);
        if measure(&candidate) <= max_width {
            return candidate;
        }
    }

    ellipsis.to_string()
}

fn paint_fitted_centered_text(
    painter: &egui::Painter,
    rect: Rect,
    text: &str,
    color: Color32,
    desired_font_px: f32,
    min_font_px: f32,
    monospace: bool,
) {
    let inner = rect.shrink2(Vec2::new(4.0, 4.0));
    if inner.width() <= 1.0 || inner.height() <= 1.0 || text.trim().is_empty() {
        return;
    }

    let mut current_font_px = desired_font_px.max(min_font_px).max(1.0);

    let (best_lines, best_font_px) = loop {
        let font_id = if monospace {
            egui::FontId::monospace(current_font_px)
        } else {
            egui::FontId::proportional(current_font_px)
        };
        let line_height = (current_font_px * 1.15).max(1.0);
        let max_lines = ((inner.height() / line_height).floor() as usize).max(1);
        let mut lines = wrap_text_to_max_width(painter, text, font_id.clone(), inner.width());
        if lines.is_empty() {
            return;
        }
        if lines.len() > max_lines {
            lines.truncate(max_lines);
        }
        for line in &mut lines {
            *line = ellipsize_text_to_width(painter, line, &font_id, inner.width(), color);
        }

        let total_h = lines.len() as f32 * line_height;
        if total_h <= inner.height() + 0.5 || current_font_px <= min_font_px + f32::EPSILON {
            break (lines, current_font_px);
        }

        let next_font_px = (current_font_px * 0.9).max(min_font_px);
        if (next_font_px - current_font_px).abs() < f32::EPSILON {
            break (lines, current_font_px);
        }
        current_font_px = next_font_px;
    };

    let font_id = if monospace {
        egui::FontId::monospace(best_font_px)
    } else {
        egui::FontId::proportional(best_font_px)
    };
    let line_height = (best_font_px * 1.15).max(1.0);
    let total_h = best_lines.len() as f32 * line_height;
    let mut y = inner.center().y - total_h * 0.5 + line_height * 0.5;
    for line in &best_lines {
        painter.text(
            Pos2::new(inner.center().x, y),
            Align2::CENTER_CENTER,
            line,
            font_id.clone(),
            color,
        );
        y += line_height;
    }
}

fn show_pointer_tooltip_text(ui: &egui::Ui, id: egui::Id, tooltip: &str) {
    let _ = egui::Tooltip::always_open(
        ui.ctx().clone(),
        ui.layer_id(),
        id,
        egui::PopupAnchor::Pointer,
    )
    .gap(12.0)
    .show(|ui| {
        ui.label(tooltip);
    });
}

fn show_pointer_tooltip_entries(ui: &egui::Ui, id: egui::Id, entries: &[LiveTooltipEntry]) {
    let _ = egui::Tooltip::always_open(
        ui.ctx().clone(),
        ui.layer_id(),
        id,
        egui::PopupAnchor::Pointer,
    )
    .gap(12.0)
    .show(|ui| {
        for entry in entries {
            let (suffix, suffix_color) = match entry.kind {
                LiveTooltipKind::Signal => ("S", Color32::from_rgb(80, 200, 120)),
                LiveTooltipKind::Parameter => ("P", Color32::from_rgb(64, 140, 255)),
            };
            ui.horizontal(|ui| {
                ui.label(&entry.datafield_name);
                ui.colored_label(suffix_color, RichText::new(suffix).strong());
                ui.label(":");
                ui.monospace(&entry.formatted_value);
            });
        }
    });
}

fn format_block_value_for_display(block: &crate::model::Block, raw: &str) -> String {
    if block.block_type == "Constant" {
        raw.trim().to_string()
    } else {
        raw.to_string()
    }
}

#[cfg(feature = "dashboard")]
fn block_live_text(app: &SubsystemApp, block: &crate::model::Block) -> Option<String> {
    app.live_block_values
        .get(&app.live_value_key_for_block(block))
        .map(crate::live_values::LiveValueEntry::formatted_text)
}

#[cfg(not(feature = "dashboard"))]
fn block_live_text(_app: &SubsystemApp, _block: &crate::model::Block) -> Option<String> {
    None
}

fn toggle_manual_switch_setting(
    app: &mut SubsystemApp,
    block: &crate::model::Block,
) -> Option<bool> {
    let block_sid = block.sid.as_ref()?;
    let path = app.path.clone();
    let system = resolve_subsystem_by_vec_mut(&mut app.root, &path)?;
    let live_block = system
        .blocks
        .iter_mut()
        .find(|candidate| candidate.sid.as_ref() == Some(block_sid))?;

    let enabled = !matches!(live_block.current_setting.as_deref(), Some("1"));
    live_block.current_setting = Some(if enabled { "1" } else { "0" }.to_string());
    app.view_cache.invalidate();
    Some(enabled)
}

fn uses_live_value_text(block_type: &str) -> bool {
    matches!(block_type, "Display" | "Constant")
}

fn should_render_live_text(live_mode_enabled: bool, block_type: &str) -> bool {
    live_mode_enabled && uses_live_value_text(block_type)
}

fn should_use_mask_display(block: &crate::model::Block) -> bool {
    block.mask.is_some()
        && block
            .mask_display_text
            .as_ref()
            .is_some_and(|text| !text.trim().is_empty())
}

fn line_has_testpoint(targets: &[crate::connection_targets::ConnectionTarget]) -> bool {
    targets.iter().any(|target| target.testpoint)
}

fn resolved_line_label(
    line: &crate::model::Line,
    targets: &[crate::connection_targets::ConnectionTarget],
) -> Option<String> {
    let explicit_name = line
        .name
        .as_ref()
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .map(str::to_string);
    if explicit_name.is_some() {
        return explicit_name;
    }

    if is_bus_line(targets) || is_mux_line(targets) {
        return None;
    }

    targets
        .iter()
        .filter(|target| target.signals_only)
        .filter(|target| {
            !matches!(
                target.origin,
                crate::connection_targets::ConnectionTargetOrigin::Mux
                    | crate::connection_targets::ConnectionTargetOrigin::BusCreator
            )
        })
        .find_map(|target| target.signal_name.clone())
}

fn is_bus_line(targets: &[crate::connection_targets::ConnectionTarget]) -> bool {
    let signal_resolves = targets
        .iter()
        .filter_map(|target| match &target.resolve {
            Some(crate::connection_targets::ConnectionTargetResolve::Signal(signal_name)) => {
                Some(signal_name.clone())
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    signal_resolves.len() > 1
}

fn is_mux_line(targets: &[crate::connection_targets::ConnectionTarget]) -> bool {
    targets.iter().any(|target| {
        matches!(
            target.origin,
            crate::connection_targets::ConnectionTargetOrigin::Mux
        )
    })
}

fn line_stroke_width(
    targets: &[crate::connection_targets::ConnectionTarget],
    is_selected: bool,
) -> f32 {
    let base_width = if is_bus_line(targets) { 3.25 } else { 2.0 };
    if is_selected {
        base_width + 1.5
    } else {
        base_width
    }
}

fn line_testpoint_marker_position(points: &[Pos2]) -> Option<Pos2> {
    let start = *points.first()?;
    for next in points.iter().skip(1) {
        let dx = next.x - start.x;
        let dy = next.y - start.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len > 1.0 {
            let offset = len.min(18.0);
            return Some(Pos2::new(
                start.x + dx / len * offset,
                start.y + dy / len * offset,
            ));
        }
    }
    None
}

fn draw_line_testpoint_marker(painter: &egui::Painter, center: Pos2, color: Color32) {
    let marker_rect = Rect::from_center_size(center, Vec2::new(20.0, 12.0));
    painter.rect_filled(
        marker_rect,
        4.0,
        Color32::from_rgba_unmultiplied(255, 255, 255, 224),
    );
    painter.rect_stroke(
        marker_rect,
        4.0,
        Stroke::new(1.0_f32, color),
        egui::StrokeKind::Inside,
    );
    painter.text(
        marker_rect.center(),
        Align2::CENTER_CENTER,
        "TP",
        egui::FontId::proportional(9.0),
        color,
    );
}

pub(crate) fn manual_switch_setting_from_live_value(value: f64) -> &'static str {
    if value >= 0.5 { "1" } else { "0" }
}

#[cfg(feature = "dashboard")]
fn block_live_value(app: &SubsystemApp, block: &crate::model::Block) -> Option<f64> {
    app.live_block_values
        .get(&app.live_value_key_for_block(block))
        .and_then(crate::live_values::LiveValueEntry::first_f64)
        .or_else(|| dashboard_live_value(app, block))
}

fn branch_hits_sid(branch: &crate::model::Branch, sid: &str) -> bool {
    if branch.dst.as_ref().is_some_and(|dst| dst.sid == sid) {
        return true;
    }
    branch
        .branches
        .iter()
        .any(|child| branch_hits_sid(child, sid))
}

#[cfg(feature = "dashboard")]
fn first_input_signal_name(
    block: &crate::model::Block,
    lines: &[crate::model::Line],
) -> Option<String> {
    let sid = block.sid.as_deref()?;
    lines.iter().find_map(|line| {
        let direct = line.dst.as_ref().is_some_and(|dst| dst.sid == sid);
        let branched = line
            .branches
            .iter()
            .any(|branch| branch_hits_sid(branch, sid));
        if direct || branched {
            line.name.clone().filter(|name| !name.trim().is_empty())
        } else {
            None
        }
    })
}

#[cfg(feature = "dashboard")]
fn scope_title_for_block(block: &crate::model::Block, lines: &[crate::model::Line]) -> String {
    if let Some(crate::model::DashboardBinding::SignalSpec { signal_name, .. }) =
        block.dashboard_binding.as_ref()
        && !signal_name.trim().is_empty()
    {
        return signal_name.clone();
    }

    first_input_signal_name(block, lines).unwrap_or_else(|| block.name.clone())
}

#[cfg(feature = "dashboard")]
fn update_scope_live_sample(
    app: &mut SubsystemApp,
    block: &crate::model::Block,
    lines: &[crate::model::Line],
) {
    if !matches!(block.block_type.as_str(), "Scope" | "DashboardScope") {
        return;
    }

    let Some(value) = block_live_value(app, block) else {
        return;
    };

    let scope_key = app.scope_key_for_block(block);
    let scope_title = scope_title_for_block(block, lines);
    let mut scopes = app.scope_instances.lock().unwrap();
    let scope = scopes.entry(scope_key.clone()).or_insert_with(|| {
        crate::egui_app::scope_widget::MiniScope::new((
            app.instance_id,
            "scope",
            scope_key.as_str(),
        ))
    });
    scope.set_signal_name(scope_title);
    scope.push_sample(value);
}

#[cfg(not(feature = "dashboard"))]
fn block_live_value(_app: &SubsystemApp, _block: &crate::model::Block) -> Option<f64> {
    None
}

/// Live value fed to a block's unified `LiveRendererFn`.
///
/// Interactive dashboard controls (those carrying a `dashboard_control` kind)
/// resolve a value through the selector-aware [`dashboard_widget_value`] so the
/// widget reflects/edits the bound parameter; every other block uses the plain
/// signal value.
#[cfg(feature = "dashboard")]
fn live_block_value_for_renderer(
    app: &SubsystemApp,
    block: &crate::model::Block,
    def: &crate::simulink_libraries::types::SimulinkBlockDefinition,
) -> Option<f64> {
    if def.dashboard_control.is_some() {
        Some(dashboard_widget_value(app, block))
    } else {
        block_live_value(app, block)
    }
}

#[cfg(not(feature = "dashboard"))]
fn live_block_value_for_renderer(
    app: &SubsystemApp,
    block: &crate::model::Block,
    _def: &crate::simulink_libraries::types::SimulinkBlockDefinition,
) -> Option<f64> {
    block_live_value(app, block)
}

pub(crate) fn update_internal(
    app: &mut SubsystemApp,
    ui: &mut egui::Ui,
    enable_context_menus: bool,
) -> UpdateResponse {
    let mut interaction = UpdateResponse::None;
    let mut navigate_to: Option<Vec<String>> = None;
    let mut clear_search = false;
    let path_snapshot = app.path.clone();

    egui::Panel::top(app.egui_id("top_panel")).show(ui, |ui| {
        ui.horizontal(|ui| {
            let up_label = egui::RichText::new(format!("{} Up", ARROW_UP.as_str()));
            let up = ui.add_enabled(!path_snapshot.is_empty(), egui::Button::new(up_label));
            if up.clicked() {
                let mut p = path_snapshot.clone();
                p.pop();
                navigate_to = Some(p);
            }
            ui.separator();
            ui.label(RichText::new("Path:").strong());
            if ui.link("Root").clicked() {
                navigate_to = Some(Vec::new());
            }
            for (i, name) in path_snapshot.iter().enumerate() {
                ui.label("/");
                if ui.link(name).clicked() {
                    navigate_to = Some(path_snapshot[..=i].to_vec());
                }
            }
        });
        ui.horizontal(|ui| {
            let resp = ui.add(
                egui::TextEdit::singleline(&mut app.search_query)
                    .hint_text("Search subsystems by name…"),
            );
            if resp.changed() {
                app.update_search_matches();
            }

            ui.separator();
            let live_label = if app.live_mode_enabled {
                "Live: On"
            } else {
                "Live: Off"
            };
            if ui
                .selectable_label(app.live_mode_enabled, live_label)
                .clicked()
            {
                app.live_mode_enabled = !app.live_mode_enabled;
                ui.ctx().request_repaint();
            }

            ui.separator();
            let move_label = if app.move_mode_enabled {
                "Edit: On"
            } else {
                "Edit: Off"
            };
            if ui
                .selectable_label(app.move_mode_enabled, move_label)
                .clicked()
            {
                app.move_mode_enabled = !app.move_mode_enabled;
            }
            if app.move_mode_enabled {
                let undo_btn = egui::Button::new(format!(
                    "{} Undo",
                    egui_phosphor_icons::icons::ARROW_COUNTER_CLOCKWISE.as_str()
                ));
                let redo_btn = egui::Button::new(format!(
                    "{} Redo",
                    egui_phosphor_icons::icons::ARROW_CLOCKWISE.as_str()
                ));
                if ui
                    .add_enabled(app.viewer_history.can_undo(), undo_btn)
                    .clicked()
                {
                    let path = app.path.clone();
                    if let Some(system) = resolve_subsystem_by_vec_mut(&mut app.root, &path) {
                        app.viewer_history.undo(system);
                    }
                    app.layout_dirty = true;
                    app.view_cache.invalidate();
                }
                if ui
                    .add_enabled(app.viewer_history.can_redo(), redo_btn)
                    .clicked()
                {
                    let path = app.path.clone();
                    if let Some(system) = resolve_subsystem_by_vec_mut(&mut app.root, &path) {
                        app.viewer_history.redo(system);
                    }
                    app.layout_dirty = true;
                    app.view_cache.invalidate();
                }
            }
            let save_label = if app.layout_dirty {
                "Save layout*"
            } else {
                "Save layout"
            };
            if ui.button(save_label).clicked() {
                let mut dialog = rfd::FileDialog::new()
                    .add_filter("rustylink layout", &["json"])
                    .set_title("Save rustylink layout");
                if let Some(default_path) = app.layout_file_path.as_ref() {
                    let default_path = std::path::Path::new(default_path.as_str());
                    if let Some(parent) = default_path.parent() {
                        dialog = dialog.set_directory(parent);
                    }
                    if let Some(file_name) = default_path.file_name().and_then(|name| name.to_str())
                    {
                        dialog = dialog.set_file_name(file_name);
                    }
                }

                if let Some(path) = dialog.save_file() {
                    match camino::Utf8PathBuf::from_path_buf(path) {
                        Ok(path) => match app.save_layout_to_path(path) {
                            Ok(()) => app.show_notification("Layout saved", 3000),
                            Err(err) => {
                                app.show_notification(format!("Save layout failed: {}", err), 5000)
                            }
                        },
                        Err(path) => app.show_notification(
                            format!(
                                "Save layout failed: selected path is not valid UTF-8 ({})",
                                path.display()
                            ),
                            5000,
                        ),
                    }
                }
            }
            if ui.button("Load layout").clicked() {
                let mut dialog = rfd::FileDialog::new()
                    .add_filter("rustylink layout", &["json"])
                    .set_title("Load rustylink layout");
                if let Some(default_path) = app.layout_file_path.as_ref() {
                    let default_path = std::path::Path::new(default_path.as_str());
                    if let Some(parent) = default_path.parent() {
                        dialog = dialog.set_directory(parent);
                    }
                    if let Some(file_name) = default_path.file_name().and_then(|name| name.to_str())
                    {
                        dialog = dialog.set_file_name(file_name);
                    }
                }

                if let Some(path) = dialog.pick_file() {
                    match camino::Utf8PathBuf::from_path_buf(path) {
                        Ok(path) => match app.load_layout_from_path(path) {
                            Ok(()) => app.show_notification("Layout loaded", 3000),
                            Err(err) => {
                                app.show_notification(format!("Load layout failed: {}", err), 5000)
                            }
                        },
                        Err(path) => app.show_notification(
                            format!(
                                "Load layout failed: selected path is not valid UTF-8 ({})",
                                path.display()
                            ),
                            5000,
                        ),
                    }
                }
            }
            if ui.button("Restore layout").clicked() {
                match app.restore_original_layout() {
                    Ok(()) => app.show_notification("Layout restored", 3000),
                    Err(err) => {
                        app.show_notification(format!("Restore layout failed: {}", err), 5000)
                    }
                }
            }

            ui.separator();
            // Open the folder containing the source model in the OS file explorer
            if ui
                .add_enabled(
                    app.source_model_path.is_some(),
                    egui::Button::new(format!("{} Open folder", FOLDER_OPEN.as_str())),
                )
                .on_hover_text("Open the folder containing this model in the file explorer")
                .clicked()
                && let Some(model_path) = app.source_model_path.as_ref()
            {
                let dir = model_path.parent().unwrap_or(model_path.as_path());
                #[cfg(target_os = "linux")]
                let cmd = "xdg-open";
                #[cfg(target_os = "macos")]
                let cmd = "open";
                #[cfg(target_os = "windows")]
                let cmd = "explorer";
                if let Err(err) = std::process::Command::new(cmd).arg(dir.as_str()).spawn() {
                    app.show_notification(format!("Failed to open folder: {err}"), 5000);
                }
            }

            // "Less color" and "Dark" toggles live in the floating zoom-controls
            // overlay (shared by editor and viewer), so they are not duplicated
            // here.

            // Render transient in-GUI notification (right-aligned in the top bar)
            if let Some((msg, expiry)) = &app.transient_notification {
                if std::time::Instant::now() > *expiry {
                    // Clear expired message
                    app.transient_notification = None;
                } else {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let label = egui::Label::new(
                            RichText::new(msg).color(Color32::from_rgb(255, 200, 80)),
                        );
                        ui.add(label);
                    });
                }
            }
        });
        if !app.search_query.trim().is_empty() && !app.search_matches.is_empty() {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for p in app.search_matches.clone() {
                            let label = format!("/{}", p.join("/"));
                            let job = highlight_query_job(&label, &app.search_query);
                            let resp = ui.add(egui::Label::new(job).sense(Sense::click()));
                            if resp.clicked() {
                                navigate_to = Some(p);
                                clear_search = true;
                            }
                        }
                    });
            });
        }
    });

    // Check if current system is valid (zero-cost, no cloning)
    let system_valid = app.current_system().is_some();

    let mut staged_zoom = app.zoom;
    let mut staged_pan = app.pan;
    let mut staged_reset = app.reset_view;
    let mut staged_view_bounds = app.view_bounds;

    // Temporary variable to store block to open as subsystem
    let mut block_to_open_subsystem: Option<crate::model::Block> = None;
    // Snapshots for use inside closure (avoid borrowing `app` immutably inside UI rendering)
    let block_click_handler_snapshot = app.block_click_handler.clone();
    let block_menu_items_snapshot = app.block_menu_items.clone();
    let signal_menu_items_snapshot = app.signal_menu_items.clone();

    egui::CentralPanel::default().show(ui, |ui| {
        if !system_valid {
            // Provide detailed diagnostics to help the user resolve missing subsystems / libraries.
            let requested = if app.path.is_empty() {
                String::from("/")
            } else {
                format!("/{}", app.path.join("/"))
            };
            ui.colored_label(Color32::RED, "Invalid path — nothing to render");
            ui.label(format!("Requested path: {}", requested));

            // Find the longest existing parent and the missing segment
            let mut existing_parent: Vec<String> = Vec::new();
            let mut missing_segment: Option<String> = None;
            for i in 0..=app.path.len() {
                let prefix = app.path[..i].to_vec();
                if resolve_subsystem_by_vec(&app.root, &prefix).is_some() {
                    existing_parent = prefix;
                } else if i > 0 {
                    missing_segment = Some(app.path[i - 1].clone());
                    break;
                }
            }
            if !existing_parent.is_empty() {
                ui.label(format!("Nearest existing parent: /{}", existing_parent.join("/")));
                if let Some(parent_sys) = resolve_subsystem_by_vec(&app.root, &existing_parent) {
                    let names: Vec<String> = parent_sys
                        .blocks
                        .iter()
                        .filter(|b| b.subsystem.is_some())
                        .map(|b| b.name.clone())
                        .collect();
                    if !names.is_empty() {
                        ui.label(format!("Available subsystems under parent: {}", names.join(", ")));
                    }
                }
            }
            if let Some(ms) = missing_segment {
                ui.colored_label(Color32::YELLOW, format!("Missing segment: '{}'", ms));
            }

            // Report unresolved Reference blocks found anywhere in the root system
            let mut unresolved_refs: Vec<(String, Option<String>)> = Vec::new();
            fn collect_unresolved(sys: &crate::model::System, acc: &mut Vec<(String, Option<String>)>) {
                for b in &sys.blocks {
                    if b.block_type == "Reference" && b.subsystem.is_none() {
                        acc.push((b.name.clone(), b.system_ref.clone()));
                    }
                    if let Some(sub) = &b.subsystem {
                        collect_unresolved(sub, acc);
                    }
                }
            }
            collect_unresolved(&app.root, &mut unresolved_refs);
            if !unresolved_refs.is_empty() {
                ui.colored_label(Color32::YELLOW, "Unresolved reference blocks found:");
                for (n, ref_name) in &unresolved_refs {
                    ui.label(format!("  - {} (SystemRef={:?})", n, ref_name));
                }
            }

            // Show where we searched for libraries (if known)
            if !app.library_search_paths.is_empty() {
                ui.colored_label(Color32::LIGHT_BLUE, "Library search paths:");
                for p in &app.library_search_paths {
                    ui.label(format!("  - {}", p));
                }
            } else {
                ui.label("Library search paths: (none) — use -L to add paths or place library .slx next to the main .slx file");
            }

            ui.separator();
            ui.label("Hints: use -L <dir> to add library search paths, or open the library .slx directly.");
            return;
        }
        // ── Keyboard shortcuts for undo/redo ──
        if app.move_mode_enabled {
            let undo_requested = ui.input(|i| {
                i.modifiers.command && !i.modifiers.shift && i.key_pressed(egui::Key::Z)
            });
            let redo_requested = ui.input(|i| {
                (i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::Z))
                    || (i.modifiers.command && i.key_pressed(egui::Key::Y))
            });
            if undo_requested && app.viewer_history.can_undo() {
                let path = app.path.clone();
                if let Some(system) = resolve_subsystem_by_vec_mut(&mut app.root, &path) {
                    app.viewer_history.undo(system);
                }
                app.layout_dirty = true;
                app.view_cache.invalidate();
            }
            if redo_requested && app.viewer_history.can_redo() {
                let path = app.path.clone();
                if let Some(system) = resolve_subsystem_by_vec_mut(&mut app.root, &path) {
                    app.viewer_history.redo(system);
                }
                app.layout_dirty = true;
                app.view_cache.invalidate();
            }
        }
        // Get system reference AFTER undo/redo (which may mutate app.root)
        let sys = match resolve_subsystem_by_vec(&app.root, &app.path) {
            Some(s) => s,
            None => return,
        };
        // Use cached cloned data when the path and generation haven't changed,
        // avoiding expensive per-frame cloning of all blocks, lines, and annotations.
        // We use std::mem::take to move data out of the cache (zero-clone) and
        // restore it after the closure. If we return early, the cache is
        // invalidated and will be rebuilt next frame.
        let cache_gen = app.view_cache.generation;
        let cache_valid = app.view_cache.is_valid(&app.path, cache_gen)
            && !app.view_cache.cached_owned_blocks.is_empty();
        if !cache_valid {
            app.view_cache.cached_sys_lines = sys.lines.clone();
            app.view_cache.cached_sys_annotations = sys.annotations.iter()
                .chain(sys.blocks.iter().flat_map(|b| b.annotations.iter()))
                .cloned()
                .collect();
            app.view_cache.cached_owned_blocks = sys.blocks.iter()
                .filter(|b| parse_block_rect(b).is_some())
                .map(shallow_clone_block)
                .collect();
            app.view_cache.cached_subsystem_block_lookup = sys.blocks.iter()
                .filter(|b| parse_block_rect(b).is_some())
                .filter(|b| (b.block_type == "SubSystem" || b.block_type == "Reference")
                    && b.subsystem.as_ref().is_some_and(|sub| sub.chart.is_none()))
                .filter_map(|b| b.sid.as_ref().map(|sid| (sid.clone(), b.clone())))
                .collect();
        }
        // Move data out of cache to avoid cloning — will be restored after closure.
        let sys_lines: Vec<crate::model::Line> = std::mem::take(&mut app.view_cache.cached_sys_lines);
        let sys_annotations: Vec<crate::model::Annotation> = std::mem::take(&mut app.view_cache.cached_sys_annotations);
        let owned_blocks: Vec<crate::model::Block> = std::mem::take(&mut app.view_cache.cached_owned_blocks);
        let subsystem_block_lookup: HashMap<String, crate::model::Block> = std::mem::take(&mut app.view_cache.cached_subsystem_block_lookup);
        let blocks: Vec<(&crate::model::Block, Rect)> = owned_blocks
            .iter()
            .filter_map(|b| parse_block_rect(b).map(|r| (b, r)))
            .collect::<Vec<_>>();
        let annotations: Vec<(&crate::model::Annotation, Rect)> = sys_annotations
            .iter()
            .filter_map(|a| {
                a.position
                    .as_deref()
                    .and_then(parse_rect_str)
                    .map(|pos| (a, pos))
            })
            .collect();
        if blocks.is_empty() && annotations.is_empty() {
            ui.colored_label(
                Color32::YELLOW,
                "No blocks or annotations with positions to render",
            );
            // Restore cache data before early return.
            app.view_cache.cached_sys_lines = sys_lines;
            app.view_cache.cached_sys_annotations = sys_annotations;
            app.view_cache.cached_owned_blocks = owned_blocks;
            app.view_cache.cached_subsystem_block_lookup = subsystem_block_lookup;
            return;
        }
        let mut content_bb = blocks.first()
            .map(|x| x.1)
            .or_else(|| annotations.first().map(|x| x.1))
            .unwrap();
        for (_, r) in &blocks {
            content_bb = content_bb.union(*r);
        }
        for (_, r) in &annotations {
            content_bb = content_bb.union(*r);
        }

        let mut bb = match staged_view_bounds {
            Some(bounds) if !staged_reset => bounds,
            _ => {
                let fitted = content_bb.expand(20.0);
                staged_view_bounds = Some(fitted);
                fitted
            }
        };

        // Interaction space
        let margin = 20.0;
        let avail = ui.available_rect_before_wrap();
        let viewport_rect = avail.expand(48.0);
        let avail_size = avail.size();
        let width = (bb.width()).max(1.0);
        let height = (bb.height()).max(1.0);
        let sx = (avail_size.x - 2.0 * margin) / width;
        let sy = (avail_size.y - 2.0 * margin) / height;
        let mut base_scale = sx.min(sy).max(0.1);

        let canvas_sense = if app.move_mode_enabled {
            Sense::click()
        } else {
            Sense::click_and_drag()
        };
        let canvas_resp = ui.interact(avail, ui.id().with("canvas"), canvas_sense);
        if !app.move_mode_enabled && canvas_resp.dragged() {
            let d = canvas_resp.drag_delta();
            staged_pan += d;
        }
        let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_y.abs() > 0.0 && canvas_resp.hovered() {
            let factor = (1.0_f32 + scroll_y * 0.001_f32).max(0.1_f32);
            let old_zoom = staged_zoom;
            let new_zoom = (old_zoom * factor).clamp(0.2, 10.0);
            if (new_zoom - old_zoom).abs() > f32::EPSILON {
                let origin = Pos2::new(avail.left() + margin, avail.top() + margin);
                let s_old = base_scale * old_zoom;
                let s_new = base_scale * new_zoom;
                let cursor = canvas_resp.hover_pos().unwrap_or(avail.center());
                let world_x = (cursor.x - origin.x - staged_pan.x) / s_old + bb.left();
                let world_y = (cursor.y - origin.y - staged_pan.y) / s_old + bb.top();
                staged_zoom = new_zoom;
                staged_pan.x = cursor.x - ((world_x - bb.left()) * s_new + origin.x);
                staged_pan.y = cursor.y - ((world_y - bb.top()) * s_new + origin.y);
            }
        }

        show_zoom_controls(
            ui.ctx(),
            app.egui_id("zoom_controls"),
            Pos2::new(avail.left() + 8.0, avail.top() + 8.0),
            &mut staged_zoom,
            &mut staged_pan,
            base_scale,
            bb,
            Pos2::new(avail.left() + margin, avail.top() + margin),
            avail.center(),
            &mut staged_reset,
            &mut app.monochrome,
        );

        // Reset always fits every block into the viewport, recomputing the
        // fit from the fresh content bounds in the same frame the button is
        // pressed (no stale authored view, no multi-frame delay).
        if staged_reset {
            let fitted = content_bb.expand(20.0);
            staged_view_bounds = Some(fitted);
            bb = fitted;
            let w = bb.width().max(1.0);
            let h = bb.height().max(1.0);
            let sx = (avail_size.x - 2.0 * margin) / w;
            let sy = (avail_size.y - 2.0 * margin) / h;
            base_scale = sx.min(sy).max(0.1);
            staged_zoom = 1.0;
            let extra_x = (avail_size.x - 2.0 * margin - bb.width() * base_scale).max(0.0);
            let extra_y = (avail_size.y - 2.0 * margin - bb.height() * base_scale).max(0.0);
            staged_pan = Vec2::new(extra_x * 0.5, extra_y * 0.5);
            staged_reset = false;
            ui.ctx().request_repaint();
        }

        let to_screen = |p: Pos2| -> Pos2 {
            let s = base_scale * staged_zoom;
            let x = (p.x - bb.left()) * s + avail.left() + margin + staged_pan.x;
            let y = (p.y - bb.top()) * s + avail.top() + margin + staged_pan.y;
            Pos2::new(x, y)
        };

        // In-canvas text/icon scaling is coupled to the model's measurement
        // unit: `base_scale * staged_zoom` is the screen-pixels-per-model-unit
        // scale (identical to the one used for block geometry), so names and
        // icons grow and shrink exactly with the on-screen block size. Smaller
        // models (fewer blocks, each drawn larger) therefore get proportionally
        // larger text than big, space-filling models. The `*_font_factor`
        // values remain the size relative to the blocks.
        let font_scale: f32 = (base_scale * staged_zoom / 2.0).max(0.01);

        let monochrome = app.monochrome;
        let dark_mode = ui.visuals().dark_mode;

        // Draw area annotations (colored regions) behind the blocks.  Areas keep
        // their model-defined colors even in "less colorful" mode.
        for (a, r_model) in &annotations {
            if let Some(fill) = area_annotation_fill(a) {
                let r_screen =
                    Rect::from_min_max(to_screen(r_model.min), to_screen(r_model.max));
                if !r_screen.intersects(viewport_rect) {
                    continue;
                }
                ui.painter().rect_filled(r_screen, 2.0, fill);
                if let Some(border) = area_annotation_border(a) {
                    ui.painter().rect_stroke(
                        r_screen,
                        2.0,
                        Stroke::new(1.0_f32, border),
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }

        // Draw blocks and setup interaction maps
        let mut sid_map: HashMap<String, Rect> = HashMap::new();
        let mut sid_screen_map: HashMap<String, Rect> = HashMap::new();
        let mut block_views: Vec<(&crate::model::Block, Rect, bool, Color32)> = Vec::new();
        let mut any_block_clicked = false;

        fn paint_selected_shadow(painter: &egui::Painter, r: Rect, rounding: f32, font_scale: f32) {
            let scale = font_scale.max(0.2);
            // Draw a soft-ish shadow using multiple outside strokes. This ensures the
            // highlight is only outside the block, never covering its interior.
            let widths = [10.0 * scale, 18.0 * scale, 28.0 * scale];
            let alphas = [50_u8, 30_u8, 18_u8];
            for (w, a) in widths.into_iter().zip(alphas) {
                let col = Color32::from_rgba_premultiplied(200, 60, 60, a);
                painter.rect_stroke(
                    r,
                    rounding,
                    Stroke::new(w, col),
                    egui::StrokeKind::Outside,
                );
            }
        }

        // Collect scope blocks for deferred liveplot rendering (after painter borrow ends).
        #[cfg(feature = "dashboard")]
        let mut deferred_scope_rects: Vec<(String, String, Rect)> = Vec::new();

        // Collect Constant blocks for deferred TextEdit rendering.
        #[cfg(feature = "dashboard")]
        let mut deferred_constant_edits: Vec<(String, Rect)> = Vec::new();

        for (b, r) in &blocks {
            let preview_r = view_transform::preview_block_rect(
                &app.viewer_drag_state,
                &app.selected_block_sids,
                b.sid.as_deref(),
                *r,
            );
            if let Some(sid) = &b.sid {
                sid_map.insert(sid.clone(), preview_r);
            }
            let r_screen = Rect::from_min_max(to_screen(preview_r.min), to_screen(preview_r.max));
            if let Some(sid) = &b.sid {
                sid_screen_map.insert(sid.clone(), r_screen);
            }
            if !r_screen.intersects(viewport_rect) {
                continue;
            }

            let block_sense = if app.move_mode_enabled {
                Sense::click_and_drag()
            } else {
                Sense::click()
            };
            let resp = ui.allocate_rect(r_screen, block_sense);
            let cfg = get_block_type_cfg(b);
            let bg = block_fill_color(b, &cfg, monochrome, dark_mode);

            if app.move_mode_enabled && resp.drag_started()
                && let Some(sid) = &b.sid {
                    if !app.selected_block_sids.contains(sid) {
                        app.selected_block_sids.clear();
                        app.selected_block_sids.insert(sid.clone());
                    }
                    app.selected_line_indices.clear();
                    app.viewer_drag_state = ViewerDragState::Blocks {
                        current_dx: 0,
                        current_dy: 0,
                    };
                }
            if app.move_mode_enabled
                && resp.dragged()
                && b
                    .sid
                    .as_ref()
                    .map(|sid| app.selected_block_sids.contains(sid))
                    .unwrap_or(false)
            {
                let s = base_scale * staged_zoom;
                let target_dx = (resp.drag_delta().x / s).round() as i32;
                let target_dy = (resp.drag_delta().y / s).round() as i32;
                if let ViewerDragState::Blocks {
                    current_dx,
                    current_dy,
                } = &mut app.viewer_drag_state
                {
                    *current_dx += target_dx;
                    *current_dy += target_dy;
                    ui.ctx().request_repaint();
                }
            }

            let mut block_action: Option<ClickAction> = None;
            if resp.double_clicked() {
                println!("Block {} double-clicked", b.name);
                crate::connection_targets::debug_print_block_targets(&app.root, &app.path, b);
                block_action = Some(ClickAction::DoublePrimary);
            } else if resp.secondary_clicked() {
                println!("Block {} secondary clicked", b.name);
                crate::connection_targets::debug_print_block_targets(&app.root, &app.path, b);
                block_action = Some(ClickAction::Secondary);
            } else if resp.clicked() {
                println!("Block {} clicked", b.name);
                crate::connection_targets::debug_print_block_targets(&app.root, &app.path, b);
                if !app.move_mode_enabled {
                    if app.live_mode_enabled && b.block_type == "ManualSwitch" {
                        if let Some(enabled) = toggle_manual_switch_setting(app, b) {
                            #[cfg(feature = "dashboard")]
                            app.queue_dashboard_control(
                                (*b).clone(),
                                DashboardControlValue::Bool(enabled),
                            );
                            #[cfg(not(feature = "dashboard"))]
                            let _ = enabled;
                            any_block_clicked = true;
                        }
                    } else {
                        // Dashboard / UI block click: print connected block and signal info.
                        // Also handle traditional signal-line blocks like Scope and Display.
                        if crate::simulink_libraries::stubs::is_dashboard_block_type(
                            &b.block_type,
                        ) || matches!(b.block_type.as_str(), "Scope" | "Display")
                        {
                            print_dashboard_connected_signals(b, &sys_lines);
                        }
                        // Open a scope popout window when a Scope/DashboardScope is clicked.
                        #[cfg(feature = "dashboard")]
                        if matches!(b.block_type.as_str(), "Scope" | "DashboardScope") {
                            let key = app.scope_key_for_block(b);
                            app.scope_popout = Some(crate::egui_app::state::ScopePopout {
                                title: scope_title_for_block(b, &sys_lines),
                                scope_key: key,
                                open: true,
                            });
                        }
                    }
                }
                if !(app.live_mode_enabled && b.block_type == "ManualSwitch") {
                    block_action = Some(ClickAction::Primary);
                }
            }

            // Selection: single-click selects, Shift-click toggles (multi-select).
            // This is independent from block dialogs (which remain available on double-click).
            if matches!(block_action, Some(ClickAction::Primary)) {
                any_block_clicked = true;
                if let Some(sid) = &b.sid {
                    let shift = ui.input(|i| i.modifiers.shift);
                    if shift {
                        if app.selected_block_sids.contains(sid) {
                            app.selected_block_sids.remove(sid);
                        } else {
                            app.selected_block_sids.insert(sid.clone());
                        }
                    } else {
                        app.selected_block_sids.clear();
                        app.selected_block_sids.insert(sid.clone());
                    }
                    app.selected_line_indices.clear();
                }
            }

            // Clear selection when clicking empty canvas.
            if canvas_resp.clicked() && !any_block_clicked
                && let Some(pos) = canvas_resp.interact_pointer_pos() {
                    let hit_any = blocks.iter().any(|(_, br)| {
                        let br_screen = Rect::from_min_max(to_screen(br.min), to_screen(br.max));
                        br_screen.contains(pos)
                    });
                    if !hit_any {
                        app.selected_block_sids.clear();
                        app.selected_line_indices.clear();
                    }
                }

            // Paint selection shadow behind the block (outside-only).
            if b
                .sid
                .as_ref()
                .map(|sid| app.selected_block_sids.contains(sid))
                .unwrap_or(false)
            {
                let rounding = if b.commented { 0.0 } else { 6.0 };
                paint_selected_shadow(ui.painter(), r_screen, rounding, font_scale);
                if app.move_mode_enabled
                    && let Some(sid) = &b.sid {
                        draw_viewer_resize_handles(
                            ui,
                            &r_screen,
                            sid,
                            &preview_r,
                            app,
                            base_scale * staged_zoom,
                        );
                    }
            }

            let effective_bg = crate::egui_app::render::fill_block_body(
                ui.painter(),
                r_screen,
                cfg.shape,
                bg,
                b.commented,
            );
            if enable_context_menus {
                resp.context_menu(|ui| {
                    if ui.button("Info").clicked() {
                        let block_for_response = b.sid.as_ref()
                            .and_then(|sid| subsystem_block_lookup.get(sid))
                            .cloned()
                            .unwrap_or_else(|| (*b).clone());
                        record_interaction(
                            &mut interaction,
                            UpdateResponse::Block {
                                action: ClickAction::Secondary,
                                block: block_for_response,
                                handled: false,
                            },
                        );
                        ui.close();
                    }
                    for item in &block_menu_items_snapshot {
                        if (item.filter)(b)
                            && ui.button(&item.label).clicked() {
                                (item.on_click)(b);
                                ui.close();
                            }
                    }
                });
            }
            if let Some(action) = block_action {
                let mut handled = false;
                if matches!(action, ClickAction::Primary) {
                    // Primary click is used for selection; keep it from opening dialogs/subsystems.
                    handled = true;
                } else if matches!(action, ClickAction::DoublePrimary) {
                    if let Some(handler) = block_click_handler_snapshot.as_ref() {
                        handled = handler(app, b);
                    }
                    // Double-click on Constant block opens inline editor.
                    #[cfg(feature = "dashboard")]
                    if !handled && b.block_type == "Constant"
                        && let Some(sid) = &b.sid {
                            // Seed the edit buffer with the current value if not yet present.
                            if !app.constant_edits.contains_key(sid.as_str()) {
                                let val = b.value.clone().unwrap_or_else(|| "1".to_string());
                                app.constant_edits.insert(sid.clone(), val);
                            }
                            deferred_constant_edits.push((sid.clone(), r_screen));
                            handled = true;
                        }
                    if !handled && b.sid.as_ref().is_some_and(|sid| subsystem_block_lookup.contains_key(sid)) {
                        // Normal subsystem open — use original block with subsystem intact
                        block_to_open_subsystem = b.sid.as_ref()
                            .and_then(|sid| subsystem_block_lookup.get(sid))
                            .cloned()
                            .or_else(|| Some((*b).clone()));
                        handled = true;
                    } else if !handled && b.block_type == "Reference" && b.subsystem.is_none() {
                        // Inform user when a Reference block can't be opened because the
                        // referenced library/subsystem was not resolved.
                        // Trim/crunch whitespace in the block name before logging.
                        let clean_name = crate::parser::helpers::clean_whitespace(&b.name);
                        let hint = match b.system_ref.as_deref() {
                            Some(r) => {
                                let r = crate::parser::helpers::clean_whitespace(r);
                                format!(" (System Ref=\"{}\")", r)
                            }
                            None => String::new(),
                        };
                        let msg = format!(
                            "Cannot open reference block '{}'{}: referenced subsystem not resolved. Try adding the library path with -L or placing the library next to the .slx file.",
                            clean_name, hint
                        );
                        println!("{}", msg);
                        // show transient in-GUI notification for 5s
                        app.show_notification(msg, 5000);
                    }
                }
                let block_for_response = b.sid.as_ref()
                    .and_then(|sid| subsystem_block_lookup.get(sid))
                    .cloned()
                    .unwrap_or_else(|| (*b).clone());
                record_interaction(
                    &mut interaction,
                    UpdateResponse::Block {
                        action,
                        block: block_for_response,
                        handled,
                    },
                );
            }
            if app.move_mode_enabled && resp.drag_stopped()
                && let ViewerDragState::Blocks {
                    current_dx,
                    current_dy,
                } = app.viewer_drag_state.clone()
                {
                    if (current_dx != 0 || current_dy != 0)
                        && b
                            .sid
                            .as_ref()
                            .map(|sid| app.selected_block_sids.contains(sid))
                            .unwrap_or(false)
                    {
                        let selected_sids = app.selected_block_sids.clone();
                        let mut layout_changed = false;
                        let mut undo_cmd = None;
                        if let Some(system) = app.current_system_mut() {
                            let indices: Vec<usize> = system
                                .blocks
                                .iter()
                                .enumerate()
                                .filter_map(|(idx, block)| {
                                    block.sid
                                        .as_ref()
                                        .filter(|sid| selected_sids.contains(*sid))
                                        .map(|_| idx)
                                })
                                .collect();
                            if !indices.is_empty() {
                                undo_cmd = Some(operations::move_blocks(system, &indices, current_dx, current_dy));
                                layout_changed = true;
                                // Auto-adjust signal line corners for moved blocks
                                for sid in &selected_sids {
                                    for line in &mut system.lines {
                                        if let Some(src) = &line.src
                                            && src.sid == *sid {
                                                corner_ops::auto_adjust_on_block_move(line, true, current_dx, current_dy);
                                            }
                                        if let Some(dst) = &line.dst
                                            && dst.sid == *sid {
                                                corner_ops::auto_adjust_on_block_move(line, false, current_dx, current_dy);
                                            }
                                        corner_ops::auto_adjust_branches_on_block_move(&mut line.branches, sid, current_dx, current_dy);
                                    }
                                }
                            }
                        }
                        if let Some(cmd) = undo_cmd {
                            app.viewer_history.push(cmd);
                        }
                        if layout_changed {
                            app.layout_dirty = true;
                            app.view_cache.invalidate();
                        }
                    }
                    app.viewer_drag_state = ViewerDragState::None;
                }
            block_views.push((b, r_screen, resp.clicked(), effective_bg));
        }

        // Draw annotations (convert HTML-rich content to plain text).  Area
        // annotations (whose colored background is drawn behind the blocks) get
        // a text color that contrasts with their fill.
        for (a, r_model) in &annotations {
            let r_screen = Rect::from_min_max(to_screen(r_model.min), to_screen(r_model.max));
            let _resp = ui.allocate_rect(r_screen, Sense::hover());
            let text_color = area_annotation_fill(a)
                .map(contrast_color)
                .unwrap_or(Color32::WHITE);
            let raw = a.text.clone().unwrap_or_default();
            let parsed =
                crate::egui_app::text::annotation_to_rich_text(&raw, a.interpreter.as_deref());
            let base_font = 12.0;
            let mut job = parsed.to_layout_job(ui.style(), font_scale, base_font);
            job.wrap.max_width = f32::INFINITY;
            let galley = ui.painter().layout_job(job.clone());
            let paint_pos = r_screen.left_top();
            if galley.size().x <= r_screen.width() {
                ui.painter().galley(paint_pos, galley, text_color);
            } else {
                job.wrap.max_width = r_screen.width();
                let job_for_wrap = job.clone();
                ui.scope_builder(egui::UiBuilder::new().max_rect(r_screen), |child_ui| {
                    let wrapped = child_ui.painter().layout_job(job_for_wrap);
                    child_ui
                        .painter()
                        .galley(paint_pos, wrapped, text_color);
                });
            }
            // no special tooltip; text is directly visible inside the rectangle
        }

        // Precompute lookup maps
        let mut sid_to_name: HashMap<String, String> = HashMap::new();
        for (b, _r) in &blocks {
            if let Some(sid) = &b.sid {
                sid_to_name.insert(sid.clone(), b.name.clone());
            }
        }

        // The connection-target resolver depends only on model topology and is
        // rebuilt only when that changes (not on navigation or layout drags).
        let connection_target_resolver = app.view_cache.ensure_resolver(&app.root);

        // Use cached line colors and port info when possible; recompute on model change.
        let cache_gen = app.view_cache.generation;
        if !app.view_cache.is_valid(&app.path, cache_gen) {
            let line_adjacency = line_coloring::compute_line_adjacency(&sys_lines);
            let bg_lum = line_coloring::rel_luminance(Color32::from_gray(245));
            app.view_cache.line_colors = line_coloring::assign_line_colors(&line_adjacency, bg_lum);

            let (pc, cp) = signal_routing::compute_port_info(
                &sys_lines,
                &owned_blocks,
            );
            app.view_cache.port_counts = pc;
            app.view_cache.connected_ports = cp;
            app.view_cache.mark_valid(&app.path, cache_gen);
        }
        // Move cached computed values out to avoid per-frame cloning.
        let line_colors = std::mem::take(&mut app.view_cache.line_colors);
        let port_counts = std::mem::take(&mut app.view_cache.port_counts);
        let connected_ports = std::mem::take(&mut app.view_cache.connected_ports);

        let line_stroke_default = Stroke::new(2.0_f32, Color32::LIGHT_GREEN);

        // Build lines in screen space and interactive hit rects
        #[allow(clippy::type_complexity)]
        let mut line_views: Vec<(
            &crate::model::Line,
            Vec<Pos2>,
            Pos2,
            egui::Response,
            usize,
            Vec<(Pos2, Pos2)>,
        )> = Vec::new();
        let mut port_label_requests: Vec<(String, u32, bool, f32)> = Vec::new();
        let mut port_y_screen: HashMap<(String, u32, bool), f32> = HashMap::new();
        // Precompute mirroring for each block SID in this view
        let mut sid_mirrored: HashMap<String, bool> = HashMap::new();
        for (b, _r) in &blocks {
            if let Some(sid) = &b.sid {
                sid_mirrored.insert(sid.clone(), b.block_mirror.unwrap_or(false));
            }
        }
        for (li, line) in sys_lines.iter().enumerate() {
            let Some(src) = line.src.as_ref() else {
                continue;
            };
            let Some(sr) = sid_map.get(&src.sid) else {
                continue;
            };
            let mut offsets_pts: Vec<Pos2> = Vec::new();
            let mirrored_src = sid_mirrored.get(&src.sid).copied().unwrap_or(false);
            let mut cur = signal_routing::endpoint_pos(*sr, src, &port_counts, mirrored_src);
            offsets_pts.push(cur);
            for off in &line.points {
                cur = Pos2::new(cur.x + off.x as f32, cur.y + off.y as f32);
                offsets_pts.push(cur);
            }
            let mut screen_pts: Vec<Pos2> = offsets_pts.iter().map(|p| to_screen(*p)).collect();
            if let Some(src_ep) = line.src.as_ref() {
                let src_screen = *screen_pts.first().unwrap_or(&to_screen(cur));
                port_label_requests.push((
                    src_ep.sid.clone(),
                    src_ep.port_index,
                    false,
                    src_screen.y,
                ));
                port_y_screen.insert((src_ep.sid.clone(), src_ep.port_index, false), src_screen.y);
            }
            if let Some(dst) = line.dst.as_ref()
                && let Some(dr) = sid_map.get(&dst.sid) {
                    let mirrored_dst = owned_blocks
                        .iter()
                        .find(|b| b.sid.as_ref() == Some(&dst.sid))
                        .and_then(|b| b.block_mirror)
                        .unwrap_or(false);
                    let dst_pt = signal_routing::endpoint_pos(*dr, dst, &port_counts, mirrored_dst);
                    let dst_screen = to_screen(dst_pt);
                    screen_pts.push(dst_screen);
                    if dst.port_type == "in" {
                        port_label_requests.push((
                            dst.sid.clone(),
                            dst.port_index,
                            true,
                            dst_screen.y,
                        ));
                        port_y_screen.insert((dst.sid.clone(), dst.port_index, true), dst_screen.y);
                    }
                }
            if screen_pts.is_empty() {
                continue;
            }
            screen_pts = signal_routing::orthogonalize_polyline(&screen_pts);
            let mut segments_all: Vec<(Pos2, Pos2)> = Vec::new();
            signal_routing::push_orthogonal_segments(&screen_pts, &mut segments_all);
            for br in &line.branches {
                collect_branch_segments_rec(
                    &to_screen,
                    &sid_map,
                    &port_counts,
                    *offsets_pts.last().unwrap_or(&cur),
                    br,
                    &mut segments_all,
                    &mut port_y_screen,
                    &sid_mirrored,
                );
            }
            let pad = 8.0;
            let (min_x, max_x, min_y, max_y) = segments_all.iter().fold(
                (
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                ),
                |(min_x, max_x, min_y, max_y), (a, b)| {
                    (
                        min_x.min(a.x.min(b.x)),
                        max_x.max(a.x.max(b.x)),
                        min_y.min(a.y.min(b.y)),
                        max_y.max(a.y.max(b.y)),
                    )
                },
            );
            let hit_rect = Rect::from_min_max(
                Pos2::new(min_x - pad, min_y - pad),
                Pos2::new(max_x + pad, max_y + pad),
            );
            if !hit_rect.intersects(viewport_rect) {
                continue;
            }
            // Use Sense::hover() instead of Sense::click() so that the
            // line bounding-box does not steal click events from blocks that
            // overlap with it.  Actual click detection is deferred to the
            // precise per-segment distance check in the second pass.
            let resp = if app.live_mode_enabled {
                if let Some(tooltip_entries) = app.live_line_tooltips.get(&li) {
                    let resp = ui.allocate_rect(hit_rect, Sense::hover());
                    if resp.hovered() {
                        show_pointer_tooltip_entries(
                            ui,
                            app.egui_id(("line_value_tooltip", li)),
                            tooltip_entries,
                        );
                    }
                    resp
                } else {
                    ui.allocate_rect(hit_rect, Sense::hover())
                }
            } else {
                ui.allocate_rect(hit_rect, Sense::hover())
            };
            let main_anchor = *offsets_pts.last().unwrap_or(&cur);
            line_views.push((
                line,
                screen_pts,
                main_anchor,
                resp,
                li,
                segments_all,
            ));
        }

        // Collect segments for a branch tree (model coords in, screen-space segments out)
        #[allow(clippy::too_many_arguments)]
        fn collect_branch_segments_rec(
            to_screen: &dyn Fn(Pos2) -> Pos2,
            sid_map: &HashMap<String, Rect>,
            port_counts: &HashMap<(String, u8), u32>,
            start: Pos2,
            br: &crate::model::Branch,
            out: &mut Vec<(Pos2, Pos2)>,
            port_y_screen: &mut HashMap<(String, u32, bool), f32>,
            sid_mirrored: &HashMap<String, bool>,
        ) {
            let mut pts: Vec<Pos2> = vec![start];
            let mut cur = start;
            for off in &br.points {
                cur = Pos2::new(cur.x + off.x as f32, cur.y + off.y as f32);
                pts.push(cur);
            }
            let screen_pts: Vec<Pos2> = pts.iter().map(|p| to_screen(*p)).collect();
            signal_routing::push_orthogonal_segments(&screen_pts, out);
            if let Some(dstb) = &br.dst
                && let Some(dr) = sid_map.get(&dstb.sid) {
                    let mirrored_dst = sid_mirrored.get(&dstb.sid).copied().unwrap_or(false);
                    let end_pt =
                        signal_routing::endpoint_pos(*dr, dstb, port_counts, mirrored_dst);
                    let a = to_screen(*pts.last().unwrap_or(&cur));
                    let b = to_screen(end_pt);
                    signal_routing::push_orthogonal_segments(&[a, b], out);
                    if dstb.port_type == "in" {
                        port_y_screen.insert((dstb.sid.clone(), dstb.port_index, true), b.y);
                    }
                }
            for sub in &br.branches {
                collect_branch_segments_rec(
                    to_screen,
                    sid_map,
                    port_counts,
                    *pts.last().unwrap_or(&cur),
                    sub,
                    out,
                    port_y_screen,
                    sid_mirrored,
                );
            }
        }

        // Draw lines and branches
        let painter = ui.painter().clone();
        fn draw_arrow_with_trim(
            painter: &egui::Painter,
            tail: Pos2,
            tip: Pos2,
            color: Color32,
            stroke: Stroke,
        ) {
            let size = 8.0_f32;
            let dir = Vec2::new(tip.x - tail.x, tip.y - tail.y);
            let len = (dir.x * dir.x + dir.y * dir.y).sqrt().max(1e-3);
            let ux = dir.x / len;
            let uy = dir.y / len;
            let inset = size * 0.6;
            let tip_adj = Pos2::new(tip.x - ux * inset, tip.y - uy * inset);
            painter.line_segment([tail, tip_adj], stroke);

            let px = -uy;
            let py = ux;
            let base = Pos2::new(tip_adj.x - ux * size, tip_adj.y - uy * size);
            let left = Pos2::new(base.x + px * (size * 0.6), base.y + py * (size * 0.6));
            let right = Pos2::new(base.x - px * (size * 0.6), base.y - py * (size * 0.6));
            painter.add(egui::Shape::convex_polygon(
                vec![tip_adj, left, right],
                color,
                Stroke::NONE,
            ));
        }

        #[allow(clippy::too_many_arguments)]
        fn draw_branch_rec(
            painter: &egui::Painter,
            to_screen: &dyn Fn(Pos2) -> Pos2,
            sid_map: &HashMap<String, Rect>,
            port_counts: &HashMap<(String, u8), u32>,
            start: Pos2,
            br: &crate::model::Branch,
            stroke: Stroke,
            color: Color32,
            port_label_requests: &mut Vec<(String, u32, bool, f32)>,
            sid_mirrored: &HashMap<String, bool>,
        ) {
            let mut pts: Vec<Pos2> = vec![start];
            let mut cur = start;
            for off in &br.points {
                cur = Pos2::new(cur.x + off.x as f32, cur.y + off.y as f32);
                pts.push(cur);
            }
            let screen_pts: Vec<Pos2> = pts.iter().map(|p| to_screen(*p)).collect();
            for seg in signal_routing::orthogonalize_polyline(&screen_pts).windows(2) {
                painter.line_segment([seg[0], seg[1]], stroke);
            }
            if let Some(dstb) = &br.dst
                && let Some(dr) = sid_map.get(&dstb.sid) {
                    let mirrored_dst = sid_mirrored.get(&dstb.sid).copied().unwrap_or(false);
                    let end_pt =
                        signal_routing::endpoint_pos(*dr, dstb, port_counts, mirrored_dst);
                    let last = *pts.last().unwrap_or(&cur);
                    let a = to_screen(last);
                    let b = to_screen(end_pt);
                    let ortho = signal_routing::orthogonalize_polyline(&[a, b]);
                    if dstb.port_type == "in" {
                        for seg in ortho.windows(2).take(ortho.len().saturating_sub(2)) {
                            painter.line_segment([seg[0], seg[1]], stroke);
                        }
                        if ortho.len() >= 2 {
                            let n = ortho.len();
                            draw_arrow_with_trim(painter, ortho[n - 2], ortho[n - 1], color, stroke);
                        }
                        port_label_requests.push((dstb.sid.clone(), dstb.port_index, true, b.y));
                    } else {
                        for seg in ortho.windows(2) {
                            painter.line_segment([seg[0], seg[1]], stroke);
                        }
                    }
                }
            for sub in &br.branches {
                draw_branch_rec(
                    painter,
                    to_screen,
                    sid_map,
                    port_counts,
                    *pts.last().unwrap_or(&cur),
                    sub,
                    stroke,
                    color,
                    port_label_requests,
                    sid_mirrored,
                );
            }
        }

        let mut signal_label_rects: Vec<(Rect, usize)> = Vec::new();
        // NOTE: Up to here we have collected port_y_screen while building lines and branches.
        // From this we create a per-block map for fast lookup during block rendering.
        let mut block_port_y_map: HashMap<String, ComputedPortYCoordinates> = HashMap::new();
        for ((sid, idx, is_input), y) in port_y_screen.iter() {
            let entry = block_port_y_map.entry(sid.clone()).or_default();
            if *is_input {
                entry.inputs.insert(*idx, *y);
            } else {
                entry.outputs.insert(*idx, *y);
            }
        }

        for (line, screen_pts, main_anchor, hover_resp, li, segments_all) in &line_views {
            let line_targets = connection_target_resolver.line_targets_for_line_ref(&app.path, line);
            let color = if monochrome {
                monochrome_line_color(dark_mode)
            } else {
                line_colors
                    .get(*li)
                    .copied()
                    .unwrap_or(line_stroke_default.color)
            };
            let show_testpoint_marker = line_has_testpoint(line_targets);
            let stroke = Stroke::new(
                line_stroke_width(line_targets, app.selected_line_indices.contains(li)),
                color,
            );
            let has_in_dst = line.dst.as_ref().is_some_and(|dst| dst.port_type == "in");
            let mut draw_pts = screen_pts.clone();
            if draw_pts.len() >= 2 {
                let dx = draw_pts[1].x - draw_pts[0].x;
                let dy = draw_pts[1].y - draw_pts[0].y;
                let len = (dx * dx + dy * dy).sqrt();
                if len > 1e-3 {
                    let inset = 8.0_f32;
                    let ux = dx / len;
                    let uy = dy / len;
                    draw_pts[0].x += ux * inset;
                    draw_pts[0].y += uy * inset;
                }
            }
            let last_idx = draw_pts.len().saturating_sub(1);
            for (seg_idx, seg) in draw_pts.windows(2).enumerate() {
                let is_last = has_in_dst && seg_idx == last_idx.saturating_sub(1);
                if is_last {
                    draw_arrow_with_trim(&painter, seg[0], seg[1], color, stroke);
                } else {
                    painter.line_segment([seg[0], seg[1]], stroke);
                }
            }
            for br in &line.branches {
                draw_branch_rec(
                    &painter,
                    &to_screen,
                    &sid_map,
                    &port_counts,
                    *main_anchor,
                    br,
                    stroke,
                    color,
                    &mut port_label_requests,
                    &sid_mirrored,
                );
            }
            if show_testpoint_marker
                && let Some(marker_pos) = line_testpoint_marker_position(&draw_pts) {
                    draw_line_testpoint_marker(&painter, marker_pos, color);
                }
            // Precise per-segment distance check.  We detect clicks via
            // pointer state instead of from the bounding-box response so that
            // line rects (which can be very large) never steal clicks from
            // blocks that overlap with them.
            {
                let mut hit_segments = segments_all.clone();
                if let Some(first) = hit_segments.first_mut() {
                    let dx = first.1.x - first.0.x;
                    let dy = first.1.y - first.0.y;
                    let len = (dx * dx + dy * dy).sqrt();
                    if len > 1e-3 {
                        let inset = 8.0_f32;
                        let ux = dx / len;
                        let uy = dy / len;
                        first.0.x += ux * inset;
                        first.0.y += uy * inset;
                    }
                }
                if let Some(cp) = ui.input(|i| i.pointer.interact_pos()) {
                    let mut min_dist = f32::INFINITY;
                    for (a, b) in &hit_segments {
                        let ab_x = b.x - a.x;
                        let ab_y = b.y - a.y;
                        let ap_x = cp.x - a.x;
                        let ap_y = cp.y - a.y;
                        let ab_len2 = (ab_x * ab_x + ab_y * ab_y).max(1e-6);
                        let t = (ap_x * ab_x + ap_y * ab_y) / ab_len2;
                        let t_clamped = t.clamp(0.0, 1.0);
                        let proj_x = a.x + ab_x * t_clamped;
                        let proj_y = a.y + ab_y * t_clamped;
                        let dx = cp.x - proj_x;
                        let dy = cp.y - proj_y;
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist < min_dist {
                            min_dist = dist;
                        }
                    }
                    let near_segment = min_dist <= 8.0;
                    if near_segment {
                        // Determine click type from pointer state.
                        let primary_clicked = ui.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary));
                        let secondary_clicked = ui.input(|i| i.pointer.button_clicked(egui::PointerButton::Secondary));
                        let double_clicked = ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary));
                        let action = if double_clicked {
                            println!("Line {} double-clicked", li);
                            crate::connection_targets::debug_print_line_targets(&app.root, &app.path, line);
                            Some(ClickAction::DoublePrimary)
                        } else if secondary_clicked {
                            println!("Line {} secondary clicked", li);
                            crate::connection_targets::debug_print_line_targets(&app.root, &app.path, line);
                            Some(ClickAction::Secondary)
                        } else if primary_clicked {
                            println!("Line {} clicked", li);
                            crate::connection_targets::debug_print_line_targets(&app.root, &app.path, line);
                            Some(ClickAction::Primary)
                        } else {
                            None
                        };
                        if let Some(action) = action {
                            if app.move_mode_enabled {
                                if matches!(action, ClickAction::Primary) {
                                    let shift = ui.input(|i| i.modifiers.shift);
                                    if shift {
                                        if app.selected_line_indices.contains(li) {
                                            app.selected_line_indices.remove(li);
                                        } else {
                                            app.selected_line_indices.insert(*li);
                                        }
                                    } else {
                                        app.selected_line_indices.clear();
                                        app.selected_line_indices.insert(*li);
                                    }
                                    app.selected_block_sids.clear();
                                }
                            } else {
                                record_interaction(
                                    &mut interaction,
                                    UpdateResponse::Signal {
                                        action,
                                        line_idx: *li,
                                        line: (*line).clone(),
                                        handled: false,
                                    },
                                );
                            }
                        }
                    }
                    // Context menu: show when secondary-clicked near a segment.
                    if near_segment && enable_context_menus {
                        hover_resp.context_menu(|ui| {
                            if ui.button("Info").clicked() {
                                record_interaction(
                                    &mut interaction,
                                    UpdateResponse::Signal {
                                        action: ClickAction::Secondary,
                                        line_idx: *li,
                                        line: (*line).clone(),
                                        handled: false,
                                    },
                                );
                                ui.close();
                            }
                            for item in &signal_menu_items_snapshot {
                                if (item.filter)(line)
                                    && ui.button(&item.label).clicked() {
                                        (item.on_click)(line);
                                        ui.close();
                                    }
                            }
                        });
                    }
                }
            }
        }

        if app.move_mode_enabled {
            for (line, _screen_pts, main_anchor, _hover_resp, li, _segments_all) in &line_views {
                if !app.selected_line_indices.contains(li) {
                    continue;
                }
                let mut cur = *main_anchor;
                for point_index in 0..line.points.len() {
                    let point = &line.points[point_index];
                    cur = Pos2::new(cur.x + point.x as f32, cur.y + point.y as f32);
                    let handle_pos = to_screen(cur);
                    let handle_rect = Rect::from_center_size(handle_pos, Vec2::splat(10.0));
                    let resp = ui.allocate_rect(
                        handle_rect,
                        Sense::click_and_drag(),
                    );
                    let color = if resp.dragged() || resp.hovered() {
                        Color32::from_rgb(80, 180, 255)
                    } else {
                        Color32::from_rgb(0, 120, 255)
                    };
                    ui.painter().rect_filled(handle_rect.shrink(2.0), 1.0, color);
                    ui.painter().rect_stroke(
                        handle_rect.shrink(2.0),
                        1.0,
                        Stroke::new(1.0_f32, Color32::WHITE),
                        egui::StrokeKind::Outside,
                    );
                    if resp.drag_started() {
                        app.viewer_drag_state = ViewerDragState::LinePointDrag {
                            line_idx: *li,
                            point_idx: point_index,
                            acc_dx: 0,
                            acc_dy: 0,
                        };
                    }
                    if resp.dragged() {
                        let s = base_scale * staged_zoom;
                        let fdx = (resp.drag_delta().x / s).round() as i32;
                        let fdy = (resp.drag_delta().y / s).round() as i32;
                        if let ViewerDragState::LinePointDrag { acc_dx, acc_dy, .. } = &mut app.viewer_drag_state {
                            *acc_dx += fdx;
                            *acc_dy += fdy;
                        }
                        ui.ctx().request_repaint();
                    }
                    if resp.drag_stopped()
                        && let ViewerDragState::LinePointDrag { line_idx, point_idx, acc_dx, acc_dy } = app.viewer_drag_state.clone() {
                            if acc_dx != 0 || acc_dy != 0 {
                                let mut layout_changed = false;
                                if let Some(system) = app.current_system_mut()
                                    && let Some(line_mut) = system.lines.get_mut(line_idx) {
                                        signal_routing::move_line_point(line_mut, point_idx, acc_dx, acc_dy);
                                        layout_changed = true;
                                    }
                                if layout_changed {
                                    app.viewer_history.push(
                                        crate::editor::operations::EditorCommand::MoveLinePoint {
                                            line_index: line_idx,
                                            point_index: point_idx,
                                            dx: acc_dx,
                                            dy: acc_dy,
                                        },
                                    );
                                    app.layout_dirty = true;
                                    app.view_cache.invalidate();
                                }
                            }
                            app.viewer_drag_state = ViewerDragState::None;
                        }
                    // Double-click on a corner handle removes it
                    if resp.double_clicked() {
                        let mut removed_point = None;
                        if let Some(system) = app.current_system_mut()
                            && let Some(line_mut) = system.lines.get_mut(*li) {
                                removed_point = corner_ops::remove_corner(&mut line_mut.points, point_index);
                            }
                        if let Some(rp) = removed_point {
                            app.viewer_history.push(
                                crate::editor::operations::EditorCommand::RemoveCorner {
                                    line_index: *li,
                                    point_index,
                                    removed_point: rp,
                                },
                            );
                            app.layout_dirty = true;
                            app.view_cache.invalidate();
                        }
                    }
                }

                let mut branch_handles: Vec<(Vec<usize>, usize, Pos2)> = Vec::new();
                signal_routing::collect_branch_handle_positions(
                    *main_anchor,
                    &line.branches,
                    &to_screen,
                    &mut Vec::new(),
                    &mut branch_handles,
                );
                for (branch_path, point_index, handle_pos) in branch_handles {
                    let handle_rect = Rect::from_center_size(handle_pos, Vec2::splat(10.0));
                    let resp = ui.allocate_rect(handle_rect, Sense::click_and_drag());
                    let color = if resp.dragged() || resp.hovered() {
                        Color32::from_rgb(255, 180, 80)
                    } else {
                        Color32::from_rgb(220, 140, 40)
                    };
                    ui.painter().circle_filled(handle_rect.center(), 4.0, color);
                    ui.painter().circle_stroke(
                        handle_rect.center(),
                        4.0,
                        Stroke::new(1.0_f32, Color32::WHITE),
                    );
                    if resp.drag_started() {
                        app.viewer_drag_state = ViewerDragState::BranchPointDrag {
                            line_idx: *li,
                            branch_path: branch_path.clone(),
                            point_idx: point_index,
                            acc_dx: 0,
                            acc_dy: 0,
                        };
                    }
                    if resp.dragged() {
                        let s = base_scale * staged_zoom;
                        let fdx = (resp.drag_delta().x / s).round() as i32;
                        let fdy = (resp.drag_delta().y / s).round() as i32;
                        if let ViewerDragState::BranchPointDrag { acc_dx, acc_dy, .. } = &mut app.viewer_drag_state {
                            *acc_dx += fdx;
                            *acc_dy += fdy;
                        }
                        ui.ctx().request_repaint();
                    }
                    if resp.drag_stopped()
                        && let ViewerDragState::BranchPointDrag { line_idx, branch_path: bp, point_idx, acc_dx, acc_dy } = app.viewer_drag_state.clone() {
                            if acc_dx != 0 || acc_dy != 0 {
                                let mut layout_changed = false;
                                if let Some(system) = app.current_system_mut()
                                    && let Some(line_mut) = system.lines.get_mut(line_idx)
                                    && let Some(branch_mut) = signal_routing::get_branch_mut(&mut line_mut.branches, &bp) {
                                            signal_routing::move_branch_point(branch_mut, point_idx, acc_dx, acc_dy);
                                            layout_changed = true;
                                        }
                                if layout_changed {
                                    app.viewer_history.push(
                                        crate::editor::operations::EditorCommand::MoveBranchPoint {
                                            line_index: line_idx,
                                            branch_path: bp,
                                            point_index: point_idx,
                                            dx: acc_dx,
                                            dy: acc_dy,
                                        },
                                    );
                                    app.layout_dirty = true;
                                    app.view_cache.invalidate();
                                }
                            }
                            app.viewer_drag_state = ViewerDragState::None;
                        }
                }
            }
        }

        // Segment midpoint "+" handles for inserting new corners
        if app.move_mode_enabled {
            for (line, _screen_pts, main_anchor, _hover_resp, li, _segments_all) in &line_views {
                if !app.selected_line_indices.contains(li) {
                    continue;
                }
                // Build model-space positions of the line's corners
                let mut model_pts: Vec<Pos2> = Vec::new();
                let mut cur = *main_anchor;
                model_pts.push(cur);
                for point in &line.points {
                    cur = Pos2::new(cur.x + point.x as f32, cur.y + point.y as f32);
                    model_pts.push(cur);
                }
                // For each segment between consecutive model points, show a "+" handle at
                // the midpoint. Clicking it inserts a new corner at that position.
                if model_pts.len() >= 2 {
                    for seg_idx in 0..model_pts.len() - 1 {
                        let a = model_pts[seg_idx];
                        let b = model_pts[seg_idx + 1];
                        let mid_model = Pos2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
                        let mid_screen = to_screen(mid_model);
                        let handle_rect = Rect::from_center_size(mid_screen, Vec2::splat(12.0));
                        let resp = ui.allocate_rect(handle_rect, Sense::click());
                        let color = if resp.hovered() {
                            Color32::from_rgb(100, 220, 100)
                        } else {
                            Color32::from_rgba_premultiplied(60, 180, 60, 160)
                        };
                        ui.painter().circle_filled(mid_screen, 5.0, color);
                        ui.painter().text(
                            mid_screen,
                            Align2::CENTER_CENTER,
                            "+",
                            egui::FontId::proportional(10.0),
                            Color32::WHITE,
                        );
                        if resp.clicked() {
                            // Compute offset: the corner splits the segment in half.
                            // The new point is at mid_model - a = half the segment.
                            let dx = ((b.x - a.x) * 0.5).round() as i32;
                            let dy = ((b.y - a.y) * 0.5).round() as i32;
                            let offset = crate::model::Point { x: dx, y: dy };
                            // point_index in the points array: seg_idx maps to the
                            // point *after* seg_idx positions (index seg_idx corresponds
                            // to the segment from anchor/model_pts[seg_idx] to points[seg_idx]).
                            let insert_idx = seg_idx;
                            if let Some(system) = app.current_system_mut()
                                && let Some(line_mut) = system.lines.get_mut(*li) {
                                    corner_ops::insert_corner(&mut line_mut.points, insert_idx, offset.clone());
                                }
                            app.viewer_history.push(
                                crate::editor::operations::EditorCommand::InsertCorner {
                                    line_index: *li,
                                    point_index: insert_idx,
                                    offset,
                                },
                            );
                            app.layout_dirty = true;
                            app.view_cache.invalidate();
                        }
                    }
                }
            }
        }

        // Label placement
        let shared_label_font_px =
            view_transform::shared_canvas_text_font_px(font_scale, app.block_name_font_factor);
        struct EguiMeasurer<'a> {
            painter: &'a egui::Painter,
            font: egui::FontId,
            color: Color32,
        }
        impl<'a> crate::label_place::Measurer for EguiMeasurer<'a> {
            fn measure(&self, text: &str) -> (f32, f32) {
                let galley =
                    self.painter
                        .layout_no_wrap(text.to_string(), self.font.clone(), self.color);
                let s = galley.size();
                (s.x, s.y)
            }
        }
        let cfg = crate::label_place::Config {
            expand_factor: 1.5,
            step_fraction: 0.25,
            perp_offset: 2.0,
        };
        let mut placed_label_rects: Vec<Rect> = Vec::new();
        let mut draw_line_labels = |line: &crate::model::Line,
                                    line_targets: &[crate::connection_targets::ConnectionTarget],
                                    screen_pts: &Vec<Pos2>,
                                    main_anchor: Pos2,
                                    color: Color32,
                                    line_idx: usize| {
            if screen_pts.len() < 2 {
                return;
            }
            let Some(label_text) = resolved_line_label(line, line_targets) else {
                return;
            };
            let mut segments: Vec<(Pos2, Pos2)> = Vec::new();
            for seg in screen_pts.windows(2) {
                segments.push((seg[0], seg[1]));
            }
            for br in &line.branches {
                collect_branch_segments_rec(
                    &to_screen,
                    &sid_map,
                    &port_counts,
                    main_anchor,
                    br,
                    &mut segments,
                    &mut port_y_screen,
                    &sid_mirrored,
                );
            }
            let mut best_len2 = -1.0f32;
            let mut best_seg: Option<(Pos2, Pos2)> = None;
            for (a, b) in &segments {
                let dx = b.x - a.x;
                let dy = b.y - a.y;
                let l2 = dx * dx + dy * dy;
                if l2 > best_len2 {
                    best_len2 = l2;
                    best_seg = Some((*a, *b));
                }
            }
            let Some((sa, sb)) = best_seg else {
                return;
            };
            let poly: Vec<crate::label_place::Vec2f> = vec![
                crate::label_place::Vec2f { x: sa.x, y: sa.y },
                crate::label_place::Vec2f { x: sb.x, y: sb.y },
            ];
            let mut avoid_rects: Vec<crate::label_place::RectF> = placed_label_rects
                .iter()
                .map(|r| {
                    crate::label_place::RectF::from_min_max(
                        crate::label_place::Vec2f {
                            x: r.left(),
                            y: r.top(),
                        },
                        crate::label_place::Vec2f {
                            x: r.right(),
                            y: r.bottom(),
                        },
                    )
                })
                .collect();
            for (_b, br, _clicked, _bg) in &block_views {
                avoid_rects.push(crate::label_place::RectF::from_min_max(
                    crate::label_place::Vec2f {
                        x: br.left(),
                        y: br.top(),
                    },
                    crate::label_place::Vec2f {
                        x: br.right(),
                        y: br.bottom(),
                    },
                ));
            }
            let line_thickness = 0.8f32;
            for (a, b) in &segments {
                let min_x = a.x.min(b.x) - line_thickness;
                let max_x = a.x.max(b.x) + line_thickness;
                let min_y = a.y.min(b.y) - line_thickness;
                let max_y = a.y.max(b.y) + line_thickness;
                avoid_rects.push(crate::label_place::RectF::from_min_max(
                    crate::label_place::Vec2f { x: min_x, y: min_y },
                    crate::label_place::Vec2f { x: max_x, y: max_y },
                ));
            }
            let mut final_drawn = false;
            let mut tried_wrap = false;
            let mut wrap_text = label_text.clone();
            while !final_drawn {
                let font_id = egui::FontId::proportional(shared_label_font_px);
                let meas = EguiMeasurer {
                    painter: ui.painter(),
                    font: font_id.clone(),
                    color,
                };
                let candidate_texts: Vec<String> = if !tried_wrap && label_text.contains(' ') {
                    let bytes: Vec<(usize, char)> = label_text.char_indices().collect();
                    let mut best_split = None;
                    let mut best_dist = usize::MAX;
                    for (i, ch) in bytes.iter() {
                        if *ch == ' ' {
                            let dist =
                                (*i as isize - (label_text.len() as isize) / 2).unsigned_abs();
                            if dist < best_dist {
                                best_dist = dist;
                                best_split = Some(*i);
                            }
                        }
                    }
                    if let Some(split) = best_split {
                        wrap_text = format!(
                            "{}\n{}",
                            &label_text[..split].trim_end(),
                            &label_text[split + 1..].trim_start()
                        );
                        vec![label_text.clone(), wrap_text.clone()]
                    } else {
                        vec![label_text.clone()]
                    }
                } else {
                    vec![label_text.clone(), wrap_text.clone()]
                };
                for candidate in candidate_texts.into_iter().filter(|s| !s.is_empty()) {
                    if let Some(result) =
                        crate::label_place::place_label(&poly, &candidate, &meas, cfg, &avoid_rects)
                    {
                        if result.horizontal {
                            let galley = ui.painter().layout_no_wrap(
                                candidate.clone(),
                                font_id.clone(),
                                color,
                            );
                            let draw_pos = Pos2::new(result.rect.min.x, result.rect.min.y);
                            painter.galley(draw_pos, galley, color);
                        } else {
                            let galley = ui.painter().layout_no_wrap(
                                candidate.clone(),
                                font_id.clone(),
                                color,
                            );
                            let draw_pos = Pos2::new(result.rect.min.x, result.rect.min.y);
                            // Draw rotated text using TextShape with angle around the draw_pos (top-left)
                            let text_shape = egui::epaint::TextShape {
                                pos: draw_pos,
                                galley,
                                fallback_color: Color32::TRANSPARENT,
                                opacity_factor: 1.0,
                                underline: Stroke::NONE,
                                override_text_color: Some(color),
                                angle: std::f32::consts::FRAC_PI_2,
                            };
                            painter.add(egui::Shape::Text(text_shape));
                        }
                        let rect = Rect::from_min_max(
                            Pos2::new(result.rect.min.x, result.rect.min.y),
                            Pos2::new(result.rect.max.x, result.rect.max.y),
                        );
                        placed_label_rects.push(rect);
                        signal_label_rects.push((rect, line_idx));
                        final_drawn = true;
                        break;
                    }
                }
                if final_drawn {
                    break;
                }
                if !tried_wrap && label_text.contains(' ') {
                    tried_wrap = true;
                } else {
                    break;
                }
            }
        };

        for (line, screen_pts, main_anchor, _resp, li, _segments_all) in &line_views {
            let line_targets = connection_target_resolver.line_targets_for_line_ref(&app.path, line);
            let color = if monochrome {
                monochrome_line_color(dark_mode)
            } else {
                line_colors
                    .get(*li)
                    .copied()
                    .unwrap_or(line_stroke_default.color)
            };
            draw_line_labels(line, line_targets, screen_pts, *main_anchor, color, *li);
        }

        // Clickable labels
        for (r, li) in &signal_label_rects {
            let resp = ui.interact(
                *r,
                ui.id().with(("signal_label", *li)),
                if app.move_mode_enabled {
                    Sense::click_and_drag()
                } else {
                    Sense::click()
                },
            );
            if app.move_mode_enabled && resp.drag_started() {
                app.selected_line_indices.clear();
                app.selected_line_indices.insert(*li);
                app.selected_block_sids.clear();
            }
            let mut label_action: Option<ClickAction> = None;
            if resp.double_clicked() {
                println!("Line {} double-clicked", li);
                label_action = Some(ClickAction::DoublePrimary);
            } else if resp.secondary_clicked() {
                println!("Line {} secondary clicked", li);
                label_action = Some(ClickAction::Secondary);
            } else if resp.clicked() {
                println!("Line {} clicked", li);
                label_action = Some(ClickAction::Primary);
            }
            if let Some(action) = label_action {
                if app.move_mode_enabled {
                    if matches!(action, ClickAction::Primary) {
                        let shift = ui.input(|i| i.modifiers.shift);
                        if shift {
                            if app.selected_line_indices.contains(li) {
                                app.selected_line_indices.remove(li);
                            } else {
                                app.selected_line_indices.insert(*li);
                            }
                        } else {
                            app.selected_line_indices.clear();
                            app.selected_line_indices.insert(*li);
                        }
                        app.selected_block_sids.clear();
                    }
                } else {
                    let line = &sys_lines[*li];
                    record_interaction(
                        &mut interaction,
                        UpdateResponse::Signal {
                            action,
                            line_idx: *li,
                            line: line.clone(),
                            handled: false,
                        },
                    );
                }
            }
            if app.move_mode_enabled && resp.drag_started() && app.selected_line_indices.contains(li) {
                app.viewer_drag_state = ViewerDragState::SignalLabelDrag {
                    line_idx: *li,
                    acc_dx: 0,
                    acc_dy: 0,
                };
            }
            if app.move_mode_enabled && resp.dragged() && app.selected_line_indices.contains(li) {
                let s = base_scale * staged_zoom;
                let fdx = (resp.drag_delta().x / s).round() as i32;
                let fdy = (resp.drag_delta().y / s).round() as i32;
                if let ViewerDragState::SignalLabelDrag { acc_dx, acc_dy, .. } = &mut app.viewer_drag_state {
                    *acc_dx += fdx;
                    *acc_dy += fdy;
                }
                ui.ctx().request_repaint();
            }
            if app.move_mode_enabled && resp.drag_stopped() && app.selected_line_indices.contains(li)
                && let ViewerDragState::SignalLabelDrag { line_idx, acc_dx, acc_dy } = app.viewer_drag_state.clone() {
                    if acc_dx != 0 || acc_dy != 0 {
                        let mut layout_changed = false;
                        if let Some(system) = app.current_system_mut()
                            && let Some(line_mut) = system.lines.get_mut(line_idx) {
                                signal_routing::move_line_layout(line_mut, acc_dx, acc_dy);
                                layout_changed = true;
                            }
                        if layout_changed {
                            app.viewer_history.push(
                                crate::editor::operations::EditorCommand::MoveLineLayout {
                                    line_index: line_idx,
                                    dx: acc_dx,
                                    dy: acc_dy,
                                },
                            );
                            app.layout_dirty = true;
                            app.view_cache.invalidate();
                        }
                    }
                    app.viewer_drag_state = ViewerDragState::None;
                }
            if enable_context_menus {
                resp.context_menu(|ui| {
                    if ui.button("Info").clicked() {
                        let line = &sys_lines[*li];
                        record_interaction(
                            &mut interaction,
                            UpdateResponse::Signal {
                                action: ClickAction::Secondary,
                                line_idx: *li,
                                line: line.clone(),
                                handled: false,
                            },
                        );
                        ui.close();
                    }
                    let line_ref = &sys_lines[*li];
                    for item in &signal_menu_items_snapshot {
                        if (item.filter)(line_ref)
                            && ui.button(&item.label).clicked() {
                                (item.on_click)(line_ref);
                                ui.close();
                            }
                    }
                });
            }
        }

        // Simulink prints a port's label whether or not a signal is attached,
        // so ask for one on every port the model/catalog actually names – the
        // requests above only cover wired-up ports.
        {
            let wired: std::collections::HashSet<(String, u32, bool)> = port_label_requests
                .iter()
                .map(|(sid, index, is_input, _)| (sid.clone(), *index, *is_input))
                .collect();
            for (b, _) in &blocks {
                let Some(sid) = b.sid.as_ref() else { continue };
                let Some(brect) = sid_screen_map.get(sid).copied() else {
                    continue;
                };
                let cfg = get_block_type_cfg(b);
                let in_count = b
                    .port_counts
                    .as_ref()
                    .and_then(|p| p.ins)
                    .unwrap_or(cfg.default_ins);
                let out_count = b
                    .port_counts
                    .as_ref()
                    .and_then(|p| p.outs)
                    .unwrap_or(cfg.default_outs);
                if in_count == 0 && out_count == 0 {
                    continue;
                }
                let (ins, outs) = crate::egui_app::geometry::port_indicator_positions_with_overrides(
                    brect,
                    in_count,
                    out_count,
                    b.block_mirror.unwrap_or(false),
                    &cfg.port_position_overrides,
                );
                let mut request = |index: usize, is_input: bool, y: f32| {
                    let index = index as u32 + 1;
                    if wired.contains(&(sid.clone(), index, is_input)) {
                        return;
                    }
                    if port_label_defined_name(b, index, is_input, &cfg).is_some() {
                        port_label_requests.push((sid.clone(), index, is_input, y));
                    }
                };
                for (i, p) in ins.iter().enumerate() {
                    request(i, true, p.y);
                }
                for (i, p) in outs.iter().enumerate() {
                    request(i, false, p.y);
                }
            }
        }

        // Pre-compute max inside-block port label widths per block (left/right).
        // The icon renderer uses this to maximize the center icon without overlapping
        // port labels, while still enforcing ≥10% outer margins.
        let mut port_label_max_widths: HashMap<String, PortLabelMaxWidths> = HashMap::new();
        {
            let mut seen: std::collections::HashSet<(String, u32, bool, i32)> = Default::default();
            for (sid, index, is_input, y) in &port_label_requests {
                let key = (sid.clone(), *index, *is_input, y.round() as i32);
                if !seen.insert(key) {
                    continue;
                }

                let Some(brect) = sid_screen_map.get(sid).copied() else {
                    continue;
                };
                let Some(block) = blocks.iter().find_map(|(b, _)| {
                    if b.sid.as_ref() == Some(sid) {
                        Some(*b)
                    } else {
                        None
                    }
                }) else {
                    continue;
                };
                if should_use_mask_display(block) {
                    continue;
                }
                let cfg = get_block_type_cfg(block);
                if (*is_input && !cfg.show_input_port_labels)
                    || (!*is_input && !cfg.show_output_port_labels)
                {
                    continue;
                }

                let pname = port_label_display_name(block, *index, *is_input, &cfg);
                let avail_w = (brect.width() - 8.0 * font_scale).max(1.0);
                let font_id = egui::FontId::proportional(view_transform::shared_canvas_text_font_px(
                    font_scale,
                    app.block_name_font_factor,
                ));
                let galley = painter.layout_no_wrap(pname, font_id.clone(), Color32::TRANSPARENT);
                let size = galley.size();

                // Match the label drawing code: skip labels that won't be drawn due to width.
                if size.x > avail_w {
                    continue;
                }

                // Same side selection as the label drawing code.
                let mirrored = block.block_mirror.unwrap_or(false);
                let is_left = *is_input ^ mirrored;
                let entry = port_label_max_widths.entry(sid.clone()).or_default();
                if is_left {
                    entry.left = entry.left.max(size.x);
                } else {
                    entry.right = entry.right.max(size.x);
                }
            }
        }

        let mut collidable_obstacle_rects: Vec<Rect> = Vec::new();
        for (_, r, _, _) in &block_views {
            collidable_obstacle_rects.push(*r);
        }
        for (_, screen_pts, _, _, _, _) in &line_views {
            for seg in screen_pts.windows(2) {
                let mut min = seg[0].min(seg[1]);
                let mut max = seg[0].max(seg[1]);
                min.x -= 2.0; min.y -= 2.0;
                max.x += 2.0; max.y += 2.0;
                collidable_obstacle_rects.push(Rect::from_min_max(min, max));
            }
        }
        for (r, _) in &signal_label_rects {
            collidable_obstacle_rects.push(*r);
        }

        // Finish blocks (border, icon/value, labels) and click handling
        for (b, r_screen, _clicked, bg) in &block_views {
            let cfg = get_block_type_cfg(b);
            // Neutral border only for blocks whose fill was actually grayed
            // (i.e. no model-authored color); model-colored blocks keep theirs.
            let border_color = if monochrome && !block_has_model_color(b) {
                monochrome_block_border(dark_mode)
            } else {
                let border_rgb = cfg.border.unwrap_or(crate::block_types::Rgb(180, 180, 200));
                Color32::from_rgb(border_rgb.0, border_rgb.1, border_rgb.2)
            };
            let stroke = Stroke::new(2.0_f32, border_color);
            crate::egui_app::render::stroke_block_body(&painter, *r_screen, cfg.shape, stroke);

            fn paint_port_chevron_placed(
                painter: &egui::Painter,
                outline: Pos2,
                is_left_side: bool,
                placement: Option<crate::simulink_libraries::types::PortPlacement>,
                font_scale: f32,
                color: Color32,
            ) {
                use crate::simulink_libraries::types::PortPlacement;
                let (h, w, stroke_w) = crate::egui_app::geometry::port_chevron_size(font_scale);

                let points = match placement {
                    Some(PortPlacement::Bottom) => {
                        // Signal enters from below: draw upward-pointing chevron (^)
                        // tip just outside (below) the block edge, base further below.
                        let tip_y = outline.y + stroke_w / 2.0;
                        let base_y = tip_y + w;
                        vec![
                            Pos2::new(outline.x - h / 2.0, base_y),
                            Pos2::new(outline.x, tip_y),
                            Pos2::new(outline.x + h / 2.0, base_y),
                        ]
                    }
                    Some(PortPlacement::Top) => {
                        // Signal enters from above: draw downward-pointing chevron (v)
                        let tip_y = outline.y - stroke_w / 2.0;
                        let base_y = tip_y - w;
                        vec![
                            Pos2::new(outline.x - h / 2.0, base_y),
                            Pos2::new(outline.x, tip_y),
                            Pos2::new(outline.x + h / 2.0, base_y),
                        ]
                    }
                    _ => {
                        // Horizontal chevron (Left/Right sides)
                        let (base_x, tip_x) = if is_left_side {
                            let tip_x = outline.x - stroke_w / 2.0;
                            (tip_x - w, tip_x)
                        } else {
                            let base_x = outline.x + stroke_w / 2.0;
                            (base_x, base_x + w)
                        };
                        vec![
                            Pos2::new(base_x, outline.y - h / 2.0),
                            Pos2::new(tip_x, outline.y),
                            Pos2::new(base_x, outline.y + h / 2.0),
                        ]
                    }
                };

                painter.add(egui::Shape::Path(egui::epaint::PathShape::line(
                    points,
                    Stroke::new(stroke_w, color),
                )));
            }

            // Draw port indicators derived from the block's own port counts.
            // This is important for virtual-library blocks (e.g. matrix_library)
            // and for unconnected blocks where no lines exist yet.
            // Chevrons are hidden for ports that have at least one connection.
            // For base blocks port_counts may be absent; fall back to the
            // virtual-library defaults carried in BlockTypeConfig.
            let in_count = b
                .port_counts
                .as_ref()
                .and_then(|p| p.ins)
                .unwrap_or(cfg.default_ins);
            let out_count = b
                .port_counts
                .as_ref()
                .and_then(|p| p.outs)
                .unwrap_or(cfg.default_outs);
            if in_count > 0 || out_count > 0 {
                let mirrored = b.block_mirror.unwrap_or(false);
                let overrides = &cfg.port_position_overrides;
                let (ins, outs) = crate::egui_app::geometry::port_indicator_positions_with_overrides(
                    *r_screen,
                    in_count,
                    out_count,
                    mirrored,
                    overrides,
                );
                let ins_left_side = !mirrored;
                let outs_left_side = mirrored;
                let block_sid = b.sid.as_deref().unwrap_or("");
                for (i, p) in ins.iter().enumerate() {
                    let port_idx = (i as u32) + 1;
                    // Skip chevron if this input port is connected
                    if connected_ports.contains(&(block_sid.to_string(), port_idx, true)) {
                        continue;
                    }
                    let ovr_placement = overrides
                        .iter()
                        .find(|o| o.matches(true, port_idx, in_count))
                        .map(|o| o.placement);
                    let left_side = ovr_placement
                        .map(|pl| crate::egui_app::geometry::port_override_is_left_side(pl, mirrored))
                        .unwrap_or(ins_left_side);
                    paint_port_chevron_placed(
                        &painter,
                        *p,
                        left_side,
                        ovr_placement,
                        font_scale,
                        Color32::from_rgb(60, 60, 200),
                    );
                }
                for (i, p) in outs.iter().enumerate() {
                    let port_idx = (i as u32) + 1;
                    // Skip chevron if this output port is connected
                    if connected_ports.contains(&(block_sid.to_string(), port_idx, false)) {
                        continue;
                    }
                    let ovr_placement = overrides
                        .iter()
                        .find(|o| o.matches(false, port_idx, out_count))
                        .map(|o| o.placement);
                    let left_side = ovr_placement
                        .map(|pl| crate::egui_app::geometry::port_override_is_left_side(pl, mirrored))
                        .unwrap_or(outs_left_side);
                    paint_port_chevron_placed(
                        &painter,
                        *p,
                        left_side,
                        ovr_placement,
                        font_scale,
                        Color32::from_rgb(200, 60, 60),
                    );
                }
            }
            // Enable/trigger/reset/event ports enter through the top edge.
            let control_count =
                crate::simulink_libraries::renderers::subsystem_control_port_count(b);
            for i in 0..control_count {
                let x = r_screen.left()
                    + (i as f32 + 1.0) / (control_count as f32 + 1.0) * r_screen.width();
                paint_port_chevron_placed(
                    &painter,
                    egui::pos2(x, r_screen.top()),
                    true,
                    Some(crate::simulink_libraries::types::PortPlacement::Top),
                    font_scale,
                    Color32::from_rgb(60, 60, 200),
                );
            }
            let fg = contrast_color(*bg);
            #[cfg(feature = "dashboard")]
            update_scope_live_sample(app, b, &sys_lines);
            let display_signal_label = if b.block_type == "Display" {
                let sid = b.sid.as_deref();
                sid.and_then(|sid| {
                    sys_lines.iter().find_map(|line| {
                        let direct = line.dst.as_ref().is_some_and(|dst| dst.sid == sid);
                        let branched = line.branches.iter().any(|br| branch_hits_sid(br, sid));
                        if direct || branched {
                            line.name.clone().filter(|s| !s.is_empty())
                        } else {
                            None
                        }
                    })
                })
            } else {
                None
            };

            let icon_port_label_widths = b
                .sid
                .as_ref()
                .and_then(|sid| port_label_max_widths.get(sid))
                .copied();
            let live_text = if should_render_live_text(app.live_mode_enabled, &b.block_type) {
                block_live_text(app, b)
                    .or_else(|| block_live_value(app, b).map(format_live_scalar_csv))
            } else {
                None
            };
            let static_constant_value = if b.block_type == "Constant" {
                #[cfg(feature = "dashboard")]
                {
                    if app.live_mode_enabled {
                        block_live_text(app, b).unwrap_or_else(|| {
                            let sid = b.sid.clone().unwrap_or_default();
                            app.constant_edits
                                .get(&sid)
                                .cloned()
                                .or_else(|| b.value.clone())
                                .unwrap_or_else(|| "1".to_string())
                        })
                    } else {
                        let sid = b.sid.clone().unwrap_or_default();
                        app.constant_edits
                            .get(&sid)
                            .cloned()
                            .or_else(|| b.value.clone())
                            .unwrap_or_else(|| "1".to_string())
                    }
                }
                #[cfg(not(feature = "dashboard"))]
                {
                    b.value.clone().unwrap_or_else(|| "1".to_string())
                }
            } else {
                String::new()
            };
            let mut value_tooltip: Option<String> = None;
            // Icon/value rendering with precedence: mask > value > custom/icon
            if let Some(text) = live_text.clone() {
                let shown_text = format_block_value_for_display(b, &text);
                if matches!(b.block_type.as_str(), "Display" | "Constant") {
                    paint_fitted_centered_text(
                        &painter,
                        *r_screen,
                        &shown_text,
                        fg,
                        12.0 * font_scale * app.block_value_font_factor,
                        12.0 * font_scale * MIN_BLOCK_VALUE_FONT_FACTOR,
                        false,
                    );
                } else {
                    let font_id = egui::FontId::proportional(12.0 * font_scale);
                    let galley = painter.layout_no_wrap(shown_text.clone(), font_id, fg);
                    let pos = r_screen.center() - galley.size() * 0.5;
                    painter.galley(pos, galley, fg);
                }
                value_tooltip = Some(shown_text);
            } else if b.block_type == "Constant" && app.live_mode_enabled {
                let shown_value = format_constant_value_for_display(&static_constant_value);
                paint_fitted_centered_text(
                    &painter,
                    *r_screen,
                    &shown_value,
                    fg,
                    12.0 * font_scale * app.block_value_font_factor,
                    12.0 * font_scale * MIN_BLOCK_VALUE_FONT_FACTOR,
                    false,
                );
                value_tooltip = Some(shown_value);
            } else if let Some(label) = display_signal_label {
                paint_fitted_centered_text(
                    &painter,
                    *r_screen,
                    &label,
                    fg,
                    12.0 * font_scale * app.block_value_font_factor,
                    12.0 * font_scale * MIN_BLOCK_VALUE_FONT_FACTOR,
                    false,
                );
                value_tooltip = Some(label);
            } else if should_use_mask_display(b) {
                let text = b.mask_display_text.as_ref().expect("checked above");
                let font_size = (b.font_size.unwrap_or(14) as f32) * font_scale;
                let font_id = egui::FontId::proportional(font_size);
                let color = fg;
                let galley = painter.layout_no_wrap(text.clone(), font_id.clone(), color);
                let pos = r_screen.center() - galley.size() * 0.5;
                painter.galley(pos, galley, color);
            } else if b
                .value
                .as_ref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
                && b.block_type != "Constant"
            {
                let text = b.value.as_ref().unwrap().clone();
                if b.block_type == "Display" {
                    paint_fitted_centered_text(
                        &painter,
                        *r_screen,
                        &text,
                        fg,
                        10.0 * font_scale * app.block_value_font_factor,
                        10.0 * font_scale * MIN_BLOCK_VALUE_FONT_FACTOR,
                        false,
                    );
                } else {
                    let beneath_font_px = 10.0 * font_scale;
                    let font_id = egui::FontId::proportional(beneath_font_px);
                    let galley = painter.layout_no_wrap(text, font_id, fg);
                    let pos = r_screen.center() - galley.size() * 0.5;
                    painter.galley(pos, galley, fg);
                }
            } else if let Some(instance_label) = crate::simulink_libraries::resolve_definition(b)
                .compute_instance_label
                .and_then(|f| f(b))
            {
                // Per-instance label from InstanceData (e.g. "≤ 3.0" for Compare To Constant).
                let beneath_font_px = 12.0 * font_scale;
                let font_id = egui::FontId::proportional(beneath_font_px);
                let color = fg;
                let galley = painter.layout_no_wrap(instance_label, font_id.clone(), color);
                let pos = r_screen.center() - galley.size() * 0.5;
                painter.galley(pos, galley, color);
            } else if app.live_mode_enabled
                && matches!(b.block_type.as_str(), "Scope" | "DashboardScope")
            {
                // The live scope view is an interactive liveplot tile that needs
                // `ui`/app state, so it cannot be a pure painter renderer and is
                // the one block kind kept special-cased here.  The static
                // waveform glyph (non-live / too-small) is handled by the general
                // renderer's static renderer.
                #[cfg(feature = "dashboard")]
                {
                    let scope_rect = r_screen.shrink(4.0);
                    if scope_rect.width() > 20.0 && scope_rect.height() > 20.0 {
                        let key = app.scope_key_for_block(b);
                        deferred_scope_rects.push((
                            key,
                            scope_title_for_block(b, &sys_lines),
                            scope_rect,
                        ));
                    } else {
                        paint_scope_glyph(&painter, r_screen);
                    }
                }
                #[cfg(not(feature = "dashboard"))]
                {
                    paint_scope_glyph(&painter, r_screen);
                }
            } else {
                // General, definition-driven interior renderer.  There is no
                // block-type-specific code here: the block's resolved
                // `SimulinkBlockDefinition` selects the static/live renderer,
                // block label and icon.
                //
                // In live mode the definition's single `LiveRendererFn` (the
                // unified live hook — interactive controls *and* non-interactive
                // gauges) is dispatched here with mutable `app`/`ui`.  Whatever
                // it declines, plus every non-live block, falls through to the
                // painter-only static path in `render_block_interior`.
                let coords_ref = b.sid.as_ref().and_then(|sid| block_port_y_map.get(sid));
                let def = crate::simulink_libraries::resolve_definition(b);
                let live_handled = if app.live_mode_enabled {
                    if let Some(live_fn) = def.live_renderer {
                        let live_value = live_block_value_for_renderer(app, b, def);
                        let live_text = block_live_text(app, b);
                        let display_opts = app.live_display_defaults.clone();
                        let metadata =
                            crate::simulink_libraries::metadata::extract_metadata(b, def);
                        let ctx = crate::simulink_libraries::types::RenderContext {
                            live_mode: true,
                            font_scale,
                            name_font_factor: app.block_name_font_factor,
                            metadata: &metadata,
                            live_value,
                            live_text: live_text.as_deref(),
                            live_display_options: Some(&display_opts),
                            port_y: coords_ref,
                            port_label_widths: icon_port_label_widths,
                            text_color: fg,
                            fill_color: *bg,
                            border_color,
                        };
                        live_fn(app, ui, b, r_screen, &ctx)
                    } else {
                        false
                    }
                } else {
                    false
                };
                if !live_handled {
                    let params = crate::simulink_libraries::render::InteriorParams {
                        live_mode: app.live_mode_enabled,
                        font_scale,
                        name_font_factor: app.block_name_font_factor,
                        live_value: None,
                        live_text: None,
                        live_display_options: Some(&app.live_display_defaults),
                        port_y: coords_ref,
                        port_label_widths: icon_port_label_widths,
                        text_color: fg,
                        fill_color: *bg,
                        border_color,
                    };
                    crate::simulink_libraries::render::render_block_interior(
                        &painter, b, r_screen, &params,
                    );
                }
            }

            let live_hover_key = app.live_value_key_for_block(b);
            let live_tooltip_entries = if app.live_mode_enabled {
                app.live_block_tooltips.get(&live_hover_key)
            } else {
                None
            };
            if live_tooltip_entries.is_some() || value_tooltip.is_some() {
                let hover_key = b.sid.clone().unwrap_or_else(|| b.name.clone());
                let resp = ui.interact(
                    *r_screen,
                    app.egui_id(("block_value_tooltip", hover_key)),
                    Sense::hover(),
                );
                if resp.hovered() {
                    if let Some(entries) = live_tooltip_entries {
                        show_pointer_tooltip_entries(ui, resp.id, entries);
                    } else if let Some(tooltip) = value_tooltip.as_deref() {
                        show_pointer_tooltip_text(ui, resp.id, tooltip);
                    }
                }
            }
            // Draw block name label near the block according to NameLocation.
            // Global default can be toggled; per-block override uses `Block::show_name`.
            let show_name = b.show_name.unwrap_or(app.show_block_names_default);
            if show_name {
                let scale = font_scale.max(0.2);

                // Keep name width bounded relative to (block + chevrons) width.
                let chevron_w = (6.0 * scale * 4.0).max(2.0 * 4.0);

                let in_count = b
                    .port_counts
                    .as_ref()
                    .and_then(|p| p.ins)
                    .unwrap_or(cfg.default_ins);
                let out_count = b
                    .port_counts
                    .as_ref()
                    .and_then(|p| p.outs)
                    .unwrap_or(cfg.default_outs);
                let mirrored = b.block_mirror.unwrap_or(false);
                let ins_left_side = !mirrored;
                let outs_left_side = mirrored;
                let has_left = (in_count > 0 && ins_left_side) || (out_count > 0 && outs_left_side);
                let has_right = (in_count > 0 && !ins_left_side)
                    || (out_count > 0 && !outs_left_side);
                let left_extra = if has_left { chevron_w } else { 0.0 };
                let right_extra = if has_right { chevron_w } else { 0.0 };
                let overall_w = r_screen.width() + left_extra + right_extra;
                let max_label_w = overall_w * 0.95 * app.block_name_extend_factor.max(0.1);

                let font_px = view_transform::shared_canvas_text_font_px(
                    font_scale,
                    app.block_name_font_factor,
                );

                let color = contrast_color(ui.visuals().panel_fill);

                let left = r_screen.left() - left_extra;
                let right = r_screen.right() + right_extra;
                let center_x = (left + right) * 0.5;

                let font = egui::FontId::proportional(font_px);
                let line_height = (font_px * 1.2).max(1.0);
                let best_lines = wrap_text_to_max_width(&painter, &b.name, font.clone(), max_label_w);
                if !best_lines.is_empty() {
                    let total_h = (best_lines.len() as f32) * line_height;
                    let mut max_w = 0.0_f32;
                    for l in &best_lines {
                        let w = painter.layout_no_wrap(l.to_string(), font.clone(), color).size().x;
                        if w > max_w {
                            max_w = w;
                        }
                    }

                    let mut rects = Vec::new();
                    match b.name_location {
                        crate::model::NameLocation::Bottom => {
                            let top = r_screen.bottom() + 2.0 * font_scale;
                            rects.push(Rect::from_min_size(
                                Pos2::new(center_x - max_w * 0.5, top),
                                eframe::egui::vec2(max_w, total_h),
                            ));
                        }
                        crate::model::NameLocation::Top => {
                            let bottom = r_screen.top() - 2.0 * font_scale;
                            rects.push(Rect::from_min_size(
                                Pos2::new(center_x - max_w * 0.5, bottom - total_h),
                                eframe::egui::vec2(max_w, total_h),
                            ));
                        }
                        crate::model::NameLocation::Left => {
                            let y_start = r_screen.center().y - total_h * 0.5;
                            let gap = 2.0 * font_scale;
                            let x_right = r_screen.left() - gap;
                            rects.push(Rect::from_min_size(
                                Pos2::new(x_right - max_w, y_start),
                                eframe::egui::vec2(max_w, total_h),
                            ));
                        }
                        crate::model::NameLocation::Right => {
                            let y_start = r_screen.center().y - total_h * 0.5;
                            let gap = 2.0 * font_scale;
                            let x_left = r_screen.right() + gap;
                            rects.push(Rect::from_min_size(
                                Pos2::new(x_left, y_start),
                                eframe::egui::vec2(max_w, total_h),
                            ));
                        }
                    }

                    collidable_obstacle_rects.extend(rects);

                    match b.name_location {
                        crate::model::NameLocation::Bottom => {
                            let mut y = r_screen.bottom() + 2.0 * font_scale;
                            for line in &best_lines {
                                let pos = Pos2::new(center_x, y);
                                painter.text(pos, Align2::CENTER_TOP, line, font.clone(), color);
                                y += line_height;
                            }
                        }
                        crate::model::NameLocation::Top => {
                            // Mirror of bottom: keep the first line closest to the block.
                            let mut y = r_screen.top() - 2.0 * font_scale;
                            // Reverse lines so the first line is highest (furthest from block)
                            for line in best_lines.iter().rev() {
                                let pos = Pos2::new(center_x, y);
                                painter.text(pos, Align2::CENTER_BOTTOM, line, font.clone(), color);
                                y -= line_height;
                            }
                        }
                        crate::model::NameLocation::Left => {
                            let total_h = (best_lines.len() as f32) * line_height;
                            let mut y = r_screen.center().y - total_h * 0.5;
                            let gap = 2.0 * font_scale;
                            let x_right = r_screen.left() - gap;
                            for line in &best_lines {
                                let galley = painter.layout_no_wrap(line.to_string(), font.clone(), color);
                                let pos = Pos2::new(x_right - galley.size().x, y);
                                painter.galley(pos, galley, color);
                                y += line_height;
                            }
                        }
                        crate::model::NameLocation::Right => {
                            let total_h = (best_lines.len() as f32) * line_height;
                            let mut y = r_screen.center().y - total_h * 0.5;
                            let x = r_screen.right() + 2.0 * font_scale;
                            for line in &best_lines {
                                let galley = painter.layout_no_wrap(line.to_string(), font.clone(), color);
                                let pos = Pos2::new(x + 2.0 * font_scale, y);
                                painter.galley(pos, galley, color);
                                y += line_height;
                            }
                        }
                    }
                }
            }
        }

        // Draw port labels
        let mut seen_port_labels: std::collections::HashSet<(String, u32, bool, i32)> =
            Default::default();
        for (sid, index, is_input, y) in port_label_requests {
            let key = (sid.clone(), index, is_input, y.round() as i32);
            if !seen_port_labels.insert(key) {
                continue;
            }
            let Some(brect) = sid_screen_map.get(&sid).copied() else {
                continue;
            };
            let Some(block) = blocks.iter().find_map(|(b, _)| {
                if b.sid.as_ref() == Some(&sid) {
                    Some(*b)
                } else {
                    None
                }
            }) else {
                continue;
            };
            // Do not show port labels if block has a mask
            if should_use_mask_display(block) {
                continue;
            }
            let cfg = get_block_type_cfg(block);
            if (is_input && !cfg.show_input_port_labels)
                || (!is_input && !cfg.show_output_port_labels)
            {
                continue;
            }
            let mirrored = block.block_mirror.unwrap_or(false);
            let pname = port_label_display_name(block, index, is_input, &cfg);
            let avail_w = (brect.width() - 8.0 * font_scale).max(1.0);
            let font_id = egui::FontId::proportional(view_transform::shared_canvas_text_font_px(
                font_scale,
                app.block_name_font_factor,
            ));
            let galley = ui.painter().layout_no_wrap(
                pname,
                font_id.clone(),
                Color32::from_rgb(40, 40, 40),
            );
            let size = galley.size();
            if size.x <= avail_w {
                let half_h = size.y * 0.5;
                let y_min = brect.top();
                let y_max = (brect.bottom() - size.y).max(y_min);
                let y_top = (y - half_h).max(y_min).min(y_max);
                let pos = if is_input ^ mirrored {
                    Pos2::new(brect.left() + 4.0 * font_scale, y_top)
                } else {
                    Pos2::new(brect.right() - 4.0 * font_scale - size.x, y_top)
                };
                painter.galley(pos, galley, Color32::from_rgb(40, 40, 40));
            }
        }

        // Deferred liveplot rendering for Scope/DashboardScope blocks.
        // This runs after the painter borrow is no longer needed so we can
        // use `ui` mutably via `scope_builder`.
        #[cfg(feature = "dashboard")]
        for (scope_key, scope_title, scope_rect) in &deferred_scope_rects {
            let visible_scope_rect = scope_rect.intersect(avail);
            if visible_scope_rect.width() <= 0.0 || visible_scope_rect.height() <= 0.0 {
                continue;
            }
            let mut scope_clicked = false;
            ui.scope_builder(
                egui::UiBuilder::new().max_rect(visible_scope_rect),
                |child_ui| {
                    child_ui.set_clip_rect(visible_scope_rect);
                    let mut scopes = app.scope_instances.lock().unwrap();
                    let scope = scopes.entry(scope_key.clone()).or_insert_with(|| {
                        crate::egui_app::scope_widget::MiniScope::new((
                            app.instance_id,
                            "embedded_scope",
                            scope_key.as_str(),
                        ))
                    });
                    child_ui.push_id(("embedded_scope_ui", scope_key.as_str()), |child_ui| {
                        scope.show_embedded(child_ui, app.instance_id, scope_key);
                        scope_clicked = child_ui.input(|input| {
                            input.pointer.button_clicked(egui::PointerButton::Primary)
                                && input
                                    .pointer
                                    .interact_pos()
                                    .map(|pos| visible_scope_rect.contains(pos))
                                    .unwrap_or(false)
                        });
                    });
                },
            );

            if scope_clicked {
                app.scope_popout = Some(crate::egui_app::state::ScopePopout {
                    title: scope_title.clone(),
                    scope_key: scope_key.clone(),
                    open: true,
                });
            }
        }

        // Deferred inline TextEdit for Constant blocks (after painter borrow ends).
        #[cfg(feature = "dashboard")]
        for (sid, edit_rect) in &deferred_constant_edits {
            if let Some(val) = app.constant_edits.get_mut(sid) {
                let inner = edit_rect.shrink(2.0);
                ui.scope_builder(
                    egui::UiBuilder::new().max_rect(inner),
                    |child_ui| {
                        let te = egui::TextEdit::singleline(val)
                            .desired_width(inner.width())
                            .horizontal_align(egui::Align::Center)
                            .font(egui::TextStyle::Body);
                        child_ui.add(te);
                    },
                );
            }
        }
        // Restore cache data moved out before the closure (zero-clone round-trip).
        app.view_cache.cached_sys_lines = sys_lines;
        app.view_cache.cached_sys_annotations = sys_annotations;
        app.view_cache.cached_owned_blocks = owned_blocks;
        app.view_cache.cached_subsystem_block_lookup = subsystem_block_lookup;
        app.view_cache.line_colors = line_colors;
        app.view_cache.port_counts = port_counts;
        app.view_cache.connected_ports = connected_ports;
    });

    // After the UI closure, call open_block_if_subsystem if needed
    if let Some(ref block) = block_to_open_subsystem {
        app.open_block_if_subsystem(block);
    }

    let navigated = navigate_to.is_some() || block_to_open_subsystem.is_some();
    if let Some(p) = navigate_to {
        app.navigate_to_path(p);
    }
    if !navigated {
        app.zoom = staged_zoom;
        app.pan = staged_pan;
        app.reset_view = staged_reset;
        app.view_bounds = staged_view_bounds;
        app.remember_current_navigation_view_state();
    }
    if clear_search {
        app.search_query.clear();
        app.search_matches.clear();
    }

    interaction
}

fn draw_viewer_resize_handles(
    ui: &mut egui::Ui,
    r_screen: &Rect,
    sid: &str,
    model_rect: &Rect,
    app: &mut SubsystemApp,
    scale: f32,
) {
    let handles = view_transform::resize_handle_positions(r_screen);
    for (pos, handle_id) in handles {
        let handle_rect = Rect::from_center_size(pos, Vec2::splat(10.0));
        let resp = ui.allocate_rect(handle_rect, Sense::click_and_drag());
        let color = if resp.dragged() || resp.hovered() {
            Color32::from_rgb(80, 180, 255)
        } else {
            Color32::from_rgb(0, 120, 255)
        };
        ui.painter()
            .rect_filled(handle_rect.shrink(2.0), 1.0, color);
        ui.painter().rect_stroke(
            handle_rect.shrink(2.0),
            1.0,
            Stroke::new(1.0_f32, Color32::WHITE),
            egui::StrokeKind::Outside,
        );
        if resp.drag_started() {
            app.viewer_drag_state = ViewerDragState::Resize {
                sid: sid.to_string(),
                handle: handle_id,
                original_l: model_rect.left().round() as i32,
                original_t: model_rect.top().round() as i32,
                original_r: model_rect.right().round() as i32,
                original_b: model_rect.bottom().round() as i32,
                current_dx: 0,
                current_dy: 0,
            };
        }
        if resp.dragged() {
            let dx = (resp.drag_delta().x / scale).round() as i32;
            let dy = (resp.drag_delta().y / scale).round() as i32;
            if let ViewerDragState::Resize {
                sid: resize_sid,
                current_dx,
                current_dy,
                ..
            } = &mut app.viewer_drag_state
                && resize_sid == sid
            {
                *current_dx += dx;
                *current_dy += dy;
                ui.ctx().request_repaint();
            }
        }
        if resp.drag_stopped()
            && let ViewerDragState::Resize {
                sid: resize_sid,
                handle,
                original_l,
                original_t,
                original_r,
                original_b,
                current_dx,
                current_dy,
            } = app.viewer_drag_state.clone()
            && resize_sid == sid
        {
            let (nl, nt, nr, nb) = view_transform::compute_resized_rect(
                original_l as f32,
                original_t as f32,
                original_r as f32,
                original_b as f32,
                handle,
                current_dx as f32,
                current_dy as f32,
            );
            let mut layout_changed = false;
            let mut undo_cmd = None;
            if let Some(system) = app.current_system_mut()
                && let Some(block_index) = system
                    .blocks
                    .iter()
                    .position(|block| block.sid.as_deref() == Some(sid))
            {
                undo_cmd = Some(operations::resize_block(
                    system,
                    block_index,
                    nl,
                    nt,
                    nr,
                    nb,
                ));
                layout_changed = true;
            }
            if let Some(cmd) = undo_cmd {
                app.viewer_history.push(cmd);
            }
            if layout_changed {
                app.layout_dirty = true;
                app.view_cache.invalidate();
            }
            app.viewer_drag_state = ViewerDragState::None;
        }
    }
}

/// Draw a lightweight static sine waveform glyph inside a block rectangle.
pub(crate) fn paint_scope_glyph(painter: &egui::Painter, rect: &Rect) {
    let inner = rect.shrink(6.0);
    if inner.width() < 10.0 || inner.height() < 10.0 {
        return;
    }
    painter.rect_filled(inner, 2.0, Color32::from_rgb(30, 30, 30));
    let color = Color32::from_rgb(50, 200, 50);
    let stroke = Stroke::new(1.5_f32, color);
    let n = 40;
    let mut pts = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / (n - 1) as f32;
        let x = inner.left() + t * inner.width();
        let y =
            inner.center().y - (t * 2.0 * std::f32::consts::PI * 2.0).sin() * inner.height() * 0.35;
        pts.push(Pos2::new(x, y));
    }
    for w in pts.windows(2) {
        painter.line_segment([w[0], w[1]], stroke);
    }
}

// Print connected block/signal information for a dashboard UI block.
///
/// Dashboard blocks do not use traditional signal lines; they use
/// `BindingPersistence` references to `.mxarray` files that describe
/// which block parameter they write to or which signal they read.
///
/// The resolved binding is stored in `Block::dashboard_binding` during
/// archive loading.
///
/// For blocks that use traditional signal lines instead of BindingPersistence
/// (e.g., `Display`, `Scope`), this function falls back to scanning the
/// current subsystem's lines for connections to/from this block.
fn print_dashboard_connected_signals(block: &crate::model::Block, lines: &[crate::model::Line]) {
    println!(
        "  [Dashboard UI] Block '{}' (type: {})",
        block.name, block.block_type
    );

    match &block.dashboard_binding {
        Some(crate::model::DashboardBinding::ParamSource {
            block_path,
            param_name,
            uuid,
            ..
        }) => {
            println!(
                "    → writes param '{}' on block '{}' (uuid: {})",
                param_name, block_path, uuid
            );
        }
        Some(crate::model::DashboardBinding::SignalSpec {
            block_path,
            signal_name,
            uuid,
            ..
        }) => {
            println!(
                "    ← reads signal '{}' from block '{}' (uuid: {})",
                signal_name, block_path, uuid
            );
        }
        None => {
            if matches!(block.block_type.as_str(), "Display" | "Scope") {
                println!(
                    "    · line-based block (no BindingPersistence expected) for SID {}",
                    block.sid.as_deref().unwrap_or("<none>")
                );
            } else {
                println!(
                    "    · dashboard binding missing or unresolved for SID {}",
                    block.sid.as_deref().unwrap_or("<none>")
                );
                print_dashboard_binding_debug(block);
            }
            // Fall back to line-based connection scanning
            print_line_based_connections(block, &[], lines);
        }
    }
}

fn print_dashboard_binding_debug(block: &crate::model::Block) {
    let binding_persistence = block.properties.get("BindingPersistence");
    let has_binding_ref =
        binding_persistence.is_some() || block.ref_properties.contains("BindingPersistence");
    println!(
        "    · tag={} library_source={:?} library_block_path={:?}",
        block.tag_name, block.library_source, block.library_block_path
    );
    println!(
        "    · BindingPersistence present: {}{}",
        if has_binding_ref { "yes" } else { "no" },
        binding_persistence
            .map(|value| format!(" ({value})"))
            .unwrap_or_default()
    );

    let property_debug = collect_dashboard_debug_properties(&block.properties);
    if property_debug.is_empty() {
        println!("    · no binding-related block properties found");
    } else {
        println!(
            "    · binding-related block properties: {}",
            property_debug.join(", ")
        );
    }

    let instance_debug = block
        .instance_data
        .as_ref()
        .map(|instance_data| collect_dashboard_debug_properties(&instance_data.properties))
        .unwrap_or_default();
    if instance_debug.is_empty() {
        println!("    · no binding-related instance-data properties found");
    } else {
        println!(
            "    · binding-related instance-data properties: {}",
            instance_debug.join(", ")
        );
    }

    let ref_debug: Vec<&str> = block
        .ref_properties
        .iter()
        .filter(|name| is_dashboard_debug_property(name))
        .map(|name| name.as_str())
        .collect();
    if ref_debug.is_empty() {
        println!("    · no binding-related ref-backed properties found");
    } else {
        println!(
            "    · binding-related ref-backed properties: {}",
            ref_debug.join(", ")
        );
    }
}

fn collect_dashboard_debug_properties(
    properties: &indexmap::IndexMap<String, String>,
) -> Vec<String> {
    properties
        .iter()
        .filter(|(name, _)| is_dashboard_debug_property(name))
        .take(10)
        .map(|(name, value)| format!("{name}={value}"))
        .collect()
}

fn is_dashboard_debug_property(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("binding")
        || lower.contains("uuid")
        || lower.contains("signal")
        || lower.contains("param")
        || lower.contains("dashboard")
        || lower.contains("widget")
        || lower.contains("hmi")
        || lower == "sid"
        || lower.contains("element")
        || lower.contains("ref")
}

#[cfg(feature = "dashboard")]
fn dashboard_live_value(app: &SubsystemApp, block: &crate::model::Block) -> Option<f64> {
    let binding = block.dashboard_binding.as_ref()?;
    let entry = app.live_values.get(binding.uuid())?;
    let selector_index = match binding {
        crate::model::DashboardBinding::ParamSource { target_path, .. } => {
            target_path.element_index_zero_based()
        }
        crate::model::DashboardBinding::SignalSpec { .. } => None,
    };

    selector_index
        .filter(|_| entry.value.data.len() > 1)
        .and_then(|index| entry.f64_at(index))
        .or_else(|| entry.first_f64())
}

#[cfg(feature = "dashboard")]
fn dashboard_input_control_kind(block: &crate::model::Block) -> Option<&'static str> {
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
fn dashboard_widget_value(app: &SubsystemApp, block: &crate::model::Block) -> f64 {
    let dashboard_value = dashboard_live_value(app, block);
    let block_value = app
        .live_block_values
        .get(&app.live_value_key_for_block(block))
        .and_then(crate::live_values::LiveValueEntry::first_f64);

    dashboard_value
        .or(block_value)
        .or_else(|| block_live_text(app, block).and_then(|value| value.trim().parse::<f64>().ok()))
        .or_else(|| {
            block
                .current_setting
                .as_ref()
                .and_then(|value| value.trim().parse::<f64>().ok())
        })
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
        .unwrap_or_else(|| match dashboard_input_control_kind(block) {
            Some("scalar") => dashboard_scalar_range(block).0,
            _ => 0.0,
        })
}

#[cfg(feature = "dashboard")]
fn dashboard_scalar_range(block: &crate::model::Block) -> (f64, f64) {
    fn parse_property(block: &crate::model::Block, keys: &[&str]) -> Option<f64> {
        keys.iter().find_map(|key| {
            block
                .properties
                .get(*key)
                .and_then(|value| value.trim().parse::<f64>().ok())
        })
    }

    let min = parse_property(block, &["Minimum", "ScaleMin", "LowerLimit", "Min"]);
    let max = parse_property(block, &["Maximum", "ScaleMax", "UpperLimit", "Max"]);
    let min = min.unwrap_or(0.0);
    let max = max.unwrap_or(100.0);
    if min < max { (min, max) } else { (0.0, 100.0) }
}

/// Scan lines in the current subsystem for connections to/from the given block.
fn print_line_based_connections(
    block: &crate::model::Block,
    blocks: &[crate::model::Block],
    lines: &[crate::model::Line],
) {
    let block_sid = match &block.sid {
        Some(s) => s.as_str(),
        None => {
            println!("    (no SID — cannot scan line connections)");
            return;
        }
    };

    // Helper: collect all destination SIDs from a line (including branches).
    fn collect_dst_sids(line: &crate::model::Line) -> Vec<&str> {
        let mut sids = Vec::new();
        if let Some(ref dst) = line.dst {
            sids.push(dst.sid.as_str());
        }
        fn branch_dsts<'a>(branches: &'a [crate::model::Branch], acc: &mut Vec<&'a str>) {
            for b in branches {
                if let Some(ref dst) = b.dst {
                    acc.push(dst.sid.as_str());
                }
                branch_dsts(&b.branches, acc);
            }
        }
        branch_dsts(&line.branches, &mut sids);
        sids
    }

    // Build a SID→name lookup for blocks in this subsystem.
    let block_name_by_sid: std::collections::HashMap<&str, &str> = blocks
        .iter()
        .filter_map(|b| b.sid.as_deref().map(|s| (s, b.name.as_str())))
        .collect();

    let mut found_any = false;

    for line in lines {
        let signal_name = line.name.as_deref().unwrap_or("<unnamed>");

        // Check if this block is a source of the line
        if let Some(ref src) = line.src
            && src.sid == block_sid
        {
            let dst_sids = collect_dst_sids(line);
            for dsid in &dst_sids {
                let dst_name = block_name_by_sid.get(dsid).copied().unwrap_or("?");
                println!(
                    "    → drives signal '{}' to block '{}' (dst SID {})",
                    signal_name, dst_name, dsid
                );
                found_any = true;
            }
        }

        // Check if this block is a destination of the line
        let all_dsts = collect_dst_sids(line);
        if all_dsts.contains(&block_sid)
            && let Some(ref src) = line.src
        {
            let src_name = block_name_by_sid
                .get(src.sid.as_str())
                .copied()
                .unwrap_or("?");
            println!(
                "    ← receives signal '{}' from block '{}' (src SID {})",
                signal_name, src_name, src.sid
            );
            found_any = true;
        }
    }

    if !found_any {
        println!("    (no signal-line connections found in current subsystem)");
    }
}

#[cfg(all(test, feature = "dashboard"))]
mod tests {
    use super::{
        dashboard_input_control_kind, dashboard_live_value, dashboard_widget_value,
        line_has_testpoint, line_stroke_width, line_testpoint_marker_position,
        manual_switch_setting_from_live_value, resolved_line_label, should_render_live_text,
        should_use_mask_display,
    };
    use crate::connection_targets::{
        ConnectionTarget, ConnectionTargetOrigin, ConnectionTargetResolve, ConnectionTargetResolver,
    };
    use crate::egui_app::SubsystemApp;
    use crate::egui_app::dashboard_widgets::dashboard_scalar_value_from_pointer;
    use crate::egui_app::shared_canvas_text_font_px;
    use crate::model::{DashboardBinding, DashboardTargetPath, EndpointRef, Line, Port, System};
    use eframe::egui::{Pos2, Rect};
    use std::collections::BTreeMap;

    #[test]
    fn dashboard_blocks_do_not_fall_back_to_live_text() {
        assert!(!should_render_live_text(true, "PushButtonBlock"));
        assert!(!should_render_live_text(true, "ComboBox"));
        assert!(!should_render_live_text(true, "DisplayBlock"));
    }

    #[test]
    fn non_dashboard_display_blocks_keep_live_text() {
        assert!(!should_render_live_text(true, "Gain"));
        assert!(should_render_live_text(true, "Constant"));
        assert!(should_render_live_text(true, "Display"));
        assert!(!should_render_live_text(false, "Display"));
        assert!(!should_render_live_text(true, "ManualSwitch"));
    }

    #[test]
    fn manual_switch_live_values_map_to_expected_setting() {
        assert_eq!(manual_switch_setting_from_live_value(0.0), "0");
        assert_eq!(manual_switch_setting_from_live_value(0.49), "0");
        assert_eq!(manual_switch_setting_from_live_value(0.5), "1");
        assert_eq!(manual_switch_setting_from_live_value(1.0), "1");
    }

    #[test]
    fn dashboard_param_source_uses_selected_vector_element_for_live_value() {
        let root = System {
            properties: indexmap::IndexMap::new(),
            blocks: Vec::new(),
            lines: Vec::new(),
            annotations: Vec::new(),
            chart: None,
        };
        let mut app = SubsystemApp::new(root, Vec::new(), BTreeMap::new(), BTreeMap::new());
        let mut block = minimal_block("Slider2", "42");
        block.block_type = "SliderBlock".to_string();
        block.dashboard_binding = Some(DashboardBinding::ParamSource {
            block_path: "Model/Slider_Vector".to_string(),
            param_name: "Value".to_string(),
            target_path: DashboardTargetPath {
                port_index: None,
                sub_path: None,
                element: None,
                element_raw_input: Some("(2)".to_string()),
            },
            uuid: "uuid-slider-2".to_string(),
        });
        app.live_values.insert(
            "uuid-slider-2".to_string(),
            crate::live_values::LiveValueEntry::new(crate::live_values::LiveValue::new(
                vec![3],
                crate::live_values::LiveValueList::Float64(vec![10.0, 20.0, 30.0]),
            )),
        );

        assert_eq!(dashboard_live_value(&app, &block), Some(20.0));
        assert_eq!(dashboard_widget_value(&app, &block), 20.0);
    }

    #[test]
    fn dashboard_widget_value_prefers_selector_aware_binding_over_block_live_value() {
        let root = System {
            properties: indexmap::IndexMap::new(),
            blocks: Vec::new(),
            lines: Vec::new(),
            annotations: Vec::new(),
            chart: None,
        };
        let mut app = SubsystemApp::new(root, Vec::new(), BTreeMap::new(), BTreeMap::new());
        let mut block = minimal_block("Slider3", "43");
        block.block_type = "SliderBlock".to_string();
        block.dashboard_binding = Some(DashboardBinding::ParamSource {
            block_path: "Model/Slider_Vector".to_string(),
            param_name: "Value".to_string(),
            target_path: DashboardTargetPath {
                port_index: None,
                sub_path: None,
                element: None,
                element_raw_input: Some("(3)".to_string()),
            },
            uuid: "uuid-slider-3".to_string(),
        });
        app.live_values.insert(
            "uuid-slider-3".to_string(),
            crate::live_values::LiveValueEntry::new(crate::live_values::LiveValue::new(
                vec![3],
                crate::live_values::LiveValueList::Float64(vec![10.0, 20.0, 30.0]),
            )),
        );
        app.live_block_values.insert(
            app.live_value_key_for_block(&block),
            crate::live_values::LiveValueEntry::new(crate::live_values::LiveValue::new(
                vec![3],
                crate::live_values::LiveValueList::Float64(vec![10.0, 20.0, 30.0]),
            )),
        );

        assert_eq!(dashboard_widget_value(&app, &block), 30.0);
    }

    #[test]
    fn dashboard_discrete_controls_are_editable_in_live_mode() {
        let combo = crate::model::Block {
            block_type: "ComboBox".to_string(),
            name: "Combo".to_string(),
            sid: None,
            tag_name: "Block".to_string(),
            position: None,
            zorder: None,
            commented: false,
            name_location: crate::model::NameLocation::default(),
            is_matlab_function: false,
            value: None,
            value_kind: crate::model::ValueKind::default(),
            value_rows: None,
            value_cols: None,
            properties: indexmap::IndexMap::new(),
            ref_properties: std::collections::BTreeSet::new(),
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
            dashboard_binding: Some(DashboardBinding::ParamSource {
                block_path: "Model/Combo".to_string(),
                param_name: "Value".to_string(),
                target_path: DashboardTargetPath::default(),
                uuid: "uuid-combo".to_string(),
            }),
            child_order: Vec::new(),
        };
        let mut radio = combo.clone();
        radio.block_type = "RadioButtonGroup".to_string();

        assert_eq!(dashboard_input_control_kind(&combo), Some("discrete"));
        assert_eq!(dashboard_input_control_kind(&radio), Some("discrete"));
    }

    #[test]
    fn rotary_switch_pointer_mapping_clamps_gap_and_returns_indices() {
        let mut block = crate::model::Block {
            block_type: "RotarySwitchBlock".to_string(),
            name: "Rotary".to_string(),
            sid: None,
            tag_name: "Block".to_string(),
            position: None,
            zorder: None,
            commented: false,
            name_location: crate::model::NameLocation::default(),
            is_matlab_function: false,
            value: None,
            value_kind: crate::model::ValueKind::default(),
            value_rows: None,
            value_cols: None,
            properties: indexmap::IndexMap::new(),
            ref_properties: std::collections::BTreeSet::new(),
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
            dashboard_binding: None,
            child_order: Vec::new(),
        };
        block
            .properties
            .insert("Values".to_string(), "Low,Medium,High".to_string());

        let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 100.0));
        let center = rect.center();

        let min_value = dashboard_scalar_value_from_pointer(
            &block,
            rect,
            Pos2::new(center.x - 10.0, rect.bottom() - 10.0),
            1.0,
        );
        let max_value = dashboard_scalar_value_from_pointer(
            &block,
            rect,
            Pos2::new(center.x + 10.0, rect.bottom() - 10.0),
            1.0,
        );
        let mid_value = dashboard_scalar_value_from_pointer(
            &block,
            rect,
            Pos2::new(center.x, rect.top() + 5.0),
            1.0,
        );

        assert_eq!(min_value, 0.0);
        assert_eq!(max_value, 2.0);
        assert_eq!(mid_value, 1.0);
    }

    #[test]
    fn line_testpoint_follows_source_output_port() {
        let mut source = minimal_block("Source", "1");
        source.ports.push(Port {
            port_type: "out".to_string(),
            index: Some(1),
            properties: indexmap::IndexMap::from_iter([(
                "TestPoint".to_string(),
                "on".to_string(),
            )]),
        });
        let sink = minimal_block("Sink", "2");
        let line = Line {
            name: None,
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
            points: Vec::new(),
            labels: None,
            branches: Vec::new(),
            properties: indexmap::IndexMap::new(),
        };

        let system = System {
            properties: indexmap::IndexMap::from_iter([("Name".to_string(), "Model".to_string())]),
            blocks: vec![source, sink],
            lines: vec![line.clone()],
            annotations: Vec::new(),
            chart: None,
        };

        let resolver = ConnectionTargetResolver::new(&system);
        let targets = resolver.line_targets_for_line(&[], &line);
        assert!(line_has_testpoint(&targets));
    }

    #[test]
    fn unnamed_line_label_stays_empty_without_explicit_line_name() {
        let mut source = minimal_block("Source Block", "1");
        source.ports.push(Port {
            port_type: "out".to_string(),
            index: Some(1),
            properties: indexmap::IndexMap::from_iter([(
                "Name".to_string(),
                "Shared Signal".to_string(),
            )]),
        });
        let sink = minimal_block("Sink", "2");
        let line = Line {
            name: None,
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
            points: Vec::new(),
            labels: None,
            branches: Vec::new(),
            properties: indexmap::IndexMap::new(),
        };

        let system = System {
            properties: indexmap::IndexMap::from_iter([("Name".to_string(), "Model".to_string())]),
            blocks: vec![source, sink],
            lines: vec![line.clone()],
            annotations: Vec::new(),
            chart: None,
        };

        let resolver = ConnectionTargetResolver::new(&system);
        let targets = resolver.line_targets_for_line(&[], &line);
        assert_eq!(resolved_line_label(&line, &targets), None);
    }

    #[test]
    fn bus_and_mux_lines_do_not_draw_propagated_fallback_names() {
        let line = Line {
            name: None,
            zorder: None,
            src: None,
            dst: None,
            points: Vec::new(),
            labels: None,
            branches: Vec::new(),
            properties: indexmap::IndexMap::new(),
        };

        let bus_targets = vec![
            ConnectionTarget {
                path: "Model/A".to_string(),
                signal_name: Some("alpha".to_string()),
                signal_names: Vec::new(),
                resolve: Some(ConnectionTargetResolve::Signal("alpha".to_string())),
                element_index: None,
                origin: ConnectionTargetOrigin::BusCreator,
                signals_only: true,
                testpoint: false,
                block_type: None,
            },
            ConnectionTarget {
                path: "Model/B".to_string(),
                signal_name: Some("beta".to_string()),
                signal_names: Vec::new(),
                resolve: Some(ConnectionTargetResolve::Signal("beta".to_string())),
                element_index: None,
                origin: ConnectionTargetOrigin::BusCreator,
                signals_only: true,
                testpoint: false,
                block_type: None,
            },
        ];
        let mux_targets = vec![ConnectionTarget {
            path: "Model/A".to_string(),
            signal_name: Some("alpha".to_string()),
            signal_names: Vec::new(),
            resolve: Some(ConnectionTargetResolve::Index(1)),
            element_index: Some(1),
            origin: ConnectionTargetOrigin::Mux,
            signals_only: true,
            testpoint: false,
            block_type: None,
        }];

        assert_eq!(resolved_line_label(&line, &bus_targets), None);
        assert_eq!(resolved_line_label(&line, &mux_targets), None);
    }

    #[test]
    fn bus_lines_draw_thicker_than_normal_lines() {
        let normal_targets = vec![ConnectionTarget {
            path: "Model/A".to_string(),
            signal_name: Some("alpha".to_string()),
            signal_names: Vec::new(),
            resolve: Some(ConnectionTargetResolve::Signal("alpha".to_string())),
            element_index: None,
            origin: ConnectionTargetOrigin::SourceBlock,
            signals_only: true,
            testpoint: false,
            block_type: None,
        }];
        let bus_targets = vec![
            ConnectionTarget {
                path: "Model/A".to_string(),
                signal_name: Some("alpha".to_string()),
                signal_names: Vec::new(),
                resolve: Some(ConnectionTargetResolve::Signal("alpha".to_string())),
                element_index: None,
                origin: ConnectionTargetOrigin::BusCreator,
                signals_only: true,
                testpoint: false,
                block_type: None,
            },
            ConnectionTarget {
                path: "Model/B".to_string(),
                signal_name: Some("beta".to_string()),
                signal_names: Vec::new(),
                resolve: Some(ConnectionTargetResolve::Signal("beta".to_string())),
                element_index: None,
                origin: ConnectionTargetOrigin::BusCreator,
                signals_only: true,
                testpoint: false,
                block_type: None,
            },
        ];

        assert!(line_stroke_width(&bus_targets, false) > line_stroke_width(&normal_targets, false));
    }

    #[test]
    fn line_testpoint_marker_uses_first_visible_segment() {
        let marker = line_testpoint_marker_position(&[
            Pos2::new(10.0, 10.0),
            Pos2::new(50.0, 10.0),
            Pos2::new(50.0, 40.0),
        ])
        .expect("marker position");

        assert!(marker.x > 10.0);
        assert_eq!(marker.y, 10.0);
    }

    #[test]
    fn mask_without_display_text_falls_back_to_normal_rendering() {
        let mut block = minimal_block("Masked", "3");
        block.mask = Some(crate::model::Mask::default());

        assert!(!should_use_mask_display(&block));

        block.mask_display_text = Some(" ".to_string());
        assert!(!should_use_mask_display(&block));

        block.mask_display_text = Some("mask label".to_string());
        assert!(should_use_mask_display(&block));
    }

    #[test]
    fn aux_label_font_scaling_uses_name_controls() {
        let scaled = shared_canvas_text_font_px(0.5, 2.0);
        assert_eq!(scaled, 32.0);

        let scaled_down = shared_canvas_text_font_px(0.5, 0.5);
        assert_eq!(scaled_down, 8.0);
    }

    fn minimal_block(name: &str, sid: &str) -> crate::model::Block {
        crate::model::Block {
            block_type: "Constant".to_string(),
            name: name.to_string(),
            sid: Some(sid.to_string()),
            tag_name: "Block".to_string(),
            position: None,
            zorder: None,
            commented: false,
            name_location: crate::model::NameLocation::default(),
            is_matlab_function: false,
            value: None,
            value_kind: crate::model::ValueKind::default(),
            value_rows: None,
            value_cols: None,
            properties: indexmap::IndexMap::new(),
            ref_properties: std::collections::BTreeSet::new(),
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
            dashboard_binding: None,
            child_order: Vec::new(),
        }
    }
}
