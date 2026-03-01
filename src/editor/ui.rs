//! Editor UI — the interactive egui interface for model editing.
//!
//! This module provides the main rendering and interaction functions for
//! the Simulink model editor. It extends the viewer UI with:
//!
//! - Block dragging with arrow-key support
//! - Connection drawing with auto-snap to ports
//! - Rectangle selection of blocks and lines
//! - Block browser popup (hotkey "A")
//! - Context menus for blocks, lines, and canvas
//! - Code editor for MATLAB Function / CFunction blocks
//! - Keyboard shortcuts (Ctrl+Z/Y, Delete, Ctrl+C/V, R, M, etc.)
//! - Grid overlay

#![cfg(feature = "egui")]

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use eframe::egui::{self, Align2, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};

use crate::model::EndpointRef;

use crate::egui_app::{
    BlockDialog, SignalDialog, endpoint_pos_maybe_mirrored,
    endpoint_pos_with_target_maybe_mirrored, get_block_type_cfg, highlight_query_job,
    parse_block_rect, parse_rect_str, render_block_icon, wrap_text_to_max_width,
};

use super::operations;
use super::state::{DragMode, EditorState};

// ────────────────────────────────────────────────────────────────────────────
// Color utilities (cloned from viewer for independence)
// ────────────────────────────────────────────────────────────────────────────

fn luminance(c: Color32) -> f32 {
    fn to_lin(u: u8) -> f32 {
        let s = (u as f32) / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * to_lin(c.r()) + 0.7152 * to_lin(c.g()) + 0.0722 * to_lin(c.b())
}

pub fn contrast_color(bg: Color32) -> Color32 {
    if luminance(bg) > 0.6 {
        Color32::from_rgb(25, 35, 45)
    } else {
        Color32::from_rgb(235, 245, 245)
    }
}

fn hsv_to_color(h: f32, s: f32, v: f32) -> Color32 {
    let h6 = (h * 6.0) % 6.0;
    let c = v * s;
    let x = c * (1.0 - ((h6 % 2.0) - 1.0).abs());
    let (r1, g1, b1) = if h6 < 1.0 {
        (c, x, 0.0)
    } else if h6 < 2.0 {
        (x, c, 0.0)
    } else if h6 < 3.0 {
        (0.0, c, x)
    } else if h6 < 4.0 {
        (0.0, x, c)
    } else if h6 < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = v - c;
    Color32::from_rgb(
        ((r1 + m) * 255.0) as u8,
        ((g1 + m) * 255.0) as u8,
        ((b1 + m) * 255.0) as u8,
    )
}

pub fn hash_color(input: &str, s: f32, v: f32) -> Color32 {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    let h = (hasher.finish() as f32 / u64::MAX as f32) % 1.0;
    hsv_to_color(h, s, v)
}

fn block_base_color(
    block: &crate::model::Block,
    cfg: &crate::block_types::BlockTypeConfig,
) -> Color32 {
    if let Some(ref color_str) = block.background_color {
        let lower = color_str.to_lowercase();
        match lower.as_str() {
            "yellow" => return Color32::from_rgb(255, 230, 120),
            "red" => return Color32::from_rgb(230, 90, 90),
            "green" => return Color32::from_rgb(120, 210, 140),
            "blue" => return Color32::from_rgb(100, 160, 230),
            "black" => return Color32::from_rgb(40, 40, 40),
            "white" => return Color32::from_rgb(235, 235, 235),
            "gray" | "grey" => return Color32::from_rgb(180, 180, 180),
            _ => {
                if lower.starts_with('#') && lower.len() == 7 {
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        u8::from_str_radix(&lower[1..3], 16),
                        u8::from_str_radix(&lower[3..5], 16),
                        u8::from_str_radix(&lower[5..7], 16),
                    ) {
                        return Color32::from_rgb(r, g, b);
                    }
                }
            }
        }
    }
    if let Some(bg) = cfg.background {
        return Color32::from_rgb(bg.0, bg.1, bg.2);
    }
    hash_color(&block.block_type, 0.35, 0.90)
}

// ────────────────────────────────────────────────────────────────────────────
// Public API
// ────────────────────────────────────────────────────────────────────────────

/// Main update function for the editor, called each frame.
///
/// This is the entry point for rendering the full editor UI inside an
/// `egui::Ui` region (analogous to the viewer's `update` function).
pub fn editor_update(state: &mut EditorState, ui: &mut egui::Ui) {
    editor_update_internal(state, ui);
}

/// Like [`editor_update`] but also shows info windows.
pub fn editor_update_with_info(state: &mut EditorState, ui: &mut egui::Ui) {
    editor_update_internal(state, ui);
    show_block_browser(state, ui);
    show_code_editor(state, ui);
}

// ────────────────────────────────────────────────────────────────────────────
// Internal rendering
// ────────────────────────────────────────────────────────────────────────────

fn editor_update_internal(state: &mut EditorState, ui: &mut egui::Ui) {
    let path_snapshot = state.app.path.clone();

    // Top panel: breadcrumbs + search + edit toolbar
    egui::TopBottomPanel::top("editor_top").show_inside(ui, |ui| {
        ui.horizontal(|ui| {
            let up_label = egui::RichText::new("⬆ Up");
            let up = ui.add_enabled(!path_snapshot.is_empty(), egui::Button::new(up_label));
            if up.clicked() {
                let mut p = path_snapshot.clone();
                p.pop();
                state.app.navigate_to_path(p);
                state.selection.clear();
            }
            ui.separator();
            ui.label(RichText::new("Path:").strong());
            if ui.link("Root").clicked() {
                state.app.navigate_to_path(Vec::new());
                state.selection.clear();
            }
            for (i, name) in path_snapshot.iter().enumerate() {
                ui.label("/");
                if ui.link(name).clicked() {
                    state.app.navigate_to_path(path_snapshot[..=i].to_vec());
                    state.selection.clear();
                }
            }
        });
        // Toolbar row
        ui.horizontal(|ui| {
            // Undo / redo
            let undo_btn = ui.add_enabled(state.history.can_undo(), egui::Button::new("↶ Undo"));
            if undo_btn.clicked() {
                state.undo();
            }
            let redo_btn = ui.add_enabled(state.history.can_redo(), egui::Button::new("↷ Redo"));
            if redo_btn.clicked() {
                state.redo();
            }
            ui.separator();

            let has_selection = !state.selection.is_empty();
            let del_btn = ui.add_enabled(has_selection, egui::Button::new("🗑 Delete"));
            if del_btn.clicked() {
                state.delete_selection();
            }
            let comment_btn = ui.add_enabled(
                !state.selection.selected_blocks.is_empty(),
                egui::Button::new("💬 Comment"),
            );
            if comment_btn.clicked() {
                state.comment_selection();
            }
            let rotate_btn = ui.add_enabled(
                !state.selection.selected_blocks.is_empty(),
                egui::Button::new("🔄 Rotate"),
            );
            if rotate_btn.clicked() {
                state.rotate_selection();
            }
            let mirror_btn = ui.add_enabled(
                !state.selection.selected_blocks.is_empty(),
                egui::Button::new("↔ Mirror"),
            );
            if mirror_btn.clicked() {
                state.mirror_selection();
            }
            ui.separator();

            let copy_btn = ui.add_enabled(
                !state.selection.selected_blocks.is_empty(),
                egui::Button::new("📋 Copy"),
            );
            if copy_btn.clicked() {
                state.copy_selection();
            }
            let paste_btn =
                ui.add_enabled(state.clipboard.has_content(), egui::Button::new("📃 Paste"));
            if paste_btn.clicked() {
                state.paste();
            }
            ui.separator();

            // Grid toggle
            ui.checkbox(&mut state.show_grid, "Grid");
            ui.checkbox(&mut state.snap_to_grid, "Snap");
            ui.add(
                egui::DragValue::new(&mut state.grid_size)
                    .prefix("Grid: ")
                    .speed(1)
                    .range(1..=50),
            );

            ui.separator();
            ui.checkbox(&mut state.app.show_block_names_default, "Block names");
            ui.label("Name size");
            ui.add(
                egui::DragValue::new(&mut state.app.block_name_font_factor)
                    .speed(0.05)
                    .range(0.2..=2.0),
            );

            // Modified indicator
            if state.dirty {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.colored_label(Color32::from_rgb(255, 200, 80), "● Modified");
                });
            }

            // Transient notification
            if let Some((msg, expiry)) = &state.app.transient_notification {
                if std::time::Instant::now() > *expiry {
                    state.app.transient_notification = None;
                } else {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.colored_label(Color32::from_rgb(255, 200, 80), msg);
                    });
                }
            }
        });
        // Search
        ui.horizontal(|ui| {
            let resp = ui.add(
                egui::TextEdit::singleline(&mut state.app.search_query)
                    .hint_text("Search subsystems…"),
            );
            if resp.changed() {
                state.app.update_search_matches();
            }
        });
        if !state.app.search_query.trim().is_empty() && !state.app.search_matches.is_empty() {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for p in state.app.search_matches.clone() {
                            let label = format!("/{}", p.join("/"));
                            let job = highlight_query_job(&label, &state.app.search_query);
                            let resp = ui.add(egui::Label::new(job).sense(Sense::click()));
                            if resp.clicked() {
                                state.app.navigate_to_path(p);
                                state.selection.clear();
                                state.app.search_query.clear();
                                state.app.search_matches.clear();
                            }
                        }
                    });
            });
        }
    });

    // Resolve current system
    let entities_opt = state.app.current_entities();
    if entities_opt.is_none() {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.colored_label(Color32::RED, "Invalid path — nothing to render");
        });
        return;
    }
    let entities = entities_opt.unwrap();
    let system_name: String = state
        .app
        .current_system()
        .and_then(|s| s.properties.get("Name").cloned())
        .or_else(|| path_snapshot.last().cloned())
        .unwrap_or_else(|| "<root>".to_string());

    // Enrich blocks with SystemName
    let mut enriched_blocks: Vec<crate::model::Block> = Vec::with_capacity(entities.blocks.len());
    for b in &entities.blocks {
        let mut bc = b.clone();
        bc.properties
            .entry("SystemName".to_string())
            .or_insert(system_name.clone());
        enriched_blocks.push(bc);
    }
    let blocks: Vec<(&crate::model::Block, Rect)> = enriched_blocks
        .iter()
        .filter_map(|b| parse_block_rect(b).map(|r| (b, r)))
        .collect();
    let annotations: Vec<(&crate::model::Annotation, Rect)> = entities
        .annotations
        .iter()
        .filter_map(|a| {
            a.position
                .as_deref()
                .and_then(|s| parse_rect_str(s))
                .map(|pos| (a, pos))
        })
        .collect();

    if blocks.is_empty() && annotations.is_empty() {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.colored_label(
                Color32::YELLOW,
                "No blocks with positions to render. Press 'A' to add blocks.",
            );
        });
        return;
    }

    // Bounding box
    let mut bb = blocks
        .first()
        .map(|x| x.1)
        .or_else(|| annotations.first().map(|x| x.1))
        .unwrap();
    for (_, r) in &blocks {
        bb = bb.union(*r);
    }
    for (_, r) in &annotations {
        bb = bb.union(*r);
    }

    let margin = 20.0;
    let avail = ui.available_rect_before_wrap();
    let avail_size = avail.size();
    let width = bb.width().max(1.0);
    let height = bb.height().max(1.0);
    let sx = (avail_size.x - 2.0 * margin) / width;
    let sy = (avail_size.y - 2.0 * margin) / height;
    let base_scale = sx.min(sy).max(0.1);

    if state.app.reset_view {
        state.app.zoom = 1.0;
        state.app.pan = Vec2::ZERO;
        state.app.reset_view = false;
    }

    // Central panel rendering
    egui::CentralPanel::default().show_inside(ui, |ui| {
        let avail = ui.available_rect_before_wrap();

        // Canvas interaction
        let canvas_resp = ui.interact(
            avail,
            ui.id().with("editor_canvas"),
            Sense::click_and_drag(),
        );

        // Handle keyboard shortcuts
        handle_keyboard_shortcuts(state, ui, &avail, base_scale, &bb);

        // Zoom with scroll
        let scroll_y = ui.input(|i| i.raw_scroll_delta.y);
        if scroll_y.abs() > 0.0 && canvas_resp.hovered() {
            let factor = (1.0 + scroll_y * 0.001).max(0.1);
            let old_zoom = state.app.zoom;
            let new_zoom = (old_zoom * factor).clamp(0.2, 10.0);
            if (new_zoom - old_zoom).abs() > f32::EPSILON {
                let origin = Pos2::new(avail.left() + margin, avail.top() + margin);
                let s_old = base_scale * old_zoom;
                let s_new = base_scale * new_zoom;
                let cursor = canvas_resp.hover_pos().unwrap_or(avail.center());
                let world_x = (cursor.x - origin.x - state.app.pan.x) / s_old + bb.left();
                let world_y = (cursor.y - origin.y - state.app.pan.y) / s_old + bb.top();
                state.app.zoom = new_zoom;
                state.app.pan.x = cursor.x - ((world_x - bb.left()) * s_new + origin.x);
                state.app.pan.y = cursor.y - ((world_y - bb.top()) * s_new + origin.y);
            }
        }

        let zoom = state.app.zoom;
        let pan = state.app.pan;

        let to_screen = |p: Pos2| -> Pos2 {
            let s = base_scale * zoom;
            let x = (p.x - bb.left()) * s + avail.left() + margin + pan.x;
            let y = (p.y - bb.top()) * s + avail.top() + margin + pan.y;
            Pos2::new(x, y)
        };

        let from_screen = |p: Pos2| -> Pos2 {
            let s = base_scale * zoom;
            let x = (p.x - avail.left() - margin - pan.x) / s + bb.left();
            let y = (p.y - avail.top() - margin - pan.y) / s + bb.top();
            Pos2::new(x, y)
        };

        let font_scale: f32 = (zoom / 2.0).max(0.01);

        // Draw grid
        if state.show_grid {
            draw_grid(
                ui,
                &avail,
                &to_screen,
                &from_screen,
                state.grid_size,
                zoom,
                base_scale,
            );
        }

        // Zoom controls
        egui::Area::new("editor_zoom_controls".into())
            .fixed_pos(Pos2::new(avail.left() + 8.0, avail.top() + 8.0))
            .show(ui.ctx(), |ui| {
                egui::Frame::menu(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.small_button("−").clicked() {
                            state.app.zoom = (state.app.zoom * 0.9).clamp(0.2, 10.0);
                        }
                        if ui.small_button("+").clicked() {
                            state.app.zoom = (state.app.zoom * 1.1).clamp(0.2, 10.0);
                        }
                        if ui.small_button("Reset").clicked() {
                            state.app.reset_view = true;
                        }
                        let percent = (state.app.zoom * 100.0).round() as i32;
                        ui.label(format!("{}%", percent));
                    });
                });
            });

        // Build SID maps
        let mut sid_map: HashMap<String, Rect> = HashMap::new();
        let mut sid_screen_map: HashMap<String, Rect> = HashMap::new();

        // Compute drag offset for live preview
        let drag_offset_model = if let DragMode::Blocks { dx, dy } = &state.drag_mode {
            Some((*dx, *dy))
        } else {
            None
        };

        // Draw blocks
        for (block_idx, (b, r)) in blocks.iter().enumerate() {
            // Compute effective model rect (offset if this block is being dragged)
            let is_selected = state.selection.is_block_selected(block_idx);
            let effective_r = if is_selected {
                if let Some((dx, dy)) = drag_offset_model {
                    Rect::from_min_max(
                        Pos2::new(r.min.x + dx, r.min.y + dy),
                        Pos2::new(r.max.x + dx, r.max.y + dy),
                    )
                } else if let DragMode::Resize {
                    block_index,
                    handle,
                    original_l,
                    original_t,
                    original_r,
                    original_b,
                    dx,
                    dy,
                } = &state.drag_mode
                {
                    if *block_index == block_idx {
                        let new_rect = compute_resized_rect(
                            *original_l as f32,
                            *original_t as f32,
                            *original_r as f32,
                            *original_b as f32,
                            *handle,
                            *dx,
                            *dy,
                            state.grid_size,
                            state.snap_to_grid,
                        );
                        Rect::from_min_max(
                            Pos2::new(new_rect.0, new_rect.1),
                            Pos2::new(new_rect.2, new_rect.3),
                        )
                    } else {
                        *r
                    }
                } else {
                    *r
                }
            } else {
                *r
            };

            if let Some(sid) = &b.sid {
                sid_map.insert(sid.clone(), effective_r);
            }
            let r_screen =
                Rect::from_min_max(to_screen(effective_r.min), to_screen(effective_r.max));
            if let Some(sid) = &b.sid {
                sid_screen_map.insert(sid.clone(), r_screen);
            }
            let cfg = get_block_type_cfg(b);
            let bg = block_base_color(b, &cfg);

            let is_selected = state.selection.is_block_selected(block_idx);

            // Render block
            if b.commented {
                let commented_bg = Color32::from_rgb(230, 230, 230);
                ui.painter().rect_filled(r_screen, 0.0, commented_bg);
                let fg = contrast_color(commented_bg);
                if let Some(lbl) = crate::builtin_libraries::compute_block_instance_label(b) {
                    let font_id = egui::FontId::proportional(12.0 * font_scale);
                    let galley = ui.painter().layout_no_wrap(lbl, font_id, fg);
                    let pos = r_screen.center() - galley.size() * 0.5;
                    ui.painter().galley(pos, galley, fg);
                } else {
                    render_block_icon(ui.painter(), b, &r_screen, font_scale, None);
                }
            } else {
                ui.painter().rect_filled(r_screen, 6.0, bg);
                let fg = contrast_color(bg);
                if let Some(lbl) = crate::builtin_libraries::compute_block_instance_label(b) {
                    let font_id = egui::FontId::proportional(12.0 * font_scale);
                    let galley = ui.painter().layout_no_wrap(lbl, font_id, fg);
                    let pos = r_screen.center() - galley.size() * 0.5;
                    ui.painter().galley(pos, galley, fg);
                } else {
                    render_block_icon(ui.painter(), b, &r_screen, font_scale, None);
                }
            }

            // Selection highlight
            if is_selected {
                ui.painter().rect_stroke(
                    r_screen.expand(2.0),
                    6.0,
                    Stroke::new(2.5, Color32::from_rgb(0, 120, 255)),
                    egui::StrokeKind::Outside,
                );
            }

            // Block label (global default + per-block override).
            let show_name = b.show_name.unwrap_or(state.app.show_block_names_default);
            if show_name {
                let scale = font_scale.max(0.2);
                let chevron_h = (8.0 * scale * 4.0).max(3.0 * 4.0);
                let chevron_w = (6.0 * scale * 4.0).max(2.0 * 4.0);

                let in_count = b.port_counts.as_ref().and_then(|p| p.ins).unwrap_or(0);
                let out_count = b.port_counts.as_ref().and_then(|p| p.outs).unwrap_or(0);
                let mirrored = b.block_mirror.unwrap_or(false);
                let ins_left_side = !mirrored;
                let outs_left_side = mirrored;
                let has_left = (in_count > 0 && ins_left_side) || (out_count > 0 && outs_left_side);
                let has_right =
                    (in_count > 0 && !ins_left_side) || (out_count > 0 && !outs_left_side);
                let left_extra = if has_left { chevron_w } else { 0.0 };
                let right_extra = if has_right { chevron_w } else { 0.0 };
                let overall_w = r_screen.width() + left_extra + right_extra;
                let max_label_w = overall_w * 0.95;

                let font_px = (chevron_h * state.app.block_name_font_factor).max(1.0);
                let label_font = egui::FontId::proportional(font_px);
                let line_height = (font_px * 1.2).max(1.0);
                let fg = if b.commented {
                    Color32::GRAY
                } else {
                    contrast_color(bg)
                };

                let lines =
                    wrap_text_to_max_width(ui.painter(), &b.name, label_font.clone(), max_label_w);
                if !lines.is_empty() {
                    let left = r_screen.left() - left_extra;
                    let right = r_screen.right() + right_extra;
                    let center_x = (left + right) * 0.5;

                    match b.name_location {
                        crate::model::NameLocation::Bottom => {
                            let mut y = r_screen.bottom() + 4.0 * font_scale;
                            for line in &lines {
                                let pos = Pos2::new(center_x, y);
                                ui.painter().text(
                                    pos,
                                    Align2::CENTER_TOP,
                                    line,
                                    label_font.clone(),
                                    fg,
                                );
                                y += line_height;
                            }
                        }
                        crate::model::NameLocation::Top => {
                            let mut y = r_screen.top() - 4.0 * font_scale;
                            for line in &lines {
                                let pos = Pos2::new(center_x, y);
                                ui.painter().text(
                                    pos,
                                    Align2::CENTER_BOTTOM,
                                    line,
                                    label_font.clone(),
                                    fg,
                                );
                                y -= line_height;
                            }
                        }
                        crate::model::NameLocation::Left => {
                            let total_h = (lines.len() as f32) * line_height;
                            let mut y = r_screen.center().y - total_h * 0.5;
                            let gap = 2.0 * font_scale;
                            let x_right = r_screen.left() - gap;
                            for line in &lines {
                                let galley = ui.painter().layout_no_wrap(
                                    line.to_string(),
                                    label_font.clone(),
                                    fg,
                                );
                                let pos = Pos2::new(x_right - galley.size().x, y);
                                ui.painter().galley(pos, galley, fg);
                                y += line_height;
                            }
                        }
                        crate::model::NameLocation::Right => {
                            let total_h = (lines.len() as f32) * line_height;
                            let mut y = r_screen.center().y - total_h * 0.5;
                            let x = r_screen.right() + 2.0 * font_scale;
                            for line in &lines {
                                let galley = ui.painter().layout_no_wrap(
                                    line.to_string(),
                                    label_font.clone(),
                                    fg,
                                );
                                let pos = Pos2::new(x + 2.0 * font_scale, y);
                                ui.painter().galley(pos, galley, fg);
                                y += line_height;
                            }
                        }
                    }
                }
            }

            // Port indicators (with clickable areas for connection dragging)
            draw_port_indicators(ui, b, &r_screen, font_scale);

            // Resize handles for selected blocks
            if is_selected && !matches!(state.drag_mode, DragMode::Blocks { .. }) {
                draw_resize_handles(ui, &r_screen, block_idx, state, &effective_r);
            }

            // Port interaction areas for connection dragging
            draw_port_interaction_areas(ui, b, &r_screen, font_scale, block_idx, state);

            // Allocate interaction rect
            let resp = ui.allocate_rect(r_screen, Sense::click_and_drag());

            // Context menu
            resp.context_menu(|ui| {
                block_context_menu(state, ui, block_idx, b);
            });

            // Click/drag handling
            if resp.drag_started() {
                if !is_selected {
                    if !ui.input(|i| i.modifiers.ctrl) {
                        state.selection.clear();
                    }
                    state.selection.toggle_block(block_idx);
                }
                // Only start block drag if not already resizing
                if !matches!(state.drag_mode, DragMode::Resize { .. })
                    && !matches!(state.drag_mode, DragMode::Connection { .. })
                {
                    state.drag_mode = DragMode::Blocks { dx: 0.0, dy: 0.0 };
                }
            }
            if resp.clicked() && !resp.dragged() {
                if ui.input(|i| i.modifiers.ctrl) {
                    state.selection.toggle_block(block_idx);
                } else {
                    state.selection.select_block(block_idx);
                }
            }
            if resp.double_clicked() {
                // Open subsystem or code editor
                handle_block_double_click(state, block_idx, b);
            }
        }

        // Handle block dragging (live preview via delta accumulation)
        if matches!(state.drag_mode, DragMode::Blocks { .. }) && canvas_resp.dragged() {
            let delta = canvas_resp.drag_delta();
            let s = base_scale * zoom;
            if let DragMode::Blocks {
                ref mut dx,
                ref mut dy,
            } = state.drag_mode
            {
                *dx += delta.x / s;
                *dy += delta.y / s;
            }
            ui.ctx().request_repaint(); // Repaint for live preview
        }
        if matches!(state.drag_mode, DragMode::Blocks { .. }) && canvas_resp.drag_stopped() {
            if let DragMode::Blocks { dx, dy } = state.drag_mode {
                let idx_dx = state.snap(dx as i32);
                let idx_dy = state.snap(dy as i32);
                if idx_dx != 0 || idx_dy != 0 {
                    let indices = state.selection.selected_blocks.clone();
                    if let Some(system) = super::state::resolve_subsystem_by_vec_mut(
                        &mut state.app.root,
                        &state.app.path,
                    ) {
                        let cmd = operations::move_blocks(system, &indices, idx_dx, idx_dy);
                        state.history.push(cmd);
                        state.dirty = true;
                    }
                }
            }
            state.drag_mode = DragMode::None;
        }

        // Handle resize dragging
        if matches!(state.drag_mode, DragMode::Resize { .. }) && canvas_resp.dragged() {
            let delta = canvas_resp.drag_delta();
            let s = base_scale * zoom;
            if let DragMode::Resize {
                ref mut dx,
                ref mut dy,
                ..
            } = state.drag_mode
            {
                *dx += delta.x / s;
                *dy += delta.y / s;
            }
            ui.ctx().request_repaint();
        }
        if matches!(state.drag_mode, DragMode::Resize { .. }) && canvas_resp.drag_stopped() {
            if let DragMode::Resize {
                block_index,
                handle,
                original_l,
                original_t,
                original_r,
                original_b,
                dx,
                dy,
            } = state.drag_mode
            {
                let (nl, nt, nr, nb) = compute_resized_rect(
                    original_l as f32,
                    original_t as f32,
                    original_r as f32,
                    original_b as f32,
                    handle,
                    dx,
                    dy,
                    state.grid_size,
                    state.snap_to_grid,
                );
                let nl = nl as i32;
                let nt = nt as i32;
                let nr = nr as i32;
                let nb = nb as i32;
                if nl != original_l || nt != original_t || nr != original_r || nb != original_b {
                    if let Some(system) = super::state::resolve_subsystem_by_vec_mut(
                        &mut state.app.root,
                        &state.app.path,
                    ) {
                        let cmd = operations::resize_block(system, block_index, nl, nt, nr, nb);
                        state.history.push(cmd);
                        state.dirty = true;
                    }
                }
            }
            state.drag_mode = DragMode::None;
        }

        // Handle connection dragging
        if matches!(state.drag_mode, DragMode::Connection { .. }) && canvas_resp.dragged() {
            if let Some(pos) = canvas_resp.hover_pos() {
                let model_pos = from_screen(pos);
                if let DragMode::Connection {
                    ref mut current_x,
                    ref mut current_y,
                    ..
                } = state.drag_mode
                {
                    *current_x = model_pos.x;
                    *current_y = model_pos.y;
                }
            }
            ui.ctx().request_repaint();
        }
        if matches!(state.drag_mode, DragMode::Connection { .. }) && canvas_resp.drag_stopped() {
            // Try to complete the connection
            if let DragMode::Connection {
                ref src_sid,
                ref src_port_type,
                src_port_index,
                current_x,
                current_y,
            } = state.drag_mode.clone()
            {
                if let Some(system) =
                    crate::egui_app::resolve_subsystem_by_vec(&state.app.root, &state.app.path)
                {
                    let snap_radius = 20.0;
                    if let Some((dst_idx, dst_port_type, dst_port_index, _px, _py)) =
                        operations::find_snap_port(system, current_x, current_y, snap_radius, None)
                    {
                        // Check we're connecting output -> input or input -> output
                        let valid = (src_port_type == "out" && dst_port_type == "in")
                            || (src_port_type == "in" && dst_port_type == "out");
                        if valid {
                            if let Some(dst_block) = system.blocks.get(dst_idx) {
                                if let Some(dst_sid) = &dst_block.sid {
                                    let (
                                        actual_src_sid,
                                        actual_src_port,
                                        actual_dst_sid,
                                        actual_dst_port,
                                    ) = if src_port_type == "out" {
                                        (
                                            src_sid.clone(),
                                            src_port_index,
                                            dst_sid.clone(),
                                            dst_port_index,
                                        )
                                    } else {
                                        (
                                            dst_sid.clone(),
                                            dst_port_index,
                                            src_sid.clone(),
                                            src_port_index,
                                        )
                                    };
                                    // Compute auto-routing
                                    let src_pos = operations::find_snap_port(
                                        system,
                                        0.0,
                                        0.0,
                                        f32::MAX,
                                        None,
                                    );
                                    let _ = src_pos; // We'll use auto_route from port positions
                                    if let Some(sys_mut) =
                                        super::state::resolve_subsystem_by_vec_mut(
                                            &mut state.app.root,
                                            &state.app.path,
                                        )
                                    {
                                        let cmd = operations::add_line(
                                            sys_mut,
                                            &actual_src_sid,
                                            actual_src_port,
                                            &actual_dst_sid,
                                            actual_dst_port,
                                            Vec::new(), // Empty points = direct connection
                                        );
                                        state.history.push(cmd);
                                        state.dirty = true;
                                        state.app.show_notification("Connection created", 1500);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            state.drag_mode = DragMode::None;
        }
        // Cancel connection on Escape (handled in keyboard shortcuts)

        // Draw annotations
        for (a, r_model) in &annotations {
            let r_screen = Rect::from_min_max(to_screen(r_model.min), to_screen(r_model.max));
            let raw = a.text.clone().unwrap_or_default();
            let parsed =
                crate::egui_app::text::annotation_to_rich_text(&raw, a.interpreter.as_deref());
            let base_font = 12.0;
            let mut job = parsed.to_layout_job(ui.style(), font_scale, base_font);
            job.wrap.max_width = f32::INFINITY;
            let galley = ui.painter().layout_job(job);
            ui.painter()
                .galley(r_screen.left_top(), galley, Color32::WHITE);
        }

        // Draw lines
        let mut sid_mirrored: HashMap<String, bool> = HashMap::new();
        for (b, _r) in &blocks {
            if let Some(sid) = &b.sid {
                sid_mirrored.insert(sid.clone(), b.block_mirror.unwrap_or(false));
            }
        }
        let mut port_counts: HashMap<(String, u8), u32> = HashMap::new();
        fn reg_ep(ep: &EndpointRef, port_counts: &mut HashMap<(String, u8), u32>) {
            let key = (ep.sid.clone(), if ep.port_type == "out" { 1 } else { 0 });
            let idx1 = if ep.port_index == 0 { 1 } else { ep.port_index };
            port_counts
                .entry(key)
                .and_modify(|v| *v = (*v).max(idx1))
                .or_insert(idx1);
        }
        fn reg_branch(br: &crate::model::Branch, port_counts: &mut HashMap<(String, u8), u32>) {
            if let Some(dst) = &br.dst {
                reg_ep(dst, port_counts);
            }
            for sub in &br.branches {
                reg_branch(sub, port_counts);
            }
        }
        for line in &entities.lines {
            if let Some(src) = &line.src {
                reg_ep(src, &mut port_counts);
            }
            if let Some(dst) = &line.dst {
                reg_ep(dst, &mut port_counts);
            }
            for br in &line.branches {
                reg_branch(br, &mut port_counts);
            }
        }

        // Color lines with graph coloring
        let line_colors = compute_line_colors(&entities.lines, &port_counts);

        for (li, line) in entities.lines.iter().enumerate() {
            let Some(src) = line.src.as_ref() else {
                continue;
            };
            let Some(sr) = sid_map.get(&src.sid) else {
                continue;
            };
            let num_src = port_counts
                .get(&(src.sid.clone(), if src.port_type == "out" { 1 } else { 0 }))
                .copied();
            let mirrored_src = sid_mirrored.get(&src.sid).copied().unwrap_or(false);
            let mut cur = endpoint_pos_maybe_mirrored(*sr, src, num_src, mirrored_src);
            let mut offsets_pts = vec![cur];
            for off in &line.points {
                cur = Pos2::new(cur.x + off.x as f32, cur.y + off.y as f32);
                offsets_pts.push(cur);
            }
            let mut screen_pts: Vec<Pos2> = offsets_pts.iter().map(|p| to_screen(*p)).collect();

            // Add final destination point
            if let Some(dst) = line.dst.as_ref() {
                if let Some(dr) = sid_map.get(&dst.sid) {
                    let num_dst = port_counts
                        .get(&(dst.sid.clone(), if dst.port_type == "out" { 1 } else { 0 }))
                        .copied();
                    let mirrored_dst = sid_mirrored.get(&dst.sid).copied().unwrap_or(false);
                    let dst_pt = endpoint_pos_with_target_maybe_mirrored(
                        *dr,
                        dst,
                        num_dst,
                        Some(cur.y),
                        mirrored_dst,
                    );
                    screen_pts.push(to_screen(dst_pt));
                }
            }

            let color = line_colors.get(li).copied().unwrap_or(Color32::LIGHT_GREEN);
            let is_selected = state.selection.is_line_selected(li);
            let stroke_width = if is_selected { 3.5 } else { 2.0 };
            let stroke = Stroke::new(stroke_width, color);

            // Draw segments
            let has_in_dst = line.dst.as_ref().map_or(false, |d| d.port_type == "in");
            for (seg_idx, seg) in screen_pts.windows(2).enumerate() {
                let is_last = has_in_dst && seg_idx == screen_pts.len().saturating_sub(2);
                if is_last {
                    draw_arrow_with_trim(ui.painter(), seg[0], seg[1], color, stroke);
                } else {
                    ui.painter().line_segment([seg[0], seg[1]], stroke);
                }
            }

            // Draw branches
            for br in &line.branches {
                draw_branch_rec(
                    ui.painter(),
                    &to_screen,
                    &sid_map,
                    &port_counts,
                    *offsets_pts.last().unwrap_or(&cur),
                    br,
                    stroke,
                    color,
                    &sid_mirrored,
                );
            }

            // Selection highlight for lines
            if is_selected {
                for seg in screen_pts.windows(2) {
                    ui.painter().line_segment(
                        [seg[0], seg[1]],
                        Stroke::new(5.0, Color32::from_rgba_unmultiplied(0, 120, 255, 60)),
                    );
                }
            }

            // Line label
            if let Some(name) = &line.name {
                if !name.is_empty() && screen_pts.len() >= 2 {
                    let mid_idx = screen_pts.len() / 2;
                    let label_pos = Pos2::new(
                        (screen_pts[mid_idx - 1].x + screen_pts[mid_idx].x) / 2.0,
                        (screen_pts[mid_idx - 1].y + screen_pts[mid_idx].y) / 2.0
                            - 10.0 * font_scale,
                    );
                    let label_font = egui::FontId::proportional(11.0 * font_scale);
                    ui.painter()
                        .text(label_pos, Align2::CENTER_BOTTOM, name, label_font, color);
                }
            }

            // Allocate hit rect for lines
            if !screen_pts.is_empty() {
                let (min_x, min_y, max_x, max_y) = screen_pts.iter().fold(
                    (
                        f32::INFINITY,
                        f32::INFINITY,
                        f32::NEG_INFINITY,
                        f32::NEG_INFINITY,
                    ),
                    |(mnx, mny, mxx, mxy), p| {
                        (mnx.min(p.x), mny.min(p.y), mxx.max(p.x), mxy.max(p.y))
                    },
                );
                let pad = 6.0;
                let hit_rect = Rect::from_min_max(
                    Pos2::new(min_x - pad, min_y - pad),
                    Pos2::new(max_x + pad, max_y + pad),
                );
                // Use Sense::hover() instead of Sense::click() so that the
                // line bounding-box does not steal click events from blocks that
                // overlap with it.  Actual click detection is deferred.
                let line_resp = ui.allocate_rect(hit_rect, Sense::hover());
                
                // Do a line-intersection near check to see if we actually clicked it.
                let mut is_near_segment = false;
                if let Some(cp) = ui.input(|i| i.pointer.interact_pos()) {
                    let mut min_dist = f32::INFINITY;
                    // Gather all segments
                    let mut segments = Vec::new();
                    for seg in screen_pts.windows(2) {
                        segments.push((seg[0], seg[1]));
                    }
                    // Collect branch segments as well
                    // Doing a quick pass to collect all points:
                    fn collect_branch_segments_editor(
                        br: &crate::model::Branch,
                        start: Pos2,
                        out: &mut Vec<(Pos2, Pos2)>,
                        to_screen: &dyn Fn(Pos2) -> Pos2,
                        sid_map: &std::collections::HashMap<String, Rect>,
                        port_counts: &std::collections::HashMap<(String, u8), u32>,
                        sid_mirrored: &std::collections::HashMap<String, bool>,
                    ) {
                        let mut cur = start;
                        for off in &br.points {
                            let next = Pos2::new(cur.x + off.x as f32, cur.y + off.y as f32);
                            out.push((to_screen(cur), to_screen(next)));
                            cur = next;
                        }
                        for child in &br.branches {
                            collect_branch_segments_editor(child, cur, out, to_screen, sid_map, port_counts, sid_mirrored);
                        }
                    }
                    let main_anchor = offsets_pts.last().copied().unwrap_or(offsets_pts.first().copied().unwrap_or(Pos2::ZERO));
                    for br in &line.branches {
                        collect_branch_segments_editor(br, main_anchor, &mut segments, &to_screen, &sid_map, &port_counts, &sid_mirrored);
                    }

                    for (a, b) in &segments {
                        let ab_x = b.x - a.x;
                        let ab_y = b.y - a.y;
                        let ap_x = cp.x - a.x;
                        let ap_y = cp.y - a.y;
                        let ab_len2 = (ab_x * ab_x + ab_y * ab_y).max(1e-6);
                        let t = (ap_x * ab_x + ap_y * ab_y) / ab_len2;
                        let t_clamped = t.max(0.0).min(1.0);
                        let proj_x = a.x + ab_x * t_clamped;
                        let proj_y = a.y + ab_y * t_clamped;
                        let dx = cp.x - proj_x;
                        let dy = cp.y - proj_y;
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist < min_dist {
                            min_dist = dist;
                        }
                    }
                    is_near_segment = min_dist <= 8.0;
                }

                if is_near_segment {
                    line_resp.context_menu(|ui| {
                        line_context_menu(state, ui, li, line);
                    });
                    let clicked = ui.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary) || i.pointer.button_clicked(egui::PointerButton::Secondary));
                    if clicked {
                        if ui.input(|i| i.modifiers.ctrl) {
                            state.selection.toggle_line(li);
                        } else {
                            state.selection.select_line(li);
                        }
                    }
                }
            }
        }

        // Draw the connection being drawn
        if let DragMode::Connection {
            ref src_sid,
            ref src_port_type,
            src_port_index,
            current_x,
            current_y,
        } = state.drag_mode
        {
            // Find start position from the actual port
            let start_screen = if let Some(sr) = sid_map.get(src_sid) {
                let mirrored = sid_mirrored.get(src_sid).copied().unwrap_or(false);
                let ep = EndpointRef {
                    sid: src_sid.clone(),
                    port_type: src_port_type.clone(),
                    port_index: src_port_index,
                };
                let num_ports = port_counts
                    .get(&(src_sid.clone(), if src_port_type == "out" { 1 } else { 0 }))
                    .copied();
                let model_pos = endpoint_pos_maybe_mirrored(*sr, &ep, num_ports, mirrored);
                Some(to_screen(model_pos))
            } else {
                sid_screen_map.get(src_sid).map(|sr| {
                    if src_port_type == "out" {
                        Pos2::new(sr.right(), sr.center().y)
                    } else {
                        Pos2::new(sr.left(), sr.center().y)
                    }
                })
            };

            if let Some(start) = start_screen {
                let end = to_screen(Pos2::new(current_x, current_y));
                let conn_color = Color32::from_rgb(80, 200, 80);
                let conn_stroke = Stroke::new(2.5, conn_color);

                // Draw orthogonal routing preview
                let mid_x = (start.x + end.x) / 2.0;
                let corner1 = Pos2::new(mid_x, start.y);
                let corner2 = Pos2::new(mid_x, end.y);
                ui.painter().line_segment([start, corner1], conn_stroke);
                ui.painter().line_segment([corner1, corner2], conn_stroke);
                ui.painter().line_segment([corner2, end], conn_stroke);

                // Start circle
                ui.painter().circle_filled(start, 4.0, conn_color);

                // Check for snap target and draw snap indicator
                if let Some(system) =
                    crate::egui_app::resolve_subsystem_by_vec(&state.app.root, &state.app.path)
                {
                    let snap_radius = 20.0;
                    if let Some((_dst_idx, _dst_pt, _dst_pi, px, py)) =
                        operations::find_snap_port(system, current_x, current_y, snap_radius, None)
                    {
                        let snap_screen = to_screen(Pos2::new(px, py));
                        // Draw snap indicator ring
                        ui.painter().circle_stroke(
                            snap_screen,
                            8.0,
                            Stroke::new(2.0, Color32::from_rgb(50, 255, 50)),
                        );
                        ui.painter().circle_filled(
                            snap_screen,
                            4.0,
                            Color32::from_rgb(50, 255, 50),
                        );
                    } else {
                        // Normal endpoint
                        ui.painter().circle_filled(end, 4.0, conn_color);
                    }
                } else {
                    ui.painter().circle_filled(end, 4.0, conn_color);
                }
            }
        }

        // Draw selection rectangle
        if let Some(rect) = &state.selection.selection_rect {
            let (min_x, min_y, max_x, max_y) = rect.normalized();
            let sel_rect = Rect::from_min_max(Pos2::new(min_x, min_y), Pos2::new(max_x, max_y));
            ui.painter().rect_filled(
                sel_rect,
                0.0,
                Color32::from_rgba_unmultiplied(0, 120, 255, 30),
            );
            ui.painter().rect_stroke(
                sel_rect,
                0.0,
                Stroke::new(1.0, Color32::from_rgb(0, 120, 255)),
                egui::StrokeKind::Outside,
            );
        }

        // Canvas context menu (right-click on empty space)
        canvas_resp.context_menu(|ui| {
            canvas_context_menu(state, ui, &from_screen, &canvas_resp);
        });

        // Rectangle selection via canvas drag (when not dragging blocks)
        if matches!(state.drag_mode, DragMode::None) && canvas_resp.drag_started() {
            if let Some(pos) = canvas_resp.hover_pos() {
                // Check if we clicked on empty space (not on a block)
                let on_block = blocks.iter().enumerate().any(|(_, (_, r))| {
                    let r_screen = Rect::from_min_max(to_screen(r.min), to_screen(r.max));
                    r_screen.contains(pos)
                });
                if !on_block {
                    if ui.input(|i| i.modifiers.shift) {
                        // Selection rectangle
                        state.selection.start_rect(pos.x, pos.y);
                        state.drag_mode = DragMode::SelectionRect;
                    } else {
                        // Pan
                        state.drag_mode = DragMode::Pan;
                    }
                }
            }
        }
        if matches!(state.drag_mode, DragMode::SelectionRect) && canvas_resp.dragged() {
            if let Some(pos) = canvas_resp.hover_pos() {
                state.selection.update_rect(pos.x, pos.y);
            }
        }
        if matches!(state.drag_mode, DragMode::SelectionRect) && canvas_resp.drag_stopped() {
            if let Some(system) =
                crate::egui_app::resolve_subsystem_by_vec(&state.app.root, &state.app.path)
            {
                state.selection.finish_rect(
                    system,
                    base_scale * zoom,
                    pan.x,
                    pan.y,
                    avail.left() + margin,
                    avail.top() + margin,
                );
            }
            state.drag_mode = DragMode::None;
        }
        if matches!(state.drag_mode, DragMode::Pan) && canvas_resp.dragged() {
            state.app.pan += canvas_resp.drag_delta();
        }
        if matches!(state.drag_mode, DragMode::Pan) && canvas_resp.drag_stopped() {
            state.drag_mode = DragMode::None;
        }

        // Click on empty space clears selection
        if canvas_resp.clicked() {
            let on_block = blocks.iter().any(|(_, r)| {
                let r_screen = Rect::from_min_max(to_screen(r.min), to_screen(r.max));
                canvas_resp
                    .hover_pos()
                    .map_or(false, |p| r_screen.contains(p))
            });
            if !on_block {
                state.selection.clear();
            }
        }
    });
}

// ────────────────────────────────────────────────────────────────────────────
// Keyboard shortcuts
// ────────────────────────────────────────────────────────────────────────────

fn handle_keyboard_shortcuts(
    state: &mut EditorState,
    ui: &mut egui::Ui,
    _avail: &Rect,
    _base_scale: f32,
    _bb: &Rect,
) {
    let input = ui.input(|i| {
        (
            i.modifiers.ctrl,
            i.modifiers.shift,
            i.key_pressed(egui::Key::Z),
            i.key_pressed(egui::Key::Y),
            i.key_pressed(egui::Key::Delete),
            i.key_pressed(egui::Key::A),
            i.key_pressed(egui::Key::C),
            i.key_pressed(egui::Key::V),
            i.key_pressed(egui::Key::R),
            i.key_pressed(egui::Key::M),
            i.key_pressed(egui::Key::ArrowUp),
            i.key_pressed(egui::Key::ArrowDown),
            i.key_pressed(egui::Key::ArrowLeft),
            i.key_pressed(egui::Key::ArrowRight),
            i.key_pressed(egui::Key::Escape),
        )
    });
    let (ctrl, _shift, z, y, delete, a, c, v, r, m, up, down, left, right, escape) = input;

    // Ctrl+Z: Undo
    if ctrl && z {
        state.undo();
    }
    // Ctrl+Y: Redo
    if ctrl && y {
        state.redo();
    }
    // Delete: Delete selection
    if delete {
        state.delete_selection();
    }
    // A: Open block browser
    if a && !ctrl {
        state.block_browser.open_at(200, 200);
    }
    // Ctrl+C: Copy
    if ctrl && c {
        state.copy_selection();
    }
    // Ctrl+V: Paste
    if ctrl && v {
        state.paste();
    }
    // R: Rotate selection
    if r && !ctrl {
        state.rotate_selection();
    }
    // M: Mirror selection
    if m && !ctrl {
        state.mirror_selection();
    }
    // Arrow keys: Move selected blocks
    let arrow_step = if ctrl { 1 } else { 5 };
    if !state.selection.selected_blocks.is_empty() {
        let (adx, ady) = match (up, down, left, right) {
            (true, _, _, _) => (0, -arrow_step),
            (_, true, _, _) => (0, arrow_step),
            (_, _, true, _) => (-arrow_step, 0),
            (_, _, _, true) => (arrow_step, 0),
            _ => (0, 0),
        };
        if adx != 0 || ady != 0 {
            let indices = state.selection.selected_blocks.clone();
            if let Some(system) =
                super::state::resolve_subsystem_by_vec_mut(&mut state.app.root, &state.app.path)
            {
                let cmd = operations::move_blocks(system, &indices, adx, ady);
                state.history.push(cmd);
                state.dirty = true;
            }
        }
    }
    // Escape: Clear selection / close browser
    if escape {
        state.selection.clear();
        state.block_browser.close();
        state.code_editor.close();
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Context menus
// ────────────────────────────────────────────────────────────────────────────

fn block_context_menu(
    state: &mut EditorState,
    ui: &mut egui::Ui,
    block_idx: usize,
    block: &crate::model::Block,
) {
    if ui.button("Delete").clicked() {
        state.selection.select_block(block_idx);
        state.delete_selection();
        ui.close();
    }
    if ui.button("Comment / Uncomment").clicked() {
        state.selection.select_block(block_idx);
        state.comment_selection();
        ui.close();
    }
    if ui.button("Rotate").clicked() {
        state.selection.select_block(block_idx);
        state.rotate_selection();
        ui.close();
    }
    if ui.button("Mirror").clicked() {
        state.selection.select_block(block_idx);
        state.mirror_selection();
        ui.close();
    }
    ui.separator();
    if ui.button("Copy").clicked() {
        state.selection.select_block(block_idx);
        state.copy_selection();
        ui.close();
    }
    ui.separator();
    if is_code_block(block) {
        if ui.button("Edit Code…").clicked() {
            open_code_editor(state, block_idx, block);
            ui.close();
        }
        ui.separator();
    }
    if is_subsystem_block(block) {
        if ui.button("Open Subsystem").clicked() {
            state.app.open_block_if_subsystem(block);
            state.selection.clear();
            ui.close();
        }
    }
    if !state.selection.selected_blocks.is_empty() && state.selection.selected_blocks.len() > 1 {
        if ui.button("Create Subsystem from Selection…").clicked() {
            let name = format!(
                "Subsystem{}",
                state.current_system().map_or(0, |s| s.blocks.len())
            );
            state.create_subsystem_from_selection(&name);
            ui.close();
        }
    }
    ui.separator();
    if ui.button("Properties…").clicked() {
        // Show block info
        state.app.block_view = Some(BlockDialog {
            title: format!("Block: {}", block.name),
            block: block.clone(),
            open: true,
        });
        ui.close();
    }
}

fn line_context_menu(
    state: &mut EditorState,
    ui: &mut egui::Ui,
    line_idx: usize,
    line: &crate::model::Line,
) {
    if ui.button("Delete").clicked() {
        state.selection.select_line(line_idx);
        state.delete_selection();
        ui.close();
    }
    ui.separator();
    // Rename label
    if ui.button("Rename Label…").clicked() {
        // For now, just set a default label (a dialog would be better in a real app)
        if let Some(system) = state.current_system_mut() {
            let new_name = if line.name.is_some() {
                None // Toggle off
            } else {
                Some(format!("signal_{}", line_idx))
            };
            let cmd = operations::rename_line(system, line_idx, new_name);
            state.history.push(cmd);
            state.mark_dirty();
        }
        ui.close();
    }
    ui.separator();
    if ui.button("Properties…").clicked() {
        state.app.signal_view = Some(SignalDialog {
            title: format!("Signal: {}", line.name.as_deref().unwrap_or("<unnamed>")),
            line_idx,
            open: true,
        });
        ui.close();
    }
}

fn canvas_context_menu(
    state: &mut EditorState,
    ui: &mut egui::Ui,
    from_screen: &dyn Fn(Pos2) -> Pos2,
    canvas_resp: &egui::Response,
) {
    if ui.button("Add Block… (A)").clicked() {
        let pos = canvas_resp
            .hover_pos()
            .map(|p| from_screen(p))
            .unwrap_or(Pos2::new(200.0, 200.0));
        state.block_browser.open_at(pos.x as i32, pos.y as i32);
        ui.close();
    }
    if ui.button("Paste").clicked() {
        state.paste();
        ui.close();
    }
    ui.separator();
    if ui.button("Select All").clicked() {
        let counts = crate::egui_app::resolve_subsystem_by_vec(&state.app.root, &state.app.path)
            .map(|s| (s.blocks.len(), s.lines.len()));
        if let Some((nb, nl)) = counts {
            state.selection.selected_blocks = (0..nb).collect();
            state.selection.selected_lines = (0..nl).collect();
        }
        ui.close();
    }
    ui.separator();
    if ui.button("Reassign SIDs").clicked() {
        if let Some(system) =
            super::state::resolve_subsystem_by_vec_mut(&mut state.app.root, &state.app.path)
        {
            let cmd = operations::assign_sids(system);
            state.history.push(cmd);
            state.dirty = true;
            state.app.show_notification("SIDs reassigned", 2000);
        }
        ui.close();
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Block browser window
// ────────────────────────────────────────────────────────────────────────────

fn show_block_browser(state: &mut EditorState, ui: &mut egui::Ui) {
    if !state.block_browser.open {
        return;
    }

    let mut open = state.block_browser.open;
    let insert_x = state.block_browser.insert_x;
    let insert_y = state.block_browser.insert_y;

    egui::Window::new("Add Block")
        .open(&mut open)
        .default_size([350.0, 500.0])
        .resizable(true)
        .show(ui.ctx(), |ui| {
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.text_edit_singleline(&mut state.block_browser.query);
            });
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                let query = state.block_browser.query.clone();
                let categories = state.block_browser.categories.clone();
                let expanded = state.block_browser.expanded_category;
                for (cat_idx, cat) in categories.iter().enumerate() {
                    let matching: Vec<_> = cat
                        .entries
                        .iter()
                        .filter(|e| query.is_empty() || e.matches_query(&query))
                        .collect();
                    if matching.is_empty() {
                        continue;
                    }

                    let is_expanded = expanded == Some(cat_idx) || !query.is_empty();

                    let header = egui::CollapsingHeader::new(
                        RichText::new(format!("{} ({})", cat.name, matching.len())).strong(),
                    )
                    .default_open(is_expanded);
                    header.show(ui, |ui| {
                        for entry in matching {
                            let label = format!("{} — {}", entry.display_name, entry.description);
                            if ui
                                .button(&entry.display_name)
                                .on_hover_text(&label)
                                .clicked()
                            {
                                // Add block to current system
                                if let Some(system) = super::state::resolve_subsystem_by_vec_mut(
                                    &mut state.app.root,
                                    &state.app.path,
                                ) {
                                    let block = operations::create_default_block(
                                        &entry.block_type,
                                        &entry.display_name,
                                        insert_x,
                                        insert_y,
                                        entry.default_inputs,
                                        entry.default_outputs,
                                    );
                                    let cmd = operations::add_block(system, block);
                                    state.history.push(cmd);
                                    state.dirty = true;
                                    state.app.show_notification(
                                        format!("Added {}", entry.display_name),
                                        2000,
                                    );
                                }
                                state.block_browser.close();
                            }
                        }
                    });
                }
            });
        });

    state.block_browser.open = open;
}

// ────────────────────────────────────────────────────────────────────────────
// Code editor window
// ────────────────────────────────────────────────────────────────────────────

fn show_code_editor(state: &mut EditorState, ui: &mut egui::Ui) {
    if !state.code_editor.open {
        return;
    }

    let mut open = state.code_editor.open;

    let title = format!(
        "Code: {}{}",
        state.code_editor.block_name,
        if state.code_editor.is_modified() {
            " *"
        } else {
            ""
        },
    );

    egui::Window::new(title)
        .open(&mut open)
        .default_size([600.0, 400.0])
        .resizable(true)
        .show(ui.ctx(), |ui| {
            ui.horizontal(|ui| {
                if ui.button("Apply").clicked() {
                    // Save code back to block
                    let block_index = state.code_editor.block_index;
                    let code = state.code_editor.code.clone();
                    if let Some(system) = super::state::resolve_subsystem_by_vec_mut(
                        &mut state.app.root,
                        &state.app.path,
                    ) {
                        if let Some(block) = system.blocks.get_mut(block_index) {
                            set_block_code(block, &code);
                            state.mark_dirty();
                            state.app.show_notification("Code applied", 1500);
                        }
                    }
                    state.code_editor.original_code = code;
                }
                if ui.button("Revert").clicked() {
                    state.code_editor.code = state.code_editor.original_code.clone();
                }
                if state.code_editor.is_modified() {
                    ui.colored_label(Color32::from_rgb(255, 200, 80), "Modified");
                }
            });
            ui.separator();

            // Code text area with syntax highlighting
            let theme = egui::TextEdit::multiline(&mut state.code_editor.code)
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY)
                .desired_rows(20);
            ui.add(theme);
        });

    state.code_editor.open = open;
}

// ────────────────────────────────────────────────────────────────────────────
// Helper functions
// ────────────────────────────────────────────────────────────────────────────

pub fn is_code_block(block: &crate::model::Block) -> bool {
    block.block_type == "SubSystem" && block.is_matlab_function
        || block.block_type == "MATLABSystem"
        || block.block_type == "Fcn"
        || block.block_type == "MATLABFcn"
        || block.block_type == "CFunction"
}

pub fn is_subsystem_block(block: &crate::model::Block) -> bool {
    (block.block_type == "SubSystem" || block.block_type == "Reference")
        && block
            .subsystem
            .as_ref()
            .map_or(false, |s| s.chart.is_none())
}

fn open_code_editor(state: &mut EditorState, block_idx: usize, block: &crate::model::Block) {
    let code = get_block_code(block);
    state
        .code_editor
        .open_for_block(block_idx, &block.name, &code);
}

pub fn get_block_code(block: &crate::model::Block) -> String {
    // Try Script property (MATLAB Function), then Code (CFunction)
    if let Some(script) = block.properties.get("Script") {
        return script.clone();
    }
    if let Some(code) = block.properties.get("Code") {
        return code.clone();
    }
    if let Some(expr) = block.properties.get("Expr") {
        return expr.clone();
    }
    String::new()
}

pub fn set_block_code(block: &mut crate::model::Block, code: &str) {
    if block.properties.contains_key("Script") {
        block
            .properties
            .insert("Script".to_string(), code.to_string());
    } else if block.properties.contains_key("Code") {
        block
            .properties
            .insert("Code".to_string(), code.to_string());
    } else if block.properties.contains_key("Expr") {
        block
            .properties
            .insert("Expr".to_string(), code.to_string());
    } else {
        // Default to Script
        block
            .properties
            .insert("Script".to_string(), code.to_string());
    }
}

fn handle_block_double_click(
    state: &mut EditorState,
    block_idx: usize,
    block: &crate::model::Block,
) {
    if is_code_block(block) {
        open_code_editor(state, block_idx, block);
    } else if is_subsystem_block(block) {
        state.app.open_block_if_subsystem(block);
        state.selection.clear();
    }
}

fn draw_port_indicators(
    ui: &mut egui::Ui,
    block: &crate::model::Block,
    r_screen: &Rect,
    font_scale: f32,
) {
    fn paint_port_chevron(
        painter: &egui::Painter,
        outline: Pos2,
        is_left_side: bool,
        font_scale: f32,
        color: Color32,
    ) {
        let scale = font_scale.max(0.2);
        // enlarge chevrons by factor of 4 relative to prior dot markers
        // thicker stroke for chevron edges
        let stroke_w = (4.0 * scale).max(1.0);
        let h = (8.0 * scale * 4.0).max(3.0 * 4.0);
        let w = (6.0 * scale * 4.0).max(2.0 * 4.0);

        let (base_x, tip_x) = if is_left_side {
            let tip_x = outline.x - stroke_w / 2.0;
            (tip_x - w, tip_x)
        } else {
            let base_x = outline.x + stroke_w / 2.0;
            (base_x, base_x + w)
        };

        let points = vec![
            Pos2::new(base_x, outline.y - h / 2.0),
            Pos2::new(tip_x, outline.y),
            Pos2::new(base_x, outline.y + h / 2.0),
        ];

        painter.add(egui::Shape::Path(egui::epaint::PathShape::line(
            points,
            Stroke::new(stroke_w, color),
        )));
    }

    let in_count = block.port_counts.as_ref().and_then(|p| p.ins).unwrap_or(0);
    let out_count = block.port_counts.as_ref().and_then(|p| p.outs).unwrap_or(0);
    let mirrored = block.block_mirror.unwrap_or(false);

    let (in_x, out_x) = if mirrored {
        (r_screen.right(), r_screen.left())
    } else {
        (r_screen.left(), r_screen.right())
    };

    let ins_left_side = !mirrored;
    let outs_left_side = mirrored;

    // Input ports
    for i in 0..in_count {
        let n = in_count.max(1);
        let y = r_screen.top() + r_screen.height() * ((i as f32 + 1.0) / (n as f32 + 1.0));
        paint_port_chevron(
            ui.painter(),
            Pos2::new(in_x, y),
            ins_left_side,
            font_scale,
            Color32::from_rgb(60, 60, 200),
        );
    }

    // Output ports
    for i in 0..out_count {
        let n = out_count.max(1);
        let y = r_screen.top() + r_screen.height() * ((i as f32 + 1.0) / (n as f32 + 1.0));
        paint_port_chevron(
            ui.painter(),
            Pos2::new(out_x, y),
            outs_left_side,
            font_scale,
            Color32::from_rgb(200, 60, 60),
        );
    }
}

fn draw_grid(
    ui: &mut egui::Ui,
    avail: &Rect,
    to_screen: &dyn Fn(Pos2) -> Pos2,
    from_screen: &dyn Fn(Pos2) -> Pos2,
    grid_size: i32,
    _zoom: f32,
    _base_scale: f32,
) {
    let tl = from_screen(avail.left_top());
    let br = from_screen(avail.right_bottom());
    let grid = grid_size.max(1) as f32;

    let start_x = (tl.x / grid).floor() as i32 * grid_size;
    let end_x = (br.x / grid).ceil() as i32 * grid_size;
    let start_y = (tl.y / grid).floor() as i32 * grid_size;
    let end_y = (br.y / grid).ceil() as i32 * grid_size;

    let grid_color = Color32::from_rgba_unmultiplied(100, 100, 100, 30);
    let grid_stroke = Stroke::new(0.5, grid_color);

    let mut x = start_x;
    while x <= end_x {
        let p1 = to_screen(Pos2::new(x as f32, start_y as f32));
        let p2 = to_screen(Pos2::new(x as f32, end_y as f32));
        ui.painter().line_segment([p1, p2], grid_stroke);
        x += grid_size;
    }

    let mut y = start_y;
    while y <= end_y {
        let p1 = to_screen(Pos2::new(start_x as f32, y as f32));
        let p2 = to_screen(Pos2::new(end_x as f32, y as f32));
        ui.painter().line_segment([p1, p2], grid_stroke);
        y += grid_size;
    }
}

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

fn draw_branch_rec(
    painter: &egui::Painter,
    to_screen: &dyn Fn(Pos2) -> Pos2,
    sid_map: &HashMap<String, Rect>,
    port_counts: &HashMap<(String, u8), u32>,
    start: Pos2,
    br: &crate::model::Branch,
    stroke: Stroke,
    color: Color32,
    sid_mirrored: &HashMap<String, bool>,
) {
    let mut pts: Vec<Pos2> = vec![start];
    let mut cur = start;
    for off in &br.points {
        cur = Pos2::new(cur.x + off.x as f32, cur.y + off.y as f32);
        pts.push(cur);
    }
    for seg in pts.windows(2) {
        let a = to_screen(seg[0]);
        let b = to_screen(seg[1]);
        painter.line_segment([a, b], stroke);
    }
    if let Some(dstb) = &br.dst {
        if let Some(dr) = sid_map.get(&dstb.sid) {
            let key = (
                dstb.sid.clone(),
                if dstb.port_type == "out" { 1 } else { 0 },
            );
            let num_dst = port_counts.get(&key).copied();
            let mirrored_dst = sid_mirrored.get(&dstb.sid).copied().unwrap_or(false);
            let end_pt = endpoint_pos_with_target_maybe_mirrored(
                *dr,
                dstb,
                num_dst,
                Some(cur.y),
                mirrored_dst,
            );
            let a = to_screen(*pts.last().unwrap_or(&cur));
            let b = to_screen(end_pt);
            if dstb.port_type == "in" {
                draw_arrow_with_trim(painter, a, b, color, stroke);
            } else {
                painter.line_segment([a, b], stroke);
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
            sid_mirrored,
        );
    }
}

pub fn compute_line_colors(
    lines: &[crate::model::Line],
    _port_counts: &HashMap<(String, u8), u32>,
) -> Vec<Color32> {
    let n = lines.len();
    if n == 0 {
        return Vec::new();
    }

    // Build adjacency
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut sid_to_lines: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, l) in lines.iter().enumerate() {
        if let Some(src) = &l.src {
            sid_to_lines.entry(src.sid.clone()).or_default().push(i);
        }
        if let Some(dst) = &l.dst {
            sid_to_lines.entry(dst.sid.clone()).or_default().push(i);
        }
        fn collect_bsids(br: &crate::model::Branch, out: &mut Vec<String>) {
            if let Some(d) = &br.dst {
                out.push(d.sid.clone());
            }
            for s in &br.branches {
                collect_bsids(s, out);
            }
        }
        let mut bsids = Vec::new();
        for br in &l.branches {
            collect_bsids(br, &mut bsids);
        }
        for sid in bsids {
            sid_to_lines.entry(sid).or_default().push(i);
        }
    }
    for idxs in sid_to_lines.values() {
        for a in 0..idxs.len() {
            for b in (a + 1)..idxs.len() {
                let i = idxs[a];
                let j = idxs[b];
                if !adj[i].contains(&j) {
                    adj[i].push(j);
                }
                if !adj[j].contains(&i) {
                    adj[j].push(i);
                }
            }
        }
    }

    fn circular_dist(a: f32, b: f32) -> f32 {
        let d = (a - b).abs();
        d.min(1.0 - d)
    }
    fn hue_to_color(h: f32) -> Color32 {
        let h6 = (h * 6.0) % 6.0;
        let c = 0.95 * 0.85;
        let x = c * (1.0 - ((h6 % 2.0) - 1.0).abs());
        let (r1, g1, b1) = if h6 < 1.0 {
            (c, x, 0.0)
        } else if h6 < 2.0 {
            (x, c, 0.0)
        } else if h6 < 3.0 {
            (0.0, c, x)
        } else if h6 < 4.0 {
            (0.0, x, c)
        } else if h6 < 5.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };
        let m = 0.95 - c;
        Color32::from_rgb(
            ((r1 + m) * 255.0) as u8,
            ((g1 + m) * 255.0) as u8,
            ((b1 + m) * 255.0) as u8,
        )
    }

    let sample_count = (n * 8).max(64);
    let candidates: Vec<f32> = (0..sample_count)
        .map(|i| i as f32 / sample_count as f32)
        .collect();

    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| (-(adj[i].len() as isize), i as isize));

    let mut assigned: Vec<Option<f32>> = vec![None; n];
    let mut remaining = candidates.clone();
    for i in order {
        let neigh: Vec<f32> = adj[i].iter().filter_map(|&j| assigned[j]).collect();
        let mut best_h = 0.0;
        let mut best_score = -1.0f32;
        for &h in &remaining {
            let used = if neigh.is_empty() {
                assigned.iter().flatten().copied().collect()
            } else {
                neigh.clone()
            };
            let score = if used.is_empty() {
                1.0
            } else {
                used.iter()
                    .map(|&u| circular_dist(h, u))
                    .fold(1.0, f32::min)
            };
            if score > best_score || (score == best_score && h < best_h) {
                best_score = score;
                best_h = h;
            }
        }
        assigned[i] = Some(best_h);
        if let Some(pos) = remaining
            .iter()
            .position(|&x| (x - best_h).abs() < f32::EPSILON)
        {
            remaining.remove(pos);
        }
    }

    assigned
        .into_iter()
        .enumerate()
        .map(|(i, h)| {
            let default_h = i as f32 / n.max(1) as f32;
            hue_to_color(h.unwrap_or(default_h))
        })
        .collect()
}

// ────────────────────────────────────────────────────────────────────────────
// Resize handles
// ────────────────────────────────────────────────────────────────────────────

/// Compute the 8 resize handle positions for a screen-space rectangle.
/// Returns [(center_pos, handle_index)] for TL, T, TR, R, BR, B, BL, L.
fn resize_handle_positions(r: &Rect) -> [(Pos2, u8); 8] {
    let cx = r.center().x;
    let cy = r.center().y;
    [
        (r.left_top(), 0),              // TL
        (Pos2::new(cx, r.top()), 1),    // T
        (r.right_top(), 2),             // TR
        (Pos2::new(r.right(), cy), 3),  // R
        (r.right_bottom(), 4),          // BR
        (Pos2::new(cx, r.bottom()), 5), // B
        (r.left_bottom(), 6),           // BL
        (Pos2::new(r.left(), cy), 7),   // L
    ]
}

/// Draw resize handles on a selected block and handle interaction.
fn draw_resize_handles(
    ui: &mut egui::Ui,
    r_screen: &Rect,
    block_idx: usize,
    state: &mut EditorState,
    model_rect: &Rect,
) {
    let handle_size = 5.0;
    let handle_color = Color32::from_rgb(0, 120, 255);
    let handle_hover_color = Color32::from_rgb(80, 180, 255);

    let handles = resize_handle_positions(r_screen);

    for (pos, handle_id) in &handles {
        let handle_rect = Rect::from_center_size(*pos, Vec2::splat(handle_size * 2.0));
        let resp = ui.allocate_rect(handle_rect, Sense::click_and_drag());

        let color = if resp.hovered() || resp.dragged() {
            handle_hover_color
        } else {
            handle_color
        };

        // Draw handle square
        ui.painter().rect_filled(
            Rect::from_center_size(*pos, Vec2::splat(handle_size)),
            0.0,
            color,
        );
        ui.painter().rect_stroke(
            Rect::from_center_size(*pos, Vec2::splat(handle_size)),
            0.0,
            Stroke::new(1.0, Color32::WHITE),
            egui::StrokeKind::Outside,
        );

        // Start resize drag
        if resp.drag_started() {
            let (l, t, r, b) = (
                model_rect.left() as i32,
                model_rect.top() as i32,
                model_rect.right() as i32,
                model_rect.bottom() as i32,
            );
            state.drag_mode = DragMode::Resize {
                block_index: block_idx,
                handle: *handle_id,
                original_l: l,
                original_t: t,
                original_r: r,
                original_b: b,
                dx: 0.0,
                dy: 0.0,
            };
        }
    }
}

/// Compute the new rect after applying a resize delta from a specific handle.
/// Returns (new_l, new_t, new_r, new_b) with minimum size enforcement and grid snapping.
fn compute_resized_rect(
    l: f32,
    t: f32,
    r: f32,
    b: f32,
    handle: u8,
    dx: f32,
    dy: f32,
    grid_size: i32,
    snap_to_grid: bool,
) -> (f32, f32, f32, f32) {
    let min_size = 10.0;
    let snap = |v: f32| -> f32 {
        if snap_to_grid && grid_size > 0 {
            ((v / grid_size as f32).round()) * grid_size as f32
        } else {
            v
        }
    };

    let (mut nl, mut nt, mut nr, mut nb) = (l, t, r, b);

    match handle {
        0 => {
            // TL
            nl = snap(l + dx);
            nt = snap(t + dy);
        }
        1 => {
            // T
            nt = snap(t + dy);
        }
        2 => {
            // TR
            nr = snap(r + dx);
            nt = snap(t + dy);
        }
        3 => {
            // R
            nr = snap(r + dx);
        }
        4 => {
            // BR
            nr = snap(r + dx);
            nb = snap(b + dy);
        }
        5 => {
            // B
            nb = snap(b + dy);
        }
        6 => {
            // BL
            nl = snap(l + dx);
            nb = snap(b + dy);
        }
        7 => {
            // L
            nl = snap(l + dx);
        }
        _ => {}
    }

    // Enforce minimum size
    if nr - nl < min_size {
        if handle == 0 || handle == 6 || handle == 7 {
            nl = nr - min_size;
        } else {
            nr = nl + min_size;
        }
    }
    if nb - nt < min_size {
        if handle == 0 || handle == 1 || handle == 2 {
            nt = nb - min_size;
        } else {
            nb = nt + min_size;
        }
    }

    (nl, nt, nr, nb)
}

// ────────────────────────────────────────────────────────────────────────────
// Port interaction areas (for initiating connection drag)
// ────────────────────────────────────────────────────────────────────────────

/// Draw invisible interaction areas over port chevrons to initiate connection dragging.
fn draw_port_interaction_areas(
    ui: &mut egui::Ui,
    block: &crate::model::Block,
    r_screen: &Rect,
    font_scale: f32,
    _block_idx: usize,
    state: &mut EditorState,
) {
    let in_count = block.port_counts.as_ref().and_then(|p| p.ins).unwrap_or(0);
    let out_count = block.port_counts.as_ref().and_then(|p| p.outs).unwrap_or(0);
    let mirrored = block.block_mirror.unwrap_or(false);

    let (in_x, out_x) = if mirrored {
        (r_screen.right(), r_screen.left())
    } else {
        (r_screen.left(), r_screen.right())
    };

    let scale = font_scale.max(0.2);
    let hit_size = (12.0 * scale * 4.0).max(8.0);

    let sid = match &block.sid {
        Some(s) => s.clone(),
        None => return,
    };

    // Input ports
    for i in 0..in_count {
        let n = in_count.max(1);
        let y = r_screen.top() + r_screen.height() * ((i as f32 + 1.0) / (n as f32 + 1.0));
        let port_center = Pos2::new(in_x, y);
        let hit_rect = Rect::from_center_size(port_center, Vec2::splat(hit_size));
        let resp = ui.allocate_rect(hit_rect, Sense::click_and_drag());

        if resp.drag_started() {
            state.drag_mode = DragMode::Connection {
                src_sid: sid.clone(),
                src_port_type: "in".to_string(),
                src_port_index: i + 1,
                current_x: port_center.x,
                current_y: port_center.y,
            };
        }
    }

    // Output ports
    for i in 0..out_count {
        let n = out_count.max(1);
        let y = r_screen.top() + r_screen.height() * ((i as f32 + 1.0) / (n as f32 + 1.0));
        let port_center = Pos2::new(out_x, y);
        let hit_rect = Rect::from_center_size(port_center, Vec2::splat(hit_size));
        let resp = ui.allocate_rect(hit_rect, Sense::click_and_drag());

        if resp.drag_started() {
            state.drag_mode = DragMode::Connection {
                src_sid: sid.clone(),
                src_port_type: "out".to_string(),
                src_port_index: i + 1,
                current_x: port_center.x,
                current_y: port_center.y,
            };
        }
    }
}
