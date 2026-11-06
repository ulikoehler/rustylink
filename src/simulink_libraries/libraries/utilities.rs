//! Model-wide utilities: documentation, info, and model-level config blocks.
//!
//! Part of the block catalog: one file per Simulink library group.

#![cfg(feature = "egui")]

use super::prelude::icon;
use crate::simulink_libraries::types::{IOPorts, SimulinkBlockDefinition};

#[rustfmt::skip]
pub static BLOCKS: &[SimulinkBlockDefinition] = &[
    // ═══════════════════════════════════════════════════════════════════════
    //  Model-Wide Utilities
    // ═══════════════════════════════════════════════════════════════════════
    SimulinkBlockDefinition::new("DocBlock", "Model-Wide Utilities")
        .with_aliases(&["Doc Text"])
        .with_description("Model documentation block")
        .with_ports(IOPorts::None, IOPorts::None)
        .with_icon(icon("doc")),
    SimulinkBlockDefinition::new("ModelInfo", "Model-Wide Utilities")
        .with_aliases(&["Model Info"])
        .with_description("Display model revision info")
        .with_ports(IOPorts::None, IOPorts::None)
        .with_icon(icon("info")),
    SimulinkBlockDefinition::new("BlockSupportTable", "Model-Wide Utilities")
        .with_description("Show which blocks a block supports")
        .with_ports(IOPorts::None, IOPorts::None)
        .with_icon(icon("table")),
    SimulinkBlockDefinition::new("SimulinkConfig", "Model-Wide Utilities")
        .with_description("Model-level Simulink configuration")
        .with_ports(IOPorts::None, IOPorts::None)
        .with_icon(icon("cfg")),
    SimulinkBlockDefinition::new("EnvironmentController", "Model-Wide Utilities")
        .with_description("Route signal between model and environment")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_icon(icon("env")),
];
