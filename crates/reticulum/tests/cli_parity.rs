use std::path::PathBuf;
use std::process::Command;
use std::sync::Once;

static BUILD_CLI_BINARIES: Once = Once::new();

fn cli_binary_path(bin: &str) -> PathBuf {
    let env_key = format!("CARGO_BIN_EXE_{bin}");
    if let Ok(path) = std::env::var(&env_key) {
        return PathBuf::from(path);
    }

    BUILD_CLI_BINARIES.call_once(|| {
        let status = Command::new("cargo")
            .args(["build", "--bins", "--features", "cli-tools"])
            .status()
            .expect("spawn cargo build --bins");
        assert!(status.success(), "failed to build CLI binaries");
    });

    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("target"));
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let profiles = [profile.as_str(), "debug", "release"];

    for candidate in profiles {
        let mut path = target_dir.join(candidate).join(bin);
        if cfg!(windows) {
            path.set_extension("exe");
        }
        if path.exists() {
            return path;
        }
    }

    panic!("CLI binary '{bin}' was not found under {}/*/{bin}", target_dir.display());
}

fn cli_help_contains_config(bin: &str) {
    let output = Command::new(cli_binary_path(bin))
        .arg("--help")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}

#[test]
fn rnstatus_help_matches_expected_flags() {
    cli_help_contains_config("rnstatus");
}

#[test]
fn rnprobe_help_matches_expected_flags() {
    cli_help_contains_config("rnprobe");
}

#[test]
fn rnpath_help_contains_config() {
    cli_help_contains_config("rnpath");
}

#[test]
fn rnid_help_contains_config() {
    cli_help_contains_config("rnid");
}

#[test]
fn rnsd_help_contains_config() {
    cli_help_contains_config("rnsd");
}

#[test]
fn rncp_help_contains_config() {
    cli_help_contains_config("rncp");
}

#[test]
fn rnx_help_contains_config() {
    cli_help_contains_config("rnx");
}

#[test]
fn rnpkg_help_contains_config() {
    cli_help_contains_config("rnpkg");
}

#[test]
fn rnodeconf_help_contains_config() {
    cli_help_contains_config("rnodeconf");
}

#[test]
fn rnir_help_contains_config() {
    cli_help_contains_config("rnir");
}
