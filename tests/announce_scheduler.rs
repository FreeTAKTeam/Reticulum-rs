use std::sync::Arc;

use reticulum::rpc::RpcDaemon;
use tokio::task::LocalSet;
use tokio::time::{advance, Duration};

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn announce_scheduler_emits_event_after_interval() {
    let daemon = Arc::new(RpcDaemon::test_instance());
    let local = LocalSet::new();

    local
        .run_until(async move {
            let _handle = daemon.clone().start_announce_scheduler(1);

            tokio::task::yield_now().await;
            advance(Duration::from_secs(1)).await;
            tokio::task::yield_now().await;

            let event = daemon.take_event().expect("announce event");
            assert_eq!(event.event_type, "announce_sent");
        })
        .await;
}
