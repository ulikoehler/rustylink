//! Built-in virtual libraries and their metadata.
//!
//! Each virtual library is represented by its own submodule.  The parser and
//! UI code can query these modules when they need to construct stub blocks or
//! otherwise reason about library-specific behavior.

pub mod virtual_library;

pub mod matrix_library;
pub mod simulink_discrete;
pub mod simulink_logic_and_bit_ops;
pub mod simulink_math_operations;

use crate::model::{Block, System};

use virtual_library::VirtualLibrarySpec;

// Re-export user library API so downstream users don't need to import the
// submodule directly.
pub use virtual_library::{
    OwnedVirtualBlock, PortPlacement, PortPositionOverride, UserVirtualLibrarySpec,
    register_virtual_library,
};

/// All built-in virtual libraries with structured metadata.
pub const VIRTUAL_LIBRARIES: &[VirtualLibrarySpec] = &[
    VirtualLibrarySpec {
        name: matrix_library::LIB_NAME,
        matches_name: matrix_library::is_matrix_library_name,
        get_blocks: matrix_library::get_blocks,
    },
    VirtualLibrarySpec {
        name: simulink_discrete::LIB_NAME,
        matches_name: simulink_discrete::is_simulink_discrete_name,
        get_blocks: simulink_discrete::get_blocks,
    },
    VirtualLibrarySpec {
        name: simulink_logic_and_bit_ops::LIB_NAME,
        matches_name: simulink_logic_and_bit_ops::is_simulink_logic_and_bit_ops_name,
        get_blocks: simulink_logic_and_bit_ops::get_blocks,
    },
    VirtualLibrarySpec {
        name: simulink_math_operations::LIB_NAME,
        matches_name: simulink_math_operations::is_simulink_math_operations_name,
        get_blocks: simulink_math_operations::get_blocks,
    },
];

/// Return an initial in-memory system for virtual libraries that carry
/// structured metadata (ports, known blocks, etc.).
///
/// Virtual libraries not listed here still exist (see `parser::is_virtual_library`),
/// but are treated as empty.
pub fn virtual_library_initial_system(lib_name: &str) -> Option<System> {
    for spec in VIRTUAL_LIBRARIES {
        if (spec.matches_name)(lib_name) {
            return Some(virtual_library::initial_system((spec.get_blocks)()));
        }
    }
    // Also search user-registered libraries.
    virtual_library::find_in_user_libraries(|spec| {
        if (spec.matches_name)(lib_name) {
            Some((spec.initial_system)())
        } else {
            None
        }
    })
}

/// Compute a per-instance inline label for a block using virtual-library label
/// functions.
///
/// Returns `Some(label)` if any registered virtual library claims ownership of
/// the block (via `matches_name`) **and** provides a block whose
/// `compute_instance_label` callback produces a non-`None` result for the
/// given block.  Returns `None` when no special label is available, in which
/// case the caller should fall through to the default icon/value rendering.
pub fn compute_block_instance_label(block: &Block) -> Option<String> {
    // Obtain the raw source-block path (prefers library_block_path which is set
    // by the parser from the SourceBlock property).
    let raw = block
        .library_block_path
        .as_deref()
        .or_else(|| block.properties.get("SourceBlock").map(|s| s.as_str()));
    let default_raw = format!("simulink/Math Operations/{}", block.block_type);
    let raw = raw.unwrap_or(&default_raw);

    // Normalize: replace embedded newlines/CRs with spaces (they act as word-wrap
    // separators in SLX XML, e.g. "Compare\nTo Constant" → "Compare To Constant"),
    // then collapse backslashes to slashes.
    let normalized = raw.replace(['\n', '\r'], " ").replace('\\', "/");
    let normalized = normalized.trim().to_string();
    if normalized.is_empty() {
        return None;
    }

    // Extract the block-name portion (last path component).
    let block_name_raw = normalized.rsplit('/').next().unwrap_or(&normalized);

    for spec in VIRTUAL_LIBRARIES {
        if (spec.matches_name)(&normalized) {
            let blocks = (spec.get_blocks)();
            let norm_name = virtual_library::normalize_block_name(block_name_raw);
            let norm_humanized = virtual_library::normalize_block_name(
                &virtual_library::humanize_camel_case(block_name_raw),
            );
            for vb in blocks {
                let vb_norm = virtual_library::normalize_block_name(vb.name);
                let matches = vb_norm == norm_name
                    || vb_norm == norm_humanized
                    || vb.aliases.iter().any(|&a| {
                        let an = virtual_library::normalize_block_name(a);
                        an == norm_name || an == norm_humanized
                    });
                if matches {
                    return vb.compute_instance_label.and_then(|f| f(block));
                }
            }
        }
    }

    // Also search user-registered libraries.
    virtual_library::find_in_user_libraries(|spec| {
        if (spec.matches_name)(&normalized) {
            let norm_name = virtual_library::normalize_block_name(block_name_raw);
            let norm_humanized = virtual_library::normalize_block_name(
                &virtual_library::humanize_camel_case(block_name_raw),
            );
            for vb in &spec.blocks {
                let vb_norm = virtual_library::normalize_block_name(&vb.name);
                let matches = vb_norm == norm_name
                    || vb_norm == norm_humanized
                    || vb.aliases.iter().any(|a| {
                        let an = virtual_library::normalize_block_name(a);
                        an == norm_name || an == norm_humanized
                    });
                if matches {
                    return vv_compute_instance_label(&vb, block);
                }
            }
            None
        } else {
            None
        }
    })
}

/// Helper: call `compute_instance_label` on an [`OwnedVirtualBlock`].
#[inline]
fn vv_compute_instance_label(
    vb: &virtual_library::OwnedVirtualBlock,
    block: &Block,
) -> Option<String> {
    vb.compute_instance_label.as_ref().and_then(|f| f(block))
}

pub use matrix_library::*;
