//! Built-in virtual libraries and their metadata.
//!
//! Each virtual library is represented by its own submodule.  The parser and
//! UI code can query these modules when they need to construct stub blocks or
//! otherwise reason about library-specific behavior.

pub mod matrix_library;

pub use matrix_library::*;
