//! Definitions for the built-in "matrix_library" virtual library.
//!
//! Previously the parser contained ad-hoc code to recognize the matrix
//! library and to manufacture stub `Block` structs with guessed port counts.
//!
//! The code here centralizes that knowledge so that other parts of the
//! application (tests, the viewer, etc.) can reason about the library in a
//! structured, data-driven way.  Future virtual libraries can follow the same
//! pattern.

use crate::model::{Block, Port, PortCounts};

/// Description of a single block type that exists in a virtual library.
#[derive(Debug)]
pub struct VirtualBlock {
    /// Canonical name appearing in the library (case preserved).
    pub name: &'static str,
    /// Number of input ports the block should have when rendered as a stub.
    pub ins: u32,
    /// Number of output ports the block should have when rendered as a stub.
    pub outs: u32,
}

/// The initial set of blocks that the matrix library exposes by default.
///
/// The original code had a hard-coded `initial` slice with these names; we
/// keep the same order here for compatibility with existing tests.
pub const BLOCKS: &[VirtualBlock] = &[
    VirtualBlock { name: "IdentityMatrix", ins: 0, outs: 1 },
    VirtualBlock { name: "IsTriangular", ins: 1, outs: 1 },
    VirtualBlock { name: "IsSymmetric", ins: 1, outs: 1 },
    VirtualBlock { name: "CrossProduct", ins: 2, outs: 1 },
    VirtualBlock { name: "MatrixMultiply", ins: 2, outs: 1 },
    VirtualBlock { name: "Submatrix", ins: 1, outs: 1 },
    VirtualBlock { name: "Transpose", ins: 1, outs: 1 },
    VirtualBlock { name: "HermitianTranspose", ins: 1, outs: 1 },
    VirtualBlock { name: "MatrixSquare", ins: 1, outs: 1 },
    VirtualBlock { name: "PermuteColumns", ins: 2, outs: 1 },
    VirtualBlock { name: "ExtractDiagonal", ins: 1, outs: 1 },
    VirtualBlock { name: "CreateDiagonalMatrix", ins: 1, outs: 1 },
    VirtualBlock { name: "ExpandScalar", ins: 1, outs: 1 },
    VirtualBlock { name: "IsHermitian", ins: 1, outs: 1 },
    VirtualBlock { name: "MatrixConcatenate", ins: 2, outs: 1 },
];

/// Determine if the given library name refers to the matrix virtual
/// library.  Mirrors the behaviour previously encoded in the parser.
///
/// Accepts forms like "matrix_library" or "matrix_library/Foo" (prefix
/// match) and is case-insensitive.
pub fn is_matrix_library_name(name: &str) -> bool {
    let norm = name.trim().to_ascii_lowercase();
    norm == "matrix_library" || norm.starts_with("matrix_library/")
}

/// Normalize a library block name for matching purposes.
///
/// All whitespace sequences are collapsed to a single ASCII space and the
/// result is lowercased.  This keeps "foo   bar" equivalent to "foo bar",
/// while distinguishing "foobar" from "foo bar".
fn normalize_name(name: &str) -> String {
    name
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

/// Return the port counts for a block name.
///
/// Matching is case-insensitive.  Rather than removing whitespace entirely, we
/// replace any whitespace run with a single space (see `normalize_name`). If the
/// name is not recognized, `(1, 1)` is returned as a reasonable default.
pub fn port_counts_for(name: &str) -> (u32, u32) {
    let norm = normalize_name(name);
    for b in BLOCKS {
        if normalize_name(b.name) == norm {
            return (b.ins, b.outs);
        }
    }
    (1, 1)
}

/// Construct a minimal `Block` stub suitable for rendering an unknown block
/// from the matrix library.
///
/// The returned block has the proper `block_type`/`name` and a set of ports
/// matching `port_counts_for`.  Other fields are left as defaults.
pub fn create_stub(name: &str) -> Block {
    let (ins, outs) = port_counts_for(name);
    let mut ports = Vec::new();
    for i in 1..=ins {
        let mut p = Port {
            port_type: "in".to_string(),
            index: Some(i),
            properties: indexmap::IndexMap::new(),
        };
        // names may be filled later by the UI code; keep empty string
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

/// Build a `System` pre-populated with the initial set of matrix-library
/// blocks.  This mirrors the earlier `matrix_library_system()` helper.
fn empty_library_system() -> crate::model::System {
    crate::model::System {
        properties: indexmap::IndexMap::new(),
        blocks: Vec::new(),
        lines: Vec::new(),
        annotations: Vec::new(),
        chart: None,
    }
}

/// Build a `System` pre-populated with the initial set of matrix-library
/// blocks.  This mirrors the earlier `matrix_library_system()` helper.
pub fn initial_system() -> crate::model::System {
    let mut sys = empty_library_system();
    for blk in BLOCKS {
        sys.blocks.push(create_stub(blk.name));
    }
    sys
}
