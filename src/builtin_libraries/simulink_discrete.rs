//! Virtual library metadata for `simulink/Discrete`.

use super::virtual_library::VirtualBlock;

pub const LIB_NAME: &str = "simulink/Discrete";

pub const BLOCKS: &[VirtualBlock] = &[
    VirtualBlock {
        name: "Discrete Derivative",
        aliases: &[],
        ins: 1,
        outs: 1,
        icon: Some("discrete/discrete_derivative.svg"),
        compute_instance_label: None,
    },
];

pub fn get_blocks() -> &'static [VirtualBlock] {
    BLOCKS
}

pub fn is_simulink_discrete_name(name: &str) -> bool {
    let norm = name.trim().replace('\\', "/").to_ascii_lowercase();
    norm == "simulink/discrete" || norm.starts_with("simulink/discrete/")
}
