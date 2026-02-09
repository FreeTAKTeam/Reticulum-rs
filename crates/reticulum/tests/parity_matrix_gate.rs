#[test]
fn parity_matrix_has_no_missing_core_items() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidate_paths = [
        manifest_dir.join("../../docs/plans/reticulum-parity-matrix.md"),
        manifest_dir.join("docs/plans/reticulum-parity-matrix.md"),
    ];

    let mut text = None;
    for path in candidate_paths {
        if let Ok(contents) = std::fs::read_to_string(&path) {
            text = Some(contents);
            break;
        }
    }
    let text = text.expect("reticulum parity matrix should exist");

    assert!(!text.contains("missing") || !text.contains("core"));
}
