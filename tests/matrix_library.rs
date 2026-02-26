use rustylink::builtin_libraries::matrix_library;

#[test]
fn triangular_and_symmetric_have_one_each() {
    assert_eq!(matrix_library::port_counts_for("IsTriangular"), (1, 1));
    assert_eq!(matrix_library::port_counts_for("IsSymmetric"), (1, 1));
    assert_eq!(matrix_library::port_counts_for("is triangular"), (1, 1));
    assert_eq!(matrix_library::port_counts_for("is   symmetric"), (1, 1));
}
