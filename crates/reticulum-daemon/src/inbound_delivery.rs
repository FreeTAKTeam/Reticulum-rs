use reticulum::storage::messages::MessageRecord;

use crate::lxmf_bridge::{decode_wire_message, rmpv_to_json};

pub fn decode_inbound_payload(destination: [u8; 16], payload: &[u8]) -> Option<MessageRecord> {
    let (message, raw_bytes) = match decode_wire_message(payload) {
        Ok(message) => (message, payload.to_vec()),
        Err(_) => {
            let mut lxmf_bytes = Vec::with_capacity(16 + payload.len());
            lxmf_bytes.extend_from_slice(&destination);
            lxmf_bytes.extend_from_slice(payload);
            let message = decode_wire_message(&lxmf_bytes).ok()?;
            (message, lxmf_bytes)
        }
    };
    let id = lxmf::message::WireMessage::unpack(&raw_bytes)
        .ok()
        .map(|wire| wire.message_id())
        .map(hex::encode)
        .unwrap_or_else(|| hex::encode(destination));

    Some(MessageRecord {
        id,
        source: hex::encode(message.source_hash.unwrap_or([0u8; 16])),
        destination: hex::encode(message.destination_hash.unwrap_or([0u8; 16])),
        title: message.title_as_string().unwrap_or_default(),
        content: message.content_as_string().unwrap_or_default(),
        timestamp: message.timestamp.map(|v| v as i64).unwrap_or(0),
        direction: "in".into(),
        fields: message.fields.as_ref().and_then(rmpv_to_json),
        receipt_status: None,
    })
}
