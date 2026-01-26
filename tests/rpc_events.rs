use reticulum::rpc::RpcDaemon;

#[tokio::test]
async fn emits_inbound_event() {
    let daemon = RpcDaemon::test_instance();
    let mut stream = daemon.subscribe_events();
    daemon.inject_inbound_test_message("hello");
    let evt = stream.recv().await.unwrap();
    assert_eq!(evt.event_type, "inbound");
}
