#[test]
fn examples_compile() {
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::Once;

    const EXAMPLES: &[&str] = &[
        "tcp_server",
        "tcp_client",
        "udp_link",
        "link_client",
        "testnet_client",
        "multihop",
    ];
    static BUILD_EXAMPLES: Once = Once::new();

    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("target"));
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let has_all_examples = EXAMPLES.iter().all(|example| {
        let mut path = target_dir.join(&profile).join("examples").join(example);
        if cfg!(windows) {
            path.set_extension("exe");
        }
        Path::new(&path).exists()
    });

    if !has_all_examples {
        BUILD_EXAMPLES.call_once(|| {
            let status = Command::new("cargo")
                .args(["build", "--examples"])
                .status()
                .expect("spawn cargo build --examples");
            assert!(status.success(), "cargo build --examples failed");
        });
    }

    for example in EXAMPLES {
        let mut path = target_dir.join(&profile).join("examples").join(example);
        if cfg!(windows) {
            path.set_extension("exe");
        }
        assert!(
            Path::new(&path).exists(),
            "example binary '{example}' missing at {}",
            path.display()
        );
    }
}
