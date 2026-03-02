use super::virtual_library::{BlockShape, VirtualBlock};

pub const LIB_NAME: &str = "simulink/Signal Routing";

pub const BLOCKS: &[VirtualBlock] = &[
    VirtualBlock {
        name: "BusCreator",
        aliases: &[],
        ins: 2,
        outs: 1,
        shape: BlockShape::FilledBlack,
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "BusSelector",
        aliases: &[],
        ins: 1,
        outs: 2,
        shape: BlockShape::FilledBlack,
        ..VirtualBlock::DEFAULT
    },
];

pub fn get_blocks() -> &'static [VirtualBlock] {
    BLOCKS
}

pub fn is_simulink_signal_routing_name(name: &str) -> bool {
    let norm = name.trim().replace('\\', "/").to_ascii_lowercase();
    norm == "simulink/signal routing" || norm.starts_with("simulink/signal routing/")
}
