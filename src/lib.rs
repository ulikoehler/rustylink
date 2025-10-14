/// Simulink System XML parser.
///
/// This crate provides a `SimulinkParser` to load and parse Simulink XML system
/// descriptions into strongly-typed Rust structures.
///
/// The binary `rustylink` demonstrates usage and prints the parsed JSON.

pub mod color;
//! Simulink System XML parser.
//! 
//! This crate provides a `SimulinkParser` to load and parse Simulink XML system
//! descriptions into strongly-typed Rust structures.
//! 
//! The binary `rustylink` demonstrates usage and prints the parsed JSON.

pub mod model;
pub mod parser;
pub mod label_place;

// Optional mask evaluation feature
pub mod mask_eval;

// Optional GUI/egui functionality lives behind the `egui` feature flag.
// This module provides an interactive viewer for Simulink subsystems and
// is used by the example in examples/egui_viewer.rs.
#[cfg(feature = "egui")]
pub mod egui_app;

// Block type registry and configuration (egui feature)
#[cfg(feature = "egui")]
pub mod block_types;

// Re-export core API so downstream users can easily access/modify the registry
#[cfg(feature = "egui")]
pub use block_types::{
	BlockTypeConfig,
	IconSpec,
	Rgb,
	get_block_type_config_map,
	set_block_type_config,
	update_block_type_config,
};
