use reticulum::rpc::{RpcDaemon, RpcRequest};

#[test]
fn status_returns_identity() {
    let daemon = RpcDaemon::test_instance();
    let resp = daemon
        .handle_rpc(RpcRequest {
            id: 1,
            method: "status".into(),
            params: None,
        })
        .unwrap();
    let result = resp.result.expect("missing result");
    assert!(result["identity_hash"].is_string());
}
