//! Signal routing blocks: mux/demux, buses, switches, Goto/From.
//!
//! Part of the block catalog: one file per Simulink library group.

#![cfg(feature = "egui")]

use super::prelude::icon;
use crate::simulink_libraries::labels;
use crate::simulink_libraries::renderers;
use crate::simulink_libraries::types::{
    BlockLabelPolicy, IOPorts, MetadataKey, PortLabelPolicy, SimulinkBlockDefinition, SimulinkIcon,
    SimulinkShape,
};

#[rustfmt::skip]
pub static BLOCKS: &[SimulinkBlockDefinition] = &[
    // ── Signal routing ─────────────────────────────────────────────────
    // Vector/matrix concatenation stacks the inputs inside a plain rectangle;
    // multidimensional-array mode draws joined cuboids instead.
    SimulinkBlockDefinition::new("Concatenate", "Signal Routing")
        .with_aliases(&["Vector Concatenate"])
        .with_description("Concatenate input signals")
        .with_ports(IOPorts::Variable(2), IOPorts::Fixed(1))
        .with_metadata_keys(&[
            MetadataKey::with_default("Mode", "Vector"),
            MetadataKey::with_default("ConcatenateDimension", "2"),
        ])
        .with_static_renderer(renderers::static_concatenate),
    // Simulink draws all four of these as a solid bar the height of the block.
    SimulinkBlockDefinition::new("Mux", "Signal Routing")
        .with_description("Combine signals into a vector")
        .with_ports(IOPorts::Variable(2), IOPorts::Fixed(1))
        .with_shape(SimulinkShape::FilledBlack),
    SimulinkBlockDefinition::new("Demux", "Signal Routing")
        .with_description("Split a vector into signals")
        .with_ports(IOPorts::Fixed(1), IOPorts::Variable(2))
        .with_shape(SimulinkShape::FilledBlack),
    SimulinkBlockDefinition::new("BusCreator", "Signal Routing")
        .with_description("Combine signals into a bus")
        .with_ports(IOPorts::Variable(2), IOPorts::Fixed(1))
        .with_shape(SimulinkShape::FilledBlack),
    SimulinkBlockDefinition::new("BusSelector", "Signal Routing")
        .with_description("Select signals from a bus")
        .with_ports(IOPorts::Fixed(1), IOPorts::Variable(2))
        .with_shape(SimulinkShape::FilledBlack),
    SimulinkBlockDefinition::new("ManualSwitch", "Signal Routing")
        .with_aliases(&["Manual Switch"])
        .with_description("Manually switch between two inputs")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_icon(SimulinkIcon::Phosphor(
            egui_material_icons::icons::ICON_CALL_MERGE.codepoint,
        ))
        .with_static_renderer(renderers::static_manual_switch)
        .with_live_renderer(renderers::live_manual_switch),
    SimulinkBlockDefinition::new("Goto", "Signal Routing")
        .with_description("Send a signal to a matching From block")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_shape(SimulinkShape::Goto)
        .with_icon(SimulinkIcon::Phosphor(
            egui_material_icons::icons::ICON_ARROW_FORWARD.codepoint,
        ))
        .with_metadata_keys(&[MetadataKey::with_default("GotoTag", "A")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::goto_tag))
        .with_static_renderer(renderers::static_goto_from),
    SimulinkBlockDefinition::new("From", "Signal Routing")
        .with_description("Receive a signal from a matching Goto block")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_shape(SimulinkShape::From)
        .with_icon(SimulinkIcon::Phosphor(
            egui_material_icons::icons::ICON_ARROW_BACK.codepoint,
        ))
        .with_metadata_keys(&[MetadataKey::with_default("GotoTag", "A")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::goto_tag))
        .with_static_renderer(renderers::static_goto_from),
    SimulinkBlockDefinition::new("SignalConversion", "Signal Routing")
        .with_aliases(&["Signal Conversion"])
        .with_description("Convert between signal types")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("ConversionOutput", "Signal copy")])
        .with_static_renderer(renderers::static_signal_conversion),
    // ═══════════════════════════════════════════════════════════════════════
    //  Signal Routing
    // ═══════════════════════════════════════════════════════════════════════
    // BusAssignment has no icon; input port labels come from AssignedSignals.
    // The first input port is always labeled "Bus", ports 2..N are the
    // assigned signal names.
    SimulinkBlockDefinition::new("BusAssignment", "Signal Routing")
        .with_aliases(&["Bus Assignment"])
        .with_description("Assign signals to a bus")
        .with_ports(IOPorts::Variable(2), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("AssignedSignals", "")])
        .with_static_renderer(renderers::static_nothing)
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(renderers::bus_assignment_port_labels),
            PortLabelPolicy::MetadataDependent(renderers::bus_assignment_port_labels),
        ),
    SimulinkBlockDefinition::new("GotoTagVisibility", "Signal Routing")
        .with_aliases(&["Goto Tag Visibility"])
        .with_description("Define scope of Goto tag visibility")
        .with_ports(IOPorts::None, IOPorts::None)
        .with_metadata_keys(&[MetadataKey::with_default("GotoTag", "A")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::goto_tag_braced)),
    SimulinkBlockDefinition::new("Merge", "Signal Routing")
        .with_description("Merge multiple signals into single output")
        .with_ports(IOPorts::Variable(2), IOPorts::Fixed(1))
        .with_block_label(BlockLabelPolicy::Fixed("merge")),
    // Both switches are drawn as a schematic: fixed contacts on the left and a
    // lever swinging to the selected one.
    SimulinkBlockDefinition::new("MultiPortSwitch", "Signal Routing")
        .with_aliases(&["Multiport Switch"])
        .with_description("Select one of N inputs based on control signal")
        .with_ports(IOPorts::Variable(4), IOPorts::Fixed(1))
        .with_metadata_keys(&[
            MetadataKey::with_default("Inputs", ""),
            MetadataKey::with_default("DataPortOrder", "One-based contiguous"),
            MetadataKey::new("DataPortIndices"),
            MetadataKey::new("DataPortForDefault"),
        ])
        .with_static_renderer(renderers::static_multiport_switch)
        .with_live_renderer(renderers::live_multiport_switch)
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(renderers::multiport_switch_port_labels),
            PortLabelPolicy::None,
        ),
    SimulinkBlockDefinition::new("Selector", "Signal Routing")
        .with_description("Select input elements from a vector/matrix")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[
            MetadataKey::with_default("NumberOfDimensions", "1"),
            MetadataKey::with_default("InputPortWidth", "3"),
            MetadataKey::with_default("Indices", "[1]"),
        ])
        .with_static_renderer(renderers::static_selector),
    SimulinkBlockDefinition::new("Switch", "Signal Routing")
        .with_description("Switch between two inputs based on threshold")
        .with_ports(IOPorts::Fixed(3), IOPorts::Fixed(1))
        .with_metadata_keys(&[
            MetadataKey::with_default("Criteria", "u2 >= Threshold"),
            MetadataKey::with_default("Threshold", "0"),
        ])
        .with_static_renderer(renderers::static_switch)
        .with_live_renderer(renderers::live_switch),
    SimulinkBlockDefinition::new("DataStoreRead", "Signal Routing")
        .with_aliases(&["Data Store Read"])
        .with_description("Read from a data store")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("DataStoreName", "A")])
        .with_static_renderer(renderers::static_data_store_access),
    SimulinkBlockDefinition::new("DataStoreWrite", "Signal Routing")
        .with_aliases(&["Data Store Write"])
        .with_description("Write to a data store")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_metadata_keys(&[MetadataKey::with_default("DataStoreName", "A")])
        .with_static_renderer(renderers::static_data_store_access),

    SimulinkBlockDefinition::new("BusElement", "Signal Routing")
        .with_description("Bus element port connector")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("bus")),
    SimulinkBlockDefinition::new("BusElementPort", "Signal Routing")
        .with_aliases(&["Bus Element Port"])
        .with_description("Bus element in/out port")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("bus io")),
];