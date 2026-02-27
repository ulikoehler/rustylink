//! Built-in virtual libraries and their metadata.
//!
//! Each virtual library is represented by its own submodule.  The parser and
//! UI code can query these modules when they need to construct stub blocks or
//! otherwise reason about library-specific behavior.

pub mod virtual_library;

pub mod matrix_library;
pub mod simulink_discrete;
pub mod simulink_logic_and_bit_ops;

use crate::model::{Block, System};

use virtual_library::VirtualLibrarySpec;

// Re-export user library API so downstream users don't need to import the
// submodule directly.
pub use virtual_library::{
    OwnedVirtualBlock, UserVirtualLibrarySpec, register_virtual_library,
};

/// All built-in virtual libraries with structured metadata.
pub const VIRTUAL_LIBRARIES: &[VirtualLibrarySpec] = &[
	VirtualLibrarySpec {
		name: matrix_library::LIB_NAME,
		blocks: matrix_library::BLOCKS,
		matches_name: matrix_library::is_matrix_library_name,
		initial_system: matrix_library::initial_system,
		compute_instance_label: None,
	},
	VirtualLibrarySpec {
		name: simulink_discrete::LIB_NAME,
		blocks: simulink_discrete::BLOCKS,
		matches_name: simulink_discrete::is_simulink_discrete_name,
		initial_system: simulink_discrete::initial_system,
		compute_instance_label: None,
	},
	VirtualLibrarySpec {
		name: simulink_logic_and_bit_ops::LIB_NAME,
		blocks: simulink_logic_and_bit_ops::BLOCKS,
		matches_name: simulink_logic_and_bit_ops::is_simulink_logic_and_bit_ops_name,
		initial_system: simulink_logic_and_bit_ops::initial_system,
		compute_instance_label: Some(simulink_logic_and_bit_ops::compute_instance_label),
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
			return Some((spec.initial_system)());
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
/// the block (via `matches_name`) **and** provides a `compute_instance_label`
/// function that produces a non-`None` result for the given block.  Returns
/// `None` when no special label is available, in which case the caller should
/// fall through to the default icon/value rendering.
pub fn compute_block_instance_label(block: &Block) -> Option<String> {
    // Obtain the raw source-block path (prefers library_block_path which is set
    // by the parser from the SourceBlock property).
    let raw = block
        .library_block_path
        .as_deref()
        .or_else(|| block.properties.get("SourceBlock").map(|s| s.as_str()));
    let raw = raw?;

    // Normalize: strip embedded newlines/CRs, collapse multiple slashes.
    let normalized = raw.replace(['\n', '\r'], "").replace('\\', "/");
    let normalized = normalized.trim().to_string();
    if normalized.is_empty() {
        return None;
    }

    for spec in VIRTUAL_LIBRARIES {
        if (spec.matches_name)(&normalized) {
            if let Some(label_fn) = spec.compute_instance_label {
                return label_fn(block);
            }
        }
    }

    // Also search user-registered libraries.
    virtual_library::find_in_user_libraries(|spec| {
        if (spec.matches_name)(&normalized) {
            if let Some(ref label_fn) = spec.compute_instance_label {
                label_fn(block)
            } else {
                None
            }
        } else {
            None
        }
    })
}

pub use matrix_library::*;
