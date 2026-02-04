use std::sync::{Arc, Mutex};

use reticulum::rpc::{OutboundBridge, RpcDaemon, RpcRequest};
use serde_json::json;

struct TestBridge {
    calls: Arc<Mutex<u32>>,
}

impl OutboundBridge for TestBridge {
    fn deliver(&self, _record: &reticulum::storage::messages::MessageRecord) -> Result<(), std::io::Error> {
        let mut guard = self.calls.lock().expect("calls");
        *guard += 1;
        Ok(())
    }
}

#[test]
fn send_message_calls_bridge() {
    let calls = Arc::new(Mutex::new(0));
    let bridge = TestBridge { calls: calls.clone() };
    let daemon = RpcDaemon::with_store_and_bridge(
        reticulum::storage::messages::MessagesStore::in_memory().expect("store"),
        "test".into(),
        Arc::new(bridge),
    );

    let request = RpcRequest {
        id: 1,
        method: "send_message".into(),
        params: Some(json!({
            "id": "msg-1",
            "source": "alice",
            "destination": "bob",
            "title": "",
            "content": "hi",
            "fields": null
        })),
    };

    daemon.handle_rpc(request).expect("response");

    let count = *calls.lock().expect("calls");
    assert_eq!(count, 1);
}
