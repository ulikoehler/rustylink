use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use eframe::egui::Color32;

pub(crate) fn luminance(c: Color32) -> f32 {
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

pub(crate) fn contrast_color(bg: Color32) -> Color32 {
    let lum = luminance(bg);
    if lum > 0.6 {
        Color32::from_rgb(25, 35, 45)
    } else {
        Color32::from_rgb(235, 245, 245)
    }
}

pub(crate) fn hsv_to_color32(h: f32, s: f32, v: f32) -> Color32 {
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

pub(crate) fn hash_color(input: &str, s: f32, v: f32) -> Color32 {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    let hash = hasher.finish();
    let h = (hash as f32 / u64::MAX as f32) % 1.0;
    hsv_to_color32(h, s, v)
}

pub(crate) fn block_base_color(
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
