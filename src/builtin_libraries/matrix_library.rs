//! Definitions for the built-in "matrix_library" virtual library.
//!
//! Previously the parser contained ad-hoc code to recognize the matrix
//! library and to manufacture stub `Block` structs with guessed port counts.
//!
//! The code here centralizes that knowledge so that other parts of the
//! application (tests, the viewer, etc.) can reason about the library in a
//! structured, data-driven way.  Future virtual libraries can follow the same
//! pattern.

use crate::model::{Block, System};

use super::virtual_library;
use super::virtual_library::VirtualBlock;

/// Canonical name of the matrix virtual library.
pub const LIB_NAME: &str = "matrix_library";

// Note: `VirtualBlock` is provided by `builtin_libraries::virtual_library`.

/// The initial set of blocks that the matrix library exposes by default.
///
/// The original code had a hard-coded `initial` slice with these names; we
/// keep the same order here for compatibility with existing tests.
pub const BLOCKS: &[VirtualBlock] = &[
    VirtualBlock {
        name: "Identity Matrix",
        ins: 0,
        outs: 1,
        icon: Some("matrix/identity_matrix.svg"),
        // SLX files store the block type as CamelCase without spaces.
        aliases: &["IdentityMatrix"],
        compute_instance_label: None,
    },
    VirtualBlock {
        name: "Is Triangular",
        ins: 1,
        outs: 1,
        icon: Some("matrix/is_triangular.svg"),
        aliases: &["IsTriangular"],
        compute_instance_label: None,
    },
    VirtualBlock {
        name: "Is Symmetric",
        ins: 1,
        outs: 1,
        icon: Some("matrix/is_symmetric.svg"),
        aliases: &["IsSymmetric"],
        compute_instance_label: None,
    },
    VirtualBlock {
        name: "Cross Product",
        ins: 2,
        outs: 1,
        icon: Some("matrix/cross_product.svg"),
        aliases: &[],
        compute_instance_label: None,
    },
    VirtualBlock {
        name: "Matrix Multiply",
        ins: 2,
        outs: 1,
        icon: Some("matrix/matrix_product.svg"),
        aliases: &[],
        compute_instance_label: None,
    },
    VirtualBlock {
        name: "Submatrix",
        ins: 1,
        outs: 1,
        icon: Some("matrix/submatrix.svg"),
        aliases: &[],
        compute_instance_label: None,
    },
    VirtualBlock {
        name: "Transpose",
        ins: 1,
        outs: 1,
        icon: Some("matrix/matrix_transpose.svg"),
        aliases: &[],
        compute_instance_label: None,
    },
    VirtualBlock {
        name: "Hermitian Transpose",
        ins: 1,
        outs: 1,
        icon: Some("matrix/hermitian_transpose.svg"),
        aliases: &[],
        compute_instance_label: None,
    },
    VirtualBlock {
        name: "Matrix Square",
        ins: 1,
        outs: 1,
        icon: Some("matrix/matrix_square.svg"),
        aliases: &["Square"],
        compute_instance_label: None,
    },
    VirtualBlock {
        // In Simulink SLX files this block appears as "Permute Matrix";
        // "Permute Columns" is the internal alias some older files use.
        name: "Permute Matrix",
        ins: 2,
        outs: 1,
        icon: None,
        aliases: &["Permute Columns", "PermuteMatrix", "PermuteColumns"],
        compute_instance_label: None,
    },
    VirtualBlock {
        name: "Extract Diagonal",
        ins: 1,
        outs: 1,
        icon: Some("matrix/extract_diagonal.svg"),
        // older versions of the catalog (and some SLX files) used the
        // shorthand "ExtractDiag"; treat it as an alias so lookup still
        // succeeds.
        aliases: &["ExtractDiag"],
        compute_instance_label: None,
    },
    VirtualBlock {
        name: "Create Diagonal Matrix",
        ins: 1,
        outs: 1,
        icon: Some("matrix/create_diagonal_matrix.svg"),
        // some Simulink files (and our catalog) historically referred to
        // this block as `DiagonalMatrix` instead of the more verbose
        // `CreateDiagonalMatrix`.  Having an alias here ensures that
        // port-count lookup, icon registration and other logic still work
        // for models created with the old name.
        aliases: &["DiagonalMatrix"],
        compute_instance_label: None,
    },
    VirtualBlock {
        name: "Expand Scalar",
        ins: 1,
        outs: 1,
        icon: Some("matrix/expand_scalar_to_matrix.svg"),
        aliases: &["ExpandScalar"],
        compute_instance_label: None,
    },
    VirtualBlock {
        name: "Is Hermitian",
        ins: 1,
        outs: 1,
        icon: None,
        aliases: &["IsHermitian"],
        compute_instance_label: None,
    },
    VirtualBlock {
        name: "Matrix Concatenate",
        ins: 2,
        outs: 1,
        icon: None,
        aliases: &[],
        compute_instance_label: None,
    },
];

/// Determine if the given library name refers to the matrix virtual
/// library.  Mirrors the behaviour previously encoded in the parser.
///
/// Accepts forms like "matrix_library" or "matrix_library/Foo" (prefix
/// match) and is case-insensitive.
///
/// Note: some Simulink files refer to the library as "matrix_library.slx".
/// We accept both with and without the `.slx` suffix.
pub fn is_matrix_library_name(name: &str) -> bool {
    let norm = name.trim().replace('\\', "/").to_ascii_lowercase();
    norm == "matrix_library"
        || norm == "matrix_library.slx"
        || norm.starts_with("matrix_library/")
        || norm.starts_with("matrix_library.slx/")
}

/// Normalize a library block name for matching purposes.
///
/// All whitespace sequences are collapsed to a single ASCII space and the
/// result is lowercased.  This keeps "foo   bar" equivalent to "foo bar",
/// while distinguishing "foobar" from "foo bar".
fn normalize_name(name: &str) -> String {
    virtual_library::normalize_block_name(name)
}

/// Return the port counts for a block name.
///
/// Matching is case-insensitive.  Rather than removing whitespace entirely, we
/// replace any whitespace run with a single space (see `normalize_name`). If the
/// name is not recognized, `(1, 1)` is returned as a reasonable default.
///
/// CamelCase names from older SLX files (e.g. `"IsTriangular"`) are handled
/// transparently by also trying the humanized (space-separated) form.
pub fn port_counts_for(name: &str) -> (u32, u32) {
    let norm = normalize_name(name);
    let norm_humanized = normalize_name(&virtual_library::humanize_camel_case(name));
    for b in BLOCKS {
        if normalize_name(b.name) == norm || normalize_name(b.name) == norm_humanized {
            return (b.ins, b.outs);
        }
        for &alias in b.aliases {
            if normalize_name(alias) == norm || normalize_name(alias) == norm_humanized {
                return (b.ins, b.outs);
            }
        }
    }
    (1, 1)
}

/// Return the port counts for a block name **only if** it is an explicitly
/// known matrix-library block.
///
/// Unlike [`port_counts_for`], this function returns `None` for unrecognised
/// block names instead of a `(1, 1)` fallback.  This is useful for
/// auto-detection code that must distinguish between "known block, apply
/// defaults" and "unknown block, do nothing".
///
/// CamelCase names from older SLX files (e.g. `"IsTriangular"`) are handled
/// transparently by also trying the humanized (space-separated) form.
pub fn port_counts_if_known(name: &str) -> Option<(u32, u32)> {
    let norm = normalize_name(name);
    let norm_humanized = normalize_name(&virtual_library::humanize_camel_case(name));
    for b in BLOCKS {
        if normalize_name(b.name) == norm || normalize_name(b.name) == norm_humanized {
            return Some((b.ins, b.outs));
        }
        for &alias in b.aliases {
            if normalize_name(alias) == norm || normalize_name(alias) == norm_humanized {
                return Some((b.ins, b.outs));
            }
        }
    }
    None
}
/// Construct a minimal `Block` stub suitable for rendering an unknown block
/// from the matrix library.
///
/// The returned block has the proper `block_type`/`name` and a set of ports
/// matching `port_counts_for`. Other fields are left as defaults.
pub fn create_stub(name: &str) -> Block {
    let (ins, outs) = port_counts_for(name);
    virtual_library::create_stub_block(name, ins, outs)
}

/// Construct the initial virtual matrix-library system.
pub fn initial_system() -> System {
    virtual_library::initial_system(BLOCKS)
}

/// Returns all block definitions for the matrix library.
pub fn get_blocks() -> &'static [VirtualBlock] {
    BLOCKS
}
