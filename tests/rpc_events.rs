use reticulum::rpc::{RpcDaemon, RpcEvent};

#[test]
fn rpc_event_queue_drains_in_fifo_order() {
    let daemon = RpcDaemon::test_instance();
    daemon.push_event(RpcEvent {
        event_type: "one".into(),
        payload: serde_json::json!({"i": 1}),
    });
    daemon.push_event(RpcEvent {
        event_type: "two".into(),
        payload: serde_json::json!({"i": 2}),
    });

    let first = daemon.take_event().expect("first");
    let second = daemon.take_event().expect("second");

    assert_eq!(first.event_type, "one");
    assert_eq!(second.event_type, "two");
}

#[test]
fn rpc_event_queue_returns_none_when_empty() {
    let daemon = RpcDaemon::test_instance();
    assert!(daemon.take_event().is_none());
}
