use rustylink::parser::{LibraryResolver, SimulinkParser, FsSource, is_virtual_library};
use rustylink::builtin_libraries::matrix_library;
use rustylink::model::System;
use camino::Utf8PathBuf;
use indexmap::IndexMap;

#[test]
fn virtual_library_detection() {
    assert!(is_virtual_library("simulink"));
    assert!(is_virtual_library("Simulink.SLX"));
    assert!(is_virtual_library("matrix_library"));
    assert!(is_virtual_library("simulink/Logic and Bit"));
    assert!(is_virtual_library("Simulink/logic and BIT"));
    assert!(!is_virtual_library("other"));
}

#[test]
fn resolving_virtual_library_does_not_error() {
    // Build a system containing a single block referencing the virtual lib
    let mut blk = rustylink::editor::operations::create_default_block("Some", "B", 0, 0, 0, 0);
    blk.properties.insert(
        "SourceBlock".to_string(),
        "simulink/Logic and Bit/Foo".to_string(),
    );
    let mut sys = System {
        properties: IndexMap::new(),
        blocks: vec![blk],
        lines: Vec::new(),
        annotations: Vec::new(),
        chart: None,
    };

    // Call the public resolver; should succeed without panicking or error.
    SimulinkParser::<FsSource>::resolve_library_references(&mut sys, &[]).unwrap();
    // The block still exists and has not received any library metadata, but
    // no error was produced.
    assert_eq!(sys.blocks.len(), 1);
    assert!(sys.blocks[0].library_source.is_none());
}

#[test]
fn matrix_library_helpers_work() {
    // name recognition
    assert!(matrix_library::is_matrix_library_name("matrix_library"));
    assert!(matrix_library::is_matrix_library_name("Matrix_Library/Thing"));
    assert!(!matrix_library::is_matrix_library_name("other"));

    // port counts for known and unknown names
    assert_eq!(matrix_library::port_counts_for("IdentityMatrix"), (0, 1));
    assert_eq!(matrix_library::port_counts_for("crossproduct"), (2, 1));
    assert_eq!(matrix_library::port_counts_for("unknown"), (1, 1));

    // whitespace collapse: multiple spaces are treated the same as a single
    // space, but spaces are not removed entirely.
    let a = matrix_library::port_counts_for("Cross   Product");
    let b = matrix_library::port_counts_for("Cross Product");
    assert_eq!(a, b);
    // however "CrossProduct" (no space) no longer matches "Cross Product".
    assert_ne!(matrix_library::port_counts_for("CrossProduct"), b);

    // block list contains a particular entry
    assert!(matrix_library::BLOCKS.iter().any(|b| b.name == "IdentityMatrix"));

    // stub creation produces a block with the expected fields
    let stub = matrix_library::create_stub("Foo");
    assert_eq!(stub.block_type, "Foo");
    assert_eq!(stub.ports.len(), 2); // default 1 in + 1 out
}
