use super::virtual_library::{BlockShape, PortPlacement, PortPositionOverride, VirtualBlock};
use crate::model::Block;

pub const LIB_NAME: &str = "simulink/Math Operations";

const SUM_PORT_OVERRIDES: &[PortPositionOverride] = &[PortPositionOverride {
    is_input: true,
    port_index: 2,
    placement: PortPlacement::Bottom,
    fraction: 0.5,
}];

pub const BLOCKS: &[VirtualBlock] = &[
    VirtualBlock {
        name: "Gain",
        aliases: &[],
        ins: 1,
        outs: 1,
        shape: BlockShape::Triangle,
        compute_instance_label: Some(gain_label),
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "Sum",
        aliases: &[],
        ins: 2,
        outs: 1,
        shape: BlockShape::Circle,
        port_position_overrides: SUM_PORT_OVERRIDES,
        ..VirtualBlock::DEFAULT
    },
];

pub fn get_blocks() -> &'static [VirtualBlock] {
    BLOCKS
}

pub fn is_simulink_math_operations_name(name: &str) -> bool {
    let norm = name.trim().replace('\\', "/").to_ascii_lowercase();
    norm == "simulink/math operations" || norm.starts_with("simulink/math operations/")
}

fn gain_label(block: &Block) -> Option<String> {
    block.properties.get("Gain").map(|s| s.trim().to_string())
}
