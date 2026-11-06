//! Dashboard-category blocks that are not one of the bridged dashboard widgets.
//!
//! Part of the block catalog: one file per Simulink library group.  Kept apart
//! from [`super::dashboard`], which holds exactly the widgets the dashboard
//! runtime knows how to bind and drive.

#![cfg(feature = "egui")]

use crate::simulink_libraries::types::{BlockLabelPolicy, IOPorts, SimulinkBlockDefinition};

pub static BLOCKS: &[SimulinkBlockDefinition] =
    &[
        SimulinkBlockDefinition::new("CustomCallbackButton", "Dashboard")
            .with_aliases(&["Callback Button", "CallbackButton", "CallbackButtonCustom"])
            .with_description("Dashboard callback button")
            .with_ports(IOPorts::None, IOPorts::None)
            .with_block_label(BlockLabelPolicy::Fixed("Button")),
    ];
