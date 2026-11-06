//! Port and subsystem blocks: In/Outports, subsystems, variants, lifecycle ports.
//!
//! Part of the block catalog: one file per Simulink library group.

#![cfg(feature = "egui")]

use super::prelude::{icon, port_labels_from_model};
use crate::simulink_libraries::labels;
use crate::simulink_libraries::renderers;
use crate::simulink_libraries::types::{
    BlockLabelPolicy, IOPorts, MetadataKey, PortLabelPolicy, SimulinkBlockDefinition, SimulinkShape,
};

#[rustfmt::skip]
pub static BLOCKS: &[SimulinkBlockDefinition] = &[
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
    // A link into a library the catalog does not describe.  Its ports and their
    // names come from the linked block's own subsystem, exactly like a subsystem
    // stored in the model; the interior names the library it links to.
    SimulinkBlockDefinition::new("Reference", "Ports & Subsystems")
        .with_description("Link to a block in a library")
        .with_ports(IOPorts::Variable(0), IOPorts::Variable(0))
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
        ),
    // ═══════════════════════════════════════════════════════════════════════
    //  Ports & Subsystems
    // ═══════════════════════════════════════════════════════════════════════
    SimulinkBlockDefinition::new("EnablePort", "Ports & Subsystems")
        .with_aliases(&["Enable"])
        .with_description("Add enable port to subsystem")
        .with_ports(IOPorts::None, IOPorts::None)
        .with_static_renderer(renderers::static_enable_port),
    SimulinkBlockDefinition::new("ForIterator", "Ports & Subsystems")
        .with_aliases(&["For Iterator"])
        .with_description("Repeat subsystem execution a specified number of times")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(icon("for")),
    SimulinkBlockDefinition::new("ForEach", "Ports & Subsystems")
        .with_aliases(&["For Each"])
        .with_description("Partition input and apply subsystem to each element")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("\u{2200}")),
    SimulinkBlockDefinition::new("TriggerPort", "Ports & Subsystems")
        .with_aliases(&["Trigger"])
        .with_description("Add trigger port to subsystem")
        .with_ports(IOPorts::None, IOPorts::None)
        .with_metadata_keys(&[MetadataKey::with_default("TriggerType", "rising")])
        .with_static_renderer(renderers::static_trigger_port),
    SimulinkBlockDefinition::new("ResetPort", "Ports & Subsystems")
        .with_aliases(&["Reset"])
        .with_description("Add reset port to subsystem")
        .with_ports(IOPorts::None, IOPorts::None)
        .with_metadata_keys(&[MetadataKey::with_default("ResetTriggerType", "rising")])
        .with_static_renderer(renderers::static_reset_port),
    SimulinkBlockDefinition::new("PMIOPort", "Ports & Subsystems")
        .with_aliases(&["Connection Port", "Simscape Port"])
        .with_description("Physical modeling connection port")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_shape(SimulinkShape::Obround)
        .with_metadata_keys(&[MetadataKey::with_default("Port", "1")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::port_number)),
    // ═══════════════════════════════════════════════════════════════════════
    //  Timing & Scheduling / Advanced
    // ═══════════════════════════════════════════════════════════════════════
    SimulinkBlockDefinition::new("EventListener", "Ports & Subsystems")
        .with_aliases(&["Event Listener"])
        .with_description("Listen for simulation events")
        .with_ports(IOPorts::None, IOPorts::None)
        .with_metadata_keys(&[
            MetadataKey::with_default("EventType", "Initialize"),
            MetadataKey::with_default("EventName", ""),
        ])
        .with_static_renderer(renderers::static_event_listener),
    SimulinkBlockDefinition::new("StateReader", "Ports & Subsystems")
        .with_aliases(&["State Reader"])
        .with_description("Read block state for logging or initialisation")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_shape(SimulinkShape::None)
        .with_metadata_keys(&[MetadataKey::new("StateOwnerBlock")])
        .with_static_renderer(renderers::static_state_parameter_access),
    SimulinkBlockDefinition::new("StateWriter", "Ports & Subsystems")
        .with_aliases(&["State Writer"])
        .with_description("Write values into block state")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_shape(SimulinkShape::None)
        .with_metadata_keys(&[MetadataKey::new("StateOwnerBlock")])
        .with_static_renderer(renderers::static_state_parameter_access),
    SimulinkBlockDefinition::new("ParameterWriter", "Ports & Subsystems")
        .with_aliases(&["Parameter Writer"])
        .with_description("Write values into another block's parameters")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_shape(SimulinkShape::None)
        .with_metadata_keys(&[
            MetadataKey::new("ParameterOwnerBlock"),
            MetadataKey::new("ParameterName"),
        ])
        .with_static_renderer(renderers::static_state_parameter_access),
    // ═══════════════════════════════════════════════════════════════════════
    //  Variant routing blocks
    // ═══════════════════════════════════════════════════════════════════════
    SimulinkBlockDefinition::new("VariantStart", "Ports & Subsystems")
        .with_aliases(&["Variant Start"])
        .with_description("Variant start — routes input to one of N variant outputs")
        .with_ports(IOPorts::Fixed(1), IOPorts::Variable(0))
        .with_shape(SimulinkShape::TrapezoidRight)
        .with_static_renderer(renderers::static_variant_connector)
        .with_live_renderer(renderers::live_variant_connector),
    SimulinkBlockDefinition::new("VariantEnd", "Ports & Subsystems")
        .with_aliases(&["Variant End"])
        .with_description("Variant end — routes one of N variant inputs to output")
        .with_ports(IOPorts::Variable(0), IOPorts::Fixed(1))
        .with_shape(SimulinkShape::TrapezoidLeft)
        .with_static_renderer(renderers::static_variant_connector)
        .with_live_renderer(renderers::live_variant_connector),
    SimulinkBlockDefinition::new("VariantSink", "Ports & Subsystems")
        .with_aliases(&["Variant Sink"])
        .with_description("Variant sink — routes input to one of N variant outputs")
        .with_ports(IOPorts::Fixed(1), IOPorts::Variable(0))
        .with_shape(SimulinkShape::TrapezoidStemRight)
        .with_static_renderer(renderers::static_variant_connector)
        .with_live_renderer(renderers::live_variant_connector),
    SimulinkBlockDefinition::new("VariantSource", "Ports & Subsystems")
        .with_aliases(&["Variant Source"])
        .with_description("Variant source — routes one of N variant inputs to output")
        .with_ports(IOPorts::Variable(0), IOPorts::Fixed(1))
        .with_shape(SimulinkShape::TrapezoidStemLeft)
        .with_static_renderer(renderers::static_variant_connector)
        .with_live_renderer(renderers::live_variant_connector),
    SimulinkBlockDefinition::new("Else", "Ports & Subsystems")
        .with_description("Else action for an If block")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("else")),
    SimulinkBlockDefinition::new("ElseIf", "Ports & Subsystems")
        .with_description("Else-if action for an If block")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("elseif")),
    SimulinkBlockDefinition::new("ForIteratorSubsystem", "Ports & Subsystems")
        .with_aliases(&["For Iterator Subsystem"])
        .with_description("Subsystem iterating a fixed count")
        .with_ports(IOPorts::Variable(1), IOPorts::Variable(1))
        .with_icon(icon("for")),
    SimulinkBlockDefinition::new("FunctionCallFeedbackLatch", "Ports & Subsystems")
        .with_description("Break feedback loops in function-call chains")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("fcl")),
    SimulinkBlockDefinition::new("FunctionCallGenerator", "Ports & Subsystems")
        .with_description("Generate function-call events")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(icon("fc")),
    SimulinkBlockDefinition::new("FunctionCallSplit", "Ports & Subsystems")
        .with_description("Split a function-call signal")
        .with_ports(IOPorts::Fixed(1), IOPorts::Variable(1))
        .with_icon(icon("fc|")),
    // ── subsystem variants: same box + port model as SubSystem, with a
    //    banner label naming the variant ──
    SimulinkBlockDefinition::new("EnabledSubsystem", "Ports & Subsystems")
        .with_aliases(&["Enabled Subsystem"])
        .with_description("Subsystem enabled by a control input")
        .with_ports(IOPorts::Variable(0), IOPorts::Variable(0))
        .with_static_renderer(renderers::static_subsystem)
        .with_block_label(crate::simulink_libraries::types::BlockLabelPolicy::Fixed("Enabled"))
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
        ),
    SimulinkBlockDefinition::new("TriggeredSubsystem", "Ports & Subsystems")
        .with_aliases(&["Triggered Subsystem"])
        .with_description("Subsystem run on a trigger edge")
        .with_ports(IOPorts::Variable(0), IOPorts::Variable(0))
        .with_static_renderer(renderers::static_subsystem)
        .with_block_label(crate::simulink_libraries::types::BlockLabelPolicy::Fixed("Triggered"))
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
        ),
    SimulinkBlockDefinition::new("IfActionSubsystem", "Ports & Subsystems")
        .with_aliases(&["If Action Subsystem"])
        .with_description("Subsystem run for an if-action")
        .with_ports(IOPorts::Variable(0), IOPorts::Variable(0))
        .with_static_renderer(renderers::static_subsystem)
        .with_block_label(crate::simulink_libraries::types::BlockLabelPolicy::Fixed("If Action"))
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
        ),
    SimulinkBlockDefinition::new("EnabledAndTriggeredSubsystem", "Ports & Subsystems")
        .with_description("Subsystem run when enabled on a trigger")
        .with_ports(IOPorts::Variable(0), IOPorts::Variable(0))
        .with_static_renderer(renderers::static_subsystem)
        .with_block_label(crate::simulink_libraries::types::BlockLabelPolicy::Fixed("Enabled+Trig"))
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
        ),
    SimulinkBlockDefinition::new("ForEachSubsystem", "Ports & Subsystems")
        .with_aliases(&["For Each Subsystem"])
        .with_description("Subsystem iterating each input element")
        .with_ports(IOPorts::Variable(0), IOPorts::Variable(0))
        .with_static_renderer(renderers::static_subsystem)
        .with_block_label(crate::simulink_libraries::types::BlockLabelPolicy::Fixed("For Each"))
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
        ),
    SimulinkBlockDefinition::new("WhileSubsystem", "Ports & Subsystems")
        .with_aliases(&["While Iterator Subsystem"])
        .with_description("Subsystem iterating while a condition holds")
        .with_ports(IOPorts::Variable(0), IOPorts::Variable(0))
        .with_static_renderer(renderers::static_subsystem)
        .with_block_label(crate::simulink_libraries::types::BlockLabelPolicy::Fixed("While"))
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
        ),
    SimulinkBlockDefinition::new("ResettableSubsystem", "Ports & Subsystems")
        .with_aliases(&["Resettable Subsystem"])
        .with_description("Subsystem resettable by a control input")
        .with_ports(IOPorts::Variable(0), IOPorts::Variable(0))
        .with_static_renderer(renderers::static_subsystem)
        .with_block_label(crate::simulink_libraries::types::BlockLabelPolicy::Fixed("Resettable"))
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
        ),
    SimulinkBlockDefinition::new("SwitchCaseActionSubsystem", "Ports & Subsystems")
        .with_description("Subsystem run for a switch-case action")
        .with_ports(IOPorts::Variable(0), IOPorts::Variable(0))
        .with_static_renderer(renderers::static_subsystem)
        .with_block_label(crate::simulink_libraries::types::BlockLabelPolicy::Fixed("Switch Case"))
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
        ),
    SimulinkBlockDefinition::new("FunctionCallSubsystem", "Ports & Subsystems")
        .with_aliases(&["Function-Call Subsystem"])
        .with_description("Subsystem run by a function call")
        .with_ports(IOPorts::Variable(0), IOPorts::Variable(0))
        .with_static_renderer(renderers::static_subsystem)
        .with_block_label(crate::simulink_libraries::types::BlockLabelPolicy::Fixed("f()"))
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
        ),
    SimulinkBlockDefinition::new("AtomicSubSystem", "Ports & Subsystems")
        .with_aliases(&["Atomic Subsystem"])
        .with_description("Non-virtual (atomic) subsystem")
        .with_ports(IOPorts::Variable(0), IOPorts::Variable(0))
        .with_static_renderer(renderers::static_subsystem)
        .with_block_label(crate::simulink_libraries::types::BlockLabelPolicy::Fixed("Atomic"))
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
        ),
    SimulinkBlockDefinition::new("MaskedSubSystem", "Ports & Subsystems")
        .with_aliases(&["Masked Subsystem"])
        .with_description("Subsystem with a parameter mask")
        .with_ports(IOPorts::Variable(0), IOPorts::Variable(0))
        .with_static_renderer(renderers::static_subsystem)
        .with_block_label(crate::simulink_libraries::types::BlockLabelPolicy::Fixed("Masked"))
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
        ),
    SimulinkBlockDefinition::new("VariantSubSystem", "Ports & Subsystems")
        .with_aliases(&["Variant Subsystem"])
        .with_description("Subsystem with variant choices")
        .with_ports(IOPorts::Variable(0), IOPorts::Variable(0))
        .with_static_renderer(renderers::static_subsystem)
        .with_block_label(crate::simulink_libraries::types::BlockLabelPolicy::Fixed("Variant"))
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
        ),
    SimulinkBlockDefinition::new("ConfigSubSystem", "Ports & Subsystems")
        .with_aliases(&["Configurable Subsystem"])
        .with_description("Subsystem with a configurable implementation")
        .with_ports(IOPorts::Variable(0), IOPorts::Variable(0))
        .with_static_renderer(renderers::static_subsystem)
        .with_block_label(crate::simulink_libraries::types::BlockLabelPolicy::Fixed("Config"))
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
        ),
];
