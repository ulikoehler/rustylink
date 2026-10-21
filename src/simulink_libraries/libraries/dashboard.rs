//! Dashboard / HMI blocks.
//!
//! Per the catalog design, every dashboard block wires its **own** static and
//! live renderer rather than routing through one shared hook.  The thin
//! adapters below unpack the [`RenderContext`] and delegate to the matching
//! per-widget drawing routine in [`crate::egui_app::dashboard_widgets`].  Only
//! genuinely near-identical widgets share a live renderer (the radial gauges,
//! the slider/linear-gauge pair and the edit-field/display pair); everything
//! else is one function per block.

#![cfg(feature = "egui")]

use eframe::egui::{Painter, Rect, Ui};

use crate::egui_app::dashboard_widgets as dw;
use crate::egui_app::state::SubsystemApp;
use crate::model::Block;
use crate::simulink_libraries::types::{
    DashboardControlKind, IOPorts, RenderContext, SimulinkBlockDefinition, SimulinkIcon,
    SimulinkShape,
};
use egui_material_icons::icons::{
    ICON_CHECK_BOX, ICON_CIRCLE, ICON_EDIT, ICON_EXPAND_MORE, ICON_RECTANGLE,
};

const fn icon(glyph: &'static str) -> SimulinkIcon {
    SimulinkIcon::Utf8(glyph)
}

/// A Phosphor icon (renders via the bundled phosphor font, so it never falls
/// back to a missing-glyph box the way some technical UTF-8 symbols do).
const fn ph(name: &'static str) -> SimulinkIcon {
    SimulinkIcon::Phosphor(name)
}

/// Generate a per-block static interior renderer that draws the widget's
/// default (non-live) appearance.
macro_rules! static_adapter {
    ($name:ident => $draw:ident) => {
        fn $name(p: &Painter, b: &Block, r: &Rect, ctx: &RenderContext<'_>) -> bool {
            dw::$draw(p, b, r, ctx.font_scale, ctx.name_font_factor);
            true
        }
    };
}

/// Generate a non-interactive live renderer that paints the widget's live value
/// (gauges, lamps, displays).  It ignores `app` and draws via `ui.painter()`;
/// returns `false` when there is no live value so the static renderer is used.
macro_rules! live_view {
    ($name:ident => $draw:ident) => {
        fn $name(
            _app: &mut SubsystemApp,
            ui: &mut Ui,
            b: &Block,
            r: &Rect,
            ctx: &RenderContext<'_>,
        ) -> bool {
            let Some(value) = ctx.live_value else {
                return false;
            };
            dw::$draw(
                &ui.painter().with_clip_rect(*r),
                b,
                r,
                ctx.font_scale,
                value,
                ctx.live_display_options,
            );
            true
        }
    };
}

/// Generate an interactive live renderer for a dashboard input control: it
/// draws the widget and handles user interaction (emitting control values)
/// using the mutable `app`/`Ui` carried by the unified [`LiveRendererFn`].
macro_rules! live_control {
    ($name:ident => $inner:ident) => {
        fn $name(
            app: &mut SubsystemApp,
            ui: &mut Ui,
            b: &Block,
            r: &Rect,
            ctx: &RenderContext<'_>,
        ) -> bool {
            dw::$inner(
                app,
                ui,
                b,
                *r,
                ctx.font_scale,
                ctx.live_value.unwrap_or(0.0),
                ctx.live_text,
            )
        }
    };
}

// ── Per-block static renderers (one per widget) ─────────────────────────────
static_adapter!(static_push_button => render_push_button);
static_adapter!(static_checkbox => render_checkbox);
static_adapter!(static_combo_box => render_combo_box);
static_adapter!(static_edit_field => render_edit_field);
static_adapter!(static_radio_button_group => render_radio_button);
static_adapter!(static_slider => render_slider);
static_adapter!(static_slider_switch => render_slider_switch);
static_adapter!(static_toggle_switch => render_toggle_switch);
static_adapter!(static_rocker_switch => render_rocker_switch);
static_adapter!(static_rotary_switch => render_rotary_switch);
static_adapter!(static_knob => render_knob);
static_adapter!(static_circular_gauge => render_circular_gauge);
static_adapter!(static_semi_circular_gauge => render_semi_circular_gauge);
static_adapter!(static_quarter_gauge => render_quarter_gauge);
static_adapter!(static_linear_gauge => render_linear_gauge);
static_adapter!(static_lamp => render_lamp);
static_adapter!(static_display => render_display_block);
static_adapter!(static_dashboard_scope => render_dashboard_scope);

// ── Non-interactive live renderers (gauges / lamps / displays) ──────────────
// Near-identical widget families share one painter routine.
live_view!(view_radial_gauge => live_radial_gauge);
live_view!(view_linear_gauge => live_slider_or_linear_gauge);
live_view!(view_field_or_display => live_field_or_display);
live_view!(view_lamp => live_lamp);

// ── Interactive live renderers (one per input control) ──────────────────────
live_control!(live_push_button => control_push_button);
live_control!(live_checkbox => control_checkbox);
live_control!(live_combo_box => control_combo_box);
live_control!(live_edit_field => control_edit_field);
live_control!(live_radio_button_group => control_radio_button_group);
live_control!(live_slider => control_slider);
live_control!(live_slider_switch => control_slider_switch);
live_control!(live_toggle_switch => control_toggle_switch);
live_control!(live_rocker_switch => control_rocker_switch);
live_control!(live_rotary_switch => control_rotary_switch);
live_control!(live_knob => control_knob);

/// A dashboard widget definition with its own static and live renderers.
const fn widget(
    block_type: &'static str,
    display_name: &'static str,
    description: &'static str,
    glyph: SimulinkIcon,
    inputs: IOPorts,
    static_fn: crate::simulink_libraries::types::StaticRendererFn,
    live_fn: crate::simulink_libraries::types::LiveRendererFn,
) -> SimulinkBlockDefinition {
    SimulinkBlockDefinition::new(block_type, "Dashboard")
        .with_display_name(display_name)
        .with_description(description)
        .with_ports(inputs, IOPorts::None)
        .with_icon(glyph)
        .with_static_renderer(static_fn)
        .with_live_renderer(live_fn)
}

pub static BLOCKS: &[SimulinkBlockDefinition] = &[
    // Value displays.
    SimulinkBlockDefinition::new("Display", "Dashboard")
        .with_display_name("Display")
        .with_description("Display the value of the connected signal")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_icon(ph(egui_material_icons::icons::ICON_MONITOR.codepoint))
        .with_static_renderer(static_display)
        .with_live_renderer(view_field_or_display),
    widget(
        "DisplayBlock",
        "Dashboard Display",
        "Dashboard numeric display widget",
        ph(egui_material_icons::icons::ICON_MONITOR.codepoint),
        IOPorts::Fixed(1),
        static_display,
        view_field_or_display,
    ),
    // Controls (inputs).
    widget(
        "PushButtonBlock",
        "Push Button",
        "Momentary push button UI control",
        ph(egui_material_icons::icons::ICON_RADIO_BUTTON_CHECKED.codepoint),
        IOPorts::None,
        static_push_button,
        live_push_button,
    )
    .with_dashboard_control(DashboardControlKind::Pulse),
    widget(
        "Checkbox",
        "Checkbox",
        "Boolean checkbox UI control",
        icon(ICON_CHECK_BOX.codepoint),
        IOPorts::None,
        static_checkbox,
        live_checkbox,
    )
    .with_dashboard_control(DashboardControlKind::Bool),
    widget(
        "ComboBox",
        "Combo Box",
        "Dropdown selection UI control",
        icon(ICON_EXPAND_MORE.codepoint),
        IOPorts::None,
        static_combo_box,
        live_combo_box,
    )
    .with_dashboard_control(DashboardControlKind::Discrete),
    widget(
        "EditField",
        "Edit Field",
        "Text/number input field UI control",
        icon(ICON_EDIT.codepoint),
        IOPorts::None,
        static_edit_field,
        live_edit_field,
    )
    .with_dashboard_control(DashboardControlKind::Scalar),
    widget(
        "RadioButtonGroup",
        "Radio Button Group",
        "Radio button selection group",
        icon(ICON_CIRCLE.codepoint),
        IOPorts::None,
        static_radio_button_group,
        live_radio_button_group,
    )
    .with_dashboard_control(DashboardControlKind::Discrete),
    widget(
        "SliderBlock",
        "Slider",
        "Continuous value slider control",
        ph(egui_material_icons::icons::ICON_TUNE.codepoint),
        IOPorts::None,
        static_slider,
        live_slider,
    )
    .with_dashboard_control(DashboardControlKind::Scalar),
    widget(
        "SliderSwitchBlock",
        "Slider Switch",
        "Two-position slider switch",
        // Same switch glyph as the Toggle/Rocker fallbacks, but drawn upright
        // (the far-zoom path rotates only the Toggle/Rocker switches 90°).
        ph(egui_material_icons::icons::ICON_TOGGLE_OFF.codepoint),
        IOPorts::None,
        static_slider_switch,
        live_slider_switch,
    )
    .with_dashboard_control(DashboardControlKind::Bool),
    widget(
        "ToggleSwitchBlock",
        "Toggle Switch",
        "Two-position toggle switch",
        ph(egui_material_icons::icons::ICON_TOGGLE_OFF.codepoint),
        IOPorts::None,
        static_toggle_switch,
        live_toggle_switch,
    )
    .with_dashboard_control(DashboardControlKind::Bool),
    widget(
        "RockerSwitchBlock",
        "Rocker Switch",
        "Rocker-style on/off switch",
        ph(egui_material_icons::icons::ICON_TOGGLE_ON.codepoint),
        IOPorts::None,
        static_rocker_switch,
        live_rocker_switch,
    )
    .with_dashboard_control(DashboardControlKind::Bool),
    widget(
        "RotarySwitchBlock",
        "Rotary Switch",
        "Multi-position rotary switch",
        icon(ICON_CIRCLE.codepoint),
        IOPorts::None,
        static_rotary_switch,
        live_rotary_switch,
    )
    .with_dashboard_control(DashboardControlKind::Discrete),
    widget(
        "KnobBlock",
        "Knob",
        "Rotary knob input control",
        icon(ICON_CIRCLE.codepoint),
        IOPorts::None,
        static_knob,
        live_knob,
    )
    .with_dashboard_control(DashboardControlKind::Scalar),
    // Gauges / indicators (outputs/displays).
    widget(
        "CircularGaugeBlock",
        "Circular Gauge",
        "Circular gauge indicator",
        icon("◔"),
        IOPorts::Fixed(1),
        static_circular_gauge,
        view_radial_gauge,
    ),
    widget(
        "SemiCircularGaugeBlock",
        "Semi-Circular Gauge",
        "Half-circle gauge indicator",
        icon("◑"),
        IOPorts::Fixed(1),
        static_semi_circular_gauge,
        view_radial_gauge,
    ),
    widget(
        "QuarterGaugeBlock",
        "Quarter Gauge",
        "Quarter-circle gauge indicator",
        icon("◕"),
        IOPorts::Fixed(1),
        static_quarter_gauge,
        view_radial_gauge,
    ),
    widget(
        "LinearGaugeBlock",
        "Linear Gauge",
        "Linear/horizontal gauge indicator",
        icon(ICON_RECTANGLE.codepoint),
        IOPorts::Fixed(1),
        static_linear_gauge,
        view_linear_gauge,
    ),
    widget(
        "LampBlock",
        "Lamp",
        "LED lamp indicator (on/off or multi-state)",
        ph(egui_material_icons::icons::ICON_LIGHTBULB.codepoint),
        IOPorts::Fixed(1),
        static_lamp,
        view_lamp,
    ),
    // DashboardScope's live view is an interactive liveplot tile owned by the
    // UI; the static fallback is a simple waveform glyph.
    SimulinkBlockDefinition::new("DashboardScope", "Dashboard")
        .with_display_name("Dashboard Scope")
        .with_description("Plot connected signals on a dashboard scope")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_shape(SimulinkShape::Rectangle)
        .with_icon(ph(egui_material_icons::icons::ICON_SHOW_CHART.codepoint))
        .with_static_renderer(static_dashboard_scope),
];
