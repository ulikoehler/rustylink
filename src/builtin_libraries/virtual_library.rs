//! Shared helpers for built-in virtual libraries.
//!
//! Virtual libraries are in-memory, structured representations of Simulink-like
//! libraries that rustylink can use when the actual `.slx` library file is not
//! present on disk.

use crate::model::{Block, Port, PortCounts, System};

/// Description of a single block type that exists in a virtual library.
#[derive(Debug, Clone, Copy)]
pub struct VirtualBlock {
    /// Canonical name appearing in the library (case preserved).
    pub name: &'static str,
    /// Additional names that may appear in SLX files for the same block.
    ///
    /// This is used to bridge naming differences between Simulink versions,
    /// localized names, or shortened internal identifiers.
    pub aliases: &'static [&'static str],
    /// Number of input ports the block should have when rendered as a stub.
    pub ins: u32,
    /// Number of output ports the block should have when rendered as a stub.
    pub outs: u32,
    /// Optional icon to show for this block in the viewer. Paths are relative
    /// to the `icons/` folder embedded by `egui_app::icon_assets`.
    pub icon: Option<&'static str>,
}

/// Descriptor for a built-in virtual library.
///
/// This allows generic code (e.g. icon registry population, stub creation,
/// etc.) to iterate over all known virtual libraries without hard-coding
/// per-library details.
#[derive(Clone, Copy)]
pub struct VirtualLibrarySpec {
    /// Canonical library name as used in SourceBlock paths (e.g. "matrix_library").
    pub name: &'static str,
    /// All virtual blocks this library exposes.
    pub blocks: &'static [VirtualBlock],
    /// Returns true if the provided library reference belongs to this library.
    pub matches_name: fn(&str) -> bool,
    /// Construct the initial virtual system for this library.
    pub initial_system: fn() -> System,
}

/// Normalize a library block name for matching purposes.
///
/// All whitespace sequences are collapsed to a single ASCII space and the
/// result is lowercased. This keeps `foo   bar` equivalent to `foo bar`.
pub fn normalize_block_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

/// Convert a CamelCase identifier to a human-readable name by inserting spaces
/// before uppercase letters.
///
/// This is intentionally simplistic; it is used for producing alternative keys
/// like `Matrix Multiply` from `MatrixMultiply`.
pub fn humanize_camel_case(name: &str) -> String {
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            let prev = name.chars().nth(i - 1).unwrap();
            if !prev.is_uppercase() {
                out.push(' ');
            }
        }
        out.push(ch);
    }
    out
}

/// Construct a minimal `Block` stub suitable for rendering.
///
/// The returned block has the provided `name` as both `block_type` and `name`
/// and a set of ports matching `ins`/`outs`. Other fields are left as defaults.
pub fn create_stub_block(name: &str, ins: u32, outs: u32) -> Block {
    let mut ports = Vec::new();
    for i in 1..=ins {
        let mut p = Port {
            port_type: "in".to_string(),
            index: Some(i),
            properties: indexmap::IndexMap::new(),
        };
        p.properties.insert("Name".to_string(), String::new());
        ports.push(p);
    }
    for i in 1..=outs {
        let mut p = Port {
            port_type: "out".to_string(),
            index: Some(i),
            properties: indexmap::IndexMap::new(),
        };
        p.properties.insert("Name".to_string(), String::new());
        ports.push(p);
    }

    let port_counts = if ins > 0 || outs > 0 {
        Some(PortCounts {
            ins: Some(ins),
            outs: Some(outs),
        })
    } else {
        None
    };

    let mut child_order = Vec::new();
    if port_counts.is_some() {
        child_order.push(crate::model::BlockChildKind::PortCounts);
    }
    child_order.push(crate::model::BlockChildKind::P("BlockType".to_string()));
    if port_counts.is_some() {
        child_order.push(crate::model::BlockChildKind::PortProperties);
    }

    Block {
        block_type: name.to_string(),
        name: name.to_string(),
        sid: None,
        tag_name: "Block".to_string(),
        position: None,
        zorder: None,
        commented: false,
        name_location: Default::default(),
        is_matlab_function: false,
        value: None,
        value_kind: Default::default(),
        value_rows: None,
        value_cols: None,
        properties: indexmap::IndexMap::new(),
        ref_properties: Default::default(),
        port_counts,
        ports,
        subsystem: None,
        system_ref: None,
        c_function: None,
        instance_data: None,
        link_data: None,
        mask: None,
        annotations: Vec::new(),
        background_color: None,
        show_name: None,
        font_size: None,
        font_weight: None,
        mask_display_text: None,
        current_setting: None,
        block_mirror: None,
        library_source: None,
        library_block_path: None,
        child_order,
    }
}

/// Build the initial `System` for a virtual library from a list of known blocks.
pub fn initial_system(blocks: &[VirtualBlock]) -> System {
    System {
        properties: indexmap::IndexMap::new(),
        blocks: blocks
            .iter()
            .map(|b| create_stub_block(b.name, b.ins, b.outs))
            .collect(),
        lines: Vec::new(),
        annotations: Vec::new(),
        chart: None,
    }
}
