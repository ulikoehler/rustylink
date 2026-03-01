use rustylink::builtin_libraries::matrix_library;

#[test]
fn triangular_and_symmetric_have_one_each() {
    assert_eq!(matrix_library::port_counts_for("IsTriangular"), (1, 1));
    assert_eq!(matrix_library::port_counts_for("IsSymmetric"), (1, 1));
    assert_eq!(matrix_library::port_counts_for("is triangular"), (1, 1));
    assert_eq!(matrix_library::port_counts_for("is   symmetric"), (1, 1));
}

#[test]
fn diagonal_matrix_alias_is_recognised() {
    // ensure the new alias behaves identically to the canonical name
    assert_eq!(matrix_library::port_counts_for("DiagonalMatrix"), (1, 1));
    assert_eq!(
        matrix_library::port_counts_if_known("DiagonalMatrix"),
        Some((1, 1))
    );

    // also check extract-diagonal alias
    assert_eq!(matrix_library::port_counts_for("ExtractDiag"), (1, 1));
    assert_eq!(
        matrix_library::port_counts_if_known("ExtractDiag"),
        Some((1, 1))
    );
}
