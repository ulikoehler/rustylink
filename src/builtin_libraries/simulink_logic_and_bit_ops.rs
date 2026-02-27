//! Virtual library metadata for `simulink/Logic and Bit Operations`.
//!
//! Provides block definitions and per-instance label generation for blocks in
//! the Simulink "Logic and Bit Operations" library.

use crate::model::Block;
use crate::model::System;

use super::virtual_library::{self, VirtualBlock};

pub const LIB_NAME: &str = "simulink/Logic and Bit Operations";

pub const BLOCKS: &[VirtualBlock] = &[VirtualBlock {
    name: "Compare To Constant",
    aliases: &["CompareToConstant"],
    ins: 1,
    outs: 1,
    icon: None,
}];

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

pub fn initial_system() -> System {
    virtual_library::initial_system(BLOCKS)
}

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

/// Top-level `compute_instance_label` entry point for this library.
///
/// Dispatches to the appropriate per-block label generator based on the block's
/// `SourceType` property or `library_block_path`.
pub fn compute_instance_label(block: &Block) -> Option<String> {
    // Identify which block within the library this is by inspecting SourceType
    // (the most direct indicator) or by the tail of the library path.
    let source_type = block.properties.get("SourceType").map(|s| s.trim());
    let path_tail = block
        .library_block_path
        .as_deref()
        .or_else(|| block.properties.get("SourceBlock").map(|s| s.as_str()))
        .and_then(|p| p.rsplit('/').next().map(|s| s.replace(['\n', '\r'], "")));
    let path_tail_str = path_tail.as_deref().map(|s| s.trim());

    let is_compare_to_constant = source_type == Some("Compare To Constant")
        || path_tail_str
            .map(|t| t.eq_ignore_ascii_case("Compare To Constant"))
            .unwrap_or(false);

    if is_compare_to_constant {
        return compare_to_constant_label(block);
    }

    None
}
