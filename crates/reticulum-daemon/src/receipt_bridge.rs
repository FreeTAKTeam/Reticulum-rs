use reticulum::transport::{DeliveryReceipt, ReceiptHandler};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone)]
pub struct ReceiptEvent {
    pub message_id: String,
    pub status: String,
}

#[derive(Clone)]
pub struct ReceiptBridge {
    map: Arc<Mutex<HashMap<String, String>>>,
    tx: UnboundedSender<ReceiptEvent>,
}

impl ReceiptBridge {
    pub fn new(map: Arc<Mutex<HashMap<String, String>>>, tx: UnboundedSender<ReceiptEvent>) -> Self {
        Self { map, tx }
    }
}

impl ReceiptHandler for ReceiptBridge {
    fn on_receipt(&self, receipt: &DeliveryReceipt) {
        let key = hex::encode(receipt.message_id);
        let message_id = self.map.lock().ok().and_then(|mut map| map.remove(&key));
        if let Some(message_id) = message_id {
            let _ = self.tx.send(ReceiptEvent {
                message_id,
                status: "delivered".into(),
            });
        }
    }
}
