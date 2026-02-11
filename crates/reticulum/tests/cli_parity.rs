#[test]
fn rnstatus_help_matches_expected_flags() {
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--bin",
            "rnstatus",
            "--features",
            "cli-tools",
            "--",
            "--help",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}

#[test]
fn rnprobe_help_matches_expected_flags() {
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--bin",
            "rnprobe",
            "--features",
            "cli-tools",
            "--",
            "--help",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}

#[test]
fn rnpath_help_matches_expected_flags() {
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--bin",
            "rnpath",
            "--features",
            "cli-tools",
            "--",
            "--help",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}

#[test]
fn rnid_help_matches_expected_flags() {
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--bin",
            "rnid",
            "--features",
            "cli-tools",
            "--",
            "--help",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}

#[test]
fn rnsd_help_matches_expected_flags() {
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--bin",
            "rnsd",
            "--features",
            "cli-tools",
            "--",
            "--help",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}

#[test]
fn rncp_help_matches_expected_flags() {
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--bin",
            "rncp",
            "--features",
            "cli-tools",
            "--",
            "--help",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}

#[test]
fn rnx_help_matches_expected_flags() {
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--bin",
            "rnx",
            "--features",
            "cli-tools",
            "--",
            "--help",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}

#[test]
fn rnpkg_help_matches_expected_flags() {
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--bin",
            "rnpkg",
            "--features",
            "cli-tools",
            "--",
            "--help",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}

#[test]
fn rnodeconf_help_matches_expected_flags() {
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--bin",
            "rnodeconf",
            "--features",
            "cli-tools",
            "--",
            "--help",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}

#[test]
fn rnir_help_matches_expected_flags() {
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--bin",
            "rnir",
            "--features",
            "cli-tools",
            "--",
            "--help",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}
