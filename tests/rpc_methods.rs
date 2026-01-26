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

#[test]
fn send_message_persists() {
    let daemon = RpcDaemon::test_instance();
    let _ = daemon
        .handle_rpc(RpcRequest {
            id: 2,
            method: "send_message".into(),
            params: Some(serde_json::json!({
                "id": "msg-1",
                "source": "me",
                "destination": "peer",
                "content": "hello"
            })),
        })
        .unwrap();

    let resp = daemon
        .handle_rpc(RpcRequest {
            id: 3,
            method: "list_messages".into(),
            params: None,
        })
        .unwrap();
    let result = resp.result.expect("missing result");
    assert_eq!(result["messages"].as_array().unwrap().len(), 1);
}
