use reticulum::e2e_harness::is_ready_line;

#[test]
fn ready_line_detects_daemon_listening() {
    assert!(is_ready_line("reticulumd listening on http://127.0.0.1:4243"));
    assert!(!is_ready_line("starting daemon..."));
}
