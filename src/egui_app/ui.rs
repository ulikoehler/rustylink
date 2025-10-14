#![cfg(feature = "egui")]

use std::collections::HashMap;

use eframe::egui::{self, Align2, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};

use crate::model::EndpointRef;

use super::geometry::{endpoint_pos, endpoint_pos_with_target, parse_block_rect, parse_rect_str};
use super::render::{get_block_type_cfg, render_block_icon};
use super::state::{BlockDialog, ChartView, SignalDialog, SubsystemApp};
use super::text::{highlight_query_job, matlab_syntax_job};

pub fn update(app: &mut SubsystemApp, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    let mut navigate_to: Option<Vec<String>> = None;
    let mut clear_search = false;
    let path_snapshot = app.path.clone();

    egui::TopBottomPanel::top("top").show(ctx, |ui| {
        ui.horizontal(|ui| {
            let up = ui.add_enabled(!path_snapshot.is_empty(), egui::Button::new("↑ Up"));
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
                egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
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

    let Some(current_system) = app.current_system() else {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.colored_label(Color32::RED, "Invalid path – nothing to render");
        });
        return;
    };

    let mut navigate_to_from_block: Option<Vec<String>> = None;
    let mut open_chart: Option<ChartView> = None;
    let mut open_signal: Option<SignalDialog> = None;
    let mut open_block: Option<BlockDialog> = None;
    let mut staged_zoom = app.zoom;
    let mut staged_pan = app.pan;
    let mut staged_reset = app.reset_view;

    egui::CentralPanel::default().show(ctx, |ui| {
        // Compute blocks and bounds
        let blocks: Vec<(&crate::model::Block, Rect)> = current_system
            .blocks
            .iter()
            .filter_map(|b| parse_block_rect(b).map(|r| (b, r)))
            .collect();
        let annotations: Vec<(&crate::model::Annotation, Rect)> = {
            let mut v: Vec<(&crate::model::Annotation, Rect)> = Vec::new();
            for a in &current_system.annotations {
                if let Some(pos) = a.position.as_deref().and_then(|s| parse_rect_str(s)) {
                    v.push((a, pos));
                }
            }
            // include block-local annotations as well
            for b in &current_system.blocks {
                for a in &b.annotations {
                    if let Some(pos) = a.position.as_deref().and_then(|s| parse_rect_str(s)) {
                        v.push((a, pos));
                    }
                }
            }
            v
        };
        if blocks.is_empty() && annotations.is_empty() {
            ui.colored_label(Color32::YELLOW, "No blocks or annotations with positions to render");
            return;
        }
        let mut bb = blocks.get(0).map(|x| x.1).or_else(|| annotations.get(0).map(|x| x.1)).unwrap();
        for (_, r) in &blocks { bb = bb.union(*r); }
        for (_, r) in &annotations { bb = bb.union(*r); }

        // Interaction space
        let margin = 20.0;
        let avail = ui.available_rect_before_wrap();
        let avail_size = avail.size();
        let width = (bb.width()).max(1.0);
        let height = (bb.height()).max(1.0);
        let sx = (avail_size.x - 2.0 * margin) / width;
        let sy = (avail_size.y - 2.0 * margin) / height;
        let base_scale = sx.min(sy).max(0.1);

        if staged_reset { staged_zoom = 1.0; staged_pan = Vec2::ZERO; staged_reset = false; }

        let canvas_resp = ui.interact(avail, ui.id().with("canvas"), Sense::drag());
        if canvas_resp.dragged() { let d = canvas_resp.drag_delta(); staged_pan += d; }
        let scroll_y = ctx.input(|i| i.raw_scroll_delta.y);
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
                        if ui.small_button("−").clicked() { zoom_by(0.9); }
                        if ui.small_button("+").clicked() { zoom_by(1.1); }
                        if ui.small_button("Reset").clicked() { staged_reset = true; }
                    });
                });
            });

        let to_screen = |p: Pos2| -> Pos2 {
            let s = base_scale * staged_zoom;
            let x = (p.x - bb.left()) * s + avail.left() + margin + staged_pan.x;
            let y = (p.y - bb.top()) * s + avail.top() + margin + staged_pan.y;
            Pos2::new(x, y)
        };

    // Draw blocks and setup interaction maps
        let mut sid_map: HashMap<String, Rect> = HashMap::new();
        let mut sid_screen_map: HashMap<String, Rect> = HashMap::new();
        let mut block_views: Vec<(&crate::model::Block, Rect, bool)> = Vec::new();
        for (b, r) in &blocks {
            if let Some(sid) = &b.sid { sid_map.insert(sid.clone(), *r); }
            let r_screen = Rect::from_min_max(to_screen(r.min), to_screen(r.max));
            if let Some(sid) = &b.sid { sid_screen_map.insert(sid.clone(), r_screen); }
            let cfg = get_block_type_cfg(&b.block_type);
            // If block.background_color is set, override type color
            let bg = if let Some(ref color_str) = b.background_color {
                match color_str.to_lowercase().as_str() {
                    "yellow" => Color32::YELLOW,
                    "red" => Color32::RED,
                    "green" => Color32::GREEN,
                    "blue" => Color32::BLUE,
                    "black" => Color32::BLACK,
                    "white" => Color32::WHITE,
                    "gray" | "grey" => Color32::from_rgb(128,128,128),
                    other => {
                        eprintln!("[rustylink] Warning: unknown block background color '{}', using default.", color_str);
                        Color32::from_rgb(210, 210, 210)
                    }
                }
            } else {
                cfg.background.map(|c| Color32::from_rgb(c.0, c.1, c.2)).unwrap_or_else(|| Color32::from_rgb(210, 210, 210))
            };
            ui.painter().rect_filled(r_screen, 6.0, bg);
            // Only show block name if show_name is not set to false
            let show_name = b.show_name.unwrap_or(true);
            if show_name {
                let font_id = egui::FontId::proportional(14.0);
                let color = Color32::BLACK;
                let galley = ui.painter().layout_no_wrap(b.name.clone(), font_id, color);
                let text_pos = r_screen.center_top() + egui::vec2(0.0, 4.0);
                ui.painter().galley(text_pos, galley, color);
            }
            let resp = ui.allocate_rect(r_screen, Sense::click());
            resp.context_menu(|ui| {
                if ui.button("Info").clicked() {
                    let title = format!("{} ({})", b.name, b.block_type);
                    open_block = Some(BlockDialog { title, block: (*b).clone(), open: true });
                    ui.close();
                }
                for item in &app.block_menu_items {
                    if (item.filter)(b) {
                        if ui.button(&item.label).clicked() { (item.on_click)(b); ui.close(); }
                    }
                }
            });
            block_views.push((b, r_screen, resp.clicked()));
        }

        // Draw annotations (convert HTML-rich content to plain text) without background
        for (a, r_model) in &annotations {
            let r_screen = Rect::from_min_max(to_screen(r_model.min), to_screen(r_model.max));
            let _resp = ui.allocate_rect(r_screen, Sense::hover());
            let raw = a.text.clone().unwrap_or_default();
            let text = crate::egui_app::text::annotation_to_plain_text(&raw, a.interpreter.as_deref());
            let font_id = egui::FontId::proportional(12.0);
            let color = Color32::from_rgb(20, 20, 20);
            let galley = ui.painter().layout_no_wrap(text.clone(), font_id.clone(), color);
            // If text wider than rect, use wrapped label via a temporary UI
            if galley.size().x <= r_screen.width() {
                ui.painter().galley(r_screen.left_top(), galley, color);
            } else {
                // wrapped
                // Use allocate_ui_at_rect to create a temporary child UI for wrapping
                ui.allocate_ui_at_rect(r_screen, |child_ui| {
                    child_ui.label(egui::RichText::new(text).size(12.0).color(color));
                });
            }
            // no special tooltip; text is directly visible inside the rectangle
        }

        // Precompute lookup maps
        let mut sid_to_name: HashMap<String, String> = HashMap::new();
        for (b, _r) in &blocks { if let Some(sid) = &b.sid { sid_to_name.insert(sid.clone(), b.name.clone()); } }

        // Build adjacency across lines for coloring
        let mut line_adjacency: Vec<Vec<usize>> = vec![Vec::new(); current_system.lines.len()];
        let mut sid_to_lines: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, l) in current_system.lines.iter().enumerate() {
            if let Some(src) = &l.src { sid_to_lines.entry(src.sid.clone()).or_default().push(i); }
            if let Some(dst) = &l.dst { sid_to_lines.entry(dst.sid.clone()).or_default().push(i); }
            fn collect_branch_sids(br: &crate::model::Branch, out: &mut Vec<String>) {
                if let Some(dst) = &br.dst { out.push(dst.sid.clone()); }
                for sub in &br.branches { collect_branch_sids(sub, out); }
            }
            let mut br_sids: Vec<String> = Vec::new();
            for br in &l.branches { collect_branch_sids(br, &mut br_sids); }
            for sid in br_sids { sid_to_lines.entry(sid).or_default().push(i); }
        }
        for (_sid, idxs) in &sid_to_lines {
            for a in 0..idxs.len() {
                for b in (a+1)..idxs.len() {
                    let i = idxs[a];
                    let j = idxs[b];
                    if !line_adjacency[i].contains(&j) { line_adjacency[i].push(j); }
                    if !line_adjacency[j].contains(&i) { line_adjacency[j].push(i); }
                }
            }
        }

        // Color assignment
        fn circular_dist(a: f32, b: f32) -> f32 { let d = (a - b).abs(); d.min(1.0 - d) }
        fn hsv_to_color32(h: f32, s: f32, v: f32) -> Color32 {
            let h6 = (h * 6.0) % 6.0;
            let c = v * s;
            let x = c * (1.0 - ((h6 % 2.0) - 1.0).abs());
            let (r1, g1, b1) = if h6 < 1.0 { (c, x, 0.0) }
            else if h6 < 2.0 { (x, c, 0.0) }
            else if h6 < 3.0 { (0.0, c, x) }
            else if h6 < 4.0 { (0.0, x, c) }
            else if h6 < 5.0 { (x, 0.0, c) } else { (c, 0.0, x) };
            let m = v - c;
            let (r, g, b) = (r1 + m, g1 + m, b1 + m);
            Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
        }
        fn hue_to_color32(h: f32) -> Color32 { hsv_to_color32(h, 0.85, 0.95) }
        fn rel_luminance(c: Color32) -> f32 {
            fn to_lin(u: u8) -> f32 { let s = (u as f32)/255.0; if s <= 0.04045 { s/12.92 } else { ((s+0.055)/1.055).powf(2.4) } }
            0.2126 * to_lin(c.r()) + 0.7152 * to_lin(c.g()) + 0.0722 * to_lin(c.b())
        }
        let n_lines = current_system.lines.len();
        let sample_count = (n_lines.max(1) * 8).max(64);
        let mut candidates: Vec<f32> = (0..sample_count).map(|i| (i as f32)/(sample_count as f32)).collect();
        let bg_lum = rel_luminance(Color32::from_gray(245));
        let max_lum = (bg_lum - 0.25).clamp(0.0, 1.0);
        candidates.retain(|&h| rel_luminance(hue_to_color32(h)) <= max_lum);
        if candidates.is_empty() { candidates = (0..sample_count).map(|i| (i as f32)/(sample_count as f32)).collect(); }
        let mut order: Vec<usize> = (0..n_lines).collect();
        order.sort_by_key(|&i| (-(line_adjacency[i].len() as isize), i as isize));
        let mut assigned_hues: Vec<Option<f32>> = vec![None; n_lines];
        let mut remaining: Vec<f32> = candidates.clone();
        for i in order {
            let neigh_hues: Vec<f32> = line_adjacency[i].iter().filter_map(|&j| assigned_hues[j]).collect();
            let mut best_h = 0.0; let mut best_score = -1.0f32;
            for &h in &remaining {
                let used: Vec<f32> = if neigh_hues.is_empty() { assigned_hues.iter().flatten().copied().collect() } else { neigh_hues.clone() };
                let score: f32 = if used.is_empty() { 1.0 } else { used.iter().map(|&u| circular_dist(h, u)).fold(1.0, |a, d| f32::min(a, d)) };
                if score > best_score || (score == best_score && h < best_h) { best_score = score; best_h = h; }
            }
            assigned_hues[i] = Some(best_h);
            if let Some(pos) = remaining.iter().position(|&x| (x - best_h).abs() < f32::EPSILON) { remaining.remove(pos); }
        }
        let line_colors: Vec<Color32> = assigned_hues.into_iter().enumerate().map(|(i, h)| {
            let default_h = (i as f32) / (n_lines.max(1) as f32);
            let c = hue_to_color32(h.unwrap_or(default_h));
            if rel_luminance(c) > max_lum { hsv_to_color32(h.unwrap_or(default_h), 0.85, 0.75) } else { c }
        }).collect();

        let line_stroke_default = Stroke::new(2.0, Color32::LIGHT_GREEN);
        let mut port_counts: HashMap<(String, u8), u32> = HashMap::new();
        fn reg_ep(ep: &EndpointRef, port_counts: &mut HashMap<(String, u8), u32>) {
            let key = (ep.sid.clone(), if ep.port_type == "out" { 1 } else { 0 });
            let idx1 = if ep.port_index == 0 { 1 } else { ep.port_index };
            port_counts.entry(key).and_modify(|v| *v = (*v).max(idx1)).or_insert(idx1);
        }
        fn reg_branch(br: &crate::model::Branch, port_counts: &mut HashMap<(String, u8), u32>) {
            if let Some(dst) = &br.dst { reg_ep(dst, port_counts); }
            for sub in &br.branches { reg_branch(sub, port_counts); }
        }
        for line in &current_system.lines {
            if let Some(src) = &line.src { reg_ep(src, &mut port_counts); }
            if let Some(dst) = &line.dst { reg_ep(dst, &mut port_counts); }
            for br in &line.branches { reg_branch(br, &mut port_counts); }
        }

        // Build lines in screen space and interactive hit rects
        let mut line_views: Vec<(&crate::model::Line, Vec<Pos2>, Pos2, bool, usize, Vec<(Pos2, Pos2)>)> = Vec::new();
        let mut port_label_requests: Vec<(String, u32, bool, f32)> = Vec::new();
        for (li, line) in current_system.lines.iter().enumerate() {
            let Some(src) = line.src.as_ref() else { continue; };
            let Some(sr) = sid_map.get(&src.sid) else { continue; };
            let mut offsets_pts: Vec<Pos2> = Vec::new();
            let num_src = port_counts.get(&(src.sid.clone(), if src.port_type == "out" { 1 } else { 0 })).copied();
            let mut cur = endpoint_pos(*sr, src, num_src);
            offsets_pts.push(cur);
            for off in &line.points { cur = Pos2::new(cur.x + off.x as f32, cur.y + off.y as f32); offsets_pts.push(cur); }
            let mut screen_pts: Vec<Pos2> = offsets_pts.iter().map(|p| to_screen(*p)).collect();
            if let Some(src_ep) = line.src.as_ref() {
                let src_screen = *screen_pts.get(0).unwrap_or(&to_screen(cur));
                port_label_requests.push((src_ep.sid.clone(), src_ep.port_index, false, src_screen.y));
            }
            if let Some(dst) = line.dst.as_ref() {
                if let Some(dr) = sid_map.get(&dst.sid) {
                    let num_dst = port_counts.get(&(dst.sid.clone(), if dst.port_type == "out" { 1 } else { 0 })).copied();
                    let dst_pt = endpoint_pos_with_target(*dr, dst, num_dst, Some(cur.y));
                    let dst_screen = to_screen(dst_pt);
                    screen_pts.push(dst_screen);
                    if dst.port_type == "in" { port_label_requests.push((dst.sid.clone(), dst.port_index, true, dst_screen.y)); }
                }
            }
            if screen_pts.is_empty() { continue; }
            let mut segments_all: Vec<(Pos2, Pos2)> = Vec::new();
            for seg in screen_pts.windows(2) { segments_all.push((seg[0], seg[1])); }
            for br in &line.branches { collect_branch_segments_rec(&to_screen, &sid_map, &port_counts, *offsets_pts.last().unwrap_or(&cur), br, &mut segments_all); }
            let pad = 8.0;
            let (min_x, max_x, min_y, max_y) = segments_all.iter().fold((f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY), |(min_x, max_x, min_y, max_y), (a,b)| {
                (min_x.min(a.x.min(b.x)), max_x.max(a.x.max(b.x)), min_y.min(a.y.min(b.y)), max_y.max(a.y.max(b.y)))
            });
            let hit_rect = Rect::from_min_max(Pos2::new(min_x - pad, min_y - pad), Pos2::new(max_x + pad, max_y + pad));
            let resp = ui.allocate_rect(hit_rect, Sense::click());
            let clicked = resp.clicked();
            resp.context_menu(|ui| {
                if ui.button("Info").clicked() {
                    let line = &current_system.lines[li];
                    let title = line.name.clone().unwrap_or("<signal>".into());
                    open_signal = Some(SignalDialog { title, line_idx: li, open: true });
                    ui.close();
                }
                let line_ref = &current_system.lines[li];
                for item in &app.signal_menu_items {
                    if (item.filter)(line_ref) { if ui.button(&item.label).clicked() { (item.on_click)(line_ref); ui.close(); } }
                }
            });
            let main_anchor = *offsets_pts.last().unwrap_or(&cur);
            line_views.push((line, screen_pts, main_anchor, clicked, li, segments_all));
        }

        // Collect segments for a branch tree (model coords in, screen-space segments out)
        fn collect_branch_segments_rec(
            to_screen: &dyn Fn(Pos2) -> Pos2,
            sid_map: &HashMap<String, Rect>,
            port_counts: &HashMap<(String, u8), u32>,
            start: Pos2,
            br: &crate::model::Branch,
            out: &mut Vec<(Pos2, Pos2)>,
        ) {
            let mut pts: Vec<Pos2> = vec![start];
            let mut cur = start;
            for off in &br.points { cur = Pos2::new(cur.x + off.x as f32, cur.y + off.y as f32); pts.push(cur); }
            for seg in pts.windows(2) { out.push((to_screen(seg[0]), to_screen(seg[1]))); }
            if let Some(dstb) = &br.dst { if let Some(dr) = sid_map.get(&dstb.sid) {
                let key = (dstb.sid.clone(), if dstb.port_type == "out" { 1 } else { 0 });
                let num_dst = port_counts.get(&key).copied();
                let end_pt = super::geometry::endpoint_pos_with_target(*dr, dstb, num_dst, Some(cur.y));
                out.push((to_screen(*pts.last().unwrap_or(&cur)), to_screen(end_pt)));
            }}
            for sub in &br.branches { collect_branch_segments_rec(to_screen, sid_map, port_counts, *pts.last().unwrap_or(&cur), sub, out); }
        }

        // Draw lines and branches
        let painter = ui.painter();
        fn draw_arrowhead(painter: &egui::Painter, tail: Pos2, tip: Pos2, color: Color32) {
            let size = 8.0_f32;
            let dir = Vec2::new(tip.x - tail.x, tip.y - tail.y);
            let len = (dir.x * dir.x + dir.y * dir.y).sqrt().max(1e-3);
            let ux = dir.x / len; let uy = dir.y / len;
            let px = -uy; let py = ux;
            let base = Pos2::new(tip.x - ux * size, tip.y - uy * size);
            let left = Pos2::new(base.x + px * (size * 0.6), base.y + py * (size * 0.6));
            let right = Pos2::new(base.x - px * (size * 0.6), base.y - py * (size * 0.6));
            painter.add(egui::Shape::convex_polygon(vec![tip, left, right], color, Stroke::NONE));
        }

        fn draw_branch_rec(
            painter: &egui::Painter,
            to_screen: &dyn Fn(Pos2) -> Pos2,
            sid_map: &HashMap<String, Rect>,
            port_counts: &HashMap<(String, u8), u32>,
            start: Pos2,
            br: &crate::model::Branch,
            color: Color32,
            port_label_requests: &mut Vec<(String, u32, bool, f32)>,
        ) {
            let mut pts: Vec<Pos2> = vec![start];
            let mut cur = start;
            for off in &br.points { cur = Pos2::new(cur.x + off.x as f32, cur.y + off.y as f32); pts.push(cur); }
            for seg in pts.windows(2) { painter.line_segment([to_screen(seg[0]), to_screen(seg[1])], Stroke::new(2.0, color)); }
            if let Some(dstb) = &br.dst { if let Some(dr) = sid_map.get(&dstb.sid) {
                let key = (dstb.sid.clone(), if dstb.port_type == "out" { 1 } else { 0 });
                let num_dst = port_counts.get(&key).copied();
                let end_pt = endpoint_pos_with_target(*dr, dstb, num_dst, Some(cur.y));
                let last = *pts.last().unwrap_or(&cur);
                let a = to_screen(last); let b = to_screen(end_pt);
                painter.line_segment([a, b], Stroke::new(2.0, color));
                if dstb.port_type == "in" { draw_arrowhead(painter, a, b, color); port_label_requests.push((dstb.sid.clone(), dstb.port_index, true, b.y)); }
            }}
            for sub in &br.branches { draw_branch_rec(painter, to_screen, sid_map, port_counts, *pts.last().unwrap_or(&cur), sub, color, port_label_requests); }
        }

        let mut signal_label_rects: Vec<(Rect, usize)> = Vec::new();

        for (line, screen_pts, main_anchor, clicked, li, segments_all) in &line_views {
            let color = line_colors.get(*li).copied().unwrap_or(line_stroke_default.color);
            let stroke = Stroke::new(2.0, color);
            for seg in screen_pts.windows(2) { painter.line_segment([seg[0], seg[1]], stroke); }
            if let Some(dst) = &line.dst { if dst.port_type == "in" && screen_pts.len() >= 2 {
                let n = screen_pts.len(); let a = screen_pts[n-2]; let b = screen_pts[n-1];
                draw_arrowhead(&painter, a, b, color);
            }}
            for br in &line.branches { draw_branch_rec(&painter, &to_screen, &sid_map, &port_counts, *main_anchor, br, color, &mut port_label_requests); }
            if *clicked {
                if let Some(cp) = ctx.input(|i| i.pointer.interact_pos()) {
                    let mut min_dist = f32::INFINITY;
                    for (a, b) in segments_all { // all segments including branches already in screen space
                        let ab_x = b.x - a.x; let ab_y = b.y - a.y;
                        let ap_x = cp.x - a.x; let ap_y = cp.y - a.y;
                        let ab_len2 = (ab_x * ab_x + ab_y * ab_y).max(1e-6);
                        let t = (ap_x * ab_x + ap_y * ab_y) / ab_len2;
                        let t_clamped = t.max(0.0).min(1.0);
                        let proj_x = a.x + ab_x * t_clamped; let proj_y = a.y + ab_y * t_clamped;
                        let dx = cp.x - proj_x; let dy = cp.y - proj_y; let dist = (dx * dx + dy * dy).sqrt();
                        if dist < min_dist { min_dist = dist; }
                    }
                    if min_dist <= 8.0 {
                        let title = line.name.clone().unwrap_or("<signal>".into());
                        open_signal = Some(SignalDialog { title, line_idx: *li, open: true });
                    }
                }
            }
        }

        // Label placement
        let block_label_font = 14.0f32;
        let signal_font = (block_label_font * 0.5 * 1.5 * 1.5).round().max(7.0);
        struct EguiMeasurer<'a> { ctx: &'a egui::Context, font: egui::FontId, color: Color32 }
        impl<'a> crate::label_place::Measurer for EguiMeasurer<'a> {
            fn measure(&self, text: &str) -> (f32, f32) {
                let galley = self.ctx.fonts(|f| f.layout_no_wrap(text.to_string(), self.font.clone(), self.color));
                let s = galley.size(); (s.x, s.y)
            }
        }
        let cfg = crate::label_place::Config { expand_factor: 1.5, step_fraction: 0.25, perp_offset: 2.0 };
        let mut placed_label_rects: Vec<Rect> = Vec::new();
        let mut draw_line_labels = |line: &crate::model::Line, screen_pts: &Vec<Pos2>, main_anchor: Pos2, color: Color32, line_idx: usize| {
            if screen_pts.len() < 2 { return; }
            let Some(label_text) = line.name.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| s.to_string()) else { return; };
            let mut segments: Vec<(Pos2, Pos2)> = Vec::new();
            for seg in screen_pts.windows(2) { segments.push((seg[0], seg[1])); }
            for br in &line.branches { collect_branch_segments_rec(&to_screen, &sid_map, &port_counts, main_anchor, br, &mut segments); }
            let mut best_len2 = -1.0f32; let mut best_seg: Option<(Pos2, Pos2)> = None;
            for (a, b) in &segments { let dx = b.x - a.x; let dy = b.y - a.y; let l2 = dx*dx + dy*dy; if l2 > best_len2 { best_len2 = l2; best_seg = Some((*a, *b)); } }
            let Some((sa, sb)) = best_seg else { return; };
            let poly: Vec<crate::label_place::Vec2f> = vec![crate::label_place::Vec2f{ x: sa.x, y: sa.y }, crate::label_place::Vec2f{ x: sb.x, y: sb.y }];
            let mut avoid_rects: Vec<crate::label_place::RectF> = placed_label_rects.iter().map(|r| crate::label_place::RectF::from_min_max(crate::label_place::Vec2f{ x: r.left(), y: r.top() }, crate::label_place::Vec2f{ x: r.right(), y: r.bottom() })).collect();
            for (_b, br, _clicked) in &block_views { avoid_rects.push(crate::label_place::RectF::from_min_max(crate::label_place::Vec2f{ x: br.left(), y: br.top() }, crate::label_place::Vec2f{ x: br.right(), y: br.bottom() })); }
            let line_thickness = 0.8f32;
            for (a, b) in &segments {
                let min_x = a.x.min(b.x) - line_thickness; let max_x = a.x.max(b.x) + line_thickness;
                let min_y = a.y.min(b.y) - line_thickness; let max_y = a.y.max(b.y) + line_thickness;
                avoid_rects.push(crate::label_place::RectF::from_min_max(crate::label_place::Vec2f{ x: min_x, y: min_y }, crate::label_place::Vec2f{ x: max_x, y: max_y }));
            }
            let mut final_drawn = false; let mut font_size = signal_font; let mut tried_wrap = false; let mut wrap_text = label_text.clone();
            while !final_drawn {
                let font_id = egui::FontId::proportional(font_size);
                let meas = EguiMeasurer { ctx, font: font_id.clone(), color };
                let candidate_texts: Vec<String> = if !tried_wrap && label_text.contains(' ') {
                    let bytes: Vec<(usize, char)> = label_text.char_indices().collect();
                    let mut best_split = None; let mut best_dist = usize::MAX;
                    for (i, ch) in bytes.iter() {
                        if *ch == ' ' { let dist = (*i as isize - (label_text.len() as isize)/2).abs() as usize; if dist < best_dist { best_dist = dist; best_split = Some(*i); } }
                    }
                    if let Some(split) = best_split { wrap_text = format!("{}\n{}", &label_text[..split].trim_end(), &label_text[split+1..].trim_start()); vec![label_text.clone(), wrap_text.clone()] } else { vec![label_text.clone()] }
                } else { vec![label_text.clone(), wrap_text.clone()] };
                for candidate in candidate_texts.into_iter().filter(|s| !s.is_empty()) {
                    if let Some(result) = crate::label_place::place_label(&poly, &candidate, &meas, cfg, &avoid_rects) {
                        let oriented_text = if result.horizontal { candidate.clone() } else {
                            let mut s = String::new(); for (i, ch) in candidate.chars().enumerate() { if i > 0 { s.push('\n'); } s.push(ch); } s
                        };
                        let galley = ctx.fonts(|f| f.layout_no_wrap(oriented_text.clone(), font_id.clone(), color));
                        let draw_pos = Pos2::new(result.rect.min.x, result.rect.min.y);
                        painter.galley(draw_pos, galley, color);
                        let rect = Rect::from_min_max(Pos2::new(result.rect.min.x, result.rect.min.y), Pos2::new(result.rect.max.x, result.rect.max.y));
                        placed_label_rects.push(rect); signal_label_rects.push((rect, line_idx)); final_drawn = true; break;
                    }
                }
                if final_drawn { break; }
                if !tried_wrap && label_text.contains(' ') { tried_wrap = true; } else { font_size *= 0.9; if font_size < 9.0 { break; } }
            }
        };

        for (line, screen_pts, main_anchor, _clicked, li, _segments_all) in &line_views {
            let color = line_colors.get(*li).copied().unwrap_or(line_stroke_default.color);
            draw_line_labels(line, screen_pts, *main_anchor, color, *li);
        }

        // Clickable labels
        for (r, li) in signal_label_rects {
            let resp = ui.interact(r, ui.id().with(("signal_label", li)), Sense::click());
            if resp.clicked() {
                let line = &current_system.lines[li];
                let title = line.name.clone().unwrap_or("<signal>".into());
                open_signal = Some(SignalDialog { title, line_idx: li, open: true });
            }
            resp.context_menu(|ui| {
                if ui.button("Info").clicked() { let line = &current_system.lines[li]; let title = line.name.clone().unwrap_or("<signal>".into()); open_signal = Some(SignalDialog { title, line_idx: li, open: true }); ui.close(); }
                let line_ref = &current_system.lines[li];
                for item in &app.signal_menu_items { if (item.filter)(line_ref) { if ui.button(&item.label).clicked() { (item.on_click)(line_ref); ui.close(); } } }
            });
        }

        // Finish blocks (border, icon, labels) and click handling
        for (b, r_screen, clicked) in &block_views {
            let cfg = get_block_type_cfg(&b.block_type);
            let border_rgb = cfg.border.unwrap_or(crate::block_types::Rgb(180, 180, 200));
            let stroke = Stroke::new(2.0, Color32::from_rgb(border_rgb.0, border_rgb.1, border_rgb.2));
            painter.rect_stroke(*r_screen, 4.0, stroke, egui::StrokeKind::Inside);
            render_block_icon(&painter, b, r_screen);
            let lines: Vec<&str> = b.name.split('\n').collect();
            let line_height = 16.0; let mut y = r_screen.bottom() + 2.0;
            for line in lines { let pos = Pos2::new(r_screen.center().x, y); painter.text(pos, Align2::CENTER_TOP, line, egui::FontId::proportional(14.0), Color32::from_rgb(40, 40, 40)); y += line_height; }
            if *clicked {
                if b.block_type == "MATLAB Function" || (b.block_type == "SubSystem" && b.is_matlab_function) {
                    let by_sid = b.sid.as_ref().and_then(|k| app.chart_map.get(k)).cloned();
                    let mut instance_name = if path_snapshot.is_empty() { b.name.clone() } else { format!("{}/{}", path_snapshot.join("/"), b.name) };
                    instance_name = instance_name.trim_matches('/').to_string();
                    let cid_opt = by_sid.or_else(|| app.chart_map.get(&instance_name).cloned());
                    if let Some(cid) = cid_opt { if let Some(chart) = app.charts.get(&cid) {
                        let title = chart.name.clone().or(chart.eml_name.clone()).unwrap_or_else(|| b.name.clone());
                        let script = chart.script.clone().unwrap_or_default();
                        open_chart = Some(ChartView { title, script, open: true });
                        let title_b = format!("{} ({})", b.name, b.block_type);
                        open_block = Some(BlockDialog { title: title_b, block: (*b).clone(), open: true });
                    }}
                } else if b.block_type == "SubSystem" {
                    if let Some(_sub) = &b.subsystem { let mut np = path_snapshot.clone(); np.push(b.name.clone()); navigate_to_from_block = Some(np); }
                } else {
                    let title = format!("{} ({})", b.name, b.block_type);
                    open_block = Some(BlockDialog { title, block: (*b).clone(), open: true });
                }
            }
        }

        // Draw port labels
        let mut seen_port_labels: std::collections::HashSet<(String, u32, bool, i32)> = Default::default();
        let font_id = egui::FontId::proportional(12.0);
        for (sid, index, is_input, y) in port_label_requests {
            let key = (sid.clone(), index, is_input, y.round() as i32);
            if !seen_port_labels.insert(key) { continue; }
            let Some(brect) = sid_screen_map.get(&sid).copied() else { continue; };
            let Some(block) = blocks.iter().find_map(|(b, _)| if b.sid.as_ref() == Some(&sid) { Some(*b) } else { None }) else { continue; };
            let cfg = get_block_type_cfg(&block.block_type);
            if (is_input && !cfg.show_input_port_labels) || (!is_input && !cfg.show_output_port_labels) { continue; }
            let pname = block.ports.iter().filter(|p| p.port_type == if is_input { "in" } else { "out" } && p.index.unwrap_or(0) == index).filter_map(|p| {
                p.properties.get("Name").cloned().or_else(|| p.properties.get("PropagatedSignals").cloned()).or_else(|| p.properties.get("name").cloned()).or_else(|| Some(format!("{}{}", if is_input { "In" } else { "Out" }, index)))
            }).next().unwrap_or_else(|| format!("{}{}", if is_input { "In" } else { "Out" }, index));
            let galley = ctx.fonts(|f| f.layout_no_wrap(pname.clone(), font_id.clone(), Color32::from_rgb(40,40,40)));
            let size = galley.size();
            let avail_w = brect.width() - 8.0;
            if size.x <= avail_w {
                let half_h = size.y * 0.5; let y_min = brect.top(); let y_max = (brect.bottom() - size.y).max(y_min);
                let y_top = (y - half_h).max(y_min).min(y_max);
                let pos = if is_input { Pos2::new(brect.left() + 4.0, y_top) } else { Pos2::new(brect.right() - 4.0 - size.x, y_top) };
                painter.galley(pos, galley, Color32::from_rgb(40,40,40));
            }
        }
    });

    if let Some(p) = navigate_to_from_block.or(navigate_to) { app.navigate_to_path(p); }
    app.zoom = staged_zoom; app.pan = staged_pan; app.reset_view = staged_reset;
    if let Some(cv) = open_chart { app.chart_view = Some(cv); }
    if let Some(sd) = open_signal { app.signal_view = Some(sd); }
    if let Some(bd) = open_block { app.block_view = Some(bd); }
    if clear_search { app.search_query.clear(); app.search_matches.clear(); }

    if let Some(cv) = &mut app.chart_view {
        let mut open_flag = cv.open;
        egui::Window::new(format!("Chart: {}", cv.title))
            .open(&mut open_flag)
            .resizable(true)
            .vscroll(true)
            .min_width(400.0)
            .min_height(200.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                    let job = matlab_syntax_job(&cv.script);
                    ui.add(egui::Label::new(job).wrap());
                });
            });
        cv.open = open_flag; if !cv.open { app.chart_view = None; }
    }

    if let Some(sd) = &app.signal_view { // signal dialog
        let mut open_flag = sd.open; let title = format!("Signal: {}", sd.title);
        let sys = app.current_system().map(|s| s.clone()); let line_idx = sd.line_idx;
        egui::Window::new(title)
            .open(&mut open_flag)
            .resizable(true)
            .vscroll(true)
            .min_width(360.0)
            .min_height(200.0)
            .show(ctx, |ui| {
                if let Some(sys) = &sys {
                    if let Some(line) = sys.lines.get(line_idx) {
                        ui.label(RichText::new("General").strong());
                        ui.horizontal_wrapped(|ui| { ui.label(format!("Name: {}", line.name.clone().unwrap_or("<unnamed>".into()))); if let Some(z) = &line.zorder { ui.label(format!("Z: {}", z)); } });
                        ui.separator();
                        let mut outputs: Vec<EndpointRef> = Vec::new();
                        fn collect_branch_dsts(br: &crate::model::Branch, out: &mut Vec<EndpointRef>) { if let Some(d) = &br.dst { out.push(d.clone()); } for s in &br.branches { collect_branch_dsts(s, out); } }
                        if let Some(d) = &line.dst { outputs.push(d.clone()); }
                        for b in &line.branches { collect_branch_dsts(b, &mut outputs); }
                        egui::CollapsingHeader::new("Inputs").default_open(true).show(ui, |ui| {
                            if let Some(src) = &line.src {
                                let bname = sys.blocks.iter().find(|b| b.sid.as_ref() == Some(&src.sid)).map(|b| b.name.clone()).unwrap_or_else(|| format!("SID{}", src.sid));
                                let pname = sys.blocks.iter().find(|b| b.sid.as_ref() == Some(&src.sid)).and_then(|b| b.ports.iter().find(|p| p.port_type == src.port_type && p.index.unwrap_or(0) == src.port_index)).and_then(|p| p.properties.get("Name").cloned().or_else(|| p.properties.get("name").cloned())).unwrap_or_else(|| format!("{}{}", if src.port_type=="in"{"In"}else{"Out"}, src.port_index));
                                ui.label(format!("{} • {}{} ({}): {}", bname, if src.port_type=="in"{"In"}else{"Out"}, src.port_index, src.port_type, pname));
                            } else { ui.label("<no source>"); }
                        });
                        egui::CollapsingHeader::new("Outputs").default_open(true).show(ui, |ui| {
                            if outputs.is_empty() { ui.label("<none>"); }
                            for d in outputs {
                                let bname = sys.blocks.iter().find(|b| b.sid.as_ref() == Some(&d.sid)).map(|b| b.name.clone()).unwrap_or_else(|| format!("SID{}", d.sid));
                                let pname = sys.blocks.iter().find(|b| b.sid.as_ref() == Some(&d.sid)).and_then(|b| b.ports.iter().find(|p| p.port_type == d.port_type && p.index.unwrap_or(0) == d.port_index)).and_then(|p| p.properties.get("Name").cloned().or_else(|| p.properties.get("name").cloned())).unwrap_or_else(|| format!("{}{}", if d.port_type=="in"{"In"}else{"Out"}, d.port_index));
                                ui.label(format!("{} • {}{} ({}): {}", bname, if d.port_type=="in"{"In"}else{"Out"}, d.port_index, d.port_type, pname));
                            }
                        });
                        if !app.signal_buttons.is_empty() {
                            ui.separator(); ui.label(RichText::new("Actions").strong());
                            ui.horizontal_wrapped(|ui| { for btn in &app.signal_buttons { if (btn.filter)(line) { if ui.button(&btn.label).clicked() { (btn.on_click)(line); } } } });
                        }
                    } else { ui.colored_label(Color32::RED, "Selected signal no longer exists in this view"); }
                }
            });
        if let Some(sd_mut) = &mut app.signal_view { sd_mut.open = open_flag; if !sd_mut.open { app.signal_view = None; } }
    }

    if let Some(bd) = &app.block_view {
        let mut open_flag = bd.open; let block = bd.block.clone();
        egui::Window::new(format!("Block: {}", bd.title))
            .open(&mut open_flag)
            .resizable(true)
            .vscroll(true)
            .min_width(360.0)
            .min_height(220.0)
            .show(ctx, |ui| {
                ui.label(RichText::new("General").strong());
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!("Name: {}", block.name));
                    ui.label(format!("Type: {}", block.block_type));
                    if let Some(sid) = block.sid.as_ref() { ui.label(format!("SID: {}", sid)); }
                    if let Some(z) = &block.zorder { ui.label(format!("Z: {}", z)); }
                    if block.commented { ui.label("commented"); }
                });
                ui.separator();
                egui::CollapsingHeader::new("Properties").default_open(true).show(ui, |ui| {
                    if block.properties.is_empty() { ui.label("<none>"); }
                    for (k, v) in &block.properties { ui.horizontal(|ui| { ui.label(RichText::new(k).strong()); ui.label(v); }); }
                });
                if block.block_type == "CFunction" { if let Some(cfg) = &block.c_function {
                    ui.separator();
                    egui::CollapsingHeader::new("C/C++ Code").default_open(true).show(ui, |ui| {
                        if let Some(s) = &cfg.start_code { ui.label(RichText::new("StartCode").strong()); ui.add(egui::TextEdit::multiline(&mut s.clone()).desired_width(f32::INFINITY)); }
                        if let Some(s) = &cfg.output_code { ui.label(RichText::new("OutputCode").strong()); ui.add(egui::TextEdit::multiline(&mut s.clone()).desired_width(f32::INFINITY)); }
                        if let Some(s) = &cfg.terminate_code { ui.label(RichText::new("TerminateCode").strong()); ui.add(egui::TextEdit::multiline(&mut s.clone()).desired_width(f32::INFINITY)); }
                        if let Some(s) = &cfg.codegen_start_code { ui.label(RichText::new("CodegenStartCode").strong()); ui.add(egui::TextEdit::multiline(&mut s.clone()).desired_width(f32::INFINITY)); }
                        if let Some(s) = &cfg.codegen_output_code { ui.label(RichText::new("CodegenOutputCode").strong()); ui.add(egui::TextEdit::multiline(&mut s.clone()).desired_width(f32::INFINITY)); }
                        if let Some(s) = &cfg.codegen_terminate_code { ui.label(RichText::new("CodegenTerminateCode").strong()); ui.add(egui::TextEdit::multiline(&mut s.clone()).desired_width(f32::INFINITY)); }
                    });
                }}
                egui::CollapsingHeader::new("Ports").default_open(true).show(ui, |ui| {
                    if block.ports.is_empty() { ui.label("<none>"); return; }
                    let mut ins: Vec<&crate::model::Port> = block.ports.iter().filter(|p| p.port_type == "in").collect();
                    let mut outs: Vec<&crate::model::Port> = block.ports.iter().filter(|p| p.port_type == "out").collect();
                    ins.sort_by_key(|p| p.index.unwrap_or(0)); outs.sort_by_key(|p| p.index.unwrap_or(0));
                    if !ins.is_empty() { ui.label(RichText::new("Inputs").strong()); }
                    for p in ins { let idx = p.index.unwrap_or(0); let name = p.properties.get("Name").or_else(|| p.properties.get("name")).cloned().unwrap_or_else(|| format!("In{}", idx)); ui.label(format!("{}{}: {}", "In", idx, name)); }
                    if !outs.is_empty() { ui.separator(); ui.label(RichText::new("Outputs").strong()); }
                    for p in outs { let idx = p.index.unwrap_or(0); let name = p.properties.get("Name").or_else(|| p.properties.get("name")).cloned().unwrap_or_else(|| format!("Out{}", idx)); ui.label(format!("{}{}: {}", "Out", idx, name)); }
                });
                if !app.block_buttons.is_empty() { ui.separator(); ui.label(RichText::new("Actions").strong()); ui.horizontal_wrapped(|ui| { for btn in &app.block_buttons { if (btn.filter)(&block) { if ui.button(&btn.label).clicked() { (btn.on_click)(&block); } } } }); }
            });
        if let Some(bd_mut) = &mut app.block_view { bd_mut.open = open_flag; if !bd_mut.open { app.block_view = None; } }
    }
}
