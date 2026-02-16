pub mod codec;
mod daemon;
pub mod http;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

use crate::storage::messages::{AnnounceRecord, MessageRecord, MessagesStore};
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PropagationState {
    pub enabled: bool,
    pub store_root: Option<String>,
    pub target_cost: u32,
    pub total_ingested: usize,
    pub last_ingest_count: usize,
    #[serde(default)]
    pub sync_state: u32,
    #[serde(default)]
    pub sync_progress: f64,
    #[serde(default)]
    pub messages_received: usize,
    #[serde(default)]
    pub state_name: String,
    #[serde(default)]
    pub selected_node: Option<String>,
    #[serde(default)]
    pub max_messages: usize,
    #[serde(default)]
    pub last_sync_started: Option<i64>,
    #[serde(default)]
    pub last_sync_completed: Option<i64>,
    #[serde(default)]
    pub last_sync_error: Option<String>,
}

impl Default for PropagationState {
    fn default() -> Self {
        Self {
            enabled: false,
            store_root: None,
            target_cost: 0,
            total_ingested: 0,
            last_ingest_count: 0,
            sync_state: 0,
            sync_progress: 0.0,
            messages_received: 0,
            state_name: "idle".to_string(),
            selected_node: None,
            max_messages: 0,
            last_sync_started: None,
            last_sync_completed: None,
            last_sync_error: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct StampPolicy {
    pub target_cost: u32,
    pub flexibility: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct IncomingMessageLimits {
    pub delivery_per_transfer_limit_kb: u32,
    pub propagation_per_transfer_limit_kb: u32,
}

impl Default for IncomingMessageLimits {
    fn default() -> Self {
        Self {
            delivery_per_transfer_limit_kb: 1024,
            propagation_per_transfer_limit_kb: 1024,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct RmspServerRecord {
    pub destination_hash: String,
    #[serde(default)]
    pub identity_hash: Option<String>,
    #[serde(default)]
    pub public_key: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub coverage: Vec<String>,
    #[serde(default)]
    pub zoom_range: Vec<u32>,
    #[serde(default)]
    pub formats: Vec<String>,
    #[serde(default)]
    pub layers: Vec<String>,
    #[serde(default)]
    pub updated: Option<i64>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub hops: Option<u32>,
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct TicketRecord {
    pub destination: String,
    pub ticket: String,
    pub expires_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DeliveryTraceEntry {
    pub status: String,
    pub timestamp: i64,
    #[serde(default)]
    pub reason_code: Option<String>,
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
    outbound_propagation_node: Mutex<Option<String>>,
    incoming_message_limits: Mutex<IncomingMessageLimits>,
    known_peer_identities: Mutex<HashMap<String, String>>,
    known_announce_identities: Mutex<HashMap<String, String>>,
    rmsp_servers: Mutex<HashMap<String, RmspServerRecord>>,
    paper_ingest_seen: Mutex<HashSet<String>>,
    stamp_policy: Mutex<StampPolicy>,
    ticket_cache: Mutex<HashMap<String, TicketRecord>>,
    delivery_traces: Mutex<HashMap<String, Vec<DeliveryTraceEntry>>>,
    outbound_bridge: Option<Arc<dyn OutboundBridge>>,
    announce_bridge: Option<Arc<dyn AnnounceBridge>>,
}

pub trait OutboundBridge: Send + Sync {
    fn deliver(
        &self,
        record: &MessageRecord,
        options: &OutboundDeliveryOptions,
    ) -> Result<(), std::io::Error>;
}

pub trait AnnounceBridge: Send + Sync {
    fn announce_now(&self) -> Result<(), std::io::Error>;
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct OutboundDeliveryOptions {
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub stamp_cost: Option<u32>,
    #[serde(default)]
    pub include_ticket: bool,
    #[serde(default)]
    pub try_propagation_on_fail: bool,
    #[serde(default)]
    pub ticket: Option<String>,
    #[serde(default)]
    pub source_private_key: Option<String>,
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
    #[serde(default)]
    source_private_key: Option<String>,
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
    #[serde(default)]
    try_propagation_on_fail: Option<bool>,
    #[serde(default)]
    source_private_key: Option<String>,
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
    #[serde(default)]
    app_data_hex: Option<String>,
    #[serde(default)]
    capabilities: Option<Vec<String>>,
    #[serde(default)]
    rssi: Option<f64>,
    #[serde(default)]
    snr: Option<f64>,
    #[serde(default)]
    q: Option<f64>,
    #[serde(default)]
    aspect: Option<String>,
    #[serde(default)]
    hops: Option<u32>,
    #[serde(default)]
    interface: Option<String>,
    #[serde(default)]
    stamp_cost: Option<u32>,
    #[serde(default)]
    stamp_cost_flexibility: Option<u32>,
    #[serde(default)]
    peering_cost: Option<u32>,
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
struct RequestMessagesFromPropagationNodeParams {
    #[serde(default)]
    identity_private_key: Option<String>,
    #[serde(default)]
    max_messages: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct HasPathParams {
    destination: String,
}

#[derive(Debug, Deserialize)]
struct RequestPathParams {
    destination: String,
}

#[derive(Debug, Deserialize)]
struct EstablishLinkParams {
    destination: String,
    #[serde(default)]
    timeout_seconds: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct IncomingMessageSizeLimitParams {
    limit_kb: u32,
}

#[derive(Debug, Deserialize)]
struct SendReactionParams {
    destination: String,
    target_message_id: String,
    emoji: String,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    source_private_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SendLocationTelemetryParams {
    destination: String,
    location_json: String,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    source_private_key: Option<String>,
    #[serde(default)]
    icon_name: Option<String>,
    #[serde(default)]
    icon_fg_color: Option<String>,
    #[serde(default)]
    icon_bg_color: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SendTelemetryRequestParams {
    destination: String,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    source_private_key: Option<String>,
    #[serde(default)]
    timebase: Option<f64>,
    #[serde(default)]
    is_collector_request: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct StorePeerIdentityParams {
    identity_hash: String,
    public_key: String,
}

#[derive(Debug, Deserialize)]
struct RecallIdentityParams {
    destination_hash_hex: String,
}

#[derive(Debug, Deserialize)]
struct RestorePeerIdentityEntry {
    identity_hash: String,
    public_key: String,
}

#[derive(Debug, Deserialize)]
struct RestorePeerIdentitiesParams {
    peers: Vec<RestorePeerIdentityEntry>,
}

#[derive(Debug, Deserialize)]
struct BulkRestoreAnnounceIdentityEntry {
    destination_hash: String,
    public_key: String,
}

#[derive(Debug, Deserialize)]
struct BulkRestoreAnnounceIdentitiesParams {
    announces: Vec<BulkRestoreAnnounceIdentityEntry>,
}

#[derive(Debug, Deserialize)]
struct RmspAnnounceParams {
    destination_hash: String,
    #[serde(default)]
    identity_hash: Option<String>,
    #[serde(default)]
    public_key: Option<String>,
    #[serde(default)]
    app_data_hex: Option<String>,
    #[serde(default)]
    hops: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct RmspServersForGeohashParams {
    geohash: String,
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

#[derive(Debug, Deserialize, Default)]
struct ListAnnouncesParams {
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    before_ts: Option<i64>,
    #[serde(default)]
    cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SetOutboundPropagationNodeParams {
    #[serde(default)]
    peer: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct RequestAlternativePropagationRelayParams {
    #[serde(default)]
    exclude_relays: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MessageDeliveryTraceParams {
    message_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
struct PropagationNodeRecord {
    peer: String,
    #[serde(default)]
    name: Option<String>,
    last_seen: i64,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    aspect: Option<String>,
    selected: bool,
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

fn normalize_capabilities(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for value in values {
        let normalized = value.trim().to_ascii_lowercase();
        if normalized.is_empty() || !seen.insert(normalized.clone()) {
            continue;
        }
        out.push(normalized);
    }
    out
}

fn parse_capabilities_from_app_data_hex(app_data_hex: Option<&str>) -> Vec<String> {
    let Some(raw_hex) = app_data_hex
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Vec::new();
    };
    let Ok(app_data) = hex::decode(raw_hex) else {
        return Vec::new();
    };
    if app_data.is_empty() {
        return Vec::new();
    }

    let Ok(value) = rmp_serde::from_slice::<serde_json::Value>(&app_data) else {
        return Vec::new();
    };
    let Some(entries) = value.as_array() else {
        return Vec::new();
    };

    let mut capabilities = Vec::new();

    // Current LXMF propagation-node announces encode node state as the third
    // tuple element (index 2). Treat active state as propagation capability.
    if entries
        .get(2)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        capabilities.push("propagation".to_string());
    }

    for entry in entries {
        if let Some(mut parsed) = extract_capabilities_from_json(entry) {
            capabilities.append(&mut parsed);
        }
    }

    normalize_capabilities(capabilities)
}

fn extract_capabilities_from_json(value: &serde_json::Value) -> Option<Vec<String>> {
    if let Some(array) = value.as_array() {
        return Some(normalize_capabilities(
            array
                .iter()
                .filter_map(|entry| entry.as_str().map(ToOwned::to_owned))
                .collect(),
        ));
    }

    let object = value.as_object()?;
    let mut out = Vec::new();
    if object
        .get("node_state")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        out.push("propagation".to_string());
    }
    if object
        .get("enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        out.push("propagation".to_string());
    }
    for key in ["caps", "capabilities"] {
        if let Some(array) = object.get(key).and_then(serde_json::Value::as_array) {
            out.extend(
                array
                    .iter()
                    .filter_map(|entry| entry.as_str().map(ToOwned::to_owned)),
            );
        }
    }

    if out.is_empty() {
        None
    } else {
        Some(normalize_capabilities(out))
    }
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
