//! Integration tests for user-registered virtual libraries and whitespace-
//! normalized block name matching.

use std::sync::Arc;

use rustylink::builtin_libraries::matrix_library;
use rustylink::builtin_libraries::virtual_library::normalize_block_name;
use rustylink::builtin_libraries::{
    OwnedVirtualBlock, UserVirtualLibrarySpec, register_virtual_library,
};

// ── normalize_block_name ─────────────────────────────────────────────────────

#[test]
fn normalize_collapses_whitespace() {
    assert_eq!(normalize_block_name("Cross   Product"), "cross product");
    assert_eq!(normalize_block_name("Cross\nProduct"), "cross product");
    assert_eq!(
        normalize_block_name("  Identity Matrix "),
        "identity matrix"
    );
}

#[test]
fn normalize_does_not_split_camel_case() {
    // normalize only collapses whitespace; CamelCase splitting is done by
    // humanize_camel_case separately.
    assert_eq!(normalize_block_name("CrossProduct"), "crossproduct");
    assert_eq!(normalize_block_name("MatrixMultiply"), "matrixmultiply");
}

// ── matrix_library name matching ─────────────────────────────────────────────

#[test]
fn matrix_library_spaced_names_are_canonical() {
    // Canonical names now use spaces.
    let names: Vec<&str> = matrix_library::BLOCKS.iter().map(|b| b.name).collect();
    for name in &names {
        // All canonical names must contain at least 1 character.
        assert!(!name.is_empty(), "empty block name");
        // Canonical names must not be pure CamelCase runs (they must either be
        // a single word OR contain spaces).  Heuristic: if CamelCase-humanizing
        // the name gives a different string, the original must already have a
        // space (or be multi-case).
        // This just ensures we didn't accidentally leave "IsTriangular" etc.
        let has_uppercase_after_start = name.chars().enumerate().any(|(i, c)| {
            i > 0 && c.is_uppercase() && !name.chars().nth(i - 1).map_or(false, |p| p == ' ')
        });
        // If it has uppercase after a non-space character (CamelCase), it should
        // also have a space somewhere (i.e. be already humanized like "Is Triangular").
        if has_uppercase_after_start {
            assert!(
                name.contains(' '),
                "block '{}' looks like unexpanded CamelCase – convert to spaced form",
                name
            );
        }
    }
}

#[test]
fn camelcase_lookup_still_works_via_humanize() {
    // Old-style CamelCase names from SLX files should still resolve via the
    // humanize_camel_case path in port_counts_for.
    assert_eq!(matrix_library::port_counts_for("IdentityMatrix"), (0, 1));
    assert_eq!(matrix_library::port_counts_for("CrossProduct"), (2, 1));
    assert_eq!(matrix_library::port_counts_for("MatrixMultiply"), (2, 1));
    assert_eq!(
        matrix_library::port_counts_for("HermitianTranspose"),
        (1, 1)
    );
    assert_eq!(
        matrix_library::port_counts_for("CreateDiagonalMatrix"),
        (1, 1)
    );
    assert_eq!(matrix_library::port_counts_for("ExtractDiagonal"), (1, 1));
}

#[test]
fn spaced_name_lookup_works_directly() {
    assert_eq!(matrix_library::port_counts_for("Identity Matrix"), (0, 1));
    assert_eq!(matrix_library::port_counts_for("Cross Product"), (2, 1));
    assert_eq!(matrix_library::port_counts_for("Matrix Multiply"), (2, 1));
    assert_eq!(
        matrix_library::port_counts_for("Hermitian Transpose"),
        (1, 1)
    );
    assert_eq!(
        matrix_library::port_counts_for("Create Diagonal Matrix"),
        (1, 1)
    );
    assert_eq!(matrix_library::port_counts_for("Extract Diagonal"), (1, 1));
}

#[test]
fn port_counts_if_known_camelcase() {
    // CamelCase lookup works for port_counts_if_known too.
    assert_eq!(
        matrix_library::port_counts_if_known("IdentityMatrix"),
        Some((0, 1))
    );
    assert_eq!(
        matrix_library::port_counts_if_known("CrossProduct"),
        Some((2, 1))
    );
    assert_eq!(matrix_library::port_counts_if_known("unknown"), None);
}

// ── user virtual library registration ────────────────────────────────────────

fn make_test_library() -> UserVirtualLibrarySpec {
    UserVirtualLibrarySpec {
        name: "test_userlib".to_string(),
        blocks: vec![
            OwnedVirtualBlock {
                name: "My Custom Block".to_string(),
                aliases: vec!["MyCustomBlock".to_string()],
                ins: 2,
                outs: 1,
                compute_instance_label: None,
                port_position_overrides: vec![],
                input_port_names: vec![],
                output_port_names: vec![],
            },
            OwnedVirtualBlock {
                name: "Another Block".to_string(),
                aliases: vec![],
                ins: 1,
                outs: 1,
                compute_instance_label: None,
                port_position_overrides: vec![],
                input_port_names: vec![],
                output_port_names: vec![],
            },
        ],
        matches_name: Arc::new(|name: &str| name.to_ascii_lowercase().starts_with("test_userlib")),
        initial_system: Arc::new(|| rustylink::model::System {
            properties: indexmap::IndexMap::new(),
            blocks: vec![],
            lines: vec![],
            annotations: vec![],
            chart: None,
        }),
    }
}

#[test]
fn register_user_library_and_get_initial_system() {
    // Register before querying (relies on global singleton – parallel tests ok
    // because we always ADD entries and only read after writing).
    register_virtual_library(make_test_library());

    let sys = rustylink::builtin_libraries::virtual_library_initial_system("test_userlib");
    // The initial system should be Some (now that the library is registered).
    assert!(sys.is_some());
}

#[test]
fn register_user_library_with_instance_label() {
    use rustylink::model::Block;

    let lib = UserVirtualLibrarySpec {
        name: "test_label_lib".to_string(),
        blocks: vec![OwnedVirtualBlock {
            name: "Labeled Block".to_string(),
            aliases: vec![],
            ins: 1,
            outs: 1,
            compute_instance_label: Some(Arc::new(|_block: &Block| {
                Some("custom label".to_string())
            })),
            port_position_overrides: vec![],
            input_port_names: vec![],
            output_port_names: vec![],
        }],
        matches_name: Arc::new(|name: &str| {
            name.to_ascii_lowercase().starts_with("test_label_lib")
        }),
        initial_system: Arc::new(|| rustylink::model::System {
            properties: indexmap::IndexMap::new(),
            blocks: vec![],
            lines: vec![],
            annotations: vec![],
            chart: None,
        }),
    };

    register_virtual_library(lib);

    // Build a minimal block that refers to this library.
    let mut block = rustylink::editor::operations::create_default_block(
        "Reference",
        "Labeled Block",
        0,
        0,
        1,
        1,
    );
    block.library_block_path = Some("test_label_lib/Labeled Block".to_string());

    let label = rustylink::builtin_libraries::compute_block_instance_label(&block);
    assert_eq!(label.as_deref(), Some("custom label"));
}
