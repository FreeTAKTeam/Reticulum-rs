use std::fs;
use std::path::PathBuf;

use reticulum_daemon::identity_store::load_or_create_identity;

fn temp_identity_path() -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("reticulumd-identity-{}", std::process::id()));
    path
}

#[test]
fn identity_persists_across_reloads() {
    let path = temp_identity_path();
    let _ = fs::remove_file(&path);

    let first = load_or_create_identity(&path).expect("create identity");
    assert!(path.exists(), "identity file should be created");

    let second = load_or_create_identity(&path).expect("load identity");
    assert_eq!(
        first.to_private_key_bytes(),
        second.to_private_key_bytes(),
        "identity should be stable across reloads"
    );

    let _ = fs::remove_file(&path);
}
