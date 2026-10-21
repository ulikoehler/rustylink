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

mod canvas;
mod menus;
mod panels;
mod resize;
mod shortcuts;

pub use canvas::compute_line_colors;
pub use panels::{get_block_code, is_code_block, is_subsystem_block, set_block_code};

use super::operations;
use crate::editor::state::{DragMode, EditorState};
use crate::egui_app::navigation::resolve_subsystem_by_vec;
use crate::egui_app::{
    get_block_type_cfg, highlight_query_job, parse_block_rect, parse_rect_str, show_zoom_controls,
    wrap_text_to_max_width,
};
use crate::model::EndpointRef;
use eframe::egui::{self, Align2, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use egui_material_icons::icons::{
    ICON_ARROW_UPWARD, ICON_AUTORENEW, ICON_CIRCLE, ICON_CONTENT_PASTE, ICON_DELETE,
    ICON_DESCRIPTION, ICON_REFRESH, ICON_SWAP_HORIZ, ICON_UNDO,
};
use std::collections::HashMap;
// ────────────────────────────────────────────────────────────────────────────
// Color utilities — re-exported from canonical `egui_app::ui::colors`
// ────────────────────────────────────────────────────────────────────────────

use self::canvas::{
    draw_arrow_with_trim, draw_branch_rec, draw_grid, draw_port_indicators,
    draw_port_interaction_areas,
};
use self::menus::{block_context_menu, canvas_context_menu, line_context_menu};
use self::panels::{handle_block_double_click, show_block_browser, show_code_editor};
use self::resize::{compute_resized_rect, draw_resize_handles};
use self::shortcuts::handle_keyboard_shortcuts;
use crate::egui_app::colors::{
    area_annotation_border, area_annotation_fill, block_fill_color, block_has_model_color,
    contrast_color, monochrome_block_border, monochrome_line_color,
};

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
    egui::Panel::top(state.app.egui_id("editor_top")).show(ui, |ui| {
        ui.horizontal(|ui| {
            let up_label = egui::RichText::new(format!("{} Up", ICON_ARROW_UPWARD.codepoint));
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
            let undo_btn = ui.add_enabled(
                state.history.can_undo(),
                egui::Button::new(format!("{} Undo", ICON_UNDO.codepoint)),
            );
            if undo_btn.clicked() {
                state.undo();
            }
            let redo_btn = ui.add_enabled(
                state.history.can_redo(),
                egui::Button::new(format!("{} Redo", ICON_REFRESH.codepoint)),
            );
            if redo_btn.clicked() {
                state.redo();
            }
            ui.separator();

            let has_selection = !state.selection.is_empty();
            let del_btn = ui.add_enabled(
                has_selection,
                egui::Button::new(format!("{} Delete", ICON_DELETE.codepoint)),
            );
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
                egui::Button::new(format!("{} Rotate", ICON_AUTORENEW.codepoint)),
            );
            if rotate_btn.clicked() {
                state.rotate_selection();
            }
            let mirror_btn = ui.add_enabled(
                !state.selection.selected_blocks.is_empty(),
                egui::Button::new(format!("{} Mirror", ICON_SWAP_HORIZ.codepoint)),
            );
            if mirror_btn.clicked() {
                state.mirror_selection();
            }
            ui.separator();

            let copy_btn = ui.add_enabled(
                !state.selection.selected_blocks.is_empty(),
                egui::Button::new(format!("{} Copy", ICON_CONTENT_PASTE.codepoint)),
            );
            if copy_btn.clicked() {
                state.copy_selection();
            }
            let paste_btn = ui.add_enabled(
                state.clipboard.has_content(),
                egui::Button::new(format!("{} Paste", ICON_DESCRIPTION.codepoint)),
            );
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
            ui.label("Name extend");
            ui.add(
                egui::DragValue::new(&mut state.app.block_name_extend_factor)
                    .speed(0.05)
                    .range(0.2..=4.0),
            );

            // Modified indicator
            if state.dirty {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.colored_label(
                        Color32::from_rgb(255, 200, 80),
                        format!("{} Modified", ICON_CIRCLE.codepoint),
                    );
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

    // Resolve current system (borrowed from state.app.root — only borrows root, not all of state)
    let sys = match resolve_subsystem_by_vec(&state.app.root, &state.app.path) {
        Some(s) => s,
        None => {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.colored_label(Color32::RED, "Invalid path — nothing to render");
            });
            return;
        }
    };
    // Shallow-clone blocks (skip subsystem trees) to avoid borrow conflicts
    // with &mut state in the closure below, while minimizing clone cost.
    let owned_blocks: Vec<crate::model::Block> = sys
        .blocks
        .iter()
        .filter(|b| parse_block_rect(b).is_some())
        .map(|b| {
            let mut c = b.clone();
            c.subsystem = None;
            c
        })
        .collect();
    // Keep original blocks (with subsystem intact) for subsystem blocks so
    // that is_subsystem_block() checks in context menus and double-click work.
    let subsystem_block_lookup: HashMap<String, crate::model::Block> = sys
        .blocks
        .iter()
        .filter(|b| parse_block_rect(b).is_some())
        .filter(|b| crate::simulink_libraries::traits::is_navigable_subsystem(b))
        .filter_map(|b| b.sid.as_ref().map(|sid| (sid.clone(), b.clone())))
        .collect();
    let blocks: Vec<(&crate::model::Block, Rect)> = owned_blocks
        .iter()
        .filter_map(|b| parse_block_rect(b).map(|r| (b, r)))
        .collect();
    // Clone annotations for use inside the closure
    let sys_annotations: Vec<crate::model::Annotation> = sys
        .annotations
        .iter()
        .chain(sys.blocks.iter().flat_map(|b| b.annotations.iter()))
        .cloned()
        .collect();
    let annotations: Vec<(&crate::model::Annotation, Rect)> = sys_annotations
        .iter()
        .filter_map(|a| {
            a.position
                .as_deref()
                .and_then(parse_rect_str)
                .map(|pos| (a, pos))
        })
        .collect();
    // Clone lines for use inside the closure (avoids borrowing state.app.root across closure)
    let sys_lines: Vec<crate::model::Line> = sys.lines.clone();

    if blocks.is_empty() && annotations.is_empty() {
        egui::CentralPanel::default().show(ui, |ui| {
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
        // Fit every block into the viewport and centre the fitted content so
        // all blocks are visible in the screen rect (not anchored to a corner).
        state.app.zoom = 1.0;
        let extra_x = (avail_size.x - 2.0 * margin - bb.width() * base_scale).max(0.0);
        let extra_y = (avail_size.y - 2.0 * margin - bb.height() * base_scale).max(0.0);
        state.app.pan = Vec2::new(extra_x * 0.5, extra_y * 0.5);
        state.app.reset_view = false;
        ui.ctx().request_repaint();
    }

    // Central panel rendering
    egui::CentralPanel::default().show(ui, |ui| {
        let avail = ui.available_rect_before_wrap();

        // Canvas interaction
        let canvas_resp = ui.interact(
            avail,
            ui.scope_id().with("editor_canvas"),
            Sense::click_and_drag(),
        );

        // Handle keyboard shortcuts
        handle_keyboard_shortcuts(state, ui, &avail, base_scale, &bb);

        // Zoom with scroll
        let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_y.abs() > 0.0 && canvas_resp.hovered() {
            let factor = (1.0_f32 + scroll_y * 0.001_f32).max(0.1_f32);
            let old_zoom = state.app.zoom;
            let new_zoom = (old_zoom * factor).clamp(0.2, 30.0);
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

        // Coupled to the model's measurement unit: `base_scale * zoom` is the
        // screen-pixels-per-model-unit scale used for block geometry, so text
        // and icons scale exactly with the on-screen block size.
        let font_scale: f32 = (base_scale * zoom / 2.0).max(0.01);

        let dark_mode = ui.visuals().dark_mode;
        let monochrome = state.app.monochrome;

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

        show_zoom_controls(
            ui.ctx(),
            state.app.egui_id("editor_zoom_controls"),
            crate::egui_app::ZoomGeometry {
                fixed_pos: Pos2::new(avail.left() + 8.0, avail.top() + 8.0),
                base_scale,
                world_bounds: bb,
                origin: Pos2::new(avail.left() + margin, avail.top() + margin),
                center: avail.center(),
            },
            &mut crate::egui_app::ZoomViewState {
                zoom: &mut state.app.zoom,
                pan: &mut state.app.pan,
                reset_requested: &mut state.app.reset_view,
                monochrome: &mut state.app.monochrome,
            },
        );

        // Build SID maps
        let mut sid_map: HashMap<String, Rect> = HashMap::new();
        let mut sid_screen_map: HashMap<String, Rect> = HashMap::new();
        let mut collidable_obstacle_rects: Vec<Rect> = Vec::new();
        let mut deferred_block_labels = Vec::new();

        // Compute drag offset for live preview
        let drag_offset_model = if let DragMode::Blocks { dx, dy } = &state.drag_mode {
            Some((*dx, *dy))
        } else {
            None
        };

        // Draw area annotations (colored regions) behind the blocks.  Areas
        // keep their model-defined colors even in "less colorful" mode.
        for (a, r_model) in &annotations {
            if let Some(fill) = area_annotation_fill(a) {
                let r_screen = Rect::from_min_max(to_screen(r_model.min), to_screen(r_model.max));
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
                            (
                                *original_l as f32,
                                *original_t as f32,
                                *original_r as f32,
                                *original_b as f32,
                            ),
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
            let bg = block_fill_color(b, &cfg, monochrome, dark_mode);

            let is_selected = state.selection.is_block_selected(block_idx);

            // Render block.  The body shape and the interior (icon / in-block
            // label / static glyph) are drawn by the single general renderer
            // shared with the viewer; no block-type-specific code lives here.
            let body_bg = crate::egui_app::render::fill_block_body(
                ui.painter(),
                r_screen,
                cfg.shape,
                bg,
                b.commented,
            );
            let fg = contrast_color(body_bg);
            let border_color = if monochrome && !block_has_model_color(b) {
                monochrome_block_border(dark_mode)
            } else {
                let border_rgb = cfg.border.unwrap_or(crate::block_types::Rgb(180, 180, 200));
                Color32::from_rgb(border_rgb.0, border_rgb.1, border_rgb.2)
            };
            let params = crate::simulink_libraries::render::InteriorParams {
                live_mode: false,
                font_scale,
                name_font_factor: state.app.block_name_font_factor,
                live_value: None,
                live_text: None,
                live_display_options: None,
                port_y: None,
                port_label_widths: None,
                text_color: fg,
                fill_color: body_bg,
                border_color,
            };
            crate::simulink_libraries::render::render_block_interior(
                ui.painter(),
                b,
                &r_screen,
                &params,
            );

            // Body outline (shape-aware, shared with the viewer).
            if !b.commented {
                crate::egui_app::render::stroke_block_body(
                    ui.painter(),
                    r_screen,
                    cfg.shape,
                    Stroke::new(1.5_f32, border_color),
                );
            }

            // Selection highlight
            if is_selected {
                ui.painter().rect_stroke(
                    r_screen.expand(2.0),
                    6.0,
                    Stroke::new(2.5_f32, Color32::from_rgb(0, 120, 255)),
                    egui::StrokeKind::Outside,
                );
            }

            // Block label (deferred)

            let show_name = b.show_name.unwrap_or(state.app.show_block_names_default);

            if show_name {
                deferred_block_labels.push(((*b).clone(), r_screen, bg, font_scale));
            }

            collidable_obstacle_rects.push(r_screen);

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
                block_context_menu(state, ui, block_idx, b, &subsystem_block_lookup);
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
                handle_block_double_click(state, block_idx, b, &subsystem_block_lookup);
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
                    if let Some(system) = crate::editor::state::resolve_subsystem_by_vec_mut(
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
                    (
                        original_l as f32,
                        original_t as f32,
                        original_r as f32,
                        original_b as f32,
                    ),
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
                if (nl != original_l || nt != original_t || nr != original_r || nb != original_b)
                    && let Some(system) = crate::editor::state::resolve_subsystem_by_vec_mut(
                        &mut state.app.root,
                        &state.app.path,
                    )
                {
                    let cmd = operations::resize_block(system, block_index, nl, nt, nr, nb);
                    state.history.push(cmd);
                    state.dirty = true;
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
                && let Some(system) =
                    crate::egui_app::resolve_subsystem_by_vec(&state.app.root, &state.app.path)
            {
                let snap_radius = 20.0;
                if let Some((dst_idx, dst_port_type, dst_port_index, _px, _py)) =
                    operations::find_snap_port(system, current_x, current_y, snap_radius, None)
                {
                    // Check we're connecting output -> input or input -> output
                    let valid = (src_port_type == "out" && dst_port_type == "in")
                        || (src_port_type == "in" && dst_port_type == "out");
                    if valid
                        && let Some(dst_block) = system.blocks.get(dst_idx)
                        && let Some(dst_sid) = &dst_block.sid
                    {
                        let (actual_src_sid, actual_src_port, actual_dst_sid, actual_dst_port) =
                            if src_port_type == "out" {
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
                        let src_pos = operations::find_snap_port(system, 0.0, 0.0, f32::MAX, None);
                        let _ = src_pos; // We'll use auto_route from port positions
                        if let Some(sys_mut) = crate::editor::state::resolve_subsystem_by_vec_mut(
                            &mut state.app.root,
                            &state.app.path,
                        ) {
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
        let mut sid_port_overrides: HashMap<
            String,
            Vec<crate::simulink_libraries::types::PortPositionOverride>,
        > = HashMap::new();
        for (b, _r) in &blocks {
            if let Some(sid) = &b.sid {
                sid_mirrored.insert(sid.clone(), b.block_mirror.unwrap_or(false));
                let cfg = crate::egui_app::get_block_type_cfg(b);
                if !cfg.port_position_overrides.is_empty() {
                    sid_port_overrides.insert(sid.clone(), cfg.port_position_overrides.clone());
                }
            }
        }
        let (port_counts, _connected_ports, _connected_control_ports) =
            crate::egui_app::ui::signal_routing::compute_port_info(&sys_lines, &owned_blocks);

        // Color lines with graph coloring
        let line_colors = compute_line_colors(&sys_lines, &port_counts);

        for (li, line) in sys_lines.iter().enumerate() {
            let Some(src) = line.src.as_ref() else {
                continue;
            };
            let Some(sr) = sid_map.get(&src.sid) else {
                continue;
            };
            let mirrored_src = sid_mirrored.get(&src.sid).copied().unwrap_or(false);
            let src_overrides = sid_port_overrides
                .get(&src.sid)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let mut cur = crate::egui_app::ui::signal_routing::endpoint_pos(
                *sr,
                src,
                &port_counts,
                mirrored_src,
                src_overrides,
            );
            let mut offsets_pts = vec![cur];
            for off in &line.points {
                cur = Pos2::new(cur.x + off.x as f32, cur.y + off.y as f32);
                offsets_pts.push(cur);
            }
            let mut screen_pts: Vec<Pos2> = offsets_pts.iter().map(|p| to_screen(*p)).collect();

            // Add final destination point
            if let Some(dst) = line.dst.as_ref()
                && let Some(dr) = sid_map.get(&dst.sid)
            {
                let mirrored_dst = sid_mirrored.get(&dst.sid).copied().unwrap_or(false);
                let dst_overrides = sid_port_overrides
                    .get(&dst.sid)
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]);
                let dst_pt = crate::egui_app::ui::signal_routing::endpoint_pos(
                    *dr,
                    dst,
                    &port_counts,
                    mirrored_dst,
                    dst_overrides,
                );
                screen_pts.push(to_screen(dst_pt));
            }

            let color = if monochrome {
                monochrome_line_color(dark_mode)
            } else {
                line_colors.get(li).copied().unwrap_or(Color32::LIGHT_GREEN)
            };
            let is_selected = state.selection.is_line_selected(li);
            let stroke_width = if is_selected { 3.5_f32 } else { 2.0_f32 };
            let stroke = Stroke::new(stroke_width, color);

            // Draw segments
            let has_in_dst = line.dst.as_ref().is_some_and(|d| {
                d.port_type == "in" || crate::egui_app::geometry::is_control_port_type(&d.port_type)
            });
            for (seg_idx, seg) in screen_pts.windows(2).enumerate() {
                let is_last = has_in_dst && seg_idx == screen_pts.len().saturating_sub(2);
                if is_last {
                    draw_arrow_with_trim(ui.painter(), seg[0], seg[1], color, stroke);
                } else {
                    ui.painter().line_segment([seg[0], seg[1]], stroke);
                }
            }

            // Draw branches
            // Draw a junction dot at the branch point when there are branches.
            if !line.branches.is_empty() {
                let anchor = offsets_pts.last().unwrap_or(&cur);
                ui.painter().circle_filled(to_screen(*anchor), 4.0, color);
            }
            for br in &line.branches {
                draw_branch_rec(
                    ui.painter(),
                    &canvas::BranchLookup {
                        to_screen: &to_screen,
                        sid_map: &sid_map,
                        port_counts: &port_counts,
                        sid_mirrored: &sid_mirrored,
                        sid_port_overrides: &sid_port_overrides,
                    },
                    *offsets_pts.last().unwrap_or(&cur),
                    br,
                    stroke,
                    color,
                );
            }

            // Selection highlight for lines

            if is_selected {
                for seg in screen_pts.windows(2) {
                    ui.painter().line_segment(
                        [seg[0], seg[1]],
                        Stroke::new(5.0_f32, Color32::from_rgba_unmultiplied(0, 120, 255, 60)),
                    );
                }

                for seg in screen_pts.windows(2) {
                    let mut min = seg[0].min(seg[1]);

                    let mut max = seg[0].max(seg[1]);

                    min.x -= 2.0;
                    min.y -= 2.0;

                    max.x += 2.0;
                    max.y += 2.0;

                    collidable_obstacle_rects.push(Rect::from_min_max(min, max));
                }
            }

            // Line label
            if let Some(name) = &line.name
                && !name.is_empty()
                && screen_pts.len() >= 2
            {
                let mid_idx = screen_pts.len() / 2;
                let label_pos = Pos2::new(
                    (screen_pts[mid_idx - 1].x + screen_pts[mid_idx].x) / 2.0,
                    (screen_pts[mid_idx - 1].y + screen_pts[mid_idx].y) / 2.0 - 10.0 * font_scale,
                );
                let label_font = egui::FontId::proportional(11.0 * font_scale);
                ui.painter()
                    .text(label_pos, Align2::CENTER_BOTTOM, name, label_font, color);
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
                    ) {
                        let mut cur = start;
                        for off in &br.points {
                            let next = Pos2::new(cur.x + off.x as f32, cur.y + off.y as f32);
                            out.push((to_screen(cur), to_screen(next)));
                            cur = next;
                        }
                        for child in &br.branches {
                            collect_branch_segments_editor(child, cur, out, to_screen);
                        }
                    }
                    let main_anchor = offsets_pts
                        .last()
                        .copied()
                        .unwrap_or(offsets_pts.first().copied().unwrap_or(Pos2::ZERO));
                    for br in &line.branches {
                        collect_branch_segments_editor(br, main_anchor, &mut segments, &to_screen);
                    }

                    for (a, b) in &segments {
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
                    is_near_segment = min_dist <= 8.0;
                }

                if is_near_segment {
                    line_resp.context_menu(|ui| {
                        line_context_menu(state, ui, li, line);
                    });
                    let clicked = ui.input(|i| {
                        i.pointer.button_clicked(egui::PointerButton::Primary)
                            || i.pointer.button_clicked(egui::PointerButton::Secondary)
                    });
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

        // Draw deferred block labels

        for (b, r_screen, bg, font_scale) in deferred_block_labels {
            let (_chevron_h, chevron_w, _chevron_stroke) =
                crate::egui_app::geometry::port_chevron_size(font_scale);

            let in_count = b.port_counts.as_ref().and_then(|p| p.ins).unwrap_or(0);

            let out_count = b.port_counts.as_ref().and_then(|p| p.outs).unwrap_or(0);

            let mirrored = b.block_mirror.unwrap_or(false);

            let ins_left_side = !mirrored;

            let outs_left_side = mirrored;

            let has_left = (in_count > 0 && ins_left_side) || (out_count > 0 && outs_left_side);

            let has_right = (in_count > 0 && !ins_left_side) || (out_count > 0 && !outs_left_side);

            let left_extra = if has_left { chevron_w } else { 0.0 };

            let right_extra = if has_right { chevron_w } else { 0.0 };

            let overall_w = r_screen.width() + left_extra + right_extra;

            let max_label_w = overall_w * 0.95 * state.app.block_name_extend_factor.max(0.1);

            let font_px = crate::egui_app::shared_canvas_text_font_px(
                font_scale,
                state.app.block_name_font_factor,
            );

            // Block-name labels sit on the canvas (not on the block body), so
            // they must contrast with the canvas background — otherwise a dark
            // glyph (chosen for a light block fill) is invisible in dark mode.
            let _ = bg;
            let fg = if b.commented {
                Color32::GRAY
            } else {
                contrast_color(ui.visuals().panel_fill)
            };

            let left = r_screen.left() - left_extra;

            let right = r_screen.right() + right_extra;

            let center_x = (left + right) * 0.5;

            let label_font = egui::FontId::proportional(font_px);

            let line_height = (font_px * 1.2).max(1.0);

            let best_lines =
                wrap_text_to_max_width(ui.painter(), &b.name, label_font.clone(), max_label_w);

            if !best_lines.is_empty() {
                let total_h = (best_lines.len() as f32) * line_height;

                let mut max_w = 0.0_f32;

                for l in &best_lines {
                    let w = ui
                        .painter()
                        .layout_no_wrap(l.to_string(), label_font.clone(), fg)
                        .size()
                        .x;

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
                        let mut y = r_screen.bottom() + 4.0 * font_scale;

                        for line in &best_lines {
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

                        for line in best_lines.iter().rev() {
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
                        let total_h = (best_lines.len() as f32) * line_height;

                        let mut y = r_screen.center().y - total_h * 0.5;

                        let gap = 2.0 * font_scale;

                        let x_right = r_screen.left() - gap;

                        for line in &best_lines {
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
                        let total_h = (best_lines.len() as f32) * line_height;

                        let mut y = r_screen.center().y - total_h * 0.5;

                        let gap = 2.0 * font_scale;

                        let x_left = r_screen.right() + gap;

                        for line in &best_lines {
                            let galley = ui.painter().layout_no_wrap(
                                line.to_string(),
                                label_font.clone(),
                                fg,
                            );

                            let pos = Pos2::new(x_left + 2.0 * font_scale, y);

                            ui.painter().galley(pos, galley, fg);

                            y += line_height;
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
                let src_overrides = sid_port_overrides
                    .get(src_sid)
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]);
                let model_pos = crate::egui_app::ui::signal_routing::endpoint_pos(
                    *sr,
                    &ep,
                    &port_counts,
                    mirrored,
                    src_overrides,
                );
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
                let conn_stroke = Stroke::new(2.5_f32, conn_color);

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
                            Stroke::new(2.0_f32, Color32::from_rgb(50, 255, 50)),
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
                Stroke::new(1.0_f32, Color32::from_rgb(0, 120, 255)),
                egui::StrokeKind::Outside,
            );
        }

        // Canvas context menu (right-click on empty space)
        canvas_resp.context_menu(|ui| {
            canvas_context_menu(state, ui, &from_screen, &canvas_resp);
        });

        // Rectangle selection via canvas drag (when not dragging blocks)
        if matches!(state.drag_mode, DragMode::None)
            && canvas_resp.drag_started()
            && let Some(pos) = canvas_resp.hover_pos()
        {
            // Check if we clicked on empty space (not on a block)
            let on_block = blocks.iter().any(|(_, r)| {
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
        if matches!(state.drag_mode, DragMode::SelectionRect)
            && canvas_resp.dragged()
            && let Some(pos) = canvas_resp.hover_pos()
        {
            state.selection.update_rect(pos.x, pos.y);
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
                    .is_some_and(|p| r_screen.contains(p))
            });
            if !on_block {
                state.selection.clear();
            }
        }
    });
}
