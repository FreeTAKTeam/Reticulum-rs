#[test]
fn examples_compile() {
    let status = std::process::Command::new("cargo")
        .args(["build", "--examples"])
        .status()
        .unwrap();
    assert!(status.success());
}
