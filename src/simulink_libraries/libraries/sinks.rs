//! Sink blocks: scopes, displays, terminators and writers.
//!
//! Part of the block catalog: one file per Simulink library group.

#![cfg(feature = "egui")]

use super::prelude::{icon, plot};
use crate::simulink_libraries::renderers;
use crate::simulink_libraries::types::{IOPorts, SimulinkBlockDefinition, SimulinkIcon};

#[rustfmt::skip]
pub static BLOCKS: &[SimulinkBlockDefinition] = &[
    SimulinkBlockDefinition::new("Scope", "Sinks")
        .with_description("Display signals over time")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_icon(SimulinkIcon::Phosphor(
            egui_material_icons::icons::ICON_SHOW_CHART.codepoint,
        ))
        .with_static_renderer(renderers::static_scope),
    SimulinkBlockDefinition::new("Terminator", "Sinks")
        .with_description("Terminate an unconnected output port")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_icon(icon("⊣")),
    // ═══════════════════════════════════════════════════════════════════════
    //  Sinks
    // ═══════════════════════════════════════════════════════════════════════
    // Simulink's XY Graph icon is a scatter of samples along a rising trend.
    SimulinkBlockDefinition::new("Record", "Sinks")
        .with_aliases(&["XY Graph", "To Workspace"])
        .with_description("Record signal data")
        .with_ports(IOPorts::Variable(1), IOPorts::None)
        .with_icon(plot(concat!(
            "r 0.04,0.04 0.96,0.96; p 0.10,0.90 0.90,0.10;",
            "d 0.22,0.72 0.045; d 0.34,0.70 0.045; d 0.44,0.54 0.045;",
            "d 0.58,0.46 0.045; d 0.68,0.30 0.045; d 0.80,0.24 0.045"
        ))),
];
