#![cfg(feature = "egui")]

use crate::block_types::{self, BlockTypeConfig};
use crate::model::Block;
use eframe::egui::{self, Align2, Color32, Pos2, Rect, Stroke, Vec2};

use super::icon_assets;
use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};

fn normalize_library_block_path(path: &str) -> Option<String> {
    let path = path.trim();
    if path.is_empty() {
        return None;
    }

    // SLX XML stores long block/path names split across multiple lines.  Replace
    // the embedded newlines (and carriage-returns) with a space before any further
    // processing so that e.g. "matrix_library/Compare\nTo Constant" becomes
    // "matrix_library/Compare To Constant", which then matches the registry key.
    // Newlines act as word-wrap separators in the XML, not as characters to delete.
    let no_newlines;
    let path = if path.contains(['\n', '\r']) {
        no_newlines = path.replace(['\n', '\r'], " ");
        no_newlines.as_str()
    } else {
        path
    };

    let path = path.replace('\\', "/");
    let Some((lib, rest)) = path.split_once('/') else {
        return Some(path);
    };
    // Some models use `Something.slx/BlockName` while our registry keys are
    // typically stored without the `.slx` suffix.
    let lib_norm = lib
        .strip_suffix(".slx")
        .or_else(|| lib.strip_suffix(".SLX"))
        .unwrap_or(lib);
    Some(format!("{lib_norm}/{rest}"))
}

/// Fill a block body according to its [`BlockShape`], returning the background
/// color actually painted (commented blocks use a fixed grey).  Shared by the
/// editor and the viewer so both render identical block bodies — there is no
/// per-block-type body-drawing code in either UI.
pub fn fill_block_body(
    painter: &egui::Painter,
    rect: Rect,
    shape: block_types::BlockShape,
    bg: Color32,
    commented: bool,
) -> Color32 {
    use block_types::BlockShape;
    if commented {
        let commented_bg = Color32::from_rgb(230, 230, 230);
        painter.rect_filled(rect, 0.0, commented_bg);
        return commented_bg;
    }
    match shape {
        BlockShape::Triangle => {
            // Gain-style: right-pointing triangle (left-top, right-center, left-bottom).
            let pts = vec![
                egui::pos2(rect.left(), rect.top()),
                egui::pos2(rect.right(), rect.center().y),
                egui::pos2(rect.left(), rect.bottom()),
            ];
            let mut tri = egui::epaint::PathShape::closed_line(pts, Stroke::NONE);
            tri.fill = bg;
            painter.add(egui::Shape::Path(tri));
        }
        BlockShape::Circle => {
            let radius = rect.size().min_elem() / 2.0;
            painter.circle_filled(rect.center(), radius, bg);
        }
        BlockShape::FilledBlack => {
            painter.rect_filled(rect, 0.0, Color32::BLACK);
        }
        BlockShape::Goto => {
            let tab = rect.height() * 0.25;
            let pts = vec![
                egui::pos2(rect.left(), rect.top()),
                egui::pos2(rect.right(), rect.top()),
                egui::pos2(rect.right(), rect.bottom()),
                egui::pos2(rect.left(), rect.bottom()),
                egui::pos2(rect.left() - tab, rect.center().y),
            ];
            let mut path = egui::epaint::PathShape::closed_line(pts, Stroke::NONE);
            path.fill = bg;
            painter.add(egui::Shape::Path(path));
        }
        BlockShape::From => {
            let tab = rect.height() * 0.25;
            let pts = vec![
                egui::pos2(rect.left(), rect.top()),
                egui::pos2(rect.right(), rect.top()),
                egui::pos2(rect.right() + tab, rect.center().y),
                egui::pos2(rect.right(), rect.bottom()),
                egui::pos2(rect.left(), rect.bottom()),
            ];
            let mut path = egui::epaint::PathShape::closed_line(pts, Stroke::NONE);
            path.fill = bg;
            painter.add(egui::Shape::Path(path));
        }
        BlockShape::Rectangle => {
            painter.rect_filled(rect, 6.0, bg);
        }
        BlockShape::Obround => {
            // Fully rounded short ends (egui clamps the corner radius to half the
            // shortest side, giving a stadium/obround).
            painter.rect_filled(rect, rect.height() * 0.5, bg);
        }
        BlockShape::None => {
            // The block's static renderer paints its own body; nothing here.
        }
    }
    bg
}

/// Stroke a block body's outline according to its [`BlockShape`].  Shared by the
/// editor and the viewer.
pub fn stroke_block_body(
    painter: &egui::Painter,
    rect: Rect,
    shape: block_types::BlockShape,
    stroke: Stroke,
) {
    use block_types::BlockShape;
    match shape {
        BlockShape::Triangle => {
            let pts = vec![
                egui::pos2(rect.left(), rect.top()),
                egui::pos2(rect.right(), rect.center().y),
                egui::pos2(rect.left(), rect.bottom()),
            ];
            painter.add(egui::Shape::Path(egui::epaint::PathShape::closed_line(
                pts, stroke,
            )));
        }
        BlockShape::Circle => {
            let radius = rect.size().min_elem() / 2.0;
            painter.circle_stroke(rect.center(), radius, stroke);
        }
        BlockShape::FilledBlack => {}
        BlockShape::Goto => {
            let tab = rect.height() * 0.25;
            let pts = vec![
                egui::pos2(rect.left(), rect.top()),
                egui::pos2(rect.right(), rect.top()),
                egui::pos2(rect.right(), rect.bottom()),
                egui::pos2(rect.left(), rect.bottom()),
                egui::pos2(rect.left() - tab, rect.center().y),
            ];
            painter.add(egui::Shape::Path(egui::epaint::PathShape::closed_line(
                pts, stroke,
            )));
        }
        BlockShape::From => {
            let tab = rect.height() * 0.25;
            let pts = vec![
                egui::pos2(rect.left(), rect.top()),
                egui::pos2(rect.right(), rect.top()),
                egui::pos2(rect.right() + tab, rect.center().y),
                egui::pos2(rect.right(), rect.bottom()),
                egui::pos2(rect.left(), rect.bottom()),
            ];
            painter.add(egui::Shape::Path(egui::epaint::PathShape::closed_line(
                pts, stroke,
            )));
        }
        BlockShape::Rectangle => {
            painter.rect_stroke(rect, 4.0, stroke, egui::StrokeKind::Inside);
        }
        BlockShape::Obround => {
            painter.rect_stroke(rect, rect.height() * 0.5, stroke, egui::StrokeKind::Inside);
        }
        BlockShape::None => {
            // The block's static renderer paints its own outline; nothing here.
        }
    }
}

pub fn get_block_type_cfg(block: &Block) -> BlockTypeConfig {
    let mut cfg = lookup_block_type_cfg(block);
    // Port placement can depend on the block's own properties (a round Sum
    // wraps its last input onto the bottom edge, the rectangular one does not).
    let def = crate::simulink_libraries::resolve_definition(block);
    if let Some(f) = def.port_overrides_fn {
        let metadata = crate::simulink_libraries::metadata::extract_metadata(block, def);
        cfg.port_position_overrides = f(block, &metadata).to_vec();
    }
    cfg
}

fn lookup_block_type_cfg(block: &Block) -> BlockTypeConfig {
    let map = block_types::get_block_type_config_map();
    let Ok(g) = map.read() else {
        return BlockTypeConfig::default();
    };

    if block.is_matlab_function {
        return g.get("MATLAB Function").cloned().unwrap_or_default();
    }

    // Build library-specific candidates (library path / SourceBlock).  These are
    // kept separate from `block_type` so that virtual-library icons always take
    // priority over the generic block-kind icon (e.g. a "Product"-typed cross-
    // product block should show the cross-product SVG, not the generic "×").
    let mut lib_candidates: Vec<String> = Vec::new();

    if let Some(ref lib_path) = block.library_block_path {
        lib_candidates.push(lib_path.clone());
        if let Some(n) = normalize_library_block_path(lib_path)
            && n != *lib_path
        {
            lib_candidates.push(n);
        }
    }
    // Always check SourceBlock as well (not only when library_block_path is absent),
    // since library_block_path is derived from it and may carry the same casing issues.
    if let Some(source_block) = block.properties.get("SourceBlock") {
        if block.library_block_path.as_deref() != Some(source_block.as_str()) {
            lib_candidates.push(source_block.clone());
        }
        if let Some(n) = normalize_library_block_path(source_block)
            && !lib_candidates.contains(&n)
        {
            lib_candidates.push(n);
        }
    }

    // Collect all unique last-path-segments from the library candidates.
    let mut last_segments: Vec<String> = Vec::new();
    for c in &lib_candidates {
        if let Some((_, name)) = c.rsplit_once('/') {
            let s = name.to_string();
            if !last_segments.contains(&s) {
                last_segments.push(s);
            }
        }
    }

    // Phase 1 – exact match against full library paths and their last segments.
    // This intentionally runs BEFORE the block_type fallback so that virtual-
    // library icons win over the generic kind icon.
    for key in lib_candidates.iter().chain(last_segments.iter()) {
        if let Some(cfg) = g.get(key.as_str()) {
            return cfg.clone();
        }
    }

    // Phase 2 – whitespace-normalized and CamelCase-humanized fallback on last
    // segments.  Handles blocks where the SLX name differs from our registry
    // key by whitespace collapsing or CamelCase spacing (e.g.
    // "Create Diagonal\nMatrix" after newline removal → "CreateDiagonalMatrix"
    // → normalize(humanize(…)) → "create diagonal matrix" → match).
    //
    // Because `register_virtual_keys` pre-registers all normalized forms, these
    // are plain O(1) hash lookups – no linear scan needed.
    for seg in &last_segments {
        use crate::simulink_libraries::stubs::{humanize_camel_case, normalize_block_name};
        let seg_norm = normalize_block_name(seg);
        if let Some(cfg) = g.get(seg_norm.as_str()) {
            return cfg.clone();
        }
        let seg_human_norm = normalize_block_name(&humanize_camel_case(seg));
        if seg_human_norm != seg_norm
            && let Some(cfg) = g.get(seg_human_norm.as_str())
        {
            return cfg.clone();
        }
    }

    // Phase 3 – Simulink-semantic overrides that are expressed through block
    // properties rather than via a SourceBlock/library path.
    // A plain Product block with Multiplication="Matrix(*)" is the standard way
    // Simulink encodes a matrix-multiply.  Show the dedicated SVG for it.
    if block.block_type == "Product"
        && block.properties.get("Multiplication").map(|v| v.trim()) == Some("Matrix(*)")
        && let Some(cfg) = g.get("matrix multiply")
    {
        return cfg.clone();
    }

    // Phase 4 – generic block-type fallback (lowest priority).
    if let Some(cfg) = g.get(block.block_type.as_str()) {
        return cfg.clone();
    }

    BlockTypeConfig::default()
}

/// Max measured width of port labels drawn *inside* the block on the left/right side.
///
/// This is used to keep the center icon from overlapping those labels.
#[derive(Clone, Copy, Debug, Default)]
pub struct PortLabelMaxWidths {
    pub left: f32,
    pub right: f32,
}

#[allow(dead_code)]
pub(crate) fn port_label_display_name(
    block: &Block,
    index: u32,
    is_input: bool,
    cfg: &BlockTypeConfig,
) -> String {
    // Note: The port-label drawing code treats mirroring as swapping the logical direction
    // when looking up Port properties. Keep this logic in one place so icon sizing and
    // label rendering stay consistent.
    let mirrored = block.block_mirror.unwrap_or(false);
    let logical_is_input = if mirrored { !is_input } else { is_input };

    let fallback_name = || {
        let names = if logical_is_input {
            &cfg.input_port_names
        } else {
            &cfg.output_port_names
        };
        if index > 0 && (index as usize) <= names.len() {
            names[(index - 1) as usize].clone()
        } else {
            format!("{}{}", if is_input { "In" } else { "Out" }, index)
        }
    };

    let explicit_port_name = || {
        block
            .ports
            .iter()
            .filter(|p| {
                p.port_type == if logical_is_input { "in" } else { "out" }
                    && p.index.unwrap_or(0) == index
            })
            .find_map(|p| {
                p.properties
                    .get("Name")
                    .cloned()
                    .or_else(|| p.properties.get("name").cloned())
                    .map(|name| name.trim().to_string())
                    .filter(|name| !name.is_empty())
            })
    };

    subsystem_boundary_port_name(block, index, logical_is_input)
        .or_else(|| crate::simulink_libraries::render::port_label(block, index, logical_is_input))
        .or_else(explicit_port_name)
        .unwrap_or_else(fallback_name)
}

pub(crate) fn subsystem_boundary_port_name(
    block: &Block,
    index: u32,
    logical_is_input: bool,
) -> Option<String> {
    let boundary_type = match block.block_type.as_str() {
        "SubSystem" | "Reference" => {
            if logical_is_input {
                "Inport"
            } else {
                "Outport"
            }
        }
        _ => return None,
    };

    block
        .subsystem
        .as_ref()?
        .blocks
        .iter()
        .filter(|child| child.block_type == boundary_type)
        .find(|child| subsystem_boundary_port_index(child) == index)
        .and_then(|child| boundary_block_display_name(child, index))
}

fn subsystem_boundary_port_index(block: &Block) -> u32 {
    block
        .properties
        .get("Port")
        .or_else(|| block.properties.get("PortNumber"))
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(1)
}

/// Replace Simulink's default `In<N>` / `Out<N>` boundary-block naming with the
/// port *number* (what the Inport block's own icon draws), while user-chosen
/// names such as `u` or `theta` are kept verbatim.  The number is the port's,
/// not the one in the block name: reordering the ports of a subsystem renumbers
/// them while the boundary blocks keep the names they were created with.
fn simplify_boundary_name(name: &str, port_index: u32) -> String {
    for prefix in ["In", "Out"] {
        if let Some(rest) = name.strip_prefix(prefix)
            && !rest.is_empty()
            && rest.chars().all(|c| c.is_ascii_digit())
        {
            return port_index.to_string();
        }
    }
    name.to_string()
}

fn boundary_block_display_name(block: &Block, port_index: u32) -> Option<String> {
    let name = block.name.trim();
    if !name.is_empty() {
        return Some(simplify_boundary_name(name, port_index));
    }

    block
        .properties
        .get("Name")
        .or_else(|| block.properties.get("name"))
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
}

pub fn wrap_text_to_max_width(
    painter: &egui::Painter,
    text: &str,
    font_id: egui::FontId,
    max_width: f32,
) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    if !max_width.is_finite() || max_width <= 1.0 {
        return text.split('\n').map(|s| s.to_string()).collect();
    }

    fn measure_width(painter: &egui::Painter, s: &str, font_id: &egui::FontId) -> f32 {
        painter
            .layout_no_wrap(s.to_string(), font_id.clone(), Color32::TRANSPARENT)
            .size()
            .x
    }

    fn split_prefix_that_fits<'a>(
        painter: &egui::Painter,
        word: &'a str,
        font_id: &egui::FontId,
        max_width: f32,
    ) -> (&'a str, &'a str) {
        if word.is_empty() {
            return ("", "");
        }

        let mut boundaries: Vec<usize> = word.char_indices().map(|(i, _)| i).collect();
        boundaries.push(word.len());
        if boundaries.len() <= 2 {
            // One character (or empty) — must make progress.
            return (word, "");
        }

        let mut best = 1usize; // at least one char
        let mut lo = 1usize;
        let mut hi = boundaries.len();
        while lo < hi {
            let mid = (lo + hi) / 2;
            let idx = boundaries[mid];
            let prefix = &word[..idx];
            if measure_width(painter, prefix, font_id) <= max_width {
                best = mid;
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }

        let split_idx = boundaries[best];
        (&word[..split_idx], &word[split_idx..])
    }

    let mut out: Vec<String> = Vec::new();
    for para in text.split('\n') {
        // Preserve explicit newlines.
        if para.trim().is_empty() {
            out.push(String::new());
            continue;
        }

        let mut current = String::new();
        for word in para.split_whitespace() {
            if current.is_empty() {
                if measure_width(painter, word, &font_id) <= max_width {
                    current.push_str(word);
                } else {
                    // Extremely long word: split by character to guarantee progress.
                    let mut rest = word;
                    while !rest.is_empty() {
                        let (prefix, new_rest) =
                            split_prefix_that_fits(painter, rest, &font_id, max_width);
                        out.push(prefix.to_string());
                        rest = new_rest;
                    }
                }
                continue;
            }

            let candidate = format!("{} {}", current, word);
            if measure_width(painter, &candidate, &font_id) <= max_width {
                current = candidate;
            } else {
                out.push(current);
                current = String::new();

                if measure_width(painter, word, &font_id) <= max_width {
                    current.push_str(word);
                } else {
                    let mut rest = word;
                    while !rest.is_empty() {
                        let (prefix, new_rest) =
                            split_prefix_that_fits(painter, rest, &font_id, max_width);
                        out.push(prefix.to_string());
                        rest = new_rest;
                    }
                }
            }
        }

        if !current.is_empty() {
            out.push(current);
        }
    }

    out
}

pub fn compute_icon_available_rect(
    rect: &Rect,
    font_scale: f32,
    port_label_widths: Option<PortLabelMaxWidths>,
) -> Rect {
    let margin_x = rect.width() * 0.10;
    let margin_y = rect.height() * 0.10;

    let mut left_inset = margin_x;
    let mut right_inset = margin_x;

    if let Some(w) = port_label_widths {
        let label_pad = 4.0 * font_scale;
        let label_gap = 2.0 * font_scale;
        if w.left > 0.0 {
            left_inset = left_inset.max(label_pad + w.left + label_gap);
        }
        if w.right > 0.0 {
            right_inset = right_inset.max(label_pad + w.right + label_gap);
        }
    }

    let mut min = Pos2::new(rect.left() + left_inset, rect.top() + margin_y);
    let mut max = Pos2::new(rect.right() - right_inset, rect.bottom() - margin_y);
    if min.x >= max.x {
        let cx = rect.center().x;
        min.x = cx;
        max.x = cx;
    }
    if min.y >= max.y {
        let cy = rect.center().y;
        min.y = cy;
        max.y = cy;
    }
    Rect::from_min_max(min, max)
}

fn maximize_glyph_font_px(painter: &egui::Painter, glyph: &str, avail: Vec2) -> f32 {
    if avail.x <= 1.0 || avail.y <= 1.0 {
        return 1.0;
    }

    // Measure once at a reference size and scale. This avoids per-block binary searches.
    let ref_px = 100.0_f32;
    let ref_galley = painter.layout_no_wrap(
        glyph.to_string(),
        egui::FontId::proportional(ref_px),
        Color32::TRANSPARENT,
    );
    let ref_size = ref_galley.size();
    if ref_size.x <= 1e-3 || ref_size.y <= 1e-3 {
        return 1.0;
    }

    let mut font_px = (ref_px * (avail.x / ref_size.x).min(avail.y / ref_size.y)).max(1.0);

    // Nudge up a tiny bit while still fitting, then nudge down if needed.
    for _ in 0..6 {
        let try_px = font_px * 1.02;
        let g = painter.layout_no_wrap(
            glyph.to_string(),
            egui::FontId::proportional(try_px),
            Color32::TRANSPARENT,
        );
        let s = g.size();
        if s.x <= avail.x && s.y <= avail.y {
            font_px = try_px;
        } else {
            break;
        }
    }
    for _ in 0..8 {
        let g = painter.layout_no_wrap(
            glyph.to_string(),
            egui::FontId::proportional(font_px),
            Color32::TRANSPARENT,
        );
        let s = g.size();
        if s.x <= avail.x && s.y <= avail.y {
            break;
        }
        font_px *= 0.98;
        if font_px <= 1.0 {
            font_px = 1.0;
            break;
        }
    }
    font_px
}

pub fn render_center_glyph_maximized(
    painter: &egui::Painter,
    rect: &Rect,
    font_scale: f32,
    glyph: &str,
    color: Color32,
    port_label_widths: Option<PortLabelMaxWidths>,
) {
    let avail_rect = compute_icon_available_rect(rect, font_scale, port_label_widths);
    let avail = avail_rect.size();
    let font_px = maximize_glyph_font_px(painter, glyph, avail);
    let font_id = egui::FontId::proportional(font_px);
    painter.text(
        avail_rect.center(),
        Align2::CENTER_CENTER,
        glyph,
        font_id,
        color,
    );
}

/// Draw a small piece of typeset math (a horizontal fraction bar, a raised
/// superscript, or an overbar) centred in `rect`.  Simulink draws these block
/// icons as 2-D math that a single one-line glyph string can't reproduce, so we
/// paint them here.  `spec` is a compact notation understood by this painter:
///
/// * `frac:NUM/DEN` – numerator over denominator with a horizontal bar
///   (`frac:1/s`, `frac:(z-1)/z`, `frac:K(z-1)/Ts z`).
/// * `sup:BASE^SUP` – `BASE` with a raised, smaller superscript
///   (`sup:z^-2`, `sup:e^u`, `sup:u^2`).  Text after a space in `SUP` returns
///   to the baseline, so `sup:A^H A` typesets `AᴴA`.
/// * `over:BASE` – `BASE` with an overbar (conjugate, e.g. `over:u` → `ū`).
/// * `lines:A|B` – stacked, centred lines at a common font size (e.g. the
///   Descriptor State-Space icon `lines:Eẋ = Ax + Bu|y = Cx + Du`).
///
/// Anything else is drawn as a plain maximised glyph (same as a `Utf8` icon).
pub fn draw_math_icon(
    painter: &egui::Painter,
    rect: &Rect,
    font_scale: f32,
    spec: &str,
    color: Color32,
    port_label_widths: Option<PortLabelMaxWidths>,
) {
    let avail = compute_icon_available_rect(rect, font_scale, port_label_widths);
    if avail.width() <= 1.0 || avail.height() <= 1.0 {
        return;
    }
    if let Some(rest) = spec.strip_prefix("frac:") {
        let (num, den) = rest.split_once('/').unwrap_or((rest, ""));
        draw_fraction(painter, &avail, num.trim(), den.trim(), color);
    } else if let Some(rest) = spec.strip_prefix("sup:") {
        let (base, sup) = rest.split_once('^').unwrap_or((rest, ""));
        let (sup, tail) = sup.split_once(' ').unwrap_or((sup, ""));
        draw_superscript(painter, &avail, base, sup, tail, color);
    } else if let Some(base) = spec.strip_prefix("over:") {
        draw_overbar(painter, &avail, base, color);
    } else if let Some(rest) = spec.strip_prefix("lines:") {
        draw_stacked_lines(painter, &avail, rest, color);
    } else {
        render_center_glyph_maximized(painter, rect, font_scale, spec, color, port_label_widths);
    }
}

/// `|`-separated lines stacked vertically and centred, all at one font size.
fn draw_stacked_lines(painter: &egui::Painter, avail: &Rect, spec: &str, color: Color32) {
    let lines: Vec<&str> = spec.split('|').map(str::trim).collect();
    if lines.is_empty() {
        return;
    }
    let n = lines.len() as f32;
    let row = Vec2::new(avail.width() * 0.96, (avail.height() * 0.94) / n);
    let font_px = lines
        .iter()
        .map(|l| fit_font_px(painter, l, row))
        .fold(f32::INFINITY, f32::min)
        .clamp(5.0, 40.0);
    let font = egui::FontId::proportional(font_px);
    let step = font_px * 1.16;
    let top = avail.center().y - step * (n - 1.0) * 0.5;
    for (i, line) in lines.iter().enumerate() {
        painter.text(
            Pos2::new(avail.center().x, top + step * i as f32),
            Align2::CENTER_CENTER,
            *line,
            font.clone(),
            color,
        );
    }
}

/// Draw a line-art block icon from a compact notation.
///
/// Simulink draws many icons (source waveforms, saturation/backlash curves,
/// scope screens, verification plots) as vector line art rather than as a
/// glyph.  `spec` is a `;`-separated list of drawing commands whose coordinates
/// are normalised to `0.0..=1.0` inside the icon area, with `y` pointing
/// **down** so the notation reads like screen space:
///
/// * `p X,Y X,Y …` – polyline through the listed points.
/// * `a X,Y X,Y …` – same, but faint: Simulink's thin grey axis cross.
/// * `b X0,Y0,X1,Y1` – translucent filled band (Simulink's grey limit bands).
/// * `r X0,Y0,X1,Y1` – stroked rectangle.
/// * `f X0,Y0,X1,Y1` – solid rectangle (Simulink's black bus bars).
/// * `c CX,CY,R` – stroked circle (`R` is a fraction of the icon width).
/// * `d CX,CY,R` – filled dot.
/// * `o CX,CY,W,H` – stroked obround (the In/Out ports of a subsystem preview).
/// * `t X,Y,H TEXT` – `TEXT` centred at `X,Y` with cap height `H` (a fraction
///   of the icon height), for the letters Simulink sets inside its pictograms
///   (`A ⇒ D`, the `U` of Is Triangular).
///
/// Unknown commands are skipped, so a malformed spec degrades to blank rather
/// than panicking.
pub fn draw_plot_icon(
    painter: &egui::Painter,
    rect: &Rect,
    font_scale: f32,
    spec: &str,
    color: Color32,
    port_label_widths: Option<PortLabelMaxWidths>,
) {
    let avail = compute_icon_available_rect(rect, font_scale, port_label_widths);
    if avail.width() <= 1.0 || avail.height() <= 1.0 {
        return;
    }
    let at = |x: f32, y: f32| {
        Pos2::new(
            avail.left() + x * avail.width(),
            avail.top() + y * avail.height(),
        )
    };
    let width = (avail.width().min(avail.height()) * 0.055).clamp(0.8, 2.2);
    let stroke = Stroke::new(width, color);
    let faint = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 90);
    let axis_stroke = Stroke::new((width * 0.7).max(0.7), faint);
    let band = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 46);

    for cmd in spec.split(';') {
        let cmd = cmd.trim();
        let Some((kind, args)) = cmd.split_once(char::is_whitespace) else {
            continue;
        };
        let nums: Vec<f32> = args
            .split([' ', ','])
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse::<f32>().ok())
            .collect();
        match kind {
            "p" | "a" => {
                let pts: Vec<Pos2> = nums.chunks_exact(2).map(|c| at(c[0], c[1])).collect();
                if pts.len() >= 2 {
                    let s = if kind == "a" { axis_stroke } else { stroke };
                    painter.add(egui::Shape::line(pts, s));
                }
            }
            "b" | "r" | "f" if nums.len() >= 4 => {
                let r = Rect::from_two_pos(at(nums[0], nums[1]), at(nums[2], nums[3]));
                match kind {
                    "b" => {
                        painter.rect_filled(r, 0.0, band);
                    }
                    "f" => {
                        painter.rect_filled(r, 0.0, color);
                    }
                    _ => {
                        painter.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Inside);
                    }
                }
            }
            "o" if nums.len() >= 4 => {
                let c = at(nums[0], nums[1]);
                let half = Vec2::new(nums[2] * avail.width(), nums[3] * avail.height()) * 0.5;
                let r = Rect::from_center_size(c, half * 2.0);
                painter.rect_stroke(r, r.height() * 0.5, stroke, egui::StrokeKind::Inside);
            }
            "t" if nums.len() >= 3 => {
                let Some((_, text)) = args.split_once(char::is_whitespace) else {
                    continue;
                };
                painter.text(
                    at(nums[0], nums[1]),
                    Align2::CENTER_CENTER,
                    text.trim(),
                    egui::FontId::proportional((nums[2] * avail.height()).clamp(4.0, 40.0)),
                    color,
                );
            }
            "c" | "d" if nums.len() >= 3 => {
                let c = at(nums[0], nums[1]);
                let r = nums[2] * avail.width();
                if kind == "c" {
                    painter.circle_stroke(c, r, stroke);
                } else {
                    painter.circle_filled(c, r, color);
                }
            }
            // Circular arc `cx,cy,r,from,to` (angles in turns, clockwise on
            // screen); `sa` puts an arrow head on the end, `sb` on both ends.
            // The radius is a fraction of the smaller side so the arc stays a
            // circle in a non-square icon area.
            "s" | "sa" | "sb" if nums.len() >= 5 => {
                let c = at(nums[0], nums[1]);
                let r = nums[2] * avail.width().min(avail.height());
                let (a0, a1) = (
                    nums[3] * std::f32::consts::TAU,
                    nums[4] * std::f32::consts::TAU,
                );
                let steps = 48;
                let point = |a: f32| Pos2::new(c.x + r * a.cos(), c.y + r * a.sin());
                let pts: Vec<Pos2> = (0..=steps)
                    .map(|i| point(a0 + (a1 - a0) * i as f32 / steps as f32))
                    .collect();
                painter.add(egui::Shape::line(pts, stroke));
                let head = |a: f32, backwards: bool| {
                    let tip = point(a);
                    // Tangent of the arc at `a`, pointing along the travel
                    // direction, and the inward normal.
                    let dir = if (a1 > a0) != backwards { 1.0 } else { -1.0 };
                    let t = Vec2::new(-a.sin(), a.cos()) * dir;
                    let n = Vec2::new(a.cos(), a.sin());
                    let len = (r * 0.28).clamp(2.0, 10.0);
                    painter.add(egui::Shape::line(
                        vec![
                            tip - t * len + n * len * 0.5,
                            tip,
                            tip - t * len - n * len * 0.5,
                        ],
                        stroke,
                    ));
                };
                if kind == "sa" || kind == "sb" {
                    head(a1, false);
                }
                if kind == "sb" {
                    head(a0, true);
                }
            }
            _ => {}
        }
    }
}

/// Font size (points) at which `text` fits within `max` bounds.
fn fit_font_px(painter: &egui::Painter, text: &str, max: Vec2) -> f32 {
    if text.is_empty() {
        return 100.0;
    }
    let ref_px = 100.0_f32;
    let g = painter.layout_no_wrap(
        text.to_string(),
        egui::FontId::proportional(ref_px),
        Color32::TRANSPARENT,
    );
    let s = g.size();
    if s.x <= 1e-3 || s.y <= 1e-3 {
        return ref_px;
    }
    ref_px * (max.x / s.x).min(max.y / s.y)
}

fn text_width(painter: &egui::Painter, text: &str, font_px: f32) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    painter
        .layout_no_wrap(
            text.to_string(),
            egui::FontId::proportional(font_px),
            Color32::TRANSPARENT,
        )
        .size()
        .x
}

/// Numerator over a horizontal bar over denominator.
fn draw_fraction(painter: &egui::Painter, avail: &Rect, num: &str, den: &str, color: Color32) {
    let row_max = Vec2::new(avail.width() * 0.94, avail.height() * 0.44);
    let font_px = fit_font_px(painter, num, row_max)
        .min(fit_font_px(painter, den, row_max))
        .clamp(6.0, 40.0);
    let font = egui::FontId::proportional(font_px);
    let cx = avail.center().x;
    let cy = avail.center().y;
    let gap = font_px * 0.14;
    painter.text(
        Pos2::new(cx, cy - gap),
        Align2::CENTER_BOTTOM,
        num,
        font.clone(),
        color,
    );
    painter.text(
        Pos2::new(cx, cy + gap),
        Align2::CENTER_TOP,
        den,
        font,
        color,
    );
    let bar_w = text_width(painter, num, font_px).max(text_width(painter, den, font_px)) * 1.08;
    let stroke = Stroke::new((font_px * 0.07).clamp(1.0, 3.0), color);
    painter.line_segment(
        [
            Pos2::new(cx - bar_w * 0.5, cy),
            Pos2::new(cx + bar_w * 0.5, cy),
        ],
        stroke,
    );
}

/// `base` with a smaller superscript raised above the baseline.
fn draw_superscript(
    painter: &egui::Painter,
    avail: &Rect,
    base: &str,
    sup: &str,
    tail: &str,
    color: Color32,
) {
    if sup.is_empty() {
        let px = fit_font_px(painter, base, avail.size() * 0.9).clamp(6.0, 40.0);
        painter.text(
            avail.center(),
            Align2::CENTER_CENTER,
            base,
            egui::FontId::proportional(px),
            color,
        );
        return;
    }
    // Size so base (full) + superscript (0.62×, raised) fit the available box.
    let sup_ratio = 0.62_f32;
    let rise = 0.42_f32; // fraction of base height the superscript rises
    let ref_px = 100.0_f32;
    let bw = text_width(painter, base, ref_px);
    let sw = text_width(painter, sup, ref_px * sup_ratio);
    let tw = text_width(painter, tail, ref_px);
    let total_w = bw + sw + tw;
    let total_h = ref_px * (1.0 + rise);
    let font_px = if total_w <= 1e-3 {
        ref_px
    } else {
        (ref_px * ((avail.width() * 0.94) / total_w).min((avail.height() * 0.94) / total_h))
            .clamp(6.0, 40.0)
    };
    let base_font = egui::FontId::proportional(font_px);
    let sup_font = egui::FontId::proportional(font_px * sup_ratio);
    let base_w = text_width(painter, base, font_px);
    let sup_w = text_width(painter, sup, font_px * sup_ratio);
    let run_w = base_w + sup_w + text_width(painter, tail, font_px);
    let left = avail.center().x - run_w * 0.5;
    let cy = avail.center().y;
    painter.text(
        Pos2::new(left, cy),
        Align2::LEFT_CENTER,
        base,
        base_font.clone(),
        color,
    );
    painter.text(
        Pos2::new(left + base_w, cy - font_px * rise * 0.5),
        Align2::LEFT_CENTER,
        sup,
        sup_font,
        color,
    );
    if !tail.is_empty() {
        painter.text(
            Pos2::new(left + base_w + sup_w, cy),
            Align2::LEFT_CENTER,
            tail,
            base_font,
            color,
        );
    }
}

/// `base` with a horizontal overbar (Simulink's conjugate icon `ū`).
fn draw_overbar(painter: &egui::Painter, avail: &Rect, base: &str, color: Color32) {
    let font_px = fit_font_px(painter, base, avail.size() * Vec2::new(0.8, 0.78)).clamp(6.0, 40.0);
    let font = egui::FontId::proportional(font_px);
    let cx = avail.center().x;
    let cy = avail.center().y + font_px * 0.08;
    painter.text(Pos2::new(cx, cy), Align2::CENTER_CENTER, base, font, color);
    let w = text_width(painter, base, font_px) * 1.05;
    let bar_y = cy - font_px * 0.52;
    let stroke = Stroke::new((font_px * 0.07).clamp(1.0, 3.0), color);
    painter.line_segment(
        [
            Pos2::new(cx - w * 0.5, bar_y),
            Pos2::new(cx + w * 0.5, bar_y),
        ],
        stroke,
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct SvgCacheKey {
    path: &'static str,
    request_w: usize,
    request_h: usize,
}

#[derive(Clone)]
struct SvgCachedTexture {
    texture: egui::TextureHandle,
    px_size: [usize; 2],
}

pub fn embedded_egui_sans_fontdb() -> Option<Arc<usvg::fontdb::Database>> {
    static FONTDB: OnceLock<Option<Arc<usvg::fontdb::Database>>> = OnceLock::new();
    FONTDB
        .get_or_init(|| {
            let font_defs = egui::FontDefinitions::default();
            let ubuntu = font_defs.font_data.get("Ubuntu-Light")?;

            let mut db = usvg::fontdb::Database::new();
            db.load_font_data(ubuntu.as_ref().font.as_ref().to_vec());

            // Ensure CSS generic `sans-serif` resolves to the embedded font.
            // Use the actual family name declared in the font (typically "Ubuntu").
            let family_name = db
                .faces()
                .next()
                .and_then(|face| face.families.first().map(|(family, _lang)| family.clone()));
            if let Some(family_name) = family_name {
                db.set_sans_serif_family(family_name.clone());
                // reasonable fallback for `serif` too, in case SVG uses it
                db.set_serif_family(family_name);
            }

            Some(Arc::new(db))
        })
        .clone()
}

fn svg_dest_size_points(avail_points: Vec2, px_size: [usize; 2], pixels_per_point: f32) -> Vec2 {
    if pixels_per_point <= 0.0 {
        return Vec2::ZERO;
    }

    let w_points = px_size[0] as f32 / pixels_per_point;
    let h_points = px_size[1] as f32 / pixels_per_point;
    if w_points <= 0.0 || h_points <= 0.0 {
        return Vec2::ZERO;
    }

    let scale = (avail_points.x / w_points)
        .min(avail_points.y / h_points)
        .clamp(0.0, 1.0);
    Vec2::new(w_points * scale, h_points * scale)
}

fn get_or_create_svg_texture(
    ctx: &egui::Context,
    path: &'static str,
    request_px: [usize; 2],
) -> Option<SvgCachedTexture> {
    let cache_id = egui::Id::new("rustylink_svg_icon_cache");
    let key = SvgCacheKey {
        path,
        request_w: request_px[0],
        request_h: request_px[1],
    };

    // IMPORTANT: never call `ctx.load_texture` inside `ctx.data_mut`, since both
    // take a write lock on the same internal context lock, which will deadlock.
    if let Some(hit) = ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<std::collections::HashMap<SvgCacheKey, SvgCachedTexture>>(
            cache_id,
        )
        .get(&key)
        .cloned()
    }) {
        return Some(hit);
    }

    let bytes = icon_assets::get(path)?;
    let mut options = usvg::Options::default();
    // usvg's font database is empty by default; populate it from egui's embedded fonts.
    // This avoids relying on system-installed fonts.
    if let Some(db) = embedded_egui_sans_fontdb() {
        options.fontdb = db;
        options.font_family = "sans-serif".to_owned();
    }

    let image = egui_extras::image::load_svg_bytes_with_size(
        &bytes,
        egui::SizeHint::Size {
            width: request_px[0].min(u32::MAX as usize) as u32,
            height: request_px[1].min(u32::MAX as usize) as u32,
            maintain_aspect_ratio: true,
        },
        &options,
    )
    .ok()?;
    let px_size = image.size;

    let texture = ctx.load_texture(
        format!("rustylink_svg:{path}:{}x{}", request_px[0], request_px[1]),
        image,
        egui::TextureOptions::LINEAR,
    );
    let value = SvgCachedTexture { texture, px_size };

    // Insert after creating the texture (to avoid deadlock), then return the stored value.
    Some(ctx.data_mut(|d| {
        let cache = d
            .get_temp_mut_or_default::<std::collections::HashMap<SvgCacheKey, SvgCachedTexture>>(
                cache_id,
            );
        cache.entry(key).or_insert_with(|| value.clone()).clone()
    }))
}

/// Emit a one-time-per-block-type warning when no icon can be resolved.
static ICON_WARNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn warn_missing_icon(block_type: &str, block_path: &str) {
    let warned = ICON_WARNED.get_or_init(|| Mutex::new(HashSet::new()));
    if let Ok(mut set) = warned.lock()
        && set.insert(block_type.to_string())
    {
        eprintln!(
            "\x1b[33m[rustylink] WARNING: {} at {} does not have a corresponding virtual library block\x1b[0m",
            block_type, block_path
        );
    }
}

/// Draw a single [`IconSpec`] centered in `rect`.
///
/// Shared by the config-map icon path (`render_block_icon`) and the
/// definition-driven icon path so the catalog definition's `icon` and the
/// legacy registry render identically.  The rendered glyph is maximized to fill
/// the available center area while leaving a margin to the block border and
/// avoiding overlap with optional inside-block port labels.
pub fn draw_icon_spec(
    painter: &egui::Painter,
    rect: &Rect,
    font_scale: f32,
    icon: &block_types::IconSpec,
    color: Color32,
    port_label_widths: Option<PortLabelMaxWidths>,
) {
    match icon {
        block_types::IconSpec::Utf8(glyph) => {
            render_center_glyph_maximized(
                painter,
                rect,
                font_scale,
                glyph,
                color,
                port_label_widths,
            );
        }
        block_types::IconSpec::Math(spec) => {
            draw_math_icon(painter, rect, font_scale, spec, color, port_label_widths);
        }
        block_types::IconSpec::Plot(spec) => {
            draw_plot_icon(painter, rect, font_scale, spec, color, port_label_widths);
        }
        block_types::IconSpec::Phosphor(name) => {
            let avail_rect = compute_icon_available_rect(rect, font_scale, port_label_widths);
            let avail_points = avail_rect.size();
            if avail_points.x <= 1.0 || avail_points.y <= 1.0 {
                return;
            }
            let font_id = egui::FontId::proportional(avail_points.y * 0.7);
            painter.text(
                avail_rect.center(),
                egui::Align2::CENTER_CENTER,
                name,
                font_id,
                color,
            );
        }
        block_types::IconSpec::Svg(path) => {
            let avail_rect = compute_icon_available_rect(rect, font_scale, port_label_widths);
            let avail_points = avail_rect.size();
            if avail_points.x <= 1.0 || avail_points.y <= 1.0 {
                return;
            }

            let ctx = painter.ctx();
            let pixels_per_point = ctx.pixels_per_point();
            let request_px = [
                (avail_points.x * pixels_per_point).round().max(1.0) as usize,
                (avail_points.y * pixels_per_point).round().max(1.0) as usize,
            ];

            let Some(svg) = get_or_create_svg_texture(ctx, path, request_px) else {
                return;
            };

            let dest_size = svg_dest_size_points(avail_points, svg.px_size, pixels_per_point);
            if dest_size.x <= 1.0 || dest_size.y <= 1.0 {
                return;
            }

            let dest_rect = Rect::from_center_size(avail_rect.center(), dest_size);
            let uv = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0));
            painter.image(svg.texture.id(), dest_rect, uv, Color32::WHITE);
        }
    }
}

/// Draw a single-glyph [`IconSpec`] (Utf8 / Phosphor) rotated 90° clockwise,
/// centered in `rect`.  Used only by the far-zoom dashboard fallback for the
/// Toggle/Rocker switches, which Simulink draws vertically.  Non-glyph specs
/// (Svg / Math) fall back to the unrotated [`draw_icon_spec`].
pub fn draw_icon_spec_rotated_quarter(
    painter: &egui::Painter,
    rect: &Rect,
    icon: &block_types::IconSpec,
    color: Color32,
) {
    let glyph: &str = match icon {
        block_types::IconSpec::Utf8(g) => g,
        block_types::IconSpec::Phosphor(n) => n,
        _ => {
            draw_icon_spec(painter, rect, 1.0, icon, color, None);
            return;
        }
    };
    let avail = compute_icon_available_rect(rect, 1.0, None);
    // The glyph is rotated, so its unrotated height must fit the available
    // width (and vice versa) — size against the smaller dimension.
    let target = avail.size().min_elem();
    if target <= 1.0 {
        return;
    }
    let font_id = egui::FontId::proportional(target * 0.7);
    let galley = painter.layout_no_wrap(glyph.to_owned(), font_id, color);
    let angle = std::f32::consts::FRAC_PI_2;
    let rot = egui::emath::Rot2::from_angle(angle);
    // TextShape rotates the galley around its `pos` (top-left); offset so the
    // galley's visual center lands on the available rect's center.
    let pos = avail.center() - rot * (galley.size() * 0.5);
    let mut shape = egui::epaint::TextShape::new(pos, galley, color);
    shape.angle = angle;
    painter.add(shape);
}

/// Contrast color for a glyph icon drawn on this block's background.
pub fn block_icon_color(block: &Block) -> Color32 {
    let cfg = get_block_type_cfg(block);
    super::ui::colors::contrast_color(super::ui::colors::block_base_color(block, &cfg))
}

pub fn render_block_icon(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    icon_color: Color32,
    port_label_widths: Option<PortLabelMaxWidths>,
) {
    // Always prefer library-specific identifiers (library path / SourceBlock)
    // over generic `block_type` mappings.
    let cfg = get_block_type_cfg(block);
    // Glyph icons use the caller-provided contrast color (matching the actual
    // block fill); SVGs keep their own colors.
    let dark_icon = icon_color;
    if let Some(icon) = cfg.icon {
        draw_icon_spec(
            painter,
            rect,
            font_scale,
            &icon,
            dark_icon,
            port_label_widths,
        );
    } else {
        // No icon for this block.  Only warn for truly unknown blocks.
        // Known virtual-library blocks that simply lack a dedicated SVG
        // (e.g. "Is Hermitian", "Permute Matrix") are silently rendered
        // as "?" without a terminal warning.
        if !cfg.known {
            let raw_path = block
                .library_block_path
                .as_deref()
                .or_else(|| block.properties.get("SourceBlock").map(|s| s.as_str()))
                .unwrap_or("<unknown>");
            // Normalize the path for display: replace newlines with spaces so the
            // warning message is readable (SLX paths are word-wrapped with newlines).
            let block_path_display = raw_path.replace(['\n', '\r'], " ").replace('\\', "/");
            warn_missing_icon(&block.block_type, &block_path_display);
        }
        render_center_glyph_maximized(painter, rect, font_scale, "?", dark_icon, port_label_widths);
    }
}
/// Screen-space Y coordinates computed for a block's ports (as used by the UI when placing
/// port labels and clamped within the block rect). Keys are 1-based port indices.
#[derive(Clone, Debug, Default)]
pub struct ComputedPortYCoordinates {
    pub inputs: std::collections::HashMap<u32, f32>,
    pub outputs: std::collections::HashMap<u32, f32>,
}

/// Custom renderer for a ManualSwitch block.
///
/// Draws a simple switch symbol with two input poles (left) and one output pole (right).
/// The pole centers are aligned to the exact y-positions of the ports so that
/// connecting lines meet them cleanly. The lever connects from the selected input
/// (current_setting: "0" => bottom, "1" => top; default "0") to the output pole.
pub fn render_manual_switch(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    _font_scale: f32,
    coords: Option<&ComputedPortYCoordinates>,
) {
    // Determine how many ports to align (fall back to common defaults)
    let mut max_in: u32 = 0;
    let mut max_out: u32 = 0;
    for p in &block.ports {
        let idx = p.index.unwrap_or(0).max(1);
        if p.port_type == "in" {
            max_in = max_in.max(idx);
        }
        if p.port_type == "out" {
            max_out = max_out.max(idx);
        }
    }
    if max_in == 0 {
        max_in = 2;
    }
    if max_out == 0 {
        max_out = 1;
    }

    // Compute port anchors (in screen space)
    use super::geometry::PortSide;
    let mirrored = block.block_mirror.unwrap_or(false);
    let in_side = if mirrored {
        PortSide::Out
    } else {
        PortSide::In
    };
    let out_side = if mirrored {
        PortSide::In
    } else {
        PortSide::Out
    };
    let default_in1 = super::geometry::port_anchor_pos(*rect, in_side, 1, Some(max_in));
    let default_in2 = super::geometry::port_anchor_pos(*rect, in_side, 2, Some(max_in));
    let default_out = super::geometry::port_anchor_pos(*rect, out_side, 1, Some(max_out));

    // Place pole centers slightly inside the block border so the circles are fully visible
    let pad = 8.0_f32; // horizontal inset from the border for the circle centers
    let r_in = (rect.height() * 0.06).clamp(2.0, 6.0) * 0.8; // 20% smaller
    let r_out = r_in;
    let stroke_w = 1.5_f32; // thinner
    let col_active = Color32::from_rgb(32, 32, 32);
    let col_inactive = Color32::from_rgb(110, 110, 110); // dark gray for inactive

    let top_in_y = coords
        .and_then(|c| c.inputs.get(&1).copied())
        .unwrap_or(default_in1.y);
    let bot_in_y = coords
        .and_then(|c| c.inputs.get(&2).copied())
        .unwrap_or(default_in2.y);
    let out_y = coords
        .and_then(|c| c.outputs.get(&1).copied())
        .unwrap_or(default_out.y);

    let (top_in_center, bot_in_center, out_center) = if !mirrored {
        (
            Pos2::new(rect.left() + pad, top_in_y),
            Pos2::new(rect.left() + pad, bot_in_y),
            Pos2::new(rect.right() - pad, out_y),
        )
    } else {
        (
            Pos2::new(rect.right() - pad, top_in_y),
            Pos2::new(rect.right() - pad, bot_in_y),
            Pos2::new(rect.left() + pad, out_y),
        )
    };

    // Horizontal leads from border to the pole circles up to circle edge
    if !mirrored {
        let in1_anchor = Pos2::new(rect.left(), top_in_y);
        let in2_anchor = Pos2::new(rect.left(), bot_in_y);
        let out_anchor = Pos2::new(rect.right(), out_y);
        painter.line_segment(
            [
                in1_anchor,
                Pos2::new(top_in_center.x - r_in, top_in_center.y),
            ],
            Stroke::new(stroke_w, col_active),
        );
        painter.line_segment(
            [
                in2_anchor,
                Pos2::new(bot_in_center.x - r_in, bot_in_center.y),
            ],
            Stroke::new(stroke_w, col_active),
        );
        painter.line_segment(
            [Pos2::new(out_center.x + r_out, out_center.y), out_anchor],
            Stroke::new(stroke_w, col_active),
        );
    } else {
        let in1_anchor = Pos2::new(rect.right(), top_in_y);
        let in2_anchor = Pos2::new(rect.right(), bot_in_y);
        let out_anchor = Pos2::new(rect.left(), out_y);
        painter.line_segment(
            [
                in1_anchor,
                Pos2::new(top_in_center.x + r_in, top_in_center.y),
            ],
            Stroke::new(stroke_w, col_active),
        );
        painter.line_segment(
            [
                in2_anchor,
                Pos2::new(bot_in_center.x + r_in, bot_in_center.y),
            ],
            Stroke::new(stroke_w, col_active),
        );
        painter.line_segment(
            [Pos2::new(out_center.x - r_out, out_center.y), out_anchor],
            Stroke::new(stroke_w, col_active),
        );
    }

    // Draw open-circuit poles
    let set_top = matches!(block.current_setting.as_deref(), Some("1"));
    let top_col = if set_top { col_active } else { col_inactive };
    let bot_col = if set_top { col_inactive } else { col_active };
    painter.circle_stroke(top_in_center, r_in, Stroke::new(stroke_w, top_col));
    painter.circle_stroke(bot_in_center, r_in, Stroke::new(stroke_w, bot_col));
    painter.circle_stroke(out_center, r_out, Stroke::new(stroke_w, col_active));

    // Small stubs from the circle edge OUTSIDE the circle (1/3 of the circle diameter)
    let stub = (2.0 * r_in / 3.0).max(0.8); // 1/3 diameter
    // Input stubs extend inside the block: to the right for left-side inputs, to the left for right-side inputs.
    let in1_color = top_col;
    let in2_color = bot_col;
    if !mirrored {
        let in1_edge = top_in_center.x + r_in; // rightmost point of top input circle
        let in2_edge = bot_in_center.x + r_in; // rightmost point of bottom input circle
        painter.line_segment(
            [
                Pos2::new(in1_edge, top_in_center.y),
                Pos2::new(in1_edge + stub, top_in_center.y),
            ],
            Stroke::new(stroke_w, in1_color),
        );
        painter.line_segment(
            [
                Pos2::new(in2_edge, bot_in_center.y),
                Pos2::new(in2_edge + stub, bot_in_center.y),
            ],
            Stroke::new(stroke_w, in2_color),
        );
        // Output stub extends to the LEFT of the output circle: [edge - stub, edge]
        let out_edge_left = out_center.x - r_out; // leftmost point of output circle
        painter.line_segment(
            [
                Pos2::new(out_edge_left - stub, out_center.y),
                Pos2::new(out_edge_left, out_center.y),
            ],
            Stroke::new(stroke_w, col_active),
        );
        // Lever connects from active input stub end to output stub end
        let from_edge = in1_edge;
        let from_edge2 = in2_edge;
        let from_y_top = top_in_center.y;
        let from_y_bot = bot_in_center.y;
        let from_edge_sel = if set_top { from_edge } else { from_edge2 };
        let from_y_sel = if set_top { from_y_top } else { from_y_bot };
        let start = Pos2::new(from_edge_sel + stub, from_y_sel);
        let end = Pos2::new(out_edge_left - stub, out_center.y);
        painter.line_segment([start, end], Stroke::new(stroke_w, col_active));
    } else {
        let in1_edge = top_in_center.x - r_in; // leftmost point of top input circle (inputs on right)
        let in2_edge = bot_in_center.x - r_in; // leftmost point of bottom input circle
        painter.line_segment(
            [
                Pos2::new(in1_edge, top_in_center.y),
                Pos2::new(in1_edge - stub, top_in_center.y),
            ],
            Stroke::new(stroke_w, in1_color),
        );
        painter.line_segment(
            [
                Pos2::new(in2_edge, bot_in_center.y),
                Pos2::new(in2_edge - stub, bot_in_center.y),
            ],
            Stroke::new(stroke_w, in2_color),
        );
        // Output stub extends to the RIGHT of the output circle (output on left): [edge, edge + stub]
        let out_edge_right = out_center.x + r_out; // rightmost point of output circle
        painter.line_segment(
            [
                Pos2::new(out_edge_right, out_center.y),
                Pos2::new(out_edge_right + stub, out_center.y),
            ],
            Stroke::new(stroke_w, col_active),
        );
        // Lever
        let from_edge = in1_edge;
        let from_edge2 = in2_edge;
        let from_y_top = top_in_center.y;
        let from_y_bot = bot_in_center.y;
        let from_edge_sel = if set_top { from_edge } else { from_edge2 };
        let from_y_sel = if set_top { from_y_top } else { from_y_bot };
        let start = Pos2::new(from_edge_sel - stub, from_y_sel);
        let end = Pos2::new(out_edge_right + stub, out_center.y);
        painter.line_segment([start, end], Stroke::new(stroke_w, col_active));
    }
}

/// Draw the interior labels (+/-) for a Sum block.
///
/// The surrounding circle fill and stroke are drawn in the main ui loop's
/// background-fill and border-stroke passes.  This function only adds the
/// operator characters at their respective input-port positions inside the
/// circle – the left-edge operator for input 1 and the bottom-edge operator
/// for input 2.
///
/// The `Inputs` property format used by Simulink is e.g. `|++`: the first
/// character is an ignored spacer, the subsequent characters are the per-port
/// operators in order ('+' or '-').
pub fn render_sum_block(
    painter: &egui::Painter,
    rect: &Rect,
    font_scale: f32,
    operators: &[char],
    round: bool,
    colors: BodyColors,
) {
    let stroke = Stroke::new((1.6 * font_scale).clamp(1.0, 3.0), colors.border);
    if round {
        let radius = rect.size().min_elem() / 2.0;
        painter.circle(rect.center(), radius, colors.fill, stroke);
    } else {
        painter.rect_filled(*rect, 4.0, colors.fill);
        painter.rect_stroke(*rect, 4.0, stroke, egui::StrokeKind::Inside);
    }

    let font_size = (rect.height() * 0.34).clamp(8.0, 22.0) * font_scale;
    let font_id = egui::FontId::proportional(font_size);
    let text = colors.text;

    if round {
        // Classic round Sum: the last operator sits at the bottom (matching the
        // bottom-placed last input port), the rest stack down the left edge.
        let ops: &[char] = if operators.is_empty() {
            &['+', '+']
        } else {
            operators
        };
        let side = ops.len() - 1;
        for (i, op) in ops[..side].iter().enumerate() {
            let f = (i as f32 + 1.0) / (side as f32 + 1.0);
            painter.text(
                Pos2::new(
                    rect.left() + rect.width() * 0.28,
                    rect.top() + (0.18 + f * 0.44) * rect.height(),
                ),
                Align2::CENTER_CENTER,
                sign_str(*op),
                font_id.clone(),
                text,
            );
        }
        painter.text(
            Pos2::new(
                rect.center().x + rect.width() * 0.04,
                rect.bottom() - rect.height() * 0.26,
            ),
            Align2::CENTER_CENTER,
            sign_str(ops[side]),
            font_id,
            text,
        );
    } else {
        // Rectangular Add: stack the per-input signs down the left edge.
        let ops: &[char] = if operators.is_empty() {
            &['+', '+']
        } else {
            operators
        };
        let n = ops.len();
        for (i, op) in ops.iter().enumerate() {
            let f = (i as f32 + 1.0) / (n as f32 + 1.0);
            painter.text(
                Pos2::new(
                    rect.left() + rect.width() * 0.24,
                    rect.top() + f * rect.height(),
                ),
                Align2::CENTER_CENTER,
                sign_str(*op),
                font_id.clone(),
                text,
            );
        }
    }
}

/// Resolved body colors (fill / outline / interior text) passed to the
/// self-painting metadata-aware renderers (Sum, Logic, Product …).
#[derive(Clone, Copy)]
pub struct BodyColors {
    pub fill: Color32,
    pub border: Color32,
    pub text: Color32,
}

/// Map a Sum/Product operator char to a display glyph (proper minus / division
/// signs instead of the ASCII forms).
fn sign_str(op: char) -> &'static str {
    match op {
        '-' => "\u{2212}", // −
        '/' => "\u{00F7}", // ÷
        '*' => "\u{00D7}", // ×
        _ => "+",
    }
}

/// Parse a Simulink `Inputs` string into per-port operator chars.
///
/// Accepts numeric forms (`"2"` → two `+`), sign strings (`"+-"`, `"|++"` –
/// the `|` spacer and other layout chars are ignored) for Sum, and `*`/`/`
/// forms for Product.
pub fn parse_input_operators(inputs: &str, default: char) -> Vec<char> {
    let s = inputs.trim();
    if s.is_empty() {
        return vec![default, default];
    }
    if let Ok(n) = s.parse::<usize>() {
        return vec![default; n.max(1)];
    }
    let ops: Vec<char> = s
        .chars()
        .filter(|c| matches!(c, '+' | '-' | '*' | '/'))
        .collect();
    if ops.is_empty() {
        vec![default, default]
    } else {
        ops
    }
}

/// Draw a Product/Divide block interior (the shared passes draw the body).
///
/// When every input multiplies, a single centred `×` is shown (Simulink's
/// element-wise product icon).  When any input divides, the per-port `×`/`÷`
/// signs are stacked down the left edge.  Matrix multiplication adds brackets.
pub fn render_product_block(
    painter: &egui::Painter,
    rect: &Rect,
    font_scale: f32,
    operators: &[char],
    matrix: bool,
    text: Color32,
) {
    let has_div = operators.contains(&'/');
    if !has_div {
        // A single multiply port collapses the input vector: Simulink shows the
        // product-of-elements symbol `∏`, drawn as large as the block allows,
        // rather than the element-wise `×`.  It is drawn rather than typeset so
        // the bar overhangs both legs the way Simulink's icon does.
        if matches!(operators, [c] if *c == '*') && !matrix {
            draw_plot_icon(
                painter,
                rect,
                font_scale,
                "p 0.08,0.16 0.92,0.16; p 0.26,0.16 0.26,0.90; p 0.74,0.16 0.74,0.90",
                text,
                None,
            );
            return;
        }
        let font_size = (rect.height() * 0.5).clamp(9.0, 30.0) * font_scale;
        let font_id = egui::FontId::proportional(font_size);
        let glyph = if matrix {
            "[\u{00D7}]"
        } else {
            "\u{00D7}" // ×
        };
        painter.text(rect.center(), Align2::CENTER_CENTER, glyph, font_id, text);
        return;
    }
    let font_size = (rect.height() * 0.34).clamp(8.0, 22.0) * font_scale;
    let font_id = egui::FontId::proportional(font_size);
    let n = operators.len().max(1);
    for (i, op) in operators.iter().enumerate() {
        let f = (i as f32 + 1.0) / (n as f32 + 1.0);
        painter.text(
            Pos2::new(
                rect.left() + rect.width() * 0.30,
                rect.top() + f * rect.height(),
            ),
            Align2::CENTER_CENTER,
            sign_str(*op),
            font_id.clone(),
            text,
        );
    }
}

/// Draw a Logic (Logical Operator) block.
///
/// `icon_shape` selects between the rectangular text box (`"rectangular"`,
/// Simulink's default) and the distinctive IEEE gate symbol (`"distinctive"`).
/// `operator` selects the gate (AND/OR/NOT/NAND/NOR/XOR/NXOR).  The block owns
/// its whole body (shape [`crate::simulink_libraries::types::SimulinkShape::None`]).
pub fn render_logic_block(
    painter: &egui::Painter,
    rect: &Rect,
    font_scale: f32,
    operator: &str,
    icon_shape: &str,
    colors: BodyColors,
) {
    let stroke = Stroke::new((1.6 * font_scale).clamp(1.0, 3.0), colors.border);
    let op = operator.trim().to_uppercase();

    if !icon_shape.eq_ignore_ascii_case("distinctive") {
        painter.rect_filled(*rect, 4.0, colors.fill);
        painter.rect_stroke(*rect, 4.0, stroke, egui::StrokeKind::Inside);
        let label = if op.is_empty() { "AND".to_string() } else { op };
        let font_size = (rect.height() * 0.30).clamp(7.0, 20.0) * font_scale;
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(font_size),
            colors.text,
        );
        return;
    }

    let (base, negated) = match op.as_str() {
        "NAND" => (GateBase::And, true),
        "NOR" => (GateBase::Or, true),
        "NXOR" | "XNOR" => (GateBase::Xor, true),
        "XOR" => (GateBase::Xor, false),
        "OR" => (GateBase::Or, false),
        "NOT" => (GateBase::Not, true),
        _ => (GateBase::And, false),
    };

    let bubble_r = (rect.height() * 0.10).clamp(2.0, 6.0);
    let body = if negated {
        Rect::from_min_max(
            rect.min,
            Pos2::new(rect.right() - 2.0 * bubble_r, rect.bottom()),
        )
    } else {
        *rect
    };

    let (points, extra_arc) = match base {
        GateBase::And => (and_gate_path(&body), None),
        GateBase::Or => (or_gate_path(&body, false), None),
        GateBase::Xor => (or_gate_path(&body, false), Some(xor_back_arc(&body))),
        GateBase::Not => (not_gate_path(&body), None),
    };

    painter.add(egui::Shape::Path(egui::epaint::PathShape {
        points,
        closed: true,
        fill: colors.fill,
        stroke: stroke.into(),
    }));
    if let Some(arc) = extra_arc {
        painter.add(egui::Shape::line(arc, stroke));
    }
    if negated {
        let c = Pos2::new(body.right() + bubble_r, body.center().y);
        painter.circle(c, bubble_r, colors.fill, stroke);
    }
}

#[derive(Clone, Copy)]
enum GateBase {
    And,
    Or,
    Xor,
    Not,
}

fn quad_bezier(p0: Pos2, ctrl: Pos2, p1: Pos2, n: usize) -> Vec<Pos2> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            Pos2::new(
                u * u * p0.x + 2.0 * u * t * ctrl.x + t * t * p1.x,
                u * u * p0.y + 2.0 * u * t * ctrl.y + t * t * p1.y,
            )
        })
        .collect()
}

/// D-shaped AND gate outline (flat left/top/bottom, semicircular right).
fn and_gate_path(r: &Rect) -> Vec<Pos2> {
    let (l, rt, t, b, cy, h) = (
        r.left(),
        r.right(),
        r.top(),
        r.bottom(),
        r.center().y,
        r.height(),
    );
    let rad = h * 0.5;
    let flat_x = (rt - rad).max(l + r.width() * 0.15);
    let center = Pos2::new(flat_x, cy);
    let mut pts = vec![Pos2::new(l, t), Pos2::new(flat_x, t)];
    let n = 16;
    for i in 0..=n {
        let th = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * (i as f32 / n as f32);
        pts.push(Pos2::new(
            center.x + rad * th.cos(),
            center.y + rad * th.sin(),
        ));
    }
    pts.push(Pos2::new(l, b));
    pts
}

/// Curved OR gate outline (pointed right, concave left back edge).
fn or_gate_path(r: &Rect, _xor: bool) -> Vec<Pos2> {
    let (l, rt, t, b, cy, w) = (
        r.left(),
        r.right(),
        r.top(),
        r.bottom(),
        r.center().y,
        r.width(),
    );
    let tip = Pos2::new(rt, cy);
    let mut pts = Vec::new();
    // Top edge: top-left → tip.
    pts.extend(quad_bezier(
        Pos2::new(l, t),
        Pos2::new(l + w * 0.60, t),
        tip,
        14,
    ));
    // Bottom edge: tip → bottom-left.
    pts.extend(quad_bezier(
        tip,
        Pos2::new(l + w * 0.60, b),
        Pos2::new(l, b),
        14,
    ));
    // Back (left) concave edge: bottom-left → top-left, bulging right.
    pts.extend(quad_bezier(
        Pos2::new(l, b),
        Pos2::new(l + w * 0.22, cy),
        Pos2::new(l, t),
        12,
    ));
    pts
}

/// The extra concave back stroke drawn to the left of an XOR gate.
fn xor_back_arc(r: &Rect) -> Vec<Pos2> {
    let (l, t, b, cy, w) = (r.left(), r.top(), r.bottom(), r.center().y, r.width());
    let x = l - w * 0.12;
    quad_bezier(
        Pos2::new(x, t),
        Pos2::new(x + w * 0.22, cy),
        Pos2::new(x, b),
        12,
    )
}

/// Right-pointing triangle for the NOT (buffer) gate; the inversion bubble is
/// drawn separately by the caller.
fn not_gate_path(r: &Rect) -> Vec<Pos2> {
    vec![
        Pos2::new(r.left(), r.top()),
        Pos2::new(r.right(), r.center().y),
        Pos2::new(r.left(), r.bottom()),
    ]
}

/// Render Goto/From blocks with their GotoTag label instead of port labels.
///
/// Simulink brackets the tag (`[A]`) inside the tag-shaped body.
pub fn render_goto_from_block(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    name_font_factor: f32,
    color: Color32,
) {
    let tag = block
        .properties
        .get("GotoTag")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("A");
    let label = format!("[{tag}]");

    let mut font_size = (rect.height() * 0.6).clamp(10.0, 24.0) * font_scale * name_font_factor;
    let fitted = fit_font_px(painter, &label, rect.size() * Vec2::new(0.72, 0.7));
    font_size = font_size.min(fitted);

    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(font_size),
        color,
    );
}

// no re-exports; keep this module focused on rendering helpers
