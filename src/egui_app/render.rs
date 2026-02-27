#![cfg(feature = "egui")]

use crate::block_types::{self, BlockTypeConfig, Rgb};
use crate::model::Block;
use eframe::egui::{self, Align2, Color32, Pos2, Rect, Stroke, Vec2};

use super::icon_assets;
use std::sync::{Arc, OnceLock};

pub(crate) fn rgb_to_color32(c: Rgb) -> Color32 {
    Color32::from_rgb(c.0, c.1, c.2)
}

fn normalize_library_block_path(path: &str) -> Option<String> {
    let path = path.trim();
    if path.is_empty() {
        return None;
    }

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


pub(crate) fn get_block_type_cfg(block: &Block) -> BlockTypeConfig {
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
        if let Some(n) = normalize_library_block_path(lib_path) {
            if n != *lib_path {
                lib_candidates.push(n);
            }
        }
    }
    // Always check SourceBlock as well (not only when library_block_path is absent),
    // since library_block_path is derived from it and may carry the same casing issues.
    if let Some(source_block) = block.properties.get("SourceBlock") {
        if block.library_block_path.as_deref() != Some(source_block.as_str()) {
            lib_candidates.push(source_block.clone());
        }
        if let Some(n) = normalize_library_block_path(source_block) {
            if !lib_candidates.contains(&n) {
                lib_candidates.push(n);
            }
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

    // Phase 2 – case-insensitive fallback on the last segments.
    // Handles blocks where the SLX uses different capitalisation than our
    // registry (e.g. "Cross product" vs "Cross Product").
    for seg in &last_segments {
        let seg_lower = seg.to_ascii_lowercase();
        if let Some(cfg) = g
            .iter()
            .find(|(k, _)| k.to_ascii_lowercase() == seg_lower)
            .map(|(_, v)| v.clone())
        {
            return cfg.clone();
        }
    }

    // Phase 3 – Simulink-semantic overrides that are expressed through block
    // properties rather than via a SourceBlock/library path.
    // A plain Product block with Multiplication="Matrix(*)" is the standard way
    // Simulink encodes a matrix-multiply.  Show the dedicated SVG for it.
    if block.block_type == "Product"
        && block
            .properties
            .get("Multiplication")
            .map(|v| v.trim())
            == Some("Matrix(*)")
    {
        if let Some(cfg) = g.get("MatrixMultiply") {
            return cfg.clone();
        }
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

pub(crate) fn port_label_display_name(block: &Block, index: u32, is_input: bool) -> String {
    // Note: The port-label drawing code treats mirroring as swapping the logical direction
    // when looking up Port properties. Keep this logic in one place so icon sizing and
    // label rendering stay consistent.
    let mirrored = block.block_mirror.unwrap_or(false);
    let logical_is_input = if mirrored { !is_input } else { is_input };
    block
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
                .or_else(|| Some(format!("{}{}", if is_input { "In" } else { "Out" }, index)))
        })
        .next()
        .unwrap_or_else(|| format!("{}{}", if is_input { "In" } else { "Out" }, index))
}

pub(crate) fn wrap_text_to_max_width(
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

fn embedded_egui_sans_fontdb() -> Option<Arc<resvg::usvg::fontdb::Database>> {
    static FONTDB: OnceLock<Option<Arc<resvg::usvg::fontdb::Database>>> = OnceLock::new();
    FONTDB
        .get_or_init(|| {
            let font_defs = egui::FontDefinitions::default();
            let ubuntu = font_defs.font_data.get("Ubuntu-Light")?;

            let mut db = resvg::usvg::fontdb::Database::new();
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
        .min(1.0)
        .max(0.0);
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
    let mut options = resvg::usvg::Options::default();
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
        let cache = d.get_temp_mut_or_default::<std::collections::HashMap<SvgCacheKey, SvgCachedTexture>>(
            cache_id,
        );
        cache
            .entry(key)
            .or_insert_with(|| value.clone())
            .clone()
    }))
}

/// Render an icon in the center of the block according to its type.
///
/// The rendered glyph is maximized to fill the available center area while:
/// - leaving at least 10% margin to the block border on all sides
/// - avoiding overlap with optional inside-block port labels (left/right)
pub fn render_block_icon(
    painter: &egui::Painter,
    block: &Block,
    rect: &Rect,
    font_scale: f32,
    port_label_widths: Option<PortLabelMaxWidths>,
) {
    let dark_icon = Color32::from_rgb(40, 40, 40); // dark color for icons
    // Always prefer library-specific identifiers (library path / SourceBlock)
    // over generic `block_type` mappings.
    let cfg = get_block_type_cfg(block);
    if let Some(icon) = cfg.icon {
        match icon {
            block_types::IconSpec::Utf8(glyph) => {
                render_center_glyph_maximized(
                    painter,
                    rect,
                    font_scale,
                    glyph,
                    dark_icon,
                    port_label_widths,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_types::IconSpec;

    #[test]
    fn icon_lookup_prefers_sourceblock_over_block_type() {
        // Simulate a matrix-library block that is internally a generic Product
        // but has a library origin that should override the generic icon.
        let mut b = crate::editor::operations::create_default_block(
            "Product",
            "Matrix Multiply",
            0,
            0,
            2,
            1,
        );
        b.properties.insert(
            "SourceBlock".to_string(),
            "matrix_library.slx/Matrix Multiply".to_string(),
        );
        b.library_block_path = None;

        let cfg = get_block_type_cfg(&b);
        assert_eq!(cfg.icon, Some(IconSpec::Svg("matrix/matrix_product.svg")));
    }

    #[test]
    fn icon_lookup_accepts_normalized_slx_library_path() {
        let mut b = crate::editor::operations::create_default_block(
            "Product",
            "MatrixMultiply",
            0,
            0,
            2,
            1,
        );
        b.library_block_path = Some("matrix_library.slx/MatrixMultiply".to_string());

        let cfg = get_block_type_cfg(&b);
        assert_eq!(cfg.icon, Some(IconSpec::Svg("matrix/matrix_product.svg")));
    }

    /// Blocks whose SLX name uses different capitalisation than the registry key
    /// (e.g. "Cross product" with a lowercase 'p') must still resolve to the
    /// correct SVG icon via the case-insensitive fallback, and must NOT fall
    /// through to the generic block_type icon (the "×" Product icon).
    #[test]
    fn icon_lookup_cross_product_case_insensitive() {
        let mut b = crate::editor::operations::create_default_block(
            "Product",
            "Cross product",
            0,
            0,
            2,
            1,
        );
        // Simulate what the parser sets: library_block_path from SourceBlock.
        b.library_block_path = Some("matrix_library/Cross product".to_string());

        let cfg = get_block_type_cfg(&b);
        assert_eq!(cfg.icon, Some(IconSpec::Svg("matrix/cross_product.svg")));
    }

    /// "Submatrix" must resolve to its dedicated SVG icon, not the old "👁" placeholder.
    #[test]
    fn icon_lookup_submatrix_uses_svg() {
        let mut b = crate::editor::operations::create_default_block(
            "SubSystem",
            "Submatrix",
            0,
            0,
            1,
            1,
        );
        b.library_block_path = Some("matrix_library/Submatrix".to_string());

        let cfg = get_block_type_cfg(&b);
        assert_eq!(cfg.icon, Some(IconSpec::Svg("matrix/submatrix.svg")));
    }

    /// A plain Simulink Product block with Multiplication="Matrix(*)" should use
    /// the matrix_product SVG rather than the generic "×" Product icon.
    /// This is how Simulink encodes a matrix-multiply without a library reference.
    #[test]
    fn icon_lookup_product_matrix_multiplication_uses_svg() {
        let mut b = crate::editor::operations::create_default_block(
            "Product",
            "Matrix Multiply",
            0,
            0,
            2,
            1,
        );
        b.properties.insert("Multiplication".to_string(), "Matrix(*)".to_string());

        let cfg = get_block_type_cfg(&b);
        assert_eq!(cfg.icon, Some(IconSpec::Svg("matrix/matrix_product.svg")));
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

// no re-exports; keep this module focused on rendering helpers
