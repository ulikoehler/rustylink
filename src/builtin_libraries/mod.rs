//! Built-in virtual libraries and their metadata.
//!
//! Each virtual library is represented by its own submodule.  The parser and
//! UI code can query these modules when they need to construct stub blocks or
//! otherwise reason about library-specific behavior.

pub mod virtual_library;

pub mod matrix_library;
pub mod simulink_discrete;

use crate::model::System;

use virtual_library::VirtualLibrarySpec;

/// All built-in virtual libraries with structured metadata.
pub const VIRTUAL_LIBRARIES: &[VirtualLibrarySpec] = &[
	VirtualLibrarySpec {
		name: matrix_library::LIB_NAME,
		blocks: matrix_library::BLOCKS,
		matches_name: matrix_library::is_matrix_library_name,
		initial_system: matrix_library::initial_system,
	},
	VirtualLibrarySpec {
		name: simulink_discrete::LIB_NAME,
		blocks: simulink_discrete::BLOCKS,
		matches_name: simulink_discrete::is_simulink_discrete_name,
		initial_system: simulink_discrete::initial_system,
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
	None
}

pub use matrix_library::*;
