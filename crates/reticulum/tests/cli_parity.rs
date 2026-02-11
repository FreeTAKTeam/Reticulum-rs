#[test]
fn rnstatus_help_matches_expected_flags() {
    let output = std::process::Command::new("cargo")
        .args(["run", "--bin", "rnstatus", "--", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}

#[test]
fn rnprobe_help_matches_expected_flags() {
    let output = std::process::Command::new("cargo")
        .args(["run", "--bin", "rnprobe", "--", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}

#[test]
fn rnpath_help_matches_expected_flags() {
    let output = std::process::Command::new("cargo")
        .args(["run", "--bin", "rnpath", "--", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}

#[test]
fn rnid_help_matches_expected_flags() {
    let output = std::process::Command::new("cargo")
        .args(["run", "--bin", "rnid", "--", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}

#[test]
fn rnsd_help_matches_expected_flags() {
    let output = std::process::Command::new("cargo")
        .args(["run", "--bin", "rnsd", "--", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}

#[test]
fn rncp_help_matches_expected_flags() {
    let output = std::process::Command::new("cargo")
        .args(["run", "--bin", "rncp", "--", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}

#[test]
fn rnx_help_matches_expected_flags() {
    let output = std::process::Command::new("cargo")
        .args(["run", "--bin", "rnx", "--", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}

#[test]
fn rnpkg_help_matches_expected_flags() {
    let output = std::process::Command::new("cargo")
        .args(["run", "--bin", "rnpkg", "--", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}

#[test]
fn rnodeconf_help_matches_expected_flags() {
    let output = std::process::Command::new("cargo")
        .args(["run", "--bin", "rnodeconf", "--", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}

#[test]
fn rnir_help_matches_expected_flags() {
    let output = std::process::Command::new("cargo")
        .args(["run", "--bin", "rnir", "--", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}
