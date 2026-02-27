//! Built-in virtual libraries and their metadata.
//!
//! Each virtual library is represented by its own submodule.  The parser and
//! UI code can query these modules when they need to construct stub blocks or
//! otherwise reason about library-specific behavior.

pub mod virtual_library;

pub mod matrix_library;
pub mod simulink_discrete;

use crate::model::System;

/// Return an initial in-memory system for virtual libraries that carry
/// structured metadata (ports, known blocks, etc.).
///
/// Virtual libraries not listed here still exist (see `parser::is_virtual_library`),
/// but are treated as empty.
pub fn virtual_library_initial_system(lib_name: &str) -> Option<System> {
	if matrix_library::is_matrix_library_name(lib_name) {
		return Some(matrix_library::initial_system());
	}
	if simulink_discrete::is_simulink_discrete_name(lib_name) {
		return Some(simulink_discrete::initial_system());
	}
	None
}

pub use matrix_library::*;
