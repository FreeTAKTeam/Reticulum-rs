use std::io::Cursor;

use lxmf::error::LxmfError;
use lxmf::message::Message;
use reticulum::identity::PrivateIdentity;
use rmpv::Value;
use serde_json::Value as JsonValue;

pub fn build_wire_message(
    source: [u8; 16],
    destination: [u8; 16],
    title: &str,
    content: &str,
    fields: Option<JsonValue>,
    signer: &PrivateIdentity,
) -> Result<Vec<u8>, LxmfError> {
    let mut message = Message::new();
    message.destination_hash = Some(destination);
    message.source_hash = Some(source);
    message.set_title_from_string(title);
    message.set_content_from_string(content);
    if let Some(fields) = fields {
        message.fields = Some(json_to_rmpv(&fields)?);
    }
    message.to_wire(Some(signer))
}

pub fn decode_wire_message(bytes: &[u8]) -> Result<Message, LxmfError> {
    Message::from_wire(bytes)
}

pub fn json_to_rmpv(value: &JsonValue) -> Result<Value, LxmfError> {
    let encoded = rmp_serde::to_vec(value).map_err(|err| LxmfError::Encode(err.to_string()))?;
    let mut cursor = Cursor::new(encoded);
    rmpv::decode::read_value(&mut cursor)
        .map_err(|err| LxmfError::Decode(err.to_string()))
}

pub fn rmpv_to_json(value: &Value) -> Option<JsonValue> {
    match value {
        Value::Nil => Some(JsonValue::Null),
        Value::Boolean(v) => Some(JsonValue::Bool(*v)),
        Value::Integer(v) => {
            if let Some(i) = v.as_i64() {
                Some(JsonValue::Number(i.into()))
            } else if let Some(u) = v.as_u64() {
                Some(JsonValue::Number(u.into()))
            } else {
                None
            }
        }
        Value::F32(v) => serde_json::Number::from_f64(f64::from(*v)).map(JsonValue::Number),
        Value::F64(v) => serde_json::Number::from_f64(*v).map(JsonValue::Number),
        Value::String(s) => s.as_str().map(|v| JsonValue::String(v.to_string())),
        Value::Binary(bytes) => Some(JsonValue::Array(
            bytes.iter().map(|b| JsonValue::Number((*b).into())).collect(),
        )),
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(rmpv_to_json(item)?);
            }
            Some(JsonValue::Array(out))
        }
        Value::Map(entries) => {
            let mut object = serde_json::Map::new();
            for (key, value) in entries {
                let key_str = match key {
                    Value::String(text) => text.as_str().map(|v| v.to_string()),
                    Value::Integer(int) => int.as_i64().map(|v| v.to_string()).or_else(|| {
                        int.as_u64().map(|v| v.to_string())
                    }),
                    other => Some(format!("{:?}", other)),
                }?;
                object.insert(key_str, rmpv_to_json(value)?);
            }
            Some(JsonValue::Object(object))
        }
        _ => None,
    }
}
