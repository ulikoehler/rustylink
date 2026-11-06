//! Message & event blocks: message-based communication between components.
//!
//! Part of the block catalog: one file per Simulink library group.

#![cfg(feature = "egui")]

use super::prelude::icon;
use crate::simulink_libraries::types::{IOPorts, SimulinkBlockDefinition};

#[rustfmt::skip]
pub static BLOCKS: &[SimulinkBlockDefinition] = &[
    // ═══════════════════════════════════════════════════════════════════════
    //  Messages & Events
    // ═══════════════════════════════════════════════════════════════════════
    SimulinkBlockDefinition::new("MessageDisplay", "Messages and Events")
        .with_description("Display message contents")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_icon(icon("msg")),
    SimulinkBlockDefinition::new("MessageInput", "Messages and Events")
        .with_description("External message input port")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(icon("in")),
    SimulinkBlockDefinition::new("MessageMerge", "Messages and Events")
        .with_description("Merge message streams")
        .with_ports(IOPorts::Variable(1), IOPorts::Fixed(1))
        .with_icon(icon("merge")),
    SimulinkBlockDefinition::new("MessageOutput", "Messages and Events")
        .with_description("External message output port")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_icon(icon("out")),
    SimulinkBlockDefinition::new("SequenceViewer", "Messages and Events")
        .with_aliases(&["Sequence Viewer"])
        .with_description("Visualize message sequences")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_icon(icon("seq")),
    SimulinkBlockDefinition::new("EventListener", "Messages and Events")
        .with_description("Listen for a simulation event")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("evt")),
];
