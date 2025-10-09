//! Simulink System XML parser.
//! 
//! This crate provides a `SimulinkParser` to load and parse Simulink XML system
//! descriptions into strongly-typed Rust structures.
//! 
//! The binary `simulink_parser_cli` demonstrates usage and prints the parsed JSON.

pub mod model;
pub mod parser;
