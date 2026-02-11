use reticulum::rpc::{RpcDaemon, RpcRequest};
use serde_json::json;

#[test]
fn daemon_status_ex_exposes_capabilities() {
    let daemon = RpcDaemon::test_instance();
    let resp = daemon
        .handle_rpc(RpcRequest {
            id: 1,
            method: "daemon_status_ex".into(),
            params: None,
        })
        .expect("status ex");

    let result = resp.result.expect("result");
    assert_eq!(result.get("running"), Some(&json!(true)));
    assert!(result.get("delivery_destination_hash").is_some());
    let caps = result
        .get("capabilities")
        .and_then(|v| v.as_array())
        .expect("capabilities");
    assert!(caps.iter().any(|c| c == "send_message_v2"));
    assert!(caps.iter().any(|c| c == "set_interfaces"));
}

#[test]
fn interfaces_roundtrip_via_rpc() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(RpcRequest {
            id: 2,
            method: "set_interfaces".into(),
            params: Some(json!({
                "interfaces": [
                    {
                        "type": "tcp_client",
                        "enabled": true,
                        "host": "rmap.world",
                        "port": 4242,
                        "name": "Public RMap"
                    }
                ]
            })),
        })
        .expect("set interfaces");

    let list = daemon
        .handle_rpc(RpcRequest {
            id: 3,
            method: "list_interfaces".into(),
            params: None,
        })
        .expect("list interfaces");
    let result = list.result.expect("result");
    let interfaces = result
        .get("interfaces")
        .and_then(|v| v.as_array())
        .expect("interfaces");
    assert_eq!(interfaces.len(), 1);
    assert_eq!(interfaces[0]["type"], "tcp_client");
    assert_eq!(interfaces[0]["host"], "rmap.world");
}

#[test]
fn peer_sync_and_unpeer_work() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(RpcRequest {
            id: 4,
            method: "peer_sync".into(),
            params: Some(json!({ "peer": "abcd0123" })),
        })
        .expect("peer_sync");

    let peers = daemon
        .handle_rpc(RpcRequest {
            id: 5,
            method: "list_peers".into(),
            params: None,
        })
        .expect("list peers");
    let result = peers.result.expect("result");
    let peers = result
        .get("peers")
        .and_then(|v| v.as_array())
        .expect("peers");
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0]["peer"], "abcd0123");
    assert_eq!(peers[0]["seen_count"], 1);
    assert!(peers[0]["first_seen"].as_i64().is_some());
    assert!(peers[0]["last_seen"].as_i64().is_some());

    let unpeer = daemon
        .handle_rpc(RpcRequest {
            id: 6,
            method: "peer_unpeer".into(),
            params: Some(json!({ "peer": "abcd0123" })),
        })
        .expect("peer_unpeer");
    assert_eq!(unpeer.result.expect("result")["removed"], true);
}

#[test]
fn send_message_v2_persists_lxmf_metadata() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(RpcRequest {
            id: 7,
            method: "send_message_v2".into(),
            params: Some(json!({
                "id": "msg-v2",
                "source": "alice",
                "destination": "bob",
                "title": "hello",
                "content": "world",
                "method": "propagated",
                "stamp_cost": 9,
                "include_ticket": true
            })),
        })
        .expect("send_message_v2");

    let list = daemon
        .handle_rpc(RpcRequest {
            id: 8,
            method: "list_messages".into(),
            params: None,
        })
        .expect("list");
    let result = list.result.expect("result");
    let messages = result
        .get("messages")
        .and_then(|v| v.as_array())
        .expect("messages");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["id"], "msg-v2");
    assert_eq!(messages[0]["fields"]["_lxmf"]["method"], "propagated");
    assert_eq!(messages[0]["fields"]["_lxmf"]["stamp_cost"], 9);
    assert_eq!(messages[0]["fields"]["_lxmf"]["include_ticket"], true);
}

#[test]
fn delivery_policy_roundtrip() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(RpcRequest {
            id: 9,
            method: "set_delivery_policy".into(),
            params: Some(json!({
                "auth_required": true,
                "allowed_destinations": ["a"],
                "ignored_destinations": ["b"]
            })),
        })
        .expect("set policy");
    let policy = daemon
        .handle_rpc(RpcRequest {
            id: 10,
            method: "get_delivery_policy".into(),
            params: None,
        })
        .expect("get policy");
    let policy = policy.result.expect("result")["policy"].clone();
    assert_eq!(policy["auth_required"], true);
    assert_eq!(policy["allowed_destinations"][0], "a");
    assert_eq!(policy["ignored_destinations"][0], "b");
}

#[test]
fn propagation_ingest_fetch_roundtrip() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(RpcRequest {
            id: 11,
            method: "propagation_enable".into(),
            params: Some(json!({
                "enabled": true,
                "store_root": "/tmp/prop",
                "target_cost": 18
            })),
        })
        .expect("propagation_enable");

    let ingest = daemon
        .handle_rpc(RpcRequest {
            id: 12,
            method: "propagation_ingest".into(),
            params: Some(json!({
                "transient_id": "abc123",
                "payload_hex": "deadbeef"
            })),
        })
        .expect("propagation_ingest");
    assert_eq!(ingest.result.expect("result")["ingested_count"], 1);

    let fetch = daemon
        .handle_rpc(RpcRequest {
            id: 13,
            method: "propagation_fetch".into(),
            params: Some(json!({ "transient_id": "abc123" })),
        })
        .expect("propagation_fetch");
    assert_eq!(fetch.result.expect("result")["payload_hex"], "deadbeef");
}

#[test]
fn paper_ingest_detects_duplicates() {
    let daemon = RpcDaemon::test_instance();
    let first = daemon
        .handle_rpc(RpcRequest {
            id: 14,
            method: "paper_ingest_uri".into(),
            params: Some(json!({
                "uri": "lxm://6b3362bd2c1dbf87b66a85f79a8d8c75HELLO"
            })),
        })
        .expect("paper ingest");
    assert_eq!(first.result.expect("result")["duplicate"], false);

    let second = daemon
        .handle_rpc(RpcRequest {
            id: 15,
            method: "paper_ingest_uri".into(),
            params: Some(json!({
                "uri": "lxm://6b3362bd2c1dbf87b66a85f79a8d8c75HELLO"
            })),
        })
        .expect("paper ingest");
    assert_eq!(second.result.expect("result")["duplicate"], true);
}

#[test]
fn stamp_policy_and_ticket_generation() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(RpcRequest {
            id: 16,
            method: "stamp_policy_set".into(),
            params: Some(json!({
                "target_cost": 10,
                "flexibility": 2
            })),
        })
        .expect("stamp_policy_set");

    let get = daemon
        .handle_rpc(RpcRequest {
            id: 17,
            method: "stamp_policy_get".into(),
            params: None,
        })
        .expect("stamp_policy_get");
    assert_eq!(
        get.result.expect("result")["stamp_policy"]["target_cost"],
        10
    );

    let ticket = daemon
        .handle_rpc(RpcRequest {
            id: 18,
            method: "ticket_generate".into(),
            params: Some(json!({
                "destination": "6b3362bd2c1dbf87b66a85f79a8d8c75",
                "ttl_secs": 30
            })),
        })
        .expect("ticket_generate");
    let ticket = ticket.result.expect("result");
    assert!(ticket["ticket"].as_str().unwrap_or_default().len() > 10);
    assert_eq!(ticket["ttl_secs"], 30);
}

#[test]
fn ticket_generation_rejects_ttl_overflow() {
    let daemon = RpcDaemon::test_instance();
    let err = daemon
        .handle_rpc(RpcRequest {
            id: 19,
            method: "ticket_generate".into(),
            params: Some(json!({
                "destination": "6b3362bd2c1dbf87b66a85f79a8d8c75",
                "ttl_secs": u64::MAX
            })),
        })
        .expect_err("overflow ttl should fail");

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(err.to_string().contains("ttl_secs"));
}
