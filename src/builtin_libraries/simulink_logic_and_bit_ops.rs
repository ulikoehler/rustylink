//! Virtual library metadata for `simulink/Logic and Bit Operations`.
//!
//! Provides block definitions and per-instance label generation for blocks in
//! the Simulink "Logic and Bit Operations" library.

use crate::model::Block;

use super::virtual_library::VirtualBlock;

pub const LIB_NAME: &str = "simulink/Logic and Bit Operations";

pub const BLOCKS: &[VirtualBlock] = &[
    VirtualBlock {
        name: "Compare To Constant",
        aliases: &["CompareToConstant"],
        ins: 1,
        outs: 1,
        compute_instance_label: Some(compare_to_constant_label),
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "Detect Change",
        aliases: &["DetectChange"],
        ins: 1,
        outs: 1,
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "Detect Increase",
        aliases: &["DetectIncrease"],
        ins: 1,
        outs: 1,
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "Detect Decrease",
        aliases: &["DetectDecrease"],
        ins: 1,
        outs: 1,
        ..VirtualBlock::DEFAULT
    },
    VirtualBlock {
        name: "Relational Operator",
        aliases: &["RelationalOperator"],
        ins: 2,
        outs: 1,
        ..VirtualBlock::DEFAULT
    },
];

pub fn get_blocks() -> &'static [VirtualBlock] {
    BLOCKS
}

/// Returns `true` when `name` refers to the "Logic and Bit Operations" library
/// or any block path within it.  Accepts both the abbreviated form
/// (`simulink/Logic and Bit`) that appears in some SLX versions and the full
/// form (`simulink/Logic and Bit Operations`).
pub fn is_simulink_logic_and_bit_ops_name(name: &str) -> bool {
    let norm = name.trim().replace('\\', "/").to_ascii_lowercase();
    norm == "simulink/logic and bit operations"
        || norm.starts_with("simulink/logic and bit operations/")
        || norm == "simulink/logic and bit"
        || norm.starts_with("simulink/logic and bit/")
}

// ── Utility functions used by per-block callbacks ────────────────────────────

/// Translate a Simulink relational-operator string into a Unicode symbol.
///
/// | Simulink `relop` | Symbol |
/// |-----------------|--------|
/// | `<=`            | `≤`    |
/// | `>=`            | `≥`    |
/// | `~=`            | `≠`    |
/// | `==`            | `=`    |
/// | `<`             | `<`    |
/// | `>`             | `>`    |
fn relop_to_symbol(relop: &str) -> &str {
    match relop.trim() {
        "<=" => "≤",
        ">=" => "≥",
        "~=" => "≠",
        "==" => "=",
        "<" => "<",
        ">" => ">",
        other => other,
    }
}

/// Compute the inline label for a "Compare To Constant" block from its
/// `InstanceData`.
///
/// Returns `Some("≤ 3.0")` for a block configured with `relop=<=` and
/// `const=3.0`, and `None` when the required parameters are absent.
fn compare_to_constant_label(block: &Block) -> Option<String> {
    let id = block.instance_data.as_ref()?;
    let relop = id.properties.get("relop")?;
    let const_val = id.properties.get("const")?;
    let sym = relop_to_symbol(relop);
    Some(format!("{} {}", sym, const_val.trim()))
}
