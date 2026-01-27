use reticulum::rpc::{RpcDaemon, RpcRequest};

#[test]
fn announce_now_emits_event() {
    let daemon = RpcDaemon::test_instance();
    let resp = daemon
        .handle_rpc(RpcRequest {
            id: 1,
            method: "announce_now".into(),
            params: None,
        })
        .unwrap();

    assert!(resp.result.is_some());
    let event = daemon.take_event().expect("announce event");
    assert_eq!(event.event_type, "announce_sent");
}

#[test]
fn announce_received_updates_peers() {
    let daemon = RpcDaemon::test_instance();
    let resp = daemon
        .handle_rpc(RpcRequest {
            id: 2,
            method: "announce_received".into(),
            params: Some(serde_json::json!({
                "peer": "peer-a",
                "timestamp": 123
            })),
        })
        .unwrap();

    assert!(resp.result.is_some());
    let event = daemon.take_event().expect("announce event");
    assert_eq!(event.event_type, "announce_received");

    let peers = daemon
        .handle_rpc(RpcRequest {
            id: 3,
            method: "list_peers".into(),
            params: None,
        })
        .unwrap()
        .result
        .unwrap()
        .get("peers")
        .unwrap()
        .as_array()
        .unwrap()
        .clone();

    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].get("peer").unwrap(), "peer-a");
}
