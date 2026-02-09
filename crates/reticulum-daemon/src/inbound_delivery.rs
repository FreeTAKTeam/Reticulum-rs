use reticulum::storage::messages::MessageRecord;

use crate::lxmf_bridge::{decode_wire_message, rmpv_to_json};

pub fn decode_inbound_payload(destination: [u8; 16], payload: &[u8]) -> Option<MessageRecord> {
    let mut decode_candidates = Vec::with_capacity(3);
    decode_candidates.push(payload.to_vec());

    let mut with_destination_prefix = Vec::with_capacity(16 + payload.len());
    with_destination_prefix.extend_from_slice(&destination);
    with_destination_prefix.extend_from_slice(payload);
    decode_candidates.push(with_destination_prefix);

    if payload.len() > 16 && payload[..16] == destination {
        decode_candidates.push(payload[16..].to_vec());
    }

    let mut decoded: Option<(lxmf::message::Message, Vec<u8>)> = None;
    for candidate in decode_candidates {
        if let Ok(message) = decode_wire_message(&candidate) {
            decoded = Some((message, candidate));
            break;
        }
    }

    let (message, raw_bytes) = decoded?;
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
