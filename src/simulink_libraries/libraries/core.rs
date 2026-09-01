//! Core Simulink blocks: sources, sinks, ports, routing primitives and the
//! common math blocks that carry custom icons, shapes or renderers.
//!
//! These definitions previously lived as hardcoded entries in `block_types.rs`
//! and as interior-renderer registrations in `egui_app::render`.  They are now
//! data in the single catalog.

#![cfg(feature = "egui")]

use crate::simulink_libraries::labels;
use crate::simulink_libraries::renderers;
use crate::simulink_libraries::types::{
    BlockLabelPolicy, IOPorts, MetadataKey, PortLabelPolicy,
    SimulinkBlockDefinition, SimulinkIcon, SimulinkShape,
};

const fn icon(glyph: &'static str) -> SimulinkIcon {
    SimulinkIcon::Utf8(glyph)
}

pub static BLOCKS: &[SimulinkBlockDefinition] = &[
    // ── Math operations ────────────────────────────────────────────────
    SimulinkBlockDefinition::new("Product", "Math Operations")
        .with_description("Multiply or divide inputs")
        .with_ports(IOPorts::Variable(2), IOPorts::Fixed(1))
        .with_icon(icon("×"))
        .with_metadata_keys(&[
            MetadataKey::with_default("Inputs", "**"),
            MetadataKey::with_default("Multiplication", "Element-wise(.*)"),
        ])
        .with_static_renderer(renderers::static_product),
    SimulinkBlockDefinition::new("Sum", "Math Operations")
        .with_description("Add or subtract inputs")
        .with_ports(IOPorts::Variable(2), IOPorts::Fixed(1))
        .with_shape(SimulinkShape::None)
        .with_metadata_keys(&[
            MetadataKey::with_default("IconShape", "round"),
            MetadataKey::with_default("Inputs", "++"),
        ])
        .with_port_overrides_fn(renderers::sum_port_overrides)
        .with_static_renderer(renderers::static_sum),
    SimulinkBlockDefinition::new("Gain", "Math Operations")
        .with_description("Multiply input by a constant")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_shape(SimulinkShape::Triangle)
        .with_metadata_keys(&[MetadataKey::with_default("Gain", "1")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::gain_value)),
    // ── Sources / sinks ────────────────────────────────────────────────
    SimulinkBlockDefinition::new("Constant", "Sources")
        .with_description("Output a constant value")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("Value", "1")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::constant_value)),
    SimulinkBlockDefinition::new("Scope", "Sinks")
        .with_description("Display signals over time")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_icon(SimulinkIcon::Phosphor(
            egui_phosphor_icons::icons::WAVEFORM.as_str(),
        ))
        .with_static_renderer(renderers::static_scope),
    SimulinkBlockDefinition::new("Terminator", "Sinks")
        .with_description("Terminate an unconnected output port")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_icon(icon("⊣")),
    // ── Ports & subsystems ─────────────────────────────────────────────
    // Simulink writes the port's number inside the obround, not an arrow.
    SimulinkBlockDefinition::new("Inport", "Ports & Subsystems")
        .with_description("Create an input port for a subsystem")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_shape(SimulinkShape::Obround)
        .with_metadata_keys(&[MetadataKey::with_default("Port", "1")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::port_number)),
    // An InportShadow shares the same outer subsystem port as an Inport
    // (identified by its `Port` property) but allows a second block inside
    // the subsystem to read from that port.  Visually identical to Inport.
    SimulinkBlockDefinition::new("InportShadow", "Ports & Subsystems")
        .with_description("Shadow input port sharing an outer subsystem port with an Inport")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_shape(SimulinkShape::Obround)
        .with_metadata_keys(&[MetadataKey::with_default("Port", "1")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::port_number)),
    SimulinkBlockDefinition::new("Outport", "Ports & Subsystems")
        .with_description("Create an output port for a subsystem")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_shape(SimulinkShape::Obround)
        .with_metadata_keys(&[MetadataKey::with_default("Port", "1")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::port_number)),
    // Simulink previews a subsystem by drawing its contents in miniature: an
    // In port per input wired across to an Out port per output.
    SimulinkBlockDefinition::new("SubSystem", "Ports & Subsystems")
        .with_description("Group blocks into a subsystem")
        // Subsystem ports are derived from the contained In/Outport blocks, so
        // the default (used only when the model carries no port info) is 0/0.
        .with_ports(IOPorts::Variable(0), IOPorts::Variable(0))
        .with_static_renderer(renderers::static_subsystem)
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
        ),
    // Simulink captions the block `fcn` (under the MATLAB membrane logo).
    SimulinkBlockDefinition::new("MATLAB Function", "User-Defined Functions")
        .with_description("Author block behaviour in MATLAB")
        .with_ports(IOPorts::Variable(1), IOPorts::Variable(1))
        .with_metadata_keys(&[MetadataKey::new(labels::MATLAB_FUNCTION_NAME_PROPERTY)])
        .with_block_label(BlockLabelPolicy::MetadataDependent(
            labels::matlab_function_name,
        ))
        .with_static_renderer(renderers::static_matlab_function)
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
        ),
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
    SimulinkBlockDefinition::new("ComplexToRealImag", "Math Operations")
        .with_description("Split a complex signal into real and imaginary parts")
        .with_ports(IOPorts::Fixed(1), IOPorts::Variable(2))
        .with_metadata_keys(&[MetadataKey::with_default("Output", "Real and imag")])
        .with_static_renderer(renderers::static_complex_to_real_imag),
    SimulinkBlockDefinition::new("ManualSwitch", "Signal Routing")
        .with_aliases(&["Manual Switch"])
        .with_description("Manually switch between two inputs")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_icon(SimulinkIcon::Phosphor(
            egui_phosphor_icons::icons::ARROWS_MERGE.as_str(),
        ))
        .with_static_renderer(renderers::static_manual_switch)
        .with_live_renderer(renderers::live_manual_switch),
    SimulinkBlockDefinition::new("Goto", "Signal Routing")
        .with_description("Send a signal to a matching From block")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_shape(SimulinkShape::Goto)
        .with_icon(SimulinkIcon::Phosphor(
            egui_phosphor_icons::icons::ARROW_RIGHT.as_str(),
        ))
        .with_metadata_keys(&[MetadataKey::with_default("GotoTag", "A")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::goto_tag))
        .with_static_renderer(renderers::static_goto_from),
    SimulinkBlockDefinition::new("From", "Signal Routing")
        .with_description("Receive a signal from a matching Goto block")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_shape(SimulinkShape::From)
        .with_icon(SimulinkIcon::Phosphor(
            egui_phosphor_icons::icons::ARROW_LEFT.as_str(),
        ))
        .with_metadata_keys(&[MetadataKey::with_default("GotoTag", "A")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::goto_tag))
        .with_static_renderer(renderers::static_goto_from),
];

/// Default port-label policy: take the labels from the parsed model.
///
/// Returning an empty vector tells the general renderer to fall back to its
/// per-port name resolution (port `Name` property, subsystem boundary names,
/// or generated `In1`/`Out1`).
fn port_labels_from_model(
    _block: &crate::model::Block,
    _meta: &crate::simulink_libraries::metadata::BlockMetadata,
    _is_input: bool,
) -> Vec<String> {
    Vec::new()
}
