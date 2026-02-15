//! SLX archive generator – regenerate `.slx` files from the parsed model.
//!
//! This module provides:
//! - [`system_xml`] – Generate system XML text from a [`System`] model.
//! - [`archive`] – Read and write complete SLX ZIP archives with round-trip fidelity.

pub mod archive;
pub mod system_xml;
