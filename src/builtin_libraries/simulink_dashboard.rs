//! Simulink Dashboard / UI block type definitions.
//!
//! This module describes all known dashboard and UI blocks that appear in
//! Simulink models.  These blocks are part of the Simulink Dashboard library
//! and include gauges, switches, buttons, editable fields, indicators, scopes,
//! and other interactive widgets.
//!
//! The block types are derived from the `BlockType` attribute values found
//! in real-world Simulink `.slx` archives (e.g., `Simulink_UI_Test.slx`).

use super::virtual_library::{BlockShape, VirtualBlock};

/// Library name used for matching during library resolution.
pub const LIB_NAME: &str = "simulink/Dashboard";

/// All known dashboard / UI block types.
pub const BLOCKS: &[VirtualBlock] = &[
    // ── Input widgets (typically 0 ports – they write to workspace) ─────
    VirtualBlock {
        name: "Checkbox",
        aliases: &["CheckboxBlock"],
        ins: 0,
        outs: 0,
        shape: BlockShape::Rectangle,
        icon: None,
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "ComboBox",
        aliases: &["ComboBoxBlock"],
        ins: 0,
        outs: 0,
        shape: BlockShape::Rectangle,
        icon: None,
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "EditField",
        aliases: &["EditFieldBlock"],
        ins: 0,
        outs: 0,
        shape: BlockShape::Rectangle,
        icon: None,
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "KnobBlock",
        aliases: &["Knob"],
        ins: 0,
        outs: 0,
        shape: BlockShape::Rectangle,
        icon: None,
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "PushButtonBlock",
        aliases: &["PushButton"],
        ins: 0,
        outs: 0,
        shape: BlockShape::Rectangle,
        icon: None,
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "RadioButtonGroup",
        aliases: &["RadioButtonGroupBlock"],
        ins: 0,
        outs: 0,
        shape: BlockShape::Rectangle,
        icon: None,
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "RockerSwitchBlock",
        aliases: &["RockerSwitch"],
        ins: 0,
        outs: 0,
        shape: BlockShape::Rectangle,
        icon: None,
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "RotarySwitchBlock",
        aliases: &["RotarySwitch"],
        ins: 0,
        outs: 0,
        shape: BlockShape::Rectangle,
        icon: None,
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "SliderBlock",
        aliases: &["Slider"],
        ins: 0,
        outs: 0,
        shape: BlockShape::Rectangle,
        icon: None,
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "SliderSwitchBlock",
        aliases: &["SliderSwitch"],
        ins: 0,
        outs: 0,
        shape: BlockShape::Rectangle,
        icon: None,
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "ToggleSwitchBlock",
        aliases: &["ToggleSwitch"],
        ins: 0,
        outs: 0,
        shape: BlockShape::Rectangle,
        icon: None,
        ..VirtualBlock::DEFAULT
    },
    // ── Output / indicator widgets ──────────────────────────────────────
    VirtualBlock {
        name: "Display",
        aliases: &["DisplaySink"],
        ins: 1,
        outs: 0,
        shape: BlockShape::Rectangle,
        icon: None,
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "DisplayBlock",
        aliases: &["DashboardDisplay"],
        ins: 0,
        outs: 0,
        shape: BlockShape::Rectangle,
        icon: None,
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "LampBlock",
        aliases: &["Lamp"],
        ins: 0,
        outs: 0,
        shape: BlockShape::Rectangle,
        icon: None,
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "CircularGaugeBlock",
        aliases: &["CircularGauge"],
        ins: 0,
        outs: 0,
        shape: BlockShape::Rectangle,
        icon: None,
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "SemiCircularGaugeBlock",
        aliases: &["SemiCircularGauge", "HalfGauge"],
        ins: 0,
        outs: 0,
        shape: BlockShape::Rectangle,
        icon: None,
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "LinearGaugeBlock",
        aliases: &["LinearGauge"],
        ins: 0,
        outs: 0,
        shape: BlockShape::Rectangle,
        icon: None,
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "QuarterGaugeBlock",
        aliases: &["QuarterGauge"],
        ins: 0,
        outs: 0,
        shape: BlockShape::Rectangle,
        icon: None,
        ..VirtualBlock::DEFAULT
    },
    // ── Scope / visualization ───────────────────────────────────────────
    VirtualBlock {
        name: "DashboardScope",
        aliases: &["DashboardScopeBlock"],
        ins: 0,
        outs: 0,
        shape: BlockShape::Rectangle,
        icon: None,
        ..VirtualBlock::DEFAULT
    },
];

/// Return the block definition table for this library.
pub fn get_blocks() -> &'static [VirtualBlock] {
    BLOCKS
}

/// Return `true` if the given library name matches the Simulink Dashboard
/// library (case-insensitive, with optional `.slx` suffix).
pub fn is_simulink_dashboard_name(name: &str) -> bool {
    let norm = name.trim().replace('\\', "/").to_ascii_lowercase();
    norm == "simulink/dashboard" || norm.starts_with("simulink/dashboard/")
}

/// All Simulink dashboard block type names that are recognised natively.
///
/// The parser can use this list to detect whether a `BlockType` value
/// corresponds to a dashboard/UI widget, even when no `SourceBlock` property
/// is present (dashboard blocks are often placed directly in the model root).
pub const DASHBOARD_BLOCK_TYPES: &[&str] = &[
    "Checkbox",
    "ComboBox",
    "DashboardScope",
    "Display",
    "DisplayBlock",
    "EditField",
    "CircularGaugeBlock",
    "SemiCircularGaugeBlock",
    "KnobBlock",
    "LampBlock",
    "LinearGaugeBlock",
    "PushButtonBlock",
    "QuarterGaugeBlock",
    "RadioButtonGroup",
    "RockerSwitchBlock",
    "RotarySwitchBlock",
    "SliderBlock",
    "SliderSwitchBlock",
    "ToggleSwitchBlock",
];

/// Return `true` if the given `BlockType` string is a known dashboard / UI
/// widget type.
pub fn is_dashboard_block_type(block_type: &str) -> bool {
    DASHBOARD_BLOCK_TYPES
        .iter()
        .any(|&t| t.eq_ignore_ascii_case(block_type))
}
