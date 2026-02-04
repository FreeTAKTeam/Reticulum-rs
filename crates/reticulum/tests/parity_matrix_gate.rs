#[test]
fn parity_matrix_has_no_missing_core_items() {
    let text = std::fs::read_to_string("docs/plans/reticulum-parity-matrix.md").unwrap();
    assert!(!text.contains("missing") || !text.contains("core"));
}
