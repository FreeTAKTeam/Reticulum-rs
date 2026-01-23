use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use reticulum::destination::{DestinationName, SingleInputDestination};
use reticulum::hash::AddressHash;
use reticulum::identity::PrivateIdentity;
use reticulum::transport::{AnnounceHandler, Transport, TransportConfig};
use rand_core::OsRng;

struct Counter {
    called: Arc<AtomicUsize>,
}

impl AnnounceHandler for Counter {
    fn on_announce(&self, _event: &reticulum::transport::AnnounceEvent) {
        self.called.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn transport_emits_announce_handler() {
    let mut transport = Transport::new(TransportConfig::default());
    let called = Arc::new(AtomicUsize::new(0));
    transport.register_announce_handler(Box::new(Counter { called: called.clone() })).await;

    let identity = PrivateIdentity::new_from_name("announce-test");
    let destination = SingleInputDestination::new(
        identity,
        DestinationName::new("lxmf", "delivery"),
    );
    let packet = destination.announce(OsRng, None).expect("announce");
    let iface = AddressHash::new_from_slice(&[9u8; 16]);

    transport.handle_announce_for_test(packet, iface).await;

    assert_eq!(called.load(Ordering::SeqCst), 1);
}
