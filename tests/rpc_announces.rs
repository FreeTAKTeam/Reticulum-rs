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
