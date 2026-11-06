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
        .with_aliases(&["XY Graph", "To Workspace", "XYGraph", "ToWorkspace"])
        .with_description("Record signal data")
        .with_ports(IOPorts::Variable(1), IOPorts::None)
        .with_icon(plot(concat!(
            "r 0.04,0.04 0.96,0.96; p 0.10,0.90 0.90,0.10;",
            "d 0.22,0.72 0.045; d 0.34,0.70 0.045; d 0.44,0.54 0.045;",
            "d 0.58,0.46 0.045; d 0.68,0.30 0.045; d 0.80,0.24 0.045"
        ))),

    // File writer: arrow-into-disk glyph.
    SimulinkBlockDefinition::new("ToFile", "Sinks")
        .with_aliases(&["To File"])
        .with_description("Write signal data to a MAT file")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_icon(plot(concat!(
            "r 0.16,0.18 0.68,0.64; p 0.10,0.50 0.30,0.50;",
            "p 0.24,0.42 0.30,0.50 0.24,0.58"
        ))),
    // Floating scope: same screen-with-wave as Scope but `ports(0,0)`.
    SimulinkBlockDefinition::new("FloatingScope", "Sinks")
        .with_aliases(&["Floating Scope"])
        .with_description("Scope not directly connected to a signal")
        .with_ports(IOPorts::None, IOPorts::None)
        .with_static_renderer(renderers::static_scope),
    // Stop simulation: octagon with STOP text.
    SimulinkBlockDefinition::new("Stop", "Sinks")
        .with_aliases(&["Stop Simulation"])
        .with_description("Stop the simulation when input is nonzero")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_icon(icon("STOP")),

    SimulinkBlockDefinition::new("TriggeredToWorkspace", "Sinks")
        .with_description("Workspace sink gated by a trigger")
        .with_ports(IOPorts::Fixed(2), IOPorts::None)
        .with_icon(icon("ws\u{2192}")),
    SimulinkBlockDefinition::new("TriggeredWriteToFile", "Sinks")
        .with_description("File sink gated by a trigger")
        .with_ports(IOPorts::Fixed(2), IOPorts::None)
        .with_icon(icon("file\u{2192}")),
];
