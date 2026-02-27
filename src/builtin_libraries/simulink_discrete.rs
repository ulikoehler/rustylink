//! Virtual library metadata for `simulink/Discrete`.

use crate::model::System;

use super::virtual_library::{self, VirtualBlock};

pub const LIB_NAME: &str = "simulink/Discrete";

pub const BLOCKS: &[VirtualBlock] = &[
    VirtualBlock {
        name: "Discrete Derivative",
        ins: 1,
        outs: 1,
        icon: Some("discrete/discrete_derivative.svg"),
    },
];

pub fn is_simulink_discrete_name(name: &str) -> bool {
    let norm = name.trim().replace('\\', "/").to_ascii_lowercase();
    norm == "simulink/discrete" || norm.starts_with("simulink/discrete/")
}

pub fn initial_system() -> System {
    virtual_library::initial_system(BLOCKS)
}
