use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use reticulum::packet::{Header, Packet, PacketType};
use reticulum::transport::{DeliveryReceipt, ReceiptHandler, Transport, TransportConfig};

struct Counter {
    called: Arc<AtomicUsize>,
}

impl ReceiptHandler for Counter {
    fn on_receipt(&self, _receipt: &DeliveryReceipt) {
        self.called.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn proof_packet_emits_receipt() {
    let mut transport = Transport::new(TransportConfig::default());
    let called = Arc::new(AtomicUsize::new(0));
    transport
        .set_receipt_handler(Box::new(Counter { called: called.clone() }))
        .await;

    let mut packet = Packet::default();
    packet.header = Header::default();
    packet.header.packet_type = PacketType::Proof;

    transport.handle_inbound_for_test(packet).await;

    assert_eq!(called.load(Ordering::SeqCst), 1);
}
