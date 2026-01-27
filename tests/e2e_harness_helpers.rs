use reticulum::e2e_harness::is_ready_line;
use reticulum::e2e_harness::{build_rpc_body, parse_rpc_response};
use serde_json::json;

#[test]
fn ready_line_detects_daemon_listening() {
    assert!(is_ready_line("reticulumd listening on http://127.0.0.1:4243"));
    assert!(!is_ready_line("starting daemon..."));
}

#[test]
fn rpc_helpers_roundtrip() {
    let body = build_rpc_body(1, "status", None);
    assert!(body.contains("\"method\":\"status\""));

    let resp = json!({"id":1,"result":{"ok":true},"error":null}).to_string();
    let parsed = parse_rpc_response(&resp).expect("parse");
    assert_eq!(parsed.id, 1);
}
