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
                "timestamp": 123,
                "name": "Alice",
                "name_source": "pn_meta"
            })),
        })
        .unwrap();

    assert!(resp.result.is_some());
    let event = daemon.take_event().expect("announce event");
    assert_eq!(event.event_type, "announce_received");
    assert_eq!(event.payload["peer"], "peer-a");
    assert_eq!(event.payload["name"], "Alice");
    assert_eq!(event.payload["name_source"], "pn_meta");
    assert_eq!(event.payload["seen_count"], 1);

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
    assert_eq!(peers[0].get("name").unwrap(), "Alice");
    assert_eq!(peers[0].get("name_source").unwrap(), "pn_meta");
    assert_eq!(peers[0].get("first_seen").unwrap(), 123);
    assert_eq!(peers[0].get("last_seen").unwrap(), 123);
    assert_eq!(peers[0].get("seen_count").unwrap(), 1);
}

#[test]
fn repeated_announces_increment_seen_count_and_preserve_first_seen() {
    let daemon = RpcDaemon::test_instance();

    daemon
        .handle_rpc(RpcRequest {
            id: 1,
            method: "announce_received".into(),
            params: Some(serde_json::json!({
                "peer": "peer-z",
                "timestamp": 100,
                "name": "Old Name",
                "name_source": "app_data_utf8"
            })),
        })
        .unwrap();

    daemon
        .handle_rpc(RpcRequest {
            id: 2,
            method: "announce_received".into(),
            params: Some(serde_json::json!({
                "peer": "peer-z",
                "timestamp": 150
            })),
        })
        .unwrap();

    daemon
        .handle_rpc(RpcRequest {
            id: 3,
            method: "announce_received".into(),
            params: Some(serde_json::json!({
                "peer": "peer-z",
                "timestamp": 200,
                "name": "New Name",
                "name_source": "pn_meta"
            })),
        })
        .unwrap();

    let peers = daemon
        .handle_rpc(RpcRequest {
            id: 4,
            method: "list_peers".into(),
            params: None,
        })
        .unwrap()
        .result
        .unwrap();

    let peer = peers
        .get("peers")
        .and_then(|value| value.as_array())
        .and_then(|items| items.first())
        .cloned()
        .expect("peer record");

    assert_eq!(peer["peer"], "peer-z");
    assert_eq!(peer["first_seen"], 100);
    assert_eq!(peer["last_seen"], 200);
    assert_eq!(peer["seen_count"], 3);
    assert_eq!(peer["name"], "New Name");
    assert_eq!(peer["name_source"], "pn_meta");
}
