pub mod codec;
mod daemon;
pub mod http;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map as JsonMap, Value as JsonValue};

use crate::storage::messages::{MessageRecord, MessagesStore};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio::time::Duration;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct RpcRequest {
    pub id: u64,
    pub method: String,
    pub params: Option<JsonValue>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct RpcResponse {
    pub id: u64,
    pub result: Option<JsonValue>,
    pub error: Option<RpcError>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct RpcError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct InterfaceRecord {
    #[serde(rename = "type")]
    pub kind: String,
    pub enabled: bool,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct DeliveryPolicy {
    pub auth_required: bool,
    pub allowed_destinations: Vec<String>,
    pub denied_destinations: Vec<String>,
    pub ignored_destinations: Vec<String>,
    pub prioritised_destinations: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct PropagationState {
    pub enabled: bool,
    pub store_root: Option<String>,
    pub target_cost: u32,
    pub total_ingested: usize,
    pub last_ingest_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct StampPolicy {
    pub target_cost: u32,
    pub flexibility: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct TicketRecord {
    pub destination: String,
    pub ticket: String,
    pub expires_at: i64,
}

pub struct RpcDaemon {
    store: MessagesStore,
    identity_hash: String,
    delivery_destination_hash: Mutex<Option<String>>,
    events: broadcast::Sender<RpcEvent>,
    event_queue: Mutex<VecDeque<RpcEvent>>,
    peers: Mutex<HashMap<String, PeerRecord>>,
    interfaces: Mutex<Vec<InterfaceRecord>>,
    delivery_policy: Mutex<DeliveryPolicy>,
    propagation_state: Mutex<PropagationState>,
    propagation_payloads: Mutex<HashMap<String, String>>,
    paper_ingest_seen: Mutex<HashSet<String>>,
    stamp_policy: Mutex<StampPolicy>,
    ticket_cache: Mutex<HashMap<String, TicketRecord>>,
    outbound_bridge: Option<Arc<dyn OutboundBridge>>,
    announce_bridge: Option<Arc<dyn AnnounceBridge>>,
}

pub trait OutboundBridge: Send + Sync {
    fn deliver(&self, record: &MessageRecord) -> Result<(), std::io::Error>;
}

pub trait AnnounceBridge: Send + Sync {
    fn announce_now(&self) -> Result<(), std::io::Error>;
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct RpcEvent {
    pub event_type: String,
    pub payload: JsonValue,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PeerRecord {
    pub peer: String,
    pub last_seen: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub name_source: Option<String>,
    #[serde(default)]
    pub first_seen: i64,
    #[serde(default)]
    pub seen_count: u64,
}

#[derive(Debug, Deserialize)]
struct SendMessageParams {
    id: String,
    source: String,
    destination: String,
    #[serde(default)]
    title: String,
    content: String,
    fields: Option<JsonValue>,
}

#[derive(Debug, Deserialize)]
struct SendMessageV2Params {
    id: String,
    source: String,
    destination: String,
    #[serde(default)]
    title: String,
    content: String,
    fields: Option<JsonValue>,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    stamp_cost: Option<u32>,
    #[serde(default)]
    include_ticket: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct RecordReceiptParams {
    message_id: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct AnnounceReceivedParams {
    peer: String,
    timestamp: Option<i64>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    name_source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SetInterfacesParams {
    interfaces: Vec<InterfaceRecord>,
}

#[derive(Debug, Deserialize)]
struct PeerOpParams {
    peer: String,
}

#[derive(Debug, Deserialize)]
struct DeliveryPolicyParams {
    #[serde(default)]
    auth_required: Option<bool>,
    #[serde(default)]
    allowed_destinations: Option<Vec<String>>,
    #[serde(default)]
    denied_destinations: Option<Vec<String>>,
    #[serde(default)]
    ignored_destinations: Option<Vec<String>>,
    #[serde(default)]
    prioritised_destinations: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct PropagationEnableParams {
    enabled: bool,
    #[serde(default)]
    store_root: Option<String>,
    #[serde(default)]
    target_cost: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct PropagationIngestParams {
    #[serde(default)]
    transient_id: Option<String>,
    #[serde(default)]
    payload_hex: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PropagationFetchParams {
    transient_id: String,
}

#[derive(Debug, Deserialize)]
struct PaperIngestUriParams {
    uri: String,
}

#[derive(Debug, Deserialize)]
struct StampPolicySetParams {
    #[serde(default)]
    target_cost: Option<u32>,
    #[serde(default)]
    flexibility: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct TicketGenerateParams {
    destination: String,
    #[serde(default)]
    ttl_secs: Option<u64>,
}

fn merge_fields_with_options(
    fields: Option<JsonValue>,
    method: Option<String>,
    stamp_cost: Option<u32>,
    include_ticket: Option<bool>,
) -> Option<JsonValue> {
    let has_options = method.is_some() || stamp_cost.is_some() || include_ticket.is_some();
    if !has_options {
        return fields;
    }

    let mut root = match fields {
        Some(JsonValue::Object(map)) => map,
        Some(other) => {
            let mut map = JsonMap::new();
            map.insert("_fields_raw".into(), other);
            map
        }
        None => JsonMap::new(),
    };

    let mut lxmf = JsonMap::new();
    if let Some(value) = method {
        lxmf.insert("method".into(), JsonValue::String(value));
    }
    if let Some(value) = stamp_cost {
        lxmf.insert("stamp_cost".into(), json!(value));
    }
    if let Some(value) = include_ticket {
        lxmf.insert("include_ticket".into(), json!(value));
    }

    root.insert("_lxmf".into(), JsonValue::Object(lxmf));
    Some(JsonValue::Object(root))
}

fn now_i64() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_secs() as i64)
        .unwrap_or(0)
}

fn first_n_chars(input: &str, n: usize) -> Option<String> {
    if n == 0 {
        return Some(String::new());
    }
    let end = input
        .char_indices()
        .nth(n - 1)
        .map(|(idx, ch)| idx + ch.len_utf8())?;
    Some(input[..end].to_string())
}

fn clean_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn encode_hex(bytes: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let bytes = bytes.as_ref();
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

pub fn handle_framed_request(daemon: &RpcDaemon, bytes: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    daemon.handle_framed_request(bytes)
}
