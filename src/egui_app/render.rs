#![cfg(feature = "egui")]

use eframe::egui::{self, Align2, Color32, Rect};
use crate::block_types::{self, BlockTypeConfig, IconSpec, Rgb};
use crate::model::Block;

pub(crate) fn rgb_to_color32(c: Rgb) -> Color32 { Color32::from_rgb(c.0, c.1, c.2) }

pub(crate) fn get_block_type_cfg(block_type: &str) -> BlockTypeConfig {
    let map = block_types::get_block_type_config_map();
    if let Ok(g) = map.read() {
        g.get(block_type).cloned().unwrap_or_default()
    } else {
        BlockTypeConfig::default()
    }
}

/// Render an icon in the center of the block according to its type using the phosphor font.
///
/// The `font_scale` parameter scales the icon size relative to the baseline size
/// (baseline is 24.0 at 400% zoom; caller should pass zoom/4.0).
pub fn render_block_icon(painter: &egui::Painter, block: &Block, rect: &Rect, font_scale: f32) {
    let icon_size = 24.0 * font_scale.max(0.01);
    let icon_center = rect.center();
    let font = egui::FontId::new(icon_size, egui::FontFamily::Name("phosphor".into()));
    let dark_icon = Color32::from_rgb(40, 40, 40); // dark color for icons
    // Lookup icon from centralized registry
    let cfg = get_block_type_cfg(&block.block_type);
    if let Some(icon) = cfg.icon {
        match icon {
            IconSpec::Phosphor(glyph) => {
                painter.text(icon_center, Align2::CENTER_CENTER, glyph, font, dark_icon);
            }
        }
    }
}

// no re-exports; keep this module focused on rendering helpers
