use reticulum::rpc::{RpcDaemon, RpcRequest};
use reticulum::storage::messages::MessagesStore;

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
    assert!(resp.result.unwrap()["identity_hash"].is_string());
}

#[test]
fn status_uses_custom_identity() {
    let store = MessagesStore::in_memory().unwrap();
    let daemon = RpcDaemon::with_store(store, "daemon-identity".into());
    let resp = daemon
        .handle_rpc(RpcRequest {
            id: 1,
            method: "status".into(),
            params: None,
        })
        .unwrap();
    assert_eq!(resp.result.unwrap()["identity_hash"], "daemon-identity");
}

#[test]
fn send_message_persists() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(RpcRequest {
            id: 2,
            method: "send_message".into(),
            params: Some(serde_json::json!({
                "id": "msg-1",
                "source": "alice",
                "destination": "bob",
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

    let items = resp.result.unwrap()["messages"].as_array().unwrap().clone();
    assert_eq!(items.len(), 1);
}

#[test]
fn list_peers_returns_empty_array() {
    let daemon = RpcDaemon::test_instance();
    let resp = daemon
        .handle_rpc(RpcRequest {
            id: 4,
            method: "list_peers".into(),
            params: None,
        })
        .unwrap();

    let peers = resp.result.unwrap()["peers"].as_array().unwrap().clone();
    assert_eq!(peers.len(), 0);
}
