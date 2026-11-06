//! E2E: block-library coverage gate — every canonical Simulink BlockType must
//! resolve to a real definition (icon or static renderer), not the generic
//! `?` fallback.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use rustylink::editor::operations::create_default_block;
use rustylink::simulink_libraries::resolve_definition;
use rustylink::simulink_libraries::traits::KNOWN_BLOCK_TYPES;

/// Block types we intentionally leave generic (aliases handled elsewhere, or
/// genuinely lacking a Simulink glyph). Kept minimal — every entry needs a
/// comment.
const ALLOWED_GENERIC: &[&str] = &[
    // Subsystem port-connector sub-types resolved through SubSystem.
    "InportShadow",
    "OutportShadow",
    // Tiny/structural blocks with no icon in Simulink.
    "GotoTagVisibility",
    "BusElement",
    "BusElementPort",
    "SignalConversion",
    "PMIOPort",
    "Video",
    // Resolves to Reference: the interior labels the linked library.
    "ModelReference",
];

#[test]
fn cov_001_every_known_type_resolves_to_a_real_definition() {
    let mut missing: Vec<&str> = Vec::new();
    for &ty in KNOWN_BLOCK_TYPES {
        let block = create_default_block(ty, ty, 0, 0, 0, 0);
        let def = resolve_definition(&block);
        // A "real" definition provides an icon, a static renderer, a live
        // renderer, or a block label — anything beyond the bare `?` fallback.
        let real = def.icon.is_some()
            || def.static_renderer.is_some()
            || def.live_renderer.is_some()
            // Any block label (fixed or metadata-derived) counts.
            || !matches!(
                def.block_label,
                rustylink::simulink_libraries::types::BlockLabelPolicy::None
            )
            // A non-default body shape (Triangle/FilledBlack/Goto/From/
            // Obround/Circle) conveys the block's identity by itself.
            || !matches!(
                def.shape,
                rustylink::simulink_libraries::types::SimulinkShape::Rectangle
                    | rustylink::simulink_libraries::types::SimulinkShape::None
            );
        if !real && !ALLOWED_GENERIC.contains(&ty) {
            missing.push(ty);
        }
    }
    assert!(
        missing.is_empty(),
        "block types resolving to the generic fallback:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn cov_002_subsystem_variants_get_subsystem_rendering() {
    for ty in [
        "SubSystem",
        "EnabledSubsystem",
        "TriggeredSubsystem",
        "IfActionSubsystem",
        "EnabledAndTriggeredSubsystem",
        "ForEachSubsystem",
        "WhileSubsystem",
        "ResettableSubsystem",
        "AtomicSubSystem",
        "MaskedSubSystem",
        "VariantSubSystem",
        "ConfigSubSystem",
    ] {
        let block = create_default_block(ty, ty, 0, 0, 0, 0);
        let def = resolve_definition(&block);
        assert!(
            def.static_renderer.is_some(),
            "{ty} should render like a subsystem"
        );
    }
}

#[test]
fn cov_003_discontinuity_icons_are_curves_not_question_marks() {
    for ty in [
        "DeadZone",
        "Quantizer",
        "RateLimiter",
        "Relay",
        "Saturate",
        "Backlash",
        "WrapToZero",
        "SaturateDynamic",
    ] {
        let block = create_default_block(ty, ty, 0, 0, 0, 0);
        let def = resolve_definition(&block);
        assert!(def.icon.is_some(), "{ty} should carry a curve icon");
    }
}
