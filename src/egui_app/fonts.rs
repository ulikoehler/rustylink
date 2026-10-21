#![cfg(feature = "egui")]

//! Custom font installation for the egui viewer/editor.

use eframe::egui;

/// Register the Material Symbols icon font so [`SimulinkIcon::Phosphor`] block
/// icons render as real glyphs instead of missing-glyph boxes.
///
/// Material glyphs are appended as a fallback to the proportional family, so
/// regular text is unaffected while the catalog's icon codepoints
/// resolve correctly.
///
/// [`SimulinkIcon::Phosphor`]: crate::simulink_libraries::types::SimulinkIcon::Phosphor
pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Add the Material Symbols font to Proportional and Monospace families so
    // icon glyphs in format! strings are found.
    let mut icon_insert = egui_material_icons::font_insert();
    icon_insert
        .families
        .push(egui::epaint::text::InsertFontFamily {
            family: egui::FontFamily::Monospace,
            priority: egui::epaint::text::FontPriority::Lowest,
        });
    fonts.font_data.insert(
        icon_insert.name.clone(),
        std::sync::Arc::new(icon_insert.data),
    );
    for fam in icon_insert.families {
        let entry = fonts.families.entry(fam.family).or_default();
        match fam.priority {
            egui::epaint::text::FontPriority::Highest => entry.insert(0, icon_insert.name.clone()),
            egui::epaint::text::FontPriority::Lowest => entry.push(icon_insert.name.clone()),
        }
    }

    // Bundle DejaVu Sans as a broad-coverage fallback so the technical/math
    // glyphs used by block icons (✓ ☰ ∠ ∿ ⊥ ⇒ superscripts …) render as real
    // glyphs instead of missing-glyph boxes. It is appended *after* the default
    // family and phosphor, so regular Latin text keeps its normal font and
    // DejaVu only supplies glyphs nothing earlier can provide.
    const DEJAVU_SANS: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans.ttf");
    fonts.font_data.insert(
        "DejaVuSans".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(DEJAVU_SANS)),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("DejaVuSans".to_owned());
    }

    ctx.set_fonts(fonts);
}
