#![cfg(feature = "egui")]
#![cfg(feature = "egui")]

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use eframe::egui::{self, Align2, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use eframe::egui::epaint::Shape;

use crate::model::EndpointRef;

use super::geometry::{
    endpoint_pos_maybe_mirrored, endpoint_pos_with_target_maybe_mirrored, parse_block_rect,
    parse_rect_str,
};
use super::render::{
    ComputedPortYCoordinates, get_block_type_cfg, render_block_icon, render_manual_switch,
};
use super::state::{BlockDialog, ChartView, SignalDialog, SubsystemApp};
use super::text::{highlight_query_job, matlab_syntax_job};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClickAction {
    Primary,
    Secondary,
    DoublePrimary,
    DoubleSecondary,
}

#[derive(Clone, Debug)]
pub enum UpdateResponse {
    None,
    Block {
        action: ClickAction,
        block: crate::model::Block,
        handled: bool,
    },
    Signal {
        action: ClickAction,
        line_idx: usize,
        line: crate::model::Line,
        handled: bool,
    },
}

fn is_block_subsystem(b: &crate::model::Block) -> bool {
    b.block_type == "SubSystem"
        && b.subsystem
            .as_ref()
            .map_or(false, |sub| sub.chart.is_none())
}

fn record_interaction(current: &mut UpdateResponse, new: UpdateResponse) {
    if matches!(new, UpdateResponse::None) {
        return;
    }
    fn is_double(resp: &UpdateResponse) -> bool {
        match resp {
            UpdateResponse::Block { action, .. }
            | UpdateResponse::Signal { action, .. } => matches!(
                action,
                ClickAction::DoublePrimary | ClickAction::DoubleSecondary
            ),
            UpdateResponse::None => false,
        }
    }

    let current_is_double = is_double(current);
    let new_is_double = is_double(&new);

    if matches!(current, UpdateResponse::None) {
        *current = new;
    } else if current_is_double && !new_is_double {
        // Preserve the earlier double-click interaction.
    } else if new_is_double && !current_is_double {
        *current = new;
    } else {
        // Default: prefer the most recent interaction.
        *current = new;
    }
}

#[allow(dead_code)]
fn expand_rect_for_label(
    rect: Rect,
    block: &crate::model::Block,
    ui: &egui::Ui,
    font_scale: f32,
) -> Rect {
    let font = egui::FontId::proportional(14.0 * font_scale);
    let label = block.name.replace('\n', " ");
    let galley = ui.painter().layout_no_wrap(label, font, Color32::BLACK);
    let padding = 16.0 * font_scale;
    let desired = galley.size().x + padding;
    if desired > rect.width() {
        let extra = desired - rect.width();
        Rect::from_min_max(
            Pos2::new(rect.min.x - extra * 0.5, rect.min.y),
            Pos2::new(rect.max.x + extra * 0.5, rect.max.y),
        )
    } else {
        rect
    }
}

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

fn contrast_color(bg: Color32) -> Color32 {
    let lum = luminance(bg);
    if lum > 0.6 {
        Color32::from_rgb(25, 35, 45)
    } else {
        Color32::from_rgb(235, 245, 245)
    }
}

fn hsv_to_color32(h: f32, s: f32, v: f32) -> Color32 {
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
    let (r, g, b) = (r1 + m, g1 + m, b1 + m);
    Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

fn hash_color(input: &str, s: f32, v: f32) -> Color32 {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    let hash = hasher.finish();
    let h = (hash as f32 / u64::MAX as f32) % 1.0;
    hsv_to_color32(h, s, v)
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

fn update_internal(
    app: &mut SubsystemApp,
    ui: &mut egui::Ui,
    enable_context_menus: bool,
) -> UpdateResponse {
    let mut interaction = UpdateResponse::None;
    let mut navigate_to: Option<Vec<String>> = None;
    let mut clear_search = false;
    let path_snapshot = app.path.clone();

    egui::TopBottomPanel::top("top").show_inside(ui, |ui| {
        ui.horizontal(|ui| {
            let up_label = egui::RichText::new("⬆ Up");
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

    // Owned snapshot for use inside the UI closure to avoid immutable borrows of `app`
    let entities_opt = app.current_entities();
    let system_valid = entities_opt.is_some();
    // Snapshot the current system name (prefer system properties, fall back to last path segment or <root>)
    let system_name_snapshot: String = app
        .current_system()
        .and_then(|s| s.properties.get("Name").cloned())
        .or_else(|| path_snapshot.last().cloned())
        .unwrap_or_else(|| "<root>".to_string());

    let mut staged_zoom = app.zoom;
    let mut staged_pan = app.pan;
    let mut staged_reset = app.reset_view;

    // Temporary variable to store block to open as subsystem
    let mut block_to_open_subsystem: Option<crate::model::Block> = None;
    // Snapshots for use inside closure (avoid borrowing `app` immutably inside UI rendering)
    let block_click_handler_snapshot = app.block_click_handler.clone();
    let block_menu_items_snapshot = app.block_menu_items.clone();
    let signal_menu_items_snapshot = app.signal_menu_items.clone();

    egui::CentralPanel::default().show_inside(ui, |ui| {
        if !system_valid {
            ui.colored_label(Color32::RED, "Invalid path – nothing to render");
            return;
        }
        // Use entities snapshot for this frame
        let entities = entities_opt.as_ref().unwrap();
        // Compute blocks with positions from snapshot. Also inject SystemName
        // into a temporary, enriched block clone so later code can read it from properties.
        let system_name = system_name_snapshot.clone();
        let mut enriched_blocks: Vec<crate::model::Block> =
            Vec::with_capacity(entities.blocks.len());
        for b in &entities.blocks {
            let mut bc = b.clone();
            // Do not overwrite if already present
            bc.properties
                .entry("SystemName".to_string())
                .or_insert(system_name.clone());
            enriched_blocks.push(bc);
        }
        let blocks: Vec<(&crate::model::Block, Rect)> = enriched_blocks
            .iter()
            .filter_map(|b| parse_block_rect(b).map(|r| (b, r)))
            .collect::<Vec<_>>();
        let annotations: Vec<(&crate::model::Annotation, Rect)> = entities_opt
            .as_ref()
            .map(|entities| {
                entities
                    .annotations
                    .iter()
                    .filter_map(|a| {
                        a.position
                            .as_deref()
                            .and_then(|s| parse_rect_str(s))
                            .map(|pos| (a, pos))
                    })
                    .collect()
            })
            .unwrap_or_default();
        if blocks.is_empty() && annotations.is_empty() {
            ui.colored_label(
                Color32::YELLOW,
                "No blocks or annotations with positions to render",
            );
            return;
        }
        let mut bb = blocks
            .get(0)
            .map(|x| x.1)
            .or_else(|| annotations.get(0).map(|x| x.1))
            .unwrap();
        for (_, r) in &blocks {
            bb = bb.union(*r);
        }
        for (_, r) in &annotations {
            bb = bb.union(*r);
        }

        // Interaction space
        let margin = 20.0;
        let avail = ui.available_rect_before_wrap();
        let avail_size = avail.size();
        let width = (bb.width()).max(1.0);
        let height = (bb.height()).max(1.0);
        let sx = (avail_size.x - 2.0 * margin) / width;
        let sy = (avail_size.y - 2.0 * margin) / height;
        let base_scale = sx.min(sy).max(0.1);

        if staged_reset {
            staged_zoom = 1.0;
            staged_pan = Vec2::ZERO;
            staged_reset = false;
        }

        let canvas_resp = ui.interact(avail, ui.id().with("canvas"), Sense::drag());
        if canvas_resp.dragged() {
            let d = canvas_resp.drag_delta();
            staged_pan += d;
        }
        let scroll_y = ui.input(|i| i.raw_scroll_delta.y);
        if scroll_y.abs() > 0.0 && canvas_resp.hovered() {
            let factor = (1.0_f32 + scroll_y as f32 * 0.001_f32).max(0.1_f32);
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

        egui::Area::new("zoom_controls".into())
            .fixed_pos(Pos2::new(avail.left() + 8.0, avail.top() + 8.0))
            .show(ui.ctx(), |ui| {
                egui::Frame::menu(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let center = avail.center();
                        let origin = Pos2::new(avail.left() + margin, avail.top() + margin);
                        let mut zoom_by = |factor: f32| {
                            let old_zoom = staged_zoom;
                            let new_zoom = (old_zoom * factor).clamp(0.2, 10.0);
                            let s_old = base_scale * old_zoom;
                            let s_new = base_scale * new_zoom;
                            let world_x = (center.x - origin.x - staged_pan.x) / s_old + bb.left();
                            let world_y = (center.y - origin.y - staged_pan.y) / s_old + bb.top();
                            staged_zoom = new_zoom;
                            staged_pan.x = center.x - ((world_x - bb.left()) * s_new + origin.x);
                            staged_pan.y = center.y - ((world_y - bb.top()) * s_new + origin.y);
                        };
                        // Menu (buttons and zoom percentage) must not scale with zoom
                        if ui.small_button("−").clicked() {
                            zoom_by(0.9);
                        }
                        if ui.small_button("+").clicked() {
                            zoom_by(1.1);
                        }
                        if ui.small_button("Reset").clicked() {
                            staged_reset = true;
                        }
                        // Display current zoom level as percent
                        let percent = (staged_zoom * 100.0).round() as i32;
                        ui.label(format!("{}%", percent));
                    });
                });
            });

        let to_screen = |p: Pos2| -> Pos2 {
            let s = base_scale * staged_zoom;
            let x = (p.x - bb.left()) * s + avail.left() + margin + staged_pan.x;
            let y = (p.y - bb.top()) * s + avail.top() + margin + staged_pan.y;
            Pos2::new(x, y)
        };

        // In-canvas font scaling: baseline is 400% zoom -> scale = zoom / 4.0
        // User requested double font size, so we use / 2.0 instead of / 4.0
        let font_scale: f32 = (staged_zoom / 2.0).max(0.01);

        // Draw blocks and setup interaction maps
        let mut sid_map: HashMap<String, Rect> = HashMap::new();
        let mut sid_screen_map: HashMap<String, Rect> = HashMap::new();
        let mut block_views: Vec<(&crate::model::Block, Rect, bool, Color32)> = Vec::new();
        for (b, r) in &blocks {
            if let Some(sid) = &b.sid {
                sid_map.insert(sid.clone(), *r);
            }
            let mut r_screen = Rect::from_min_max(to_screen(r.min), to_screen(r.max));
            if let Some(sid) = &b.sid {
                sid_screen_map.insert(sid.clone(), r_screen);
            }
            let cfg = get_block_type_cfg(&b.block_type);
            let bg = block_base_color(b, &cfg);
            let mut effective_bg = bg;
            if b.commented {
                // Light gray background, no outline
                let commented_bg = Color32::from_rgb(230, 230, 230);
                effective_bg = commented_bg;
                ui.painter().rect_filled(r_screen, 0.0, commented_bg);
                // Icon in dark gray
                let icon_size = 24.0 * font_scale.max(0.01);
                let icon_center = r_screen.center();
                let font = egui::FontId::proportional(icon_size);
                let dark_icon = Color32::from_rgb(80, 80, 80);
                if let Some(icon) = cfg.icon {
                    match icon {
                        crate::block_types::IconSpec::Utf8(glyph) => {
                            ui.painter().text(
                                icon_center,
                                Align2::CENTER_CENTER,
                                glyph,
                                font,
                                dark_icon,
                            );
                        }
                    }
                }
            } else {
                // Normal block rendering
                ui.painter().rect_filled(r_screen, 6.0, bg);
            }
            let resp = ui.allocate_rect(r_screen, Sense::click());
            let mut block_action: Option<ClickAction> = None;
            if resp.double_clicked() {
                println!("Block {} double-clicked", b.name);
                block_action = Some(ClickAction::DoublePrimary);
            } else if resp.secondary_clicked() {
                println!("Block {} secondary clicked", b.name);
                block_action = Some(ClickAction::Secondary);
            } else if resp.clicked() {
                println!("Block {} clicked", b.name);
                block_action = Some(ClickAction::Primary);
            }
            if enable_context_menus {
                resp.context_menu(|ui| {
                    if ui.button("Info").clicked() {
                        record_interaction(
                            &mut interaction,
                            UpdateResponse::Block {
                                action: ClickAction::Secondary,
                                block: (*b).clone(),
                                handled: false,
                            },
                        );
                        ui.close();
                    }
                    for item in &block_menu_items_snapshot {
                        if (item.filter)(b) {
                            if ui.button(&item.label).clicked() {
                                (item.on_click)(b);
                                ui.close();
                            }
                        }
                    }
                });
            }
            if let Some(action) = block_action {
                let mut handled = false;
                if matches!(action, ClickAction::Primary | ClickAction::DoublePrimary) {
                    if let Some(handler) = block_click_handler_snapshot.as_ref() {
                        handled = handler(app, b);
                    }
                    if !handled && is_block_subsystem(b) {
                        block_to_open_subsystem = Some((*b).clone());
                    }
                }
                record_interaction(
                    &mut interaction,
                    UpdateResponse::Block {
                        action,
                        block: (*b).clone(),
                        handled,
                    },
                );
            }
            block_views.push((b, r_screen, resp.clicked(), effective_bg));
        }

        // Draw annotations (convert HTML-rich content to plain text) without background
        for (a, r_model) in &annotations {
            let r_screen = Rect::from_min_max(to_screen(r_model.min), to_screen(r_model.max));
            let _resp = ui.allocate_rect(r_screen, Sense::hover());
            let raw = a.text.clone().unwrap_or_default();
            let parsed =
                crate::egui_app::text::annotation_to_rich_text(&raw, a.interpreter.as_deref());
            let base_font = 12.0;
            let mut job = parsed.to_layout_job(ui.style(), font_scale, base_font);
            job.wrap.max_width = f32::INFINITY;
            let galley = ui.painter().layout_job(job.clone());
            let paint_pos = r_screen.left_top();
            if galley.size().x <= r_screen.width() {
                ui.painter().galley(paint_pos, galley, Color32::WHITE);
            } else {
                job.wrap.max_width = r_screen.width();
                let job_for_wrap = job.clone();
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(r_screen), |child_ui| {
                    let wrapped = child_ui.painter().layout_job(job_for_wrap);
                    child_ui
                        .painter()
                        .galley(paint_pos, wrapped, Color32::WHITE);
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

        // Build adjacency across lines for coloring
        let mut line_adjacency: Vec<Vec<usize>> = vec![Vec::new(); entities.lines.len()];
        let mut sid_to_lines: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, l) in entities.lines.iter().enumerate() {
            if let Some(src) = &l.src {
                sid_to_lines.entry(src.sid.clone()).or_default().push(i);
            }
            if let Some(dst) = &l.dst {
                sid_to_lines.entry(dst.sid.clone()).or_default().push(i);
            }
            fn collect_branch_sids(br: &crate::model::Branch, out: &mut Vec<String>) {
                if let Some(dst) = &br.dst {
                    out.push(dst.sid.clone());
                }
                for sub in &br.branches {
                    collect_branch_sids(sub, out);
                }
            }
            let mut br_sids: Vec<String> = Vec::new();
            for br in &l.branches {
                collect_branch_sids(br, &mut br_sids);
            }
            for sid in br_sids {
                sid_to_lines.entry(sid).or_default().push(i);
            }
        }
        for (_sid, idxs) in &sid_to_lines {
            for a in 0..idxs.len() {
                for b in (a + 1)..idxs.len() {
                    let i = idxs[a];
                    let j = idxs[b];
                    if !line_adjacency[i].contains(&j) {
                        line_adjacency[i].push(j);
                    }
                    if !line_adjacency[j].contains(&i) {
                        line_adjacency[j].push(i);
                    }
                }
            }
        }

        // Color assignment
        fn circular_dist(a: f32, b: f32) -> f32 {
            let d = (a - b).abs();
            d.min(1.0 - d)
        }
        fn hsv_to_color32(h: f32, s: f32, v: f32) -> Color32 {
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
            let (r, g, b) = (r1 + m, g1 + m, b1 + m);
            Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
        }
        fn hue_to_color32(h: f32) -> Color32 {
            hsv_to_color32(h, 0.85, 0.95)
        }
        fn rel_luminance(c: Color32) -> f32 {
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
        let n_lines = entities.lines.len();
        let sample_count = (n_lines.max(1) * 8).max(64);
        let mut candidates: Vec<f32> = (0..sample_count)
            .map(|i| (i as f32) / (sample_count as f32))
            .collect();
        let bg_lum = rel_luminance(Color32::from_gray(245));
        let max_lum = (bg_lum - 0.25).clamp(0.0, 1.0);
        candidates.retain(|&h| rel_luminance(hue_to_color32(h)) <= max_lum);
        if candidates.is_empty() {
            candidates = (0..sample_count)
                .map(|i| (i as f32) / (sample_count as f32))
                .collect();
        }
        let mut order: Vec<usize> = (0..n_lines).collect();
        order.sort_by_key(|&i| (-(line_adjacency[i].len() as isize), i as isize));
        let mut assigned_hues: Vec<Option<f32>> = vec![None; n_lines];
        let mut remaining: Vec<f32> = candidates.clone();
        for i in order {
            let neigh_hues: Vec<f32> = line_adjacency[i]
                .iter()
                .filter_map(|&j| assigned_hues[j])
                .collect();
            let mut best_h = 0.0;
            let mut best_score = -1.0f32;
            for &h in &remaining {
                let used: Vec<f32> = if neigh_hues.is_empty() {
                    assigned_hues.iter().flatten().copied().collect()
                } else {
                    neigh_hues.clone()
                };
                let score: f32 = if used.is_empty() {
                    1.0
                } else {
                    used.iter()
                        .map(|&u| circular_dist(h, u))
                        .fold(1.0, |a, d| f32::min(a, d))
                };
                if score > best_score || (score == best_score && h < best_h) {
                    best_score = score;
                    best_h = h;
                }
            }
            assigned_hues[i] = Some(best_h);
            if let Some(pos) = remaining
                .iter()
                .position(|&x| (x - best_h).abs() < f32::EPSILON)
            {
                remaining.remove(pos);
            }
        }
        let line_colors: Vec<Color32> = assigned_hues
            .into_iter()
            .enumerate()
            .map(|(i, h)| {
                let default_h = (i as f32) / (n_lines.max(1) as f32);
                let c = hue_to_color32(h.unwrap_or(default_h));
                if rel_luminance(c) > max_lum {
                    hsv_to_color32(h.unwrap_or(default_h), 0.85, 0.75)
                } else {
                    c
                }
            })
            .collect();

        let line_stroke_default = Stroke::new(2.0, Color32::LIGHT_GREEN);
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

        // Build lines in screen space and interactive hit rects
        let mut line_views: Vec<(
            &crate::model::Line,
            Vec<Pos2>,
            Pos2,
            Option<ClickAction>,
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
        for (li, line) in entities.lines.iter().enumerate() {
            let Some(src) = line.src.as_ref() else {
                continue;
            };
            let Some(sr) = sid_map.get(&src.sid) else {
                continue;
            };
            let mut offsets_pts: Vec<Pos2> = Vec::new();
            let num_src = port_counts
                .get(&(src.sid.clone(), if src.port_type == "out" { 1 } else { 0 }))
                .copied();
            let mirrored_src = sid_mirrored.get(&src.sid).copied().unwrap_or(false);
            let mut cur = endpoint_pos_maybe_mirrored(*sr, src, num_src, mirrored_src);
            offsets_pts.push(cur);
            for off in &line.points {
                cur = Pos2::new(cur.x + off.x as f32, cur.y + off.y as f32);
                offsets_pts.push(cur);
            }
            let mut screen_pts: Vec<Pos2> = offsets_pts.iter().map(|p| to_screen(*p)).collect();
            if let Some(src_ep) = line.src.as_ref() {
                let src_screen = *screen_pts.get(0).unwrap_or(&to_screen(cur));
                port_label_requests.push((
                    src_ep.sid.clone(),
                    src_ep.port_index,
                    false,
                    src_screen.y,
                ));
                port_y_screen.insert((src_ep.sid.clone(), src_ep.port_index, false), src_screen.y);
            }
            if let Some(dst) = line.dst.as_ref() {
                if let Some(dr) = sid_map.get(&dst.sid) {
                    let num_dst = port_counts
                        .get(&(dst.sid.clone(), if dst.port_type == "out" { 1 } else { 0 }))
                        .copied();
                    let mirrored_dst = entities
                        .blocks
                        .iter()
                        .find(|b| b.sid.as_ref() == Some(&dst.sid))
                        .and_then(|b| b.block_mirror)
                        .unwrap_or(false);
                    let dst_pt = endpoint_pos_with_target_maybe_mirrored(
                        *dr,
                        dst,
                        num_dst,
                        Some(cur.y),
                        mirrored_dst,
                    );
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
            }
            if screen_pts.is_empty() {
                continue;
            }
            let mut segments_all: Vec<(Pos2, Pos2)> = Vec::new();
            for seg in screen_pts.windows(2) {
                segments_all.push((seg[0], seg[1]));
            }
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
            let resp = ui.allocate_rect(hit_rect, Sense::click());
            let mut signal_action: Option<ClickAction> = None;
            if resp.double_clicked() {
                println!("Line {} double-clicked", li);
                signal_action = Some(ClickAction::DoublePrimary);
            } else if resp.secondary_clicked() {
                println!("Line {} secondary clicked", li);
                signal_action = Some(ClickAction::Secondary);
            } else if resp.clicked() {
                println!("Line {} clicked", li);
                signal_action = Some(ClickAction::Primary);
            }
            if enable_context_menus {
                resp.context_menu(|ui| {
                    if ui.button("Info").clicked() {
                        let line = &entities.lines[li];
                        record_interaction(
                            &mut interaction,
                            UpdateResponse::Signal {
                                action: ClickAction::Secondary,
                                line_idx: li,
                                line: line.clone(),
                                handled: false,
                            },
                        );
                        ui.close();
                    }
                    let line_ref = &entities.lines[li];
                    for item in &signal_menu_items_snapshot {
                        if (item.filter)(line_ref) {
                            if ui.button(&item.label).clicked() {
                                (item.on_click)(line_ref);
                                ui.close();
                            }
                        }
                    }
                });
            }
            let main_anchor = *offsets_pts.last().unwrap_or(&cur);
            line_views.push((
                line,
                screen_pts,
                main_anchor,
                signal_action,
                li,
                segments_all,
            ));
        }

        // Collect segments for a branch tree (model coords in, screen-space segments out)
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
            for seg in pts.windows(2) {
                out.push((to_screen(seg[0]), to_screen(seg[1])));
            }
            if let Some(dstb) = &br.dst {
                if let Some(dr) = sid_map.get(&dstb.sid) {
                    let key = (
                        dstb.sid.clone(),
                        if dstb.port_type == "out" { 1 } else { 0 },
                    );
                    let num_dst = port_counts.get(&key).copied();
                    let mirrored_dst = sid_mirrored.get(&dstb.sid).copied().unwrap_or(false);
                    let end_pt = super::geometry::endpoint_pos_with_target_maybe_mirrored(
                        *dr,
                        dstb,
                        num_dst,
                        Some(cur.y),
                        mirrored_dst,
                    );
                    let a = to_screen(*pts.last().unwrap_or(&cur));
                    let b = to_screen(end_pt);
                    out.push((a, b));
                    if dstb.port_type == "in" {
                        port_y_screen.insert((dstb.sid.clone(), dstb.port_index, true), b.y);
                    }
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
        let painter = ui.painter();
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
            let start_inset = size * 0.6;
            let tail_adj = Pos2::new(tail.x + ux * start_inset, tail.y + uy * start_inset);
            let tip_adj = Pos2::new(tip.x - ux * inset, tip.y - uy * inset);
            painter.line_segment([tail_adj, tip_adj], stroke);

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
            port_label_requests: &mut Vec<(String, u32, bool, f32)>,
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
                    let last = *pts.last().unwrap_or(&cur);
                    let a = to_screen(last);
                    let b = to_screen(end_pt);
                    if dstb.port_type == "in" {
                        draw_arrow_with_trim(painter, a, b, color, stroke);
                        port_label_requests.push((dstb.sid.clone(), dstb.port_index, true, b.y));
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

        for (line, screen_pts, main_anchor, action_opt, li, segments_all) in &line_views {
            let color = line_colors
                .get(*li)
                .copied()
                .unwrap_or(line_stroke_default.color);
            let stroke = Stroke::new(2.0, color);
            let has_in_dst = line.dst.as_ref().map_or(false, |dst| dst.port_type == "in");
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
            if let Some(action) = action_opt.clone() {
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
                    for (a, b) in hit_segments {
                        // all segments including branches already in screen space
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
                    if min_dist <= 8.0 {
                        let title = line.name.clone().unwrap_or("<signal>".into());
                        let handled = false;
                        record_interaction(
                            &mut interaction,
                            UpdateResponse::Signal {
                                action,
                                line_idx: *li,
                                line: (*line).clone(),
                                handled,
                            },
                        );
                    }
                }
            }
        }

        // Label placement
        let block_label_font = 14.0f32 * font_scale;
        let signal_font = (block_label_font * 0.5 * 1.5 * 1.5)
            .round()
            .max(7.0 * font_scale);
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
                                    screen_pts: &Vec<Pos2>,
                                    main_anchor: Pos2,
                                    color: Color32,
                                    line_idx: usize| {
            if screen_pts.len() < 2 {
                return;
            }
            let Some(label_text) = line
                .name
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
            else {
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
            let mut font_size = signal_font;
            let mut tried_wrap = false;
            let mut wrap_text = label_text.clone();
            while !final_drawn {
                let font_id = egui::FontId::proportional(font_size);
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
                                (*i as isize - (label_text.len() as isize) / 2).abs() as usize;
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
                    font_size *= 0.9;
                    if font_size < 9.0 * font_scale {
                        break;
                    }
                }
            }
        };

        for (line, screen_pts, main_anchor, _action, li, _segments_all) in &line_views {
            let color = line_colors
                .get(*li)
                .copied()
                .unwrap_or(line_stroke_default.color);
            draw_line_labels(line, screen_pts, *main_anchor, color, *li);
        }

        // Clickable labels
        for (r, li) in signal_label_rects {
            let resp = ui.interact(r, ui.id().with(("signal_label", li)), Sense::click());
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
                let line = &entities.lines[li];
                record_interaction(
                    &mut interaction,
                    UpdateResponse::Signal {
                        action,
                        line_idx: li,
                        line: line.clone(),
                        handled: false,
                    },
                );
            }
            if enable_context_menus {
                resp.context_menu(|ui| {
                    if ui.button("Info").clicked() {
                        let line = &entities.lines[li];
                        record_interaction(
                            &mut interaction,
                            UpdateResponse::Signal {
                                action: ClickAction::Secondary,
                                line_idx: li,
                                line: line.clone(),
                                handled: false,
                            },
                        );
                        ui.close();
                    }
                    let line_ref = &entities.lines[li];
                    for item in &signal_menu_items_snapshot {
                        if (item.filter)(line_ref) {
                            if ui.button(&item.label).clicked() {
                                (item.on_click)(line_ref);
                                ui.close();
                            }
                        }
                    }
                });
            }
        }

        // Finish blocks (border, icon/value, labels) and click handling
        for (b, r_screen, _clicked, bg) in &block_views {
            let cfg = get_block_type_cfg(&b.block_type);
            let border_rgb = cfg.border.unwrap_or(crate::block_types::Rgb(180, 180, 200));
            let stroke = Stroke::new(
                2.0,
                Color32::from_rgb(border_rgb.0, border_rgb.1, border_rgb.2),
            );
            painter.rect_stroke(*r_screen, 4.0, stroke, egui::StrokeKind::Inside);
            let fg = contrast_color(*bg);
            let display_signal_label = if b.block_type == "Display" {
                let sid = b.sid.as_deref();
                sid.and_then(|sid| {
                    fn branch_hits(br: &crate::model::Branch, sid: &str) -> bool {
                        if br.dst.as_ref().map_or(false, |d| d.sid == sid) {
                            return true;
                        }
                        br.branches.iter().any(|sub| branch_hits(sub, sid))
                    }

                    entities.lines.iter().find_map(|line| {
                        let direct = line.dst.as_ref().map_or(false, |dst| dst.sid == sid);
                        let branched = line.branches.iter().any(|br| branch_hits(br, sid));
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
            // Icon/value rendering with precedence: mask > value > custom/icon
            if b.block_type == "Constant" {
                let icon_font = egui::FontId::proportional(18.0 * font_scale);
                painter.text(r_screen.center(), Align2::CENTER_CENTER, "C", icon_font, fg);
            } else if b.mask.is_some() {
                if let Some(text) = b.mask_display_text.as_ref() {
                    let font_size = (b.font_size.unwrap_or(14) as f32) * font_scale;
                    let font_id = egui::FontId::proportional(font_size);
                    let color = fg;
                    let galley = painter.layout_no_wrap(text.clone(), font_id.clone(), color);
                    let pos = r_screen.center() - galley.size() * 0.5;
                    painter.galley(pos, galley, color);
                }
            } else if b
                .value
                .as_ref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
            {
                // Render block value centered; smaller than label: use beneath-label font size
                let beneath_font_px = 10.0 * font_scale; // same as label beneath block
                let font_id = egui::FontId::proportional(beneath_font_px);
                let color = fg;
                let text = b.value.as_ref().unwrap().clone();
                let galley = painter.layout_no_wrap(text, font_id.clone(), color);
                let pos = r_screen.center() - galley.size() * 0.5;
                painter.galley(pos, galley, color);
            } else if let Some(label) = display_signal_label {
                let beneath_font_px = 12.0 * font_scale;
                let font_id = egui::FontId::proportional(beneath_font_px);
                let color = fg;
                let galley = painter.layout_no_wrap(label, font_id.clone(), color);
                let pos = r_screen.center() - galley.size() * 0.5;
                painter.galley(pos, galley, color);
            } else if b.block_type == "ManualSwitch" {
                let coords_ref = b.sid.as_ref().and_then(|sid| block_port_y_map.get(sid));
                render_manual_switch(&painter, b, r_screen, font_scale, coords_ref);
            } else {
                render_block_icon(&painter, b, r_screen, font_scale);
            }
            // Respect ShowName flag when drawing label near the block according to NameLocation.
            // If value is shown or mask display is used, do not draw the name label
            let show_name = b.show_name.unwrap_or(true);
            let suppress_label = b.mask.is_some();
            if show_name && !suppress_label {
                let lines: Vec<&str> = b.name.split('\n').collect();
                let font = egui::FontId::proportional(10.0 * font_scale);
                // Force white labels beneath/around blocks for consistent readability
                let color = Color32::WHITE;
                let line_height = 16.0 * font_scale;
                match b.name_location {
                    crate::model::NameLocation::Bottom => {
                        let mut y = r_screen.bottom() + 2.0 * font_scale;
                        for line in lines.iter().copied() {
                            let pos = Pos2::new(r_screen.center().x, y);
                            painter.text(pos, Align2::CENTER_TOP, line, font.clone(), color);
                            y += line_height;
                        }
                    }
                    crate::model::NameLocation::Top => {
                        // Mirror of bottom: start just above the block and stack upwards.
                        // Keep the first line closest to the block (same as bottom behavior).
                        let mut y = r_screen.top() - 2.0 * font_scale;
                        for line in lines.iter().copied() {
                            let pos = Pos2::new(r_screen.center().x, y);
                            painter.text(pos, Align2::CENTER_BOTTOM, line, font.clone(), color);
                            y -= line_height;
                        }
                    }
                    crate::model::NameLocation::Left => {
                        // Place labels to the left of the block without overlap: align each line's right edge
                        // to r_screen.left() - gap.
                        let mut galleys: Vec<(std::sync::Arc<egui::Galley>, f32)> = Vec::new();
                        for line in lines.iter().copied() {
                            let galley =
                                painter.layout_no_wrap(line.to_string(), font.clone(), color);
                            galleys.push((galley, line_height));
                        }
                        let total_h = (lines.len() as f32) * line_height;
                        let mut y = r_screen.center().y - total_h * 0.5;
                        let gap = 2.0 * font_scale;
                        let x_right = r_screen.left() - gap;
                        for (galley, lh) in galleys {
                            let pos = Pos2::new(x_right - galley.size().x, y);
                            painter.galley(pos, galley, color);
                            y += lh;
                        }
                    }
                    crate::model::NameLocation::Right => {
                        // Place labels to the right of the block
                        let mut galleys: Vec<(std::sync::Arc<egui::Galley>, f32)> = Vec::new();
                        let mut max_w = 0.0f32;
                        for line in lines.iter().copied() {
                            let galley =
                                painter.layout_no_wrap(line.to_string(), font.clone(), color);
                            max_w = max_w.max(galley.size().x);
                            galleys.push((galley, line_height));
                        }
                        let total_h = (lines.len() as f32) * line_height;
                        let mut y = r_screen.center().y - total_h * 0.5;
                        let x = r_screen.right() + 2.0 * font_scale;
                        for (galley, lh) in galleys {
                            // draw at left-top; ensure we offset slightly from block on the right
                            let pos = Pos2::new(x + 2.0 * font_scale, y);
                            painter.galley(pos, galley, color);
                            y += lh;
                        }
                    }
                }
            }
        }

        // Draw port labels
        let mut seen_port_labels: std::collections::HashSet<(String, u32, bool, i32)> =
            Default::default();
        let font_id = egui::FontId::proportional(12.0 * font_scale);
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
            if block.mask.is_some() {
                continue;
            }
            let cfg = get_block_type_cfg(&block.block_type);
            if (is_input && !cfg.show_input_port_labels)
                || (!is_input && !cfg.show_output_port_labels)
            {
                continue;
            }
            // Swap label side if block is mirrored (inputs on right, outputs on left)
            let mirrored = block.block_mirror.unwrap_or(false);
            let logical_is_input = if mirrored { !is_input } else { is_input };
            let pname = block
                .ports
                .iter()
                .filter(|p| {
                    p.port_type == if logical_is_input { "in" } else { "out" }
                        && p.index.unwrap_or(0) == index
                })
                .filter_map(|p| {
                    p.properties
                        .get("Name")
                        .cloned()
                        .or_else(|| p.properties.get("PropagatedSignals").cloned())
                        .or_else(|| p.properties.get("name").cloned())
                        .or_else(|| {
                            Some(format!("{}{}", if is_input { "In" } else { "Out" }, index))
                        })
                })
                .next()
                .unwrap_or_else(|| format!("{}{}", if is_input { "In" } else { "Out" }, index));
            let galley = ui.painter().layout_no_wrap(
                pname.clone(),
                font_id.clone(),
                Color32::from_rgb(40, 40, 40),
            );
            let size = galley.size();
            let avail_w = brect.width() - 8.0 * font_scale;
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
    });

    // After the UI closure, call open_block_if_subsystem if needed
    if let Some(block) = block_to_open_subsystem {
        app.open_block_if_subsystem(&block);
    }

    if let Some(p) = navigate_to {
        app.navigate_to_path(p);
    }
    app.zoom = staged_zoom;
    app.pan = staged_pan;
    app.reset_view = staged_reset;
    if clear_search {
        app.search_query.clear();
        app.search_matches.clear();
    }

    interaction
}

fn build_chart_view_for_block(
    app: &SubsystemApp,
    block: &crate::model::Block,
) -> Option<ChartView> {
    let is_chart_block = block.block_type == "MATLAB Function"
        || (block.block_type == "SubSystem" && block.is_matlab_function);
    if !is_chart_block {
        return None;
    }
    let by_sid = block
        .sid
        .as_ref()
        .and_then(|sid| app.chart_map.get(sid))
        .cloned();
    let mut instance_name = if app.path.is_empty() {
        block.name.clone()
    } else {
        format!("{}/{}", app.path.join("/"), block.name)
    };
    instance_name = instance_name.trim_matches('/').to_string();
    let cid_opt = by_sid.or_else(|| app.chart_map.get(&instance_name).cloned());
    let chart = cid_opt.and_then(|cid| app.charts.get(&cid));
    chart.map(|chart| ChartView {
        title: chart
            .name
            .clone()
            .or(chart.eml_name.clone())
            .unwrap_or_else(|| block.name.clone()),
        script: chart.script.clone().unwrap_or_default(),
        open: true,
    })
}

pub fn apply_update_response(app: &mut SubsystemApp, response: &UpdateResponse) {
    match response {
        UpdateResponse::None => {}
        UpdateResponse::Signal {
            line_idx,
            line,
            handled,
            ..
        } => {
            if *handled {
                return;
            }
            let title = line.name.clone().unwrap_or("<signal>".into());
            app.signal_view = Some(SignalDialog {
                title,
                line_idx: *line_idx,
                open: true,
            });
        }
        UpdateResponse::Block { block, handled, .. } => {
            if *handled {
                return;
            }
            if is_block_subsystem(block) {
                return;
            }
            if let Some(cv) = build_chart_view_for_block(app, block) {
                app.chart_view = Some(cv);
                let title_b = format!("{} ({})", block.name, block.block_type);
                app.block_view = Some(BlockDialog {
                    title: title_b,
                    block: block.clone(),
                    open: true,
                });
            } else {
                let title = format!("{} ({})", block.name, block.block_type);
                app.block_view = Some(BlockDialog {
                    title,
                    block: block.clone(),
                    open: true,
                });
            }
        }
    }
}

fn show_chart_window(app: &mut SubsystemApp, ui: &mut egui::Ui) {
    if let Some(cv) = &mut app.chart_view {
        let mut open_flag = cv.open;
        egui::Window::new(format!("Chart: {}", cv.title))
            .open(&mut open_flag)
            .resizable(true)
            .vscroll(true)
            .min_width(400.0)
            .min_height(200.0)
            .show(ui.ctx(), |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        let job = matlab_syntax_job(&cv.script);
                        ui.add(egui::Label::new(job).wrap());
                    });
            });
        cv.open = open_flag;
        if !cv.open {
            app.chart_view = None;
        }
    }
}

fn show_signal_window(app: &mut SubsystemApp, ui: &mut egui::Ui) {
    if let Some(sd) = &app.signal_view {
        let mut open_flag = sd.open;
        let title = format!("Signal: {}", sd.title);
        let sys = app.current_system().map(|s| s.clone());
        let line_idx = sd.line_idx;
        egui::Window::new(title)
            .open(&mut open_flag)
            .resizable(true)
            .vscroll(true)
            .min_width(360.0)
            .min_height(200.0)
            .show(ui.ctx(), |ui| {
                if let Some(sys) = &sys {
                    if let Some(line) = sys.lines.get(line_idx) {
                        ui.label(RichText::new("General").strong());
                        ui.horizontal_wrapped(|ui| {
                            ui.label(format!(
                                "Name: {}",
                                line.name.clone().unwrap_or("<unnamed>".into())
                            ));
                            if let Some(z) = &line.zorder {
                                ui.label(format!("Z: {}", z));
                            }
                        });
                        ui.separator();
                        let mut outputs: Vec<EndpointRef> = Vec::new();
                        fn collect_branch_dsts(
                            br: &crate::model::Branch,
                            out: &mut Vec<EndpointRef>,
                        ) {
                            if let Some(d) = &br.dst {
                                out.push(d.clone());
                            }
                            for s in &br.branches {
                                collect_branch_dsts(s, out);
                            }
                        }
                        if let Some(d) = &line.dst {
                            outputs.push(d.clone());
                        }
                        for b in &line.branches {
                            collect_branch_dsts(b, &mut outputs);
                        }
                        egui::CollapsingHeader::new("Inputs")
                            .default_open(true)
                            .show(ui, |ui| {
                                if let Some(src) = &line.src {
                                    let bname = sys
                                        .blocks
                                        .iter()
                                        .find(|b| b.sid.as_ref() == Some(&src.sid))
                                        .map(|b| b.name.clone())
                                        .unwrap_or_else(|| format!("SID{}", src.sid));
                                    let pname = sys
                                        .blocks
                                        .iter()
                                        .find(|b| b.sid.as_ref() == Some(&src.sid))
                                        .and_then(|b| {
                                            b.ports.iter().find(|p| {
                                                p.port_type == src.port_type
                                                    && p.index.unwrap_or(0) == src.port_index
                                            })
                                        })
                                        .and_then(|p| {
                                            p.properties
                                                .get("Name")
                                                .cloned()
                                                .or_else(|| p.properties.get("name").cloned())
                                        })
                                        .unwrap_or_else(|| {
                                            format!(
                                                "{}{}",
                                                if src.port_type == "in" { "In" } else { "Out" },
                                                src.port_index
                                            )
                                        });
                                    ui.label(format!(
                                        "{} • {}{} ({}): {}",
                                        bname,
                                        if src.port_type == "in" { "In" } else { "Out" },
                                        src.port_index,
                                        src.port_type,
                                        pname
                                    ));
                                } else {
                                    ui.label("<no source>");
                                }
                            });
                        egui::CollapsingHeader::new("Outputs")
                            .default_open(true)
                            .show(ui, |ui| {
                                if outputs.is_empty() {
                                    ui.label("<none>");
                                }
                                for d in outputs {
                                    let bname = sys
                                        .blocks
                                        .iter()
                                        .find(|b| b.sid.as_ref() == Some(&d.sid))
                                        .map(|b| b.name.clone())
                                        .unwrap_or_else(|| format!("SID{}", d.sid));
                                    let pname = sys
                                        .blocks
                                        .iter()
                                        .find(|b| b.sid.as_ref() == Some(&d.sid))
                                        .and_then(|b| {
                                            b.ports.iter().find(|p| {
                                                p.port_type == d.port_type
                                                    && p.index.unwrap_or(0) == d.port_index
                                            })
                                        })
                                        .and_then(|p| {
                                            p.properties
                                                .get("Name")
                                                .cloned()
                                                .or_else(|| p.properties.get("name").cloned())
                                        })
                                        .unwrap_or_else(|| {
                                            format!(
                                                "{}{}",
                                                if d.port_type == "in" { "In" } else { "Out" },
                                                d.port_index
                                            )
                                        });
                                    ui.label(format!(
                                        "{} • {}{} ({}): {}",
                                        bname,
                                        if d.port_type == "in" { "In" } else { "Out" },
                                        d.port_index,
                                        d.port_type,
                                        pname
                                    ));
                                }
                            });
                        if !app.signal_buttons.is_empty() {
                            ui.separator();
                            ui.label(RichText::new("Actions").strong());
                            ui.horizontal_wrapped(|ui| {
                                for btn in &app.signal_buttons {
                                    if (btn.filter)(line) {
                                        if ui.button(&btn.label).clicked() {
                                            (btn.on_click)(line);
                                        }
                                    }
                                }
                            });
                        }
                    } else {
                        ui.colored_label(
                            Color32::RED,
                            "Selected signal no longer exists in this view",
                        );
                    }
                }
            });
        if let Some(sd_mut) = &mut app.signal_view {
            sd_mut.open = open_flag;
            if !sd_mut.open {
                app.signal_view = None;
            }
        }
    }
}

fn show_block_window(app: &mut SubsystemApp, ui: &mut egui::Ui) {
    if let Some(bd) = &app.block_view {
        let mut open_flag = bd.open;
        let block = bd.block.clone();
        egui::Window::new(format!("Block: {}", bd.title))
            .open(&mut open_flag)
            .resizable(true)
            .vscroll(true)
            .min_width(360.0)
            .min_height(220.0)
            .show(ui.ctx(), |ui| {
                ui.label(RichText::new("General").strong());
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!("Name: {}", block.name));
                    ui.label(format!("Type: {}", block.block_type));
                    if let Some(sid) = block.sid.as_ref() {
                        ui.label(format!("SID: {}", sid));
                    }
                    if let Some(z) = &block.zorder {
                        ui.label(format!("Z: {}", z));
                    }
                    if block.commented {
                        ui.label("commented");
                    }
                });
                ui.separator();
                egui::CollapsingHeader::new("Properties")
                    .default_open(true)
                    .show(ui, |ui| {
                        if block.properties.is_empty() {
                            ui.label("<none>");
                        }
                        for (k, v) in &block.properties {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(k).strong());
                                ui.label(v);
                            });
                        }
                    });
                if block.block_type == "CFunction" {
                    if let Some(cfg) = &block.c_function {
                        ui.separator();
                        egui::CollapsingHeader::new("C/C++ Code")
                            .default_open(true)
                            .show(ui, |ui| {
                                if let Some(s) = &cfg.start_code {
                                    ui.label(RichText::new("StartCode").strong());
                                    ui.add(
                                        egui::TextEdit::multiline(&mut s.clone())
                                            .desired_width(f32::INFINITY),
                                    );
                                }
                                if let Some(s) = &cfg.output_code {
                                    ui.label(RichText::new("OutputCode").strong());
                                    ui.add(
                                        egui::TextEdit::multiline(&mut s.clone())
                                            .desired_width(f32::INFINITY),
                                    );
                                }
                                if let Some(s) = &cfg.terminate_code {
                                    ui.label(RichText::new("TerminateCode").strong());
                                    ui.add(
                                        egui::TextEdit::multiline(&mut s.clone())
                                            .desired_width(f32::INFINITY),
                                    );
                                }
                                if let Some(s) = &cfg.codegen_start_code {
                                    ui.label(RichText::new("CodegenStartCode").strong());
                                    ui.add(
                                        egui::TextEdit::multiline(&mut s.clone())
                                            .desired_width(f32::INFINITY),
                                    );
                                }
                                if let Some(s) = &cfg.codegen_output_code {
                                    ui.label(RichText::new("CodegenOutputCode").strong());
                                    ui.add(
                                        egui::TextEdit::multiline(&mut s.clone())
                                            .desired_width(f32::INFINITY),
                                    );
                                }
                                if let Some(s) = &cfg.codegen_terminate_code {
                                    ui.label(RichText::new("CodegenTerminateCode").strong());
                                    ui.add(
                                        egui::TextEdit::multiline(&mut s.clone())
                                            .desired_width(f32::INFINITY),
                                    );
                                }
                            });
                    }
                }
                egui::CollapsingHeader::new("Ports")
                    .default_open(true)
                    .show(ui, |ui| {
                        if block.ports.is_empty() {
                            ui.label("<none>");
                            return;
                        }
                        let mut ins: Vec<&crate::model::Port> =
                            block.ports.iter().filter(|p| p.port_type == "in").collect();
                        let mut outs: Vec<&crate::model::Port> = block
                            .ports
                            .iter()
                            .filter(|p| p.port_type == "out")
                            .collect();
                        ins.sort_by_key(|p| p.index.unwrap_or(0));
                        outs.sort_by_key(|p| p.index.unwrap_or(0));
                        if !ins.is_empty() {
                            ui.label(RichText::new("Inputs").strong());
                        }
                        for p in ins {
                            let idx = p.index.unwrap_or(0);
                            let name = p
                                .properties
                                .get("Name")
                                .or_else(|| p.properties.get("name"))
                                .cloned()
                                .unwrap_or_else(|| format!("In{}", idx));
                            ui.label(format!("{}{}: {}", "In", idx, name));
                        }
                        if !outs.is_empty() {
                            ui.separator();
                            ui.label(RichText::new("Outputs").strong());
                        }
                        for p in outs {
                            let idx = p.index.unwrap_or(0);
                            let name = p
                                .properties
                                .get("Name")
                                .or_else(|| p.properties.get("name"))
                                .cloned()
                                .unwrap_or_else(|| format!("Out{}", idx));
                            ui.label(format!("{}{}: {}", "Out", idx, name));
                        }
                    });
                if !app.block_buttons.is_empty() {
                    ui.separator();
                    ui.label(RichText::new("Actions").strong());
                    ui.horizontal_wrapped(|ui| {
                        for btn in &app.block_buttons {
                            if (btn.filter)(&block) {
                                if ui.button(&btn.label).clicked() {
                                    (btn.on_click)(&block);
                                }
                            }
                        }
                    });
                }
            });
        if let Some(bd_mut) = &mut app.block_view {
            bd_mut.open = open_flag;
            if !bd_mut.open {
                app.block_view = None;
            }
        }
    }
}

pub fn show_info_windows(app: &mut SubsystemApp, ui: &mut egui::Ui) {
    show_chart_window(app, ui);
    show_signal_window(app, ui);
    show_block_window(app, ui);
}

pub fn update(app: &mut SubsystemApp, ui: &mut egui::Ui) -> UpdateResponse {
    update_internal(app, ui, false)
}

pub fn update_with_info(app: &mut SubsystemApp, ui: &mut egui::Ui) -> UpdateResponse {
    let response = update_internal(app, ui, true);
    apply_update_response(app, &response);
    show_info_windows(app, ui);
    response
}
