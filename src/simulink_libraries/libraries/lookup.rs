//! Lookup table blocks: n-D tables, interpolation, prelookup.
//!
//! Part of the block catalog: one file per Simulink library group.

#![cfg(feature = "egui")]

use super::prelude::{axes, plot};
use crate::simulink_libraries::renderers;
use crate::simulink_libraries::types::{
    IOPorts, MetadataKey, PortLabelPolicy, SimulinkBlockDefinition,
};

#[rustfmt::skip]
pub static BLOCKS: &[SimulinkBlockDefinition] = &[
    // ═══════════════════════════════════════════════════════════════════════
    //  Lookup Tables
    // ═══════════════════════════════════════════════════════════════════════
    // Simulink captions the table with its dimensionality and plots the
    // breakpoint curve underneath.
    SimulinkBlockDefinition::new("Lookup_n-D", "Lookup Tables")
        .with_aliases(&["1-D Lookup Table", "2-D Lookup Table", "n-D Lookup Table"])
        .with_description("n-dimensional lookup table interpolation")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("NumberOfTableDimensions", "1")])
        .with_static_renderer(renderers::static_lookup_table),
    SimulinkBlockDefinition::new("Cosine", "Lookup Tables")
        .with_description("Sine and Cosine lookup-table function")
        .with_ports(IOPorts::Fixed(1), IOPorts::Variable(1))
        .with_metadata_keys(&[MetadataKey::with_default("Formula", "")])
        // Simulink draws no icon for these lookup-table Reference blocks; their
        // identity comes from the output port labels (`Formula`), so claim the
        // interior to keep the `?` placeholder away.
        .with_static_renderer(renderers::static_nothing)
        .with_port_labels(
            PortLabelPolicy::Fixed(&["u"]),
            PortLabelPolicy::MetadataDependent(renderers::sine_cosine_output_labels),
        ),
    SimulinkBlockDefinition::new("Sine", "Lookup Tables")
        .with_description("Sine lookup-table function")
        .with_ports(IOPorts::Fixed(1), IOPorts::Variable(1))
        .with_metadata_keys(&[MetadataKey::with_default("Formula", "")])
        .with_static_renderer(renderers::static_nothing)
        .with_port_labels(
            PortLabelPolicy::Fixed(&["u"]),
            PortLabelPolicy::MetadataDependent(renderers::sine_cosine_output_labels),
        ),
    // Direct n-D lookup without interpolation — index straight into the table.
    SimulinkBlockDefinition::new("DirectLookup", "Lookup Tables")
        .with_aliases(&["Direct Lookup Table (n-D)"])
        .with_description("n-dimensional direct table index")
        .with_ports(IOPorts::Variable(1), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            axes!(),
            "p 0.20,0.80 0.80,0.80; p 0.20,0.62 0.80,0.62;",
            "p 0.20,0.44 0.80,0.44; p 0.20,0.26 0.80,0.26;",
            "p 0.34,0.12 0.34,0.88; p 0.58,0.12 0.58,0.88"
        ))),
    // Interpolation block: linear-interp curve through breakpoints.
    SimulinkBlockDefinition::new("Interpolation", "Lookup Tables")
        .with_aliases(&["Interpolation Using Prelookup"])
        .with_description("n-dimensional interpolation using prelookup")
        .with_ports(IOPorts::Variable(1), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            axes!(),
            "p 0.10,0.85 0.35,0.60 0.55,0.65 0.78,0.30 0.92,0.32;",
            "d 0.35,0.60 0.045; d 0.55,0.65 0.045; d 0.78,0.30 0.045"
        ))),
    SimulinkBlockDefinition::new("Lookup1D", "Lookup Tables")
        .with_aliases(&["1-D Lookup Table"])
        .with_description("1-dimensional lookup table")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_static_renderer(renderers::static_lookup_table),
    SimulinkBlockDefinition::new("Lookup2D", "Lookup Tables")
        .with_aliases(&["2-D Lookup Table"])
        .with_description("2-dimensional lookup table")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_static_renderer(renderers::static_lookup_table),
    SimulinkBlockDefinition::new("LookupDynamic", "Lookup Tables")
        .with_aliases(&["Dynamic Lookup Table"])
        .with_description("Lookup table whose data comes from an input")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            axes!(),
            "p 0.10,0.85 0.35,0.55 0.62,0.62 0.88,0.22;",
            "d 0.35,0.55 0.045; d 0.62,0.62 0.045"
        ))),
    // Prelookup: breakpoint index finder — small axis with a marker.
    SimulinkBlockDefinition::new("PreLookup", "Lookup Tables")
        .with_description("Compute index and fraction for interpolation")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            axes!(),
            "p 0.10,0.85 0.30,0.60 0.50,0.60 0.70,0.35 0.90,0.35;",
            "d 0.50,0.60 0.05"
        ))),
];
