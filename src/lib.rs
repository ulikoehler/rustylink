pub mod block;
/// Simulink System XML parser.
///
/// This crate provides a `SimulinkParser` to load and parse Simulink XML system
/// descriptions into strongly-typed Rust structures.
///
/// The binary `rustylink` demonstrates usage and prints the parsed JSON.
pub mod color;
pub mod connection_targets;
pub mod label_place;
pub mod live_values;
pub mod model;
pub mod parser;

/// SLX archive generator – regenerates `.slx` files from the parsed model.
pub mod generator;

// Unified Simulink block-definition catalog. The rich rendering definitions
// require the `egui` feature, but the lightweight, parser-facing stub metadata
// (`simulink_libraries::stubs`) is always available.
pub mod simulink_libraries;

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

// Comprehensive model editor (egui feature)
#[cfg(feature = "egui")]
pub mod editor;

// Re-export core API so downstream users can easily access/modify the registry
#[cfg(feature = "egui")]
pub use block_types::{
    BlockTypeConfig, IconSpec, Rgb, get_block_type_config_map, set_block_type_config,
    update_block_type_config,
};

// Re-export the catalog's runtime block-registration API for downstream users
// who prefer to add a definition in code rather than as a catalog file.
#[cfg(feature = "egui")]
pub use simulink_libraries::register_user_definition;

// Re-export the variant configuration API so embedding programs can select
// the active sim/codegen variant before building a resolver.
pub use connection_targets::{SimCodegenMode, set_sim_codegen_mode};
