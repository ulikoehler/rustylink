//! Simulink System XML parser.
//! 
//! This crate provides a `SimulinkParser` to load and parse Simulink XML system
//! descriptions into strongly-typed Rust structures.
//! 
//! The binary `rustylink` demonstrates usage and prints the parsed JSON.

pub mod model;
pub mod parser;
pub mod label_place;

// Optional GUI/egui functionality lives behind the `egui` feature flag.
// This module provides an interactive viewer for Simulink subsystems and
// is used by the example in examples/egui_viewer.rs.
#[cfg(feature = "egui")]
pub mod egui_app;
