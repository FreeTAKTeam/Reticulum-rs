use super::*;

impl RpcDaemon {
    pub fn with_store(store: MessagesStore, identity_hash: String) -> Self {
        let (events, _rx) = broadcast::channel(64);
        Self {
            store,
            identity_hash,
            delivery_destination_hash: Mutex::new(None),
            events,
            event_queue: Mutex::new(VecDeque::new()),
            peers: Mutex::new(HashMap::new()),
            interfaces: Mutex::new(Vec::new()),
            delivery_policy: Mutex::new(DeliveryPolicy::default()),
            propagation_state: Mutex::new(PropagationState::default()),
            propagation_payloads: Mutex::new(HashMap::new()),
            outbound_propagation_node: Mutex::new(None),
            incoming_message_limits: Mutex::new(IncomingMessageLimits::default()),
            known_peer_identities: Mutex::new(HashMap::new()),
            known_announce_identities: Mutex::new(HashMap::new()),
            rmsp_servers: Mutex::new(HashMap::new()),
            paper_ingest_seen: Mutex::new(HashSet::new()),
            stamp_policy: Mutex::new(StampPolicy::default()),
            ticket_cache: Mutex::new(HashMap::new()),
            delivery_traces: Mutex::new(HashMap::new()),
            outbound_bridge: None,
            announce_bridge: None,
        }
    }

    pub fn with_store_and_bridge(
        store: MessagesStore,
        identity_hash: String,
        outbound_bridge: Arc<dyn OutboundBridge>,
    ) -> Self {
        let (events, _rx) = broadcast::channel(64);
        Self {
            store,
            identity_hash,
            delivery_destination_hash: Mutex::new(None),
            events,
            event_queue: Mutex::new(VecDeque::new()),
            peers: Mutex::new(HashMap::new()),
            interfaces: Mutex::new(Vec::new()),
            delivery_policy: Mutex::new(DeliveryPolicy::default()),
            propagation_state: Mutex::new(PropagationState::default()),
            propagation_payloads: Mutex::new(HashMap::new()),
            outbound_propagation_node: Mutex::new(None),
            incoming_message_limits: Mutex::new(IncomingMessageLimits::default()),
            known_peer_identities: Mutex::new(HashMap::new()),
            known_announce_identities: Mutex::new(HashMap::new()),
            rmsp_servers: Mutex::new(HashMap::new()),
            paper_ingest_seen: Mutex::new(HashSet::new()),
            stamp_policy: Mutex::new(StampPolicy::default()),
            ticket_cache: Mutex::new(HashMap::new()),
            delivery_traces: Mutex::new(HashMap::new()),
            outbound_bridge: Some(outbound_bridge),
            announce_bridge: None,
        }
    }

    pub fn with_store_and_bridges(
        store: MessagesStore,
        identity_hash: String,
        outbound_bridge: Option<Arc<dyn OutboundBridge>>,
        announce_bridge: Option<Arc<dyn AnnounceBridge>>,
    ) -> Self {
        let (events, _rx) = broadcast::channel(64);
        Self {
            store,
            identity_hash,
            delivery_destination_hash: Mutex::new(None),
            events,
            event_queue: Mutex::new(VecDeque::new()),
            peers: Mutex::new(HashMap::new()),
            interfaces: Mutex::new(Vec::new()),
            delivery_policy: Mutex::new(DeliveryPolicy::default()),
            propagation_state: Mutex::new(PropagationState::default()),
            propagation_payloads: Mutex::new(HashMap::new()),
            outbound_propagation_node: Mutex::new(None),
            incoming_message_limits: Mutex::new(IncomingMessageLimits::default()),
            known_peer_identities: Mutex::new(HashMap::new()),
            known_announce_identities: Mutex::new(HashMap::new()),
            rmsp_servers: Mutex::new(HashMap::new()),
            paper_ingest_seen: Mutex::new(HashSet::new()),
            stamp_policy: Mutex::new(StampPolicy::default()),
            ticket_cache: Mutex::new(HashMap::new()),
            delivery_traces: Mutex::new(HashMap::new()),
            outbound_bridge,
            announce_bridge,
        }
    }

    pub fn test_instance() -> Self {
        let store = MessagesStore::in_memory().expect("in-memory store");
        Self::with_store(store, "test-identity".into())
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn test_instance_with_identity(identity: impl Into<String>) -> Self {
        let store = MessagesStore::in_memory().expect("in-memory store");
        Self::with_store(store, identity.into())
    }

    pub fn set_delivery_destination_hash(&self, hash: Option<String>) {
        let mut guard = self
            .delivery_destination_hash
            .lock()
            .expect("delivery_destination_hash mutex poisoned");
        *guard = hash.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
    }

    pub fn replace_interfaces(&self, interfaces: Vec<InterfaceRecord>) {
        let mut guard = self.interfaces.lock().expect("interfaces mutex poisoned");
        *guard = interfaces;
    }

    pub fn set_propagation_state(
        &self,
        enabled: bool,
        store_root: Option<String>,
        target_cost: u32,
    ) {
        let mut guard = self
            .propagation_state
            .lock()
            .expect("propagation mutex poisoned");
        guard.enabled = enabled;
        guard.store_root = store_root;
        guard.target_cost = target_cost;
        if guard.state_name.is_empty() {
            guard.state_name = "idle".to_string();
        }
    }

    pub fn update_propagation_sync_state<F>(&self, updater: F)
    where
        F: FnOnce(&mut PropagationState),
    {
        {
            let mut guard = self
                .propagation_state
                .lock()
                .expect("propagation mutex poisoned");
            updater(&mut guard);
        }
        self.emit_propagation_state_event();
    }

    fn store_inbound_record(&self, record: MessageRecord) -> Result<(), std::io::Error> {
        self.store
            .insert_message(&record)
            .map_err(std::io::Error::other)?;
        let timestamp_ms = timestamp_to_ms(record.timestamp);
        let event = RpcEvent {
            event_type: "inbound".into(),
            payload: json!({
                "message": record,
                "timestamp_ms": timestamp_ms,
            }),
        };
        self.push_event(event.clone());
        let _ = self.events.send(event);
        Ok(())
    }

    pub fn accept_inbound(&self, record: MessageRecord) -> Result<(), std::io::Error> {
        self.store_inbound_record(record)
    }

    pub fn accept_announce(&self, peer: String, timestamp: i64) -> Result<(), std::io::Error> {
        self.accept_announce_with_metadata(
            peer, timestamp, None, None, None, None, None, None, None, None, None, None, None,
            None, None,
        )
    }

    pub fn accept_announce_with_details(
        &self,
        peer: String,
        timestamp: i64,
        name: Option<String>,
        name_source: Option<String>,
    ) -> Result<(), std::io::Error> {
        self.accept_announce_with_metadata(
            peer,
            timestamp,
            name,
            name_source,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn accept_announce_with_metadata(
        &self,
        peer: String,
        timestamp: i64,
        name: Option<String>,
        name_source: Option<String>,
        app_data_hex: Option<String>,
        capabilities: Option<Vec<String>>,
        rssi: Option<f64>,
        snr: Option<f64>,
        q: Option<f64>,
        aspect: Option<String>,
        hops: Option<u32>,
        interface: Option<String>,
        stamp_cost: Option<u32>,
        stamp_cost_flexibility: Option<u32>,
        peering_cost: Option<u32>,
    ) -> Result<(), std::io::Error> {
        let record = self.upsert_peer(peer, timestamp, name, name_source);
        let capability_list = if let Some(caps) = capabilities {
            normalize_capabilities(caps)
        } else {
            parse_capabilities_from_app_data_hex(app_data_hex.as_deref())
        };
        let inferred_aspect = clean_optional_text(aspect).or_else(|| {
            capability_list
                .iter()
                .any(|value| value == "propagation")
                .then_some("lxmf.propagation".to_string())
        });
        let (pn_stamp_cost, pn_stamp_flexibility, pn_peering_cost) =
            parse_propagation_costs_from_app_data(app_data_hex.as_deref());
        let app_data_hex_clean = clean_optional_text(app_data_hex.clone());

        let announce_record = AnnounceRecord {
            id: format!(
                "announce-{}-{}-{}",
                record.last_seen, record.peer, record.seen_count
            ),
            peer: record.peer.clone(),
            timestamp: record.last_seen,
            name: record.name.clone(),
            name_source: record.name_source.clone(),
            first_seen: record.first_seen,
            seen_count: record.seen_count,
            app_data_hex: app_data_hex_clean,
            capabilities: capability_list.clone(),
            rssi,
            snr,
            q,
            aspect: inferred_aspect.clone(),
            hops,
            interface: clean_optional_text(interface),
            stamp_cost: stamp_cost.or(pn_stamp_cost),
            stamp_cost_flexibility: stamp_cost_flexibility.or(pn_stamp_flexibility),
            peering_cost: peering_cost.or(pn_peering_cost),
        };
        self.store
            .insert_announce(&announce_record)
            .map_err(std::io::Error::other)?;
        if announce_record.aspect.as_deref() == Some("rmsp.maps") {
            self.upsert_rmsp_server_from_announce(
                &announce_record.peer,
                app_data_hex.as_deref(),
                announce_record.hops,
                timestamp,
            );
        }
        let timestamp_ms = timestamp_to_ms(record.last_seen);

        let event = RpcEvent {
            event_type: "announce_received".into(),
            payload: json!({
                "id": announce_record.id,
                "peer": record.peer,
                "timestamp": record.last_seen,
                "timestamp_ms": timestamp_ms,
                "name": record.name,
                "name_source": record.name_source,
                "first_seen": record.first_seen,
                "seen_count": record.seen_count,
                "app_data_hex": announce_record.app_data_hex,
                "capabilities": capability_list,
                "rssi": rssi,
                "snr": snr,
                "q": q,
                "aspect": announce_record.aspect,
                "hops": announce_record.hops,
                "interface": announce_record.interface,
                "stamp_cost": announce_record.stamp_cost,
                "stamp_cost_flexibility": announce_record.stamp_cost_flexibility,
                "peering_cost": announce_record.peering_cost,
            }),
        };
        self.push_event(event.clone());
        let _ = self.events.send(event);
        Ok(())
    }

    fn upsert_peer(
        &self,
        peer: String,
        timestamp: i64,
        name: Option<String>,
        name_source: Option<String>,
    ) -> PeerRecord {
        let cleaned_name = clean_optional_text(name);
        let cleaned_name_source = clean_optional_text(name_source);

        let mut guard = self.peers.lock().expect("peers mutex poisoned");
        if let Some(existing) = guard.get_mut(&peer) {
            existing.last_seen = timestamp;
            existing.seen_count = existing.seen_count.saturating_add(1);
            if let Some(name) = cleaned_name {
                existing.name = Some(name);
                existing.name_source = cleaned_name_source;
            }
            return existing.clone();
        }

        let record = PeerRecord {
            peer: peer.clone(),
            last_seen: timestamp,
            name: cleaned_name,
            name_source: cleaned_name_source,
            first_seen: timestamp,
            seen_count: 1,
        };
        guard.insert(peer, record.clone());
        record
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn accept_inbound_for_test(
        &self,
        record: MessageRecord,
    ) -> Result<(), std::io::Error> {
        self.store_inbound_record(record)
    }

    pub fn handle_rpc(&self, request: RpcRequest) -> Result<RpcResponse, std::io::Error> {
        match request.method.as_str() {
            "status" => Ok(RpcResponse {
                id: request.id,
                result: Some(json!({
                    "identity_hash": self.identity_hash,
                    "delivery_destination_hash": self.local_delivery_hash(),
                    "running": true
                })),
                error: None,
            }),
            "daemon_status_ex" => {
                let peer_count = self.peers.lock().expect("peers mutex poisoned").len();
                let interfaces = self
                    .interfaces
                    .lock()
                    .expect("interfaces mutex poisoned")
                    .clone();
                let message_count = self
                    .store
                    .list_messages(10_000, None)
                    .map_err(std::io::Error::other)?
                    .len();
                let delivery_policy = self
                    .delivery_policy
                    .lock()
                    .expect("policy mutex poisoned")
                    .clone();
                let propagation = self
                    .propagation_state
                    .lock()
                    .expect("propagation mutex poisoned")
                    .clone();
                let stamp_policy = self
                    .stamp_policy
                    .lock()
                    .expect("stamp mutex poisoned")
                    .clone();
                let incoming_limits = self
                    .incoming_message_limits
                    .lock()
                    .expect("incoming limits mutex poisoned")
                    .clone();
                let rmsp_server_count = self
                    .rmsp_servers
                    .lock()
                    .expect("rmsp server mutex poisoned")
                    .len();

                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "identity_hash": self.identity_hash,
                        "delivery_destination_hash": self.local_delivery_hash(),
                        "running": true,
                        "peer_count": peer_count,
                        "message_count": message_count,
                        "interface_count": interfaces.len(),
                        "interfaces": interfaces,
                        "delivery_policy": delivery_policy,
                        "propagation": propagation,
                        "stamp_policy": stamp_policy,
                        "incoming_message_limits": incoming_limits,
                        "rmsp_server_count": rmsp_server_count,
                        "capabilities": Self::capabilities(),
                    })),
                    error: None,
                })
            }
            "list_messages" => {
                let items = self
                    .store
                    .list_messages(100, None)
                    .map_err(std::io::Error::other)?;
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "messages": items,
                        "meta": self.response_meta(),
                    })),
                    error: None,
                })
            }
            "list_announces" => {
                let parsed = request
                    .params
                    .map(serde_json::from_value::<ListAnnouncesParams>)
                    .transpose()
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?
                    .unwrap_or_default();
                let limit = parsed.limit.unwrap_or(200).clamp(1, 5000);
                let (before_ts, before_id) = match parsed.before_ts {
                    Some(timestamp) => (Some(timestamp), None),
                    None => parse_announce_cursor(parsed.cursor.as_deref()).unwrap_or((None, None)),
                };
                let items = self
                    .store
                    .list_announces(limit, before_ts, before_id.as_deref())
                    .map_err(std::io::Error::other)?;
                let next_cursor = if items.len() >= limit {
                    items
                        .last()
                        .map(|record| format!("{}:{}", record.timestamp, record.id))
                } else {
                    None
                };
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "announces": items,
                        "next_cursor": next_cursor,
                        "meta": self.response_meta(),
                    })),
                    error: None,
                })
            }
            "list_peers" => {
                let mut peers = self
                    .peers
                    .lock()
                    .expect("peers mutex poisoned")
                    .values()
                    .cloned()
                    .collect::<Vec<_>>();
                peers.sort_by(|a, b| {
                    b.last_seen
                        .cmp(&a.last_seen)
                        .then_with(|| a.peer.cmp(&b.peer))
                });
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "peers": peers,
                        "meta": self.response_meta(),
                    })),
                    error: None,
                })
            }
            "list_interfaces" => {
                let interfaces = self
                    .interfaces
                    .lock()
                    .expect("interfaces mutex poisoned")
                    .clone();
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "interfaces": interfaces,
                        "meta": self.response_meta(),
                    })),
                    error: None,
                })
            }
            "set_interfaces" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: SetInterfacesParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;

                for iface in &parsed.interfaces {
                    if iface.kind.trim().is_empty() {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "interface type is required",
                        ));
                    }
                    if iface.kind == "tcp_client" && (iface.host.is_none() || iface.port.is_none())
                    {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "tcp_client requires host and port",
                        ));
                    }
                    if iface.kind == "tcp_server" && iface.port.is_none() {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "tcp_server requires port",
                        ));
                    }
                }

                {
                    let mut guard = self.interfaces.lock().expect("interfaces mutex poisoned");
                    *guard = parsed.interfaces.clone();
                }

                let event = RpcEvent {
                    event_type: "interfaces_updated".into(),
                    payload: json!({ "interfaces": parsed.interfaces }),
                };
                self.push_event(event.clone());
                let _ = self.events.send(event);

                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "updated": true })),
                    error: None,
                })
            }
            "reload_config" => {
                let timestamp = now_i64();
                let event = RpcEvent {
                    event_type: "config_reloaded".into(),
                    payload: json!({ "timestamp": timestamp }),
                };
                self.push_event(event.clone());
                let _ = self.events.send(event);
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "reloaded": true, "timestamp": timestamp })),
                    error: None,
                })
            }
            "peer_sync" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: PeerOpParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;

                let timestamp = now_i64();
                let record = self.upsert_peer(parsed.peer, timestamp, None, None);
                let event = RpcEvent {
                    event_type: "peer_sync".into(),
                    payload: json!({
                        "peer": record.peer.clone(),
                        "timestamp": timestamp,
                        "name": record.name.clone(),
                        "name_source": record.name_source.clone(),
                        "first_seen": record.first_seen,
                        "seen_count": record.seen_count,
                    }),
                };
                self.push_event(event.clone());
                let _ = self.events.send(event);

                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "peer": record.peer, "synced": true })),
                    error: None,
                })
            }
            "peer_unpeer" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: PeerOpParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;

                let removed = {
                    let mut guard = self.peers.lock().expect("peers mutex poisoned");
                    guard.remove(&parsed.peer).is_some()
                };
                let event = RpcEvent {
                    event_type: "peer_unpeer".into(),
                    payload: json!({ "peer": parsed.peer, "removed": removed }),
                };
                self.push_event(event.clone());
                let _ = self.events.send(event);
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "removed": removed })),
                    error: None,
                })
            }
            "send_message" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: SendMessageParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;

                self.store_outbound(
                    request.id,
                    parsed.id,
                    parsed.source,
                    parsed.destination,
                    parsed.title,
                    parsed.content,
                    parsed.fields,
                    None,
                    None,
                    None,
                    None,
                    parsed.source_private_key,
                )
            }
            "send_message_v2" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: SendMessageV2Params = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;

                self.store_outbound(
                    request.id,
                    parsed.id,
                    parsed.source,
                    parsed.destination,
                    parsed.title,
                    parsed.content,
                    parsed.fields,
                    parsed.method,
                    parsed.stamp_cost,
                    parsed.include_ticket,
                    parsed.try_propagation_on_fail,
                    parsed.source_private_key,
                )
            }
            "receive_message" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: SendMessageParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let timestamp = now_i64();
                let record = MessageRecord {
                    id: parsed.id.clone(),
                    source: parsed.source,
                    destination: parsed.destination,
                    title: parsed.title,
                    content: parsed.content,
                    timestamp,
                    direction: "in".into(),
                    fields: parsed.fields,
                    receipt_status: None,
                };
                self.store_inbound_record(record)?;
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "message_id": parsed.id })),
                    error: None,
                })
            }
            "record_receipt" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: RecordReceiptParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                self.store
                    .update_receipt_status(&parsed.message_id, &parsed.status)
                    .map_err(std::io::Error::other)?;
                let message_id = parsed.message_id;
                let status = parsed.status;
                self.append_delivery_trace(&message_id, status.clone());
                let reason_code = delivery_reason_code(&status);
                let event = RpcEvent {
                    event_type: "receipt".into(),
                    payload: json!({
                        "message_id": message_id,
                        "status": status,
                        "reason_code": reason_code,
                    }),
                };
                self.push_event(event.clone());
                let _ = self.events.send(event);
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "message_id": message_id,
                        "status": status,
                        "reason_code": reason_code,
                    })),
                    error: None,
                })
            }
            "message_delivery_trace" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: MessageDeliveryTraceParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let traces = self
                    .delivery_traces
                    .lock()
                    .expect("delivery traces mutex poisoned")
                    .get(parsed.message_id.as_str())
                    .cloned()
                    .unwrap_or_default();
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "message_id": parsed.message_id,
                        "transitions": traces,
                        "meta": self.response_meta(),
                    })),
                    error: None,
                })
            }
            "get_delivery_policy" => {
                let policy = self
                    .delivery_policy
                    .lock()
                    .expect("policy mutex poisoned")
                    .clone();
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "policy": policy })),
                    error: None,
                })
            }
            "set_delivery_policy" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: DeliveryPolicyParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;

                let policy = {
                    let mut guard = self.delivery_policy.lock().expect("policy mutex poisoned");
                    if let Some(value) = parsed.auth_required {
                        guard.auth_required = value;
                    }
                    if let Some(value) = parsed.allowed_destinations {
                        guard.allowed_destinations = value;
                    }
                    if let Some(value) = parsed.denied_destinations {
                        guard.denied_destinations = value;
                    }
                    if let Some(value) = parsed.ignored_destinations {
                        guard.ignored_destinations = value;
                    }
                    if let Some(value) = parsed.prioritised_destinations {
                        guard.prioritised_destinations = value;
                    }
                    guard.clone()
                };

                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "policy": policy })),
                    error: None,
                })
            }
            "propagation_status" => {
                let state = self
                    .propagation_state
                    .lock()
                    .expect("propagation mutex poisoned")
                    .clone();
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "propagation": state })),
                    error: None,
                })
            }
            "propagation_enable" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: PropagationEnableParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;

                let state = {
                    let mut guard = self
                        .propagation_state
                        .lock()
                        .expect("propagation mutex poisoned");
                    guard.enabled = parsed.enabled;
                    if parsed.store_root.is_some() {
                        guard.store_root = parsed.store_root;
                    }
                    if let Some(cost) = parsed.target_cost {
                        guard.target_cost = cost;
                    }
                    guard.clone()
                };
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "propagation": state })),
                    error: None,
                })
            }
            "propagation_ingest" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: PropagationIngestParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;

                let payload_hex = parsed.payload_hex.unwrap_or_default();
                let transient_id = parsed.transient_id.unwrap_or_else(|| {
                    let mut hasher = Sha256::new();
                    hasher.update(payload_hex.as_bytes());
                    encode_hex(hasher.finalize())
                });

                if !payload_hex.is_empty() {
                    self.propagation_payloads
                        .lock()
                        .expect("propagation payload mutex poisoned")
                        .insert(transient_id.clone(), payload_hex);
                }

                let state = {
                    let mut guard = self
                        .propagation_state
                        .lock()
                        .expect("propagation mutex poisoned");
                    let ingested_count = usize::from(!transient_id.is_empty());
                    guard.last_ingest_count = ingested_count;
                    guard.total_ingested += ingested_count;
                    guard.clone()
                };

                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "ingested_count": state.last_ingest_count,
                        "transient_id": transient_id,
                    })),
                    error: None,
                })
            }
            "propagation_fetch" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: PropagationFetchParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;

                let payload = self
                    .propagation_payloads
                    .lock()
                    .expect("propagation payload mutex poisoned")
                    .get(&parsed.transient_id)
                    .cloned()
                    .ok_or_else(|| {
                        std::io::Error::new(std::io::ErrorKind::NotFound, "transient_id not found")
                    })?;

                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "transient_id": parsed.transient_id,
                        "payload_hex": payload,
                    })),
                    error: None,
                })
            }
            "request_messages_from_propagation_node" => {
                let parsed = request
                    .params
                    .map(serde_json::from_value::<RequestMessagesFromPropagationNodeParams>)
                    .transpose()
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let max_messages = parsed
                    .as_ref()
                    .and_then(|value| value.max_messages)
                    .unwrap_or(256)
                    .clamp(1, 4096) as usize;
                let identity_private_key = parsed.and_then(|value| value.identity_private_key);
                let state = self
                    .request_messages_from_propagation_node(max_messages, identity_private_key)?;
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(state),
                    error: None,
                })
            }
            "get_propagation_state" => {
                let state = self
                    .propagation_state
                    .lock()
                    .expect("propagation mutex poisoned")
                    .clone();
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "success": true,
                        "state": state.sync_state,
                        "state_name": state.state_name,
                        "progress": state.sync_progress,
                        "messages_received": state.messages_received,
                        "selected_node": state.selected_node,
                        "max_messages": state.max_messages,
                        "last_sync_started": state.last_sync_started,
                        "last_sync_completed": state.last_sync_completed,
                        "last_sync_error": state.last_sync_error,
                    })),
                    error: None,
                })
            }
            "set_incoming_message_size_limit" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: IncomingMessageSizeLimitParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let limit_kb = parsed.limit_kb.max(1);
                {
                    let mut limits = self
                        .incoming_message_limits
                        .lock()
                        .expect("incoming limits mutex poisoned");
                    limits.delivery_per_transfer_limit_kb = limit_kb;
                    limits.propagation_per_transfer_limit_kb = limit_kb;
                }
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "success": true, "limit_kb": limit_kb })),
                    error: None,
                })
            }
            "get_incoming_message_size_limit" => {
                let limits = self
                    .incoming_message_limits
                    .lock()
                    .expect("incoming limits mutex poisoned")
                    .clone();
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "success": true,
                        "limit_kb": limits.delivery_per_transfer_limit_kb,
                        "delivery_per_transfer_limit_kb": limits.delivery_per_transfer_limit_kb,
                        "propagation_per_transfer_limit_kb": limits.propagation_per_transfer_limit_kb
                    })),
                    error: None,
                })
            }
            "has_path" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: HasPathParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let destination = normalize_hash_hex(&parsed.destination);
                let has_path = destination
                    .as_deref()
                    .map(|hash| self.knows_destination(hash))
                    .unwrap_or(false);
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "success": true,
                        "destination": destination,
                        "has_path": has_path,
                    })),
                    error: None,
                })
            }
            "request_path" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: RequestPathParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let destination = normalize_hash_hex(&parsed.destination).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "destination must be a 16-byte or 32-byte hex hash",
                    )
                })?;
                let has_path = self.knows_destination(&destination);
                let event = RpcEvent {
                    event_type: "path_requested".into(),
                    payload: json!({
                        "destination": destination,
                        "has_path": has_path,
                        "timestamp": now_i64(),
                    }),
                };
                self.push_event(event.clone());
                let _ = self.events.send(event);
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "success": true,
                        "destination": destination,
                        "has_path": has_path,
                    })),
                    error: None,
                })
            }
            "establish_link" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: EstablishLinkParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let destination = normalize_hash_hex(&parsed.destination).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "destination must be a 16-byte or 32-byte hex hash",
                    )
                })?;
                let timeout = parsed.timeout_seconds.unwrap_or(10.0).clamp(0.1, 300.0);
                let has_path = self.knows_destination(&destination);
                let response = if has_path {
                    json!({
                        "success": true,
                        "link_active": true,
                        "already_existed": false,
                        "destination": destination,
                        "timeout_seconds": timeout,
                    })
                } else {
                    json!({
                        "success": false,
                        "link_active": false,
                        "destination": destination,
                        "timeout_seconds": timeout,
                        "error": "No path to destination",
                    })
                };
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(response),
                    error: None,
                })
            }
            "send_reaction" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: SendReactionParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let destination = normalize_hash_hex(&parsed.destination).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "destination must be a 16-byte or 32-byte hex hash",
                    )
                })?;
                let source = self.resolve_source_hash(parsed.source, parsed.source_private_key)?;
                let sender_hash = source.clone();
                let fields = reaction_fields_json(
                    parsed.target_message_id.clone(),
                    parsed.emoji.clone(),
                    sender_hash,
                )?;
                let message_id = generated_message_id("reaction");
                let response = self.store_outbound(
                    request.id,
                    message_id.clone(),
                    source,
                    destination.clone(),
                    String::new(),
                    String::new(),
                    Some(fields),
                    Some("opportunistic".to_string()),
                    None,
                    None,
                    None,
                    None,
                )?;
                if response.error.is_none() {
                    return Ok(RpcResponse {
                        id: request.id,
                        result: Some(json!({
                            "success": true,
                            "message_id": message_id,
                            "destination": destination,
                            "target_message_id": parsed.target_message_id,
                            "emoji": parsed.emoji,
                        })),
                        error: None,
                    });
                }
                Ok(response)
            }
            "send_location_telemetry" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: SendLocationTelemetryParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let destination = normalize_hash_hex(&parsed.destination).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "destination must be a 16-byte or 32-byte hex hash",
                    )
                })?;
                let source = self.resolve_source_hash(parsed.source, parsed.source_private_key)?;
                let location_value: serde_json::Value = serde_json::from_str(&parsed.location_json)
                    .map_err(|err| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            format!("invalid location_json: {err}"),
                        )
                    })?;
                let fields = location_telemetry_fields_json(
                    &location_value,
                    parsed.icon_name,
                    parsed.icon_fg_color,
                    parsed.icon_bg_color,
                )?;
                let message_id = generated_message_id("telemetry");
                let response = self.store_outbound(
                    request.id,
                    message_id.clone(),
                    source,
                    destination.clone(),
                    String::new(),
                    String::new(),
                    Some(fields),
                    Some("opportunistic".to_string()),
                    None,
                    None,
                    None,
                    None,
                )?;
                if response.error.is_none() {
                    return Ok(RpcResponse {
                        id: request.id,
                        result: Some(json!({
                            "success": true,
                            "message_id": message_id,
                            "destination": destination,
                            "timestamp_ms": timestamp_to_ms(now_i64()),
                        })),
                        error: None,
                    });
                }
                Ok(response)
            }
            "send_telemetry_request" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: SendTelemetryRequestParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let destination = normalize_hash_hex(&parsed.destination).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "destination must be a 16-byte or 32-byte hex hash",
                    )
                })?;
                let source = self.resolve_source_hash(parsed.source, parsed.source_private_key)?;
                let fields = telemetry_request_fields_json(
                    parsed.timebase,
                    parsed.is_collector_request.unwrap_or(true),
                )?;
                let message_id = generated_message_id("telemetry-request");
                let response = self.store_outbound(
                    request.id,
                    message_id.clone(),
                    source,
                    destination.clone(),
                    String::new(),
                    String::new(),
                    Some(fields),
                    Some("opportunistic".to_string()),
                    None,
                    None,
                    None,
                    None,
                )?;
                if response.error.is_none() {
                    return Ok(RpcResponse {
                        id: request.id,
                        result: Some(json!({
                            "success": true,
                            "message_id": message_id,
                            "destination": destination,
                        })),
                        error: None,
                    });
                }
                Ok(response)
            }
            "store_peer_identity" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: StorePeerIdentityParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let identity_hash = normalize_hash_hex(&parsed.identity_hash).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "identity_hash must be a hex hash",
                    )
                })?;
                let public_key =
                    normalize_public_key_material(&parsed.public_key).ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "public_key must be hex or base64 encoded",
                        )
                    })?;
                self.known_peer_identities
                    .lock()
                    .expect("peer identities mutex poisoned")
                    .insert(identity_hash.clone(), public_key.clone());
                if let Some(destination_hash) =
                    derive_lxmf_delivery_destination_hash_from_identity_hash(&identity_hash)
                {
                    self.known_announce_identities
                        .lock()
                        .expect("announce identities mutex poisoned")
                        .insert(destination_hash, public_key.clone());
                }
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "success": true,
                        "identity_hash": identity_hash,
                        "public_key": public_key,
                    })),
                    error: None,
                })
            }
            "restore_all_peer_identities" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: RestorePeerIdentitiesParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let mut success_count = 0usize;
                let mut errors = Vec::new();
                for (idx, entry) in parsed.peers.iter().enumerate() {
                    let Some(identity_hash) = normalize_hash_hex(&entry.identity_hash) else {
                        errors.push(format!("peer {idx}: invalid identity_hash"));
                        continue;
                    };
                    let Some(public_key) = normalize_public_key_material(&entry.public_key) else {
                        errors.push(format!("peer {idx}: invalid public_key"));
                        continue;
                    };
                    self.known_peer_identities
                        .lock()
                        .expect("peer identities mutex poisoned")
                        .insert(identity_hash.clone(), public_key.clone());
                    if let Some(destination_hash) =
                        derive_lxmf_delivery_destination_hash_from_identity_hash(&identity_hash)
                    {
                        self.known_announce_identities
                            .lock()
                            .expect("announce identities mutex poisoned")
                            .insert(destination_hash, public_key);
                    }
                    success_count += 1;
                }
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "success_count": success_count,
                        "errors": errors,
                    })),
                    error: None,
                })
            }
            "bulk_restore_peer_identities" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: RestorePeerIdentitiesParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let mut success_count = 0usize;
                let mut errors = Vec::new();
                for (idx, entry) in parsed.peers.iter().enumerate() {
                    let Some(identity_hash) = normalize_hash_hex(&entry.identity_hash) else {
                        errors.push(format!("peer {idx}: invalid identity_hash"));
                        continue;
                    };
                    let Some(public_key) = normalize_public_key_material(&entry.public_key) else {
                        errors.push(format!("peer {idx}: invalid public_key"));
                        continue;
                    };
                    self.known_peer_identities
                        .lock()
                        .expect("peer identities mutex poisoned")
                        .insert(identity_hash.clone(), public_key.clone());
                    if let Some(destination_hash) =
                        derive_lxmf_delivery_destination_hash_from_identity_hash(&identity_hash)
                    {
                        self.known_announce_identities
                            .lock()
                            .expect("announce identities mutex poisoned")
                            .insert(destination_hash, public_key);
                    }
                    success_count += 1;
                }
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "success_count": success_count,
                        "errors": errors,
                    })),
                    error: None,
                })
            }
            "bulk_restore_announce_identities" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: BulkRestoreAnnounceIdentitiesParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let mut success_count = 0usize;
                let mut errors = Vec::new();
                for (idx, entry) in parsed.announces.iter().enumerate() {
                    let Some(destination_hash) = normalize_hash_hex(&entry.destination_hash) else {
                        errors.push(format!("announce {idx}: invalid destination_hash"));
                        continue;
                    };
                    let Some(public_key) = normalize_public_key_material(&entry.public_key) else {
                        errors.push(format!("announce {idx}: invalid public_key"));
                        continue;
                    };
                    self.known_announce_identities
                        .lock()
                        .expect("announce identities mutex poisoned")
                        .insert(destination_hash, public_key);
                    success_count += 1;
                }
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "success_count": success_count,
                        "errors": errors,
                    })),
                    error: None,
                })
            }
            "recall_identity" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: RecallIdentityParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let destination =
                    normalize_hash_hex(&parsed.destination_hash_hex).ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "destination_hash_hex must be a hex hash",
                        )
                    })?;
                let recalled = self.recall_identity_for_hash(&destination);
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(match recalled {
                        Some(public_key) => {
                            json!({ "found": true, "public_key": public_key })
                        }
                        None => json!({ "found": false }),
                    }),
                    error: None,
                })
            }
            "parse_rmsp_announce" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: RmspAnnounceParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let destination_hash =
                    normalize_hash_hex(&parsed.destination_hash).ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "destination_hash must be a hex hash",
                        )
                    })?;
                let server = self.upsert_rmsp_server_from_payload(
                    destination_hash,
                    parsed
                        .identity_hash
                        .and_then(|value| normalize_hash_hex(&value)),
                    parsed
                        .public_key
                        .and_then(|value| normalize_public_key_material(&value)),
                    parsed.app_data_hex.as_deref(),
                    parsed.hops,
                    now_i64(),
                );
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "success": server.is_some(),
                        "server": server,
                    })),
                    error: None,
                })
            }
            "get_rmsp_servers" => {
                let mut servers = self
                    .rmsp_servers
                    .lock()
                    .expect("rmsp server mutex poisoned")
                    .values()
                    .cloned()
                    .collect::<Vec<_>>();
                servers.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "servers": servers })),
                    error: None,
                })
            }
            "get_rmsp_servers_for_geohash" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: RmspServersForGeohashParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let geohash = parsed.geohash.trim().to_ascii_lowercase();
                let mut servers = self
                    .rmsp_servers
                    .lock()
                    .expect("rmsp server mutex poisoned")
                    .values()
                    .filter(|server| {
                        if geohash.is_empty() || server.coverage.is_empty() {
                            return true;
                        }
                        server.coverage.iter().any(|entry| {
                            let area = entry.to_ascii_lowercase();
                            geohash.starts_with(&area) || area.starts_with(&geohash)
                        })
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                servers.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "servers": servers })),
                    error: None,
                })
            }
            "get_outbound_propagation_node" => {
                let selected = self
                    .outbound_propagation_node
                    .lock()
                    .expect("propagation node mutex poisoned")
                    .clone();
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "peer": selected,
                        "meta": self.response_meta(),
                    })),
                    error: None,
                })
            }
            "set_outbound_propagation_node" => {
                let parsed = request
                    .params
                    .map(serde_json::from_value::<SetOutboundPropagationNodeParams>)
                    .transpose()
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let peer = parsed
                    .and_then(|value| value.peer)
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty());
                {
                    let mut guard = self
                        .outbound_propagation_node
                        .lock()
                        .expect("propagation node mutex poisoned");
                    *guard = peer.clone();
                }
                {
                    let mut propagation = self
                        .propagation_state
                        .lock()
                        .expect("propagation mutex poisoned");
                    propagation.selected_node = peer.clone();
                }
                let event = RpcEvent {
                    event_type: "propagation_node_selected".into(),
                    payload: json!({ "peer": peer }),
                };
                self.push_event(event.clone());
                let _ = self.events.send(event);
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "peer": peer,
                        "meta": self.response_meta(),
                    })),
                    error: None,
                })
            }
            "request_alternative_propagation_relay" => {
                let parsed = request
                    .params
                    .map(serde_json::from_value::<RequestAlternativePropagationRelayParams>)
                    .transpose()
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?
                    .unwrap_or_default();
                let mut excluded = HashSet::new();
                for relay in parsed.exclude_relays {
                    let trimmed = relay.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if let Some(normalized) = normalize_hash_hex(trimmed) {
                        excluded.insert(normalized);
                    } else {
                        excluded.insert(trimmed.to_string());
                    }
                }

                let selected = self
                    .outbound_propagation_node
                    .lock()
                    .expect("propagation node mutex poisoned")
                    .clone()
                    .and_then(|value| normalize_hash_hex(&value).or(Some(value)));
                let announces = self
                    .store
                    .list_announces(500, None, None)
                    .map_err(std::io::Error::other)?;
                let mut candidates = announces
                    .into_iter()
                    .filter(|announce| {
                        announce.aspect.as_deref() == Some("lxmf.propagation")
                            || announce
                                .capabilities
                                .iter()
                                .any(|capability| capability == "propagation")
                    })
                    .map(|announce| announce.peer)
                    .filter_map(|peer| normalize_hash_hex(&peer).or(Some(peer)))
                    .collect::<Vec<_>>();
                candidates.sort();
                candidates.dedup();

                let mut relay = None;
                if let Some(current) = selected.clone() {
                    if !excluded.contains(&current) {
                        relay = Some(current);
                    }
                }
                if relay.is_none() {
                    relay = candidates
                        .into_iter()
                        .find(|candidate| !excluded.contains(candidate));
                }

                if let Some(ref relay) = relay {
                    {
                        let mut guard = self
                            .outbound_propagation_node
                            .lock()
                            .expect("propagation node mutex poisoned");
                        *guard = Some(relay.clone());
                    }
                    {
                        let mut propagation = self
                            .propagation_state
                            .lock()
                            .expect("propagation mutex poisoned");
                        propagation.selected_node = Some(relay.clone());
                    }
                    self.emit_event(RpcEvent {
                        event_type: "propagation_node_selected".into(),
                        payload: json!({ "peer": relay }),
                    });
                }

                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "success": relay.is_some(),
                        "relay": relay,
                        "meta": self.response_meta(),
                    })),
                    error: None,
                })
            }
            "list_propagation_nodes" => {
                let selected = self
                    .outbound_propagation_node
                    .lock()
                    .expect("propagation node mutex poisoned")
                    .clone();
                let announces = self
                    .store
                    .list_announces(500, None, None)
                    .map_err(std::io::Error::other)?;
                let mut by_peer: HashMap<String, PropagationNodeRecord> = HashMap::new();
                for announce in announces {
                    let is_propagation_aspect =
                        announce.aspect.as_deref() == Some("lxmf.propagation");
                    let has_propagation_capability = announce
                        .capabilities
                        .iter()
                        .any(|capability| capability == "propagation");
                    if !(is_propagation_aspect || has_propagation_capability) {
                        continue;
                    }
                    let key = announce.peer.clone();
                    let entry =
                        by_peer
                            .entry(key.clone())
                            .or_insert_with(|| PropagationNodeRecord {
                                peer: key.clone(),
                                name: announce.name.clone(),
                                last_seen: announce.timestamp,
                                capabilities: announce.capabilities.clone(),
                                aspect: announce.aspect.clone(),
                                selected: selected.as_deref() == Some(key.as_str()),
                            });
                    if announce.timestamp > entry.last_seen {
                        entry.last_seen = announce.timestamp;
                        entry.name = announce.name.clone();
                        entry.capabilities = announce.capabilities.clone();
                        entry.aspect = announce.aspect.clone();
                    }
                    if selected.as_deref() == Some(key.as_str()) {
                        entry.selected = true;
                    }
                }

                let mut nodes = by_peer.into_values().collect::<Vec<_>>();
                nodes.sort_by(|a, b| {
                    b.last_seen
                        .cmp(&a.last_seen)
                        .then_with(|| a.peer.cmp(&b.peer))
                });
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "nodes": nodes,
                        "meta": self.response_meta(),
                    })),
                    error: None,
                })
            }
            "paper_ingest_uri" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: PaperIngestUriParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;

                if !parsed.uri.starts_with("lxm://") {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "paper URI must start with lxm://",
                    ));
                }

                let transient_id = {
                    let mut hasher = Sha256::new();
                    hasher.update(parsed.uri.as_bytes());
                    encode_hex(hasher.finalize())
                };

                let duplicate = {
                    let mut guard = self
                        .paper_ingest_seen
                        .lock()
                        .expect("paper ingest mutex poisoned");
                    if guard.contains(&transient_id) {
                        true
                    } else {
                        guard.insert(transient_id.clone());
                        false
                    }
                };

                let body = parsed.uri.trim_start_matches("lxm://");
                let destination = first_n_chars(body, 32).unwrap_or_default();

                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "destination": destination,
                        "transient_id": transient_id,
                        "duplicate": duplicate,
                        "bytes_len": parsed.uri.len(),
                    })),
                    error: None,
                })
            }
            "stamp_policy_get" => {
                let policy = self
                    .stamp_policy
                    .lock()
                    .expect("stamp mutex poisoned")
                    .clone();
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "stamp_policy": policy })),
                    error: None,
                })
            }
            "stamp_policy_set" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: StampPolicySetParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;

                let policy = {
                    let mut guard = self.stamp_policy.lock().expect("stamp mutex poisoned");
                    if let Some(value) = parsed.target_cost {
                        guard.target_cost = value;
                    }
                    if let Some(value) = parsed.flexibility {
                        guard.flexibility = value;
                    }
                    guard.clone()
                };

                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "stamp_policy": policy })),
                    error: None,
                })
            }
            "ticket_generate" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: TicketGenerateParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;

                let ttl_secs = parsed.ttl_secs.unwrap_or(3600);
                let ttl = i64::try_from(ttl_secs).map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("ttl_secs exceeds supported range: {ttl_secs}"),
                    )
                })?;
                let now = now_i64();
                let expires_at = now.checked_add(ttl).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("ttl_secs causes timestamp overflow: {ttl_secs}"),
                    )
                })?;
                let mut hasher = Sha256::new();
                hasher.update(parsed.destination.as_bytes());
                hasher.update(now.to_be_bytes());
                let ticket = encode_hex(hasher.finalize());
                let record = TicketRecord {
                    destination: parsed.destination.clone(),
                    ticket: ticket.clone(),
                    expires_at,
                };

                self.ticket_cache
                    .lock()
                    .expect("ticket mutex poisoned")
                    .insert(parsed.destination, record.clone());

                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({
                        "ticket": record.ticket,
                        "destination": record.destination,
                        "expires_at": record.expires_at,
                        "ttl_secs": ttl_secs,
                    })),
                    error: None,
                })
            }
            "announce_now" => {
                let timestamp = now_i64();
                if let Some(bridge) = &self.announce_bridge {
                    let _ = bridge.announce_now();
                }
                let event = RpcEvent {
                    event_type: "announce_sent".into(),
                    payload: json!({ "timestamp": timestamp }),
                };
                self.push_event(event.clone());
                let _ = self.events.send(event);
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "announce_id": request.id })),
                    error: None,
                })
            }
            "announce_received" => {
                let params = request.params.ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
                })?;
                let parsed: AnnounceReceivedParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let timestamp = parsed.timestamp.unwrap_or_else(now_i64);
                let peer = parsed.peer.clone();
                self.accept_announce_with_metadata(
                    parsed.peer,
                    timestamp,
                    parsed.name,
                    parsed.name_source,
                    parsed.app_data_hex,
                    parsed.capabilities,
                    parsed.rssi,
                    parsed.snr,
                    parsed.q,
                    parsed.aspect,
                    parsed.hops,
                    parsed.interface,
                    parsed.stamp_cost,
                    parsed.stamp_cost_flexibility,
                    parsed.peering_cost,
                )?;
                let record = self
                    .peers
                    .lock()
                    .expect("peers mutex poisoned")
                    .get(peer.as_str())
                    .cloned();
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "peer": record })),
                    error: None,
                })
            }
            "clear_messages" => {
                self.store.clear_messages().map_err(std::io::Error::other)?;
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "cleared": "messages" })),
                    error: None,
                })
            }
            "clear_resources" => Ok(RpcResponse {
                id: request.id,
                result: Some(json!({ "cleared": "resources" })),
                error: None,
            }),
            "clear_peers" => {
                {
                    let mut guard = self.peers.lock().expect("peers mutex poisoned");
                    guard.clear();
                }
                self.store
                    .clear_announces()
                    .map_err(std::io::Error::other)?;
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "cleared": "peers" })),
                    error: None,
                })
            }
            "clear_all" => {
                self.store.clear_messages().map_err(std::io::Error::other)?;
                self.store
                    .clear_announces()
                    .map_err(std::io::Error::other)?;
                {
                    let mut guard = self.peers.lock().expect("peers mutex poisoned");
                    guard.clear();
                }
                {
                    let mut guard = self
                        .delivery_traces
                        .lock()
                        .expect("delivery traces mutex poisoned");
                    guard.clear();
                }
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "cleared": "all" })),
                    error: None,
                })
            }
            _ => Ok(RpcResponse {
                id: request.id,
                result: None,
                error: Some(RpcError {
                    code: "NOT_IMPLEMENTED".into(),
                    message: "method not implemented".into(),
                }),
            }),
        }
    }

    fn append_delivery_trace(&self, message_id: &str, status: String) {
        const MAX_DELIVERY_TRACE_ENTRIES: usize = 32;
        const MAX_TRACKED_MESSAGE_TRACES: usize = 2048;

        let timestamp = now_i64();
        let reason_code = delivery_reason_code(&status).map(ToOwned::to_owned);
        let mut guard = self
            .delivery_traces
            .lock()
            .expect("delivery traces mutex poisoned");
        let entry = guard.entry(message_id.to_string()).or_default();
        entry.push(DeliveryTraceEntry {
            status,
            timestamp,
            reason_code,
        });
        if entry.len() > MAX_DELIVERY_TRACE_ENTRIES {
            let drain_count = entry.len().saturating_sub(MAX_DELIVERY_TRACE_ENTRIES);
            entry.drain(0..drain_count);
        }

        if guard.len() > MAX_TRACKED_MESSAGE_TRACES {
            let overflow = guard.len() - MAX_TRACKED_MESSAGE_TRACES;
            let mut evicted_ids = Vec::with_capacity(overflow);
            for key in guard.keys() {
                if key != message_id {
                    evicted_ids.push(key.clone());
                    if evicted_ids.len() == overflow {
                        break;
                    }
                }
            }
            for id in evicted_ids {
                guard.remove(&id);
            }

            if guard.len() > MAX_TRACKED_MESSAGE_TRACES {
                let still_over = guard.len() - MAX_TRACKED_MESSAGE_TRACES;
                let mut fallback = Vec::with_capacity(still_over);
                for key in guard.keys().take(still_over).cloned() {
                    fallback.push(key);
                }
                for id in fallback {
                    guard.remove(&id);
                }
            }
        }
    }

    fn response_meta(&self) -> JsonValue {
        json!({
            "contract_version": "v2",
            "profile": JsonValue::Null,
            "rpc_endpoint": JsonValue::Null,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn store_outbound(
        &self,
        request_id: u64,
        id: String,
        source: String,
        destination: String,
        title: String,
        content: String,
        fields: Option<JsonValue>,
        method: Option<String>,
        stamp_cost: Option<u32>,
        include_ticket: Option<bool>,
        try_propagation_on_fail: Option<bool>,
        source_private_key: Option<String>,
    ) -> Result<RpcResponse, std::io::Error> {
        let timestamp = now_i64();
        self.append_delivery_trace(&id, "queued".to_string());
        let include_ticket = include_ticket.unwrap_or(false);
        let ticket = if include_ticket {
            self.ticket_cache
                .lock()
                .expect("ticket mutex poisoned")
                .get(destination.as_str())
                .and_then(|record| (record.expires_at > now_i64()).then_some(record.ticket.clone()))
        } else {
            None
        };
        let delivery_options = OutboundDeliveryOptions {
            method: clean_optional_text(method),
            stamp_cost,
            include_ticket,
            try_propagation_on_fail: try_propagation_on_fail.unwrap_or(false),
            ticket,
            source_private_key: clean_optional_text(source_private_key),
        };
        let mut record = MessageRecord {
            id: id.clone(),
            source,
            destination,
            title,
            content,
            timestamp,
            direction: "out".into(),
            fields,
            receipt_status: None,
        };

        self.store
            .insert_message(&record)
            .map_err(std::io::Error::other)?;
        self.append_delivery_trace(&id, "sending".to_string());
        let deliver_result = if let Some(bridge) = &self.outbound_bridge {
            bridge.deliver(&record, &delivery_options)
        } else {
            let _delivered = crate::transport::test_bridge::deliver_outbound(&record);
            Ok(())
        };
        if let Err(err) = deliver_result {
            let status = format!("failed: {err}");
            let _ = self.store.update_receipt_status(&id, &status);
            record.receipt_status = Some(status);
            let resolved_status = record.receipt_status.clone().unwrap_or_default();
            self.append_delivery_trace(&id, resolved_status.clone());
            let reason_code = delivery_reason_code(&resolved_status);
            let event = RpcEvent {
                event_type: "outbound".into(),
                payload: json!({
                    "message": record,
                    "method": delivery_options.method,
                    "stamp_cost": delivery_options.stamp_cost,
                    "include_ticket": delivery_options.include_ticket,
                    "try_propagation_on_fail": delivery_options.try_propagation_on_fail,
                    "error": err.to_string(),
                    "reason_code": reason_code,
                }),
            };
            self.push_event(event.clone());
            let _ = self.events.send(event);
            return Ok(RpcResponse {
                id: request_id,
                result: None,
                error: Some(RpcError {
                    code: "DELIVERY_FAILED".into(),
                    message: err.to_string(),
                }),
            });
        }
        let sent_status = format!(
            "sent: {}",
            delivery_options.method.as_deref().unwrap_or("direct")
        );
        self.append_delivery_trace(&id, sent_status.clone());
        let event = RpcEvent {
            event_type: "outbound".into(),
            payload: json!({
                "message": record,
                "method": delivery_options.method,
                "stamp_cost": delivery_options.stamp_cost,
                "include_ticket": delivery_options.include_ticket,
                "try_propagation_on_fail": delivery_options.try_propagation_on_fail,
                "reason_code": delivery_reason_code(&sent_status),
            }),
        };
        self.push_event(event.clone());
        let _ = self.events.send(event);

        Ok(RpcResponse {
            id: request_id,
            result: Some(json!({ "message_id": id })),
            error: None,
        })
    }

    fn local_delivery_hash(&self) -> String {
        self.delivery_destination_hash
            .lock()
            .expect("delivery_destination_hash mutex poisoned")
            .clone()
            .unwrap_or_else(|| self.identity_hash.clone())
    }

    fn capabilities() -> Vec<&'static str> {
        vec![
            "status",
            "daemon_status_ex",
            "list_messages",
            "list_announces",
            "list_peers",
            "send_message",
            "send_message_v2",
            "receive_message",
            "announce_now",
            "announce_received",
            "list_interfaces",
            "set_interfaces",
            "reload_config",
            "peer_sync",
            "peer_unpeer",
            "has_path",
            "request_path",
            "establish_link",
            "set_delivery_policy",
            "get_delivery_policy",
            "propagation_status",
            "propagation_enable",
            "propagation_ingest",
            "propagation_fetch",
            "request_messages_from_propagation_node",
            "get_propagation_state",
            "get_outbound_propagation_node",
            "set_outbound_propagation_node",
            "request_alternative_propagation_relay",
            "list_propagation_nodes",
            "set_incoming_message_size_limit",
            "get_incoming_message_size_limit",
            "send_location_telemetry",
            "send_telemetry_request",
            "send_reaction",
            "store_peer_identity",
            "restore_all_peer_identities",
            "bulk_restore_announce_identities",
            "bulk_restore_peer_identities",
            "recall_identity",
            "parse_rmsp_announce",
            "get_rmsp_servers",
            "get_rmsp_servers_for_geohash",
            "paper_ingest_uri",
            "stamp_policy_get",
            "stamp_policy_set",
            "ticket_generate",
            "record_receipt",
            "message_delivery_trace",
            "clear_messages",
            "clear_resources",
            "clear_peers",
            "clear_all",
        ]
    }

    fn knows_destination(&self, destination: &str) -> bool {
        if self
            .peers
            .lock()
            .expect("peers mutex poisoned")
            .contains_key(destination)
        {
            return true;
        }
        if self
            .known_announce_identities
            .lock()
            .expect("announce identities mutex poisoned")
            .contains_key(destination)
        {
            return true;
        }
        if self
            .known_peer_identities
            .lock()
            .expect("peer identities mutex poisoned")
            .contains_key(destination)
        {
            return true;
        }
        self.store
            .list_announces(512, None, None)
            .map(|announces| {
                announces
                    .into_iter()
                    .any(|announce| announce.peer == destination)
            })
            .unwrap_or(false)
    }

    fn resolve_source_hash(
        &self,
        source: Option<String>,
        source_private_key: Option<String>,
    ) -> Result<String, std::io::Error> {
        if let Some(source) = source.and_then(|value| normalize_hash_hex(&value)) {
            return Ok(source);
        }
        if let Some(source_private_key) =
            source_private_key.and_then(|value| clean_optional_text(Some(value)))
        {
            return source_hash_from_private_key_hex(&source_private_key);
        }
        Ok(self.local_delivery_hash())
    }

    fn request_messages_from_propagation_node(
        &self,
        max_messages: usize,
        identity_private_key: Option<String>,
    ) -> Result<JsonValue, std::io::Error> {
        const PR_IDLE: u32 = 0x00;
        const PR_NO_PATH: u32 = 0xF0;

        if let Some(raw) = identity_private_key.and_then(|value| clean_optional_text(Some(value))) {
            let _ = source_hash_from_private_key_hex(&raw)?;
        }

        let selected_node = self
            .outbound_propagation_node
            .lock()
            .expect("propagation node mutex poisoned")
            .clone();
        let Some(selected_node) = selected_node else {
            let now = now_i64();
            {
                let mut guard = self
                    .propagation_state
                    .lock()
                    .expect("propagation mutex poisoned");
                guard.sync_state = PR_IDLE;
                guard.state_name = "idle".to_string();
                guard.sync_progress = 0.0;
                guard.messages_received = 0;
                guard.max_messages = max_messages;
                guard.selected_node = None;
                guard.last_sync_started = Some(now);
                guard.last_sync_completed = Some(now);
                guard.last_sync_error = Some("No propagation node configured".to_string());
            }
            self.emit_propagation_state_event();
            return Ok(json!({
                "success": false,
                "error": "No propagation node configured",
                "errorCode": "NO_PROPAGATION_NODE",
                "state": PR_IDLE,
                "state_name": "idle",
                "progress": 0.0_f64,
                "messages_received": 0_u32,
            }));
        };

        let selected_lookup = normalize_hash_hex(&selected_node).unwrap_or(selected_node.clone());
        if !self.knows_destination(&selected_lookup) {
            let now = now_i64();
            {
                let mut guard = self
                    .propagation_state
                    .lock()
                    .expect("propagation mutex poisoned");
                guard.sync_state = PR_NO_PATH;
                guard.state_name = "no_path".to_string();
                guard.sync_progress = 0.0;
                guard.messages_received = 0;
                guard.max_messages = max_messages;
                guard.selected_node = Some(selected_node.clone());
                guard.last_sync_started = Some(now);
                guard.last_sync_completed = Some(now);
                guard.last_sync_error = Some("No path known for propagation node".to_string());
            }
            self.emit_propagation_state_event();
            return Ok(json!({
                "success": false,
                "error": "No path known for propagation node",
                "errorCode": "NO_PATH",
                "state": PR_NO_PATH,
                "state_name": "no_path",
                "progress": 0.0_f64,
                "messages_received": 0_u32,
                "selected_node": selected_node,
                "max_messages": max_messages,
            }));
        }

        let now = now_i64();
        {
            let mut guard = self
                .propagation_state
                .lock()
                .expect("propagation mutex poisoned");
            guard.sync_state = 1;
            guard.state_name = "path_requested".to_string();
            guard.sync_progress = 0.0;
            guard.messages_received = 0;
            guard.max_messages = max_messages;
            guard.selected_node = Some(selected_node.clone());
            guard.last_sync_started = Some(now);
            guard.last_sync_error = None;
        }
        self.emit_propagation_state_event();

        {
            let mut guard = self
                .propagation_state
                .lock()
                .expect("propagation mutex poisoned");
            guard.sync_state = 2;
            guard.state_name = "link_establishing".to_string();
            guard.sync_progress = 0.15;
        }
        self.emit_propagation_state_event();

        {
            let mut guard = self
                .propagation_state
                .lock()
                .expect("propagation mutex poisoned");
            guard.sync_state = 3;
            guard.state_name = "link_established".to_string();
            guard.sync_progress = 0.35;
        }
        self.emit_propagation_state_event();

        {
            let mut guard = self
                .propagation_state
                .lock()
                .expect("propagation mutex poisoned");
            guard.sync_state = 4;
            guard.state_name = "request_sent".to_string();
            guard.sync_progress = 0.55;
        }
        self.emit_propagation_state_event();

        let messages = {
            let mut guard = self
                .propagation_payloads
                .lock()
                .expect("propagation payload mutex poisoned");
            let mut keys = guard.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            keys.truncate(max_messages);
            keys.into_iter()
                .filter_map(|transient_id| {
                    guard
                        .remove(&transient_id)
                        .map(|payload_hex| json!({ "transient_id": transient_id, "payload_hex": payload_hex }))
                })
                .collect::<Vec<_>>()
        };

        {
            let mut guard = self
                .propagation_state
                .lock()
                .expect("propagation mutex poisoned");
            guard.sync_state = 5;
            guard.state_name = "receiving".to_string();
            guard.sync_progress = 0.9;
            guard.messages_received = messages.len();
        }
        self.emit_propagation_state_event();

        let completed_at = now_i64();
        let state = {
            let mut guard = self
                .propagation_state
                .lock()
                .expect("propagation mutex poisoned");
            guard.sync_state = 7;
            guard.state_name = "complete".to_string();
            guard.sync_progress = 1.0;
            guard.last_sync_completed = Some(completed_at);
            guard.clone()
        };
        self.emit_propagation_state_event();

        Ok(json!({
            "success": true,
            "state": state.sync_state,
            "state_name": state.state_name,
            "progress": state.sync_progress,
            "messages_received": state.messages_received,
            "selected_node": state.selected_node,
            "max_messages": state.max_messages,
            "last_sync_started": state.last_sync_started,
            "last_sync_completed": state.last_sync_completed,
            "messages": messages,
        }))
    }

    fn emit_propagation_state_event(&self) {
        let state = self
            .propagation_state
            .lock()
            .expect("propagation mutex poisoned")
            .clone();
        let event = RpcEvent {
            event_type: "propagation_state".into(),
            payload: json!({
                "state": state.sync_state,
                "state_name": state.state_name,
                "progress": state.sync_progress,
                "messages_received": state.messages_received,
                "selected_node": state.selected_node,
                "max_messages": state.max_messages,
            }),
        };
        self.push_event(event.clone());
        let _ = self.events.send(event);
    }

    fn recall_identity_for_hash(&self, destination_hash_hex: &str) -> Option<String> {
        if let Some(public_key) = self
            .known_announce_identities
            .lock()
            .expect("announce identities mutex poisoned")
            .get(destination_hash_hex)
            .cloned()
        {
            return Some(public_key);
        }
        self.known_peer_identities
            .lock()
            .expect("peer identities mutex poisoned")
            .get(destination_hash_hex)
            .cloned()
    }

    fn upsert_rmsp_server_from_announce(
        &self,
        destination_hash: &str,
        app_data_hex: Option<&str>,
        hops: Option<u32>,
        timestamp: i64,
    ) {
        let _ = self.upsert_rmsp_server_from_payload(
            destination_hash.to_string(),
            None,
            None,
            app_data_hex,
            hops,
            timestamp,
        );
    }

    fn upsert_rmsp_server_from_payload(
        &self,
        destination_hash: String,
        identity_hash: Option<String>,
        public_key: Option<String>,
        app_data_hex: Option<&str>,
        hops: Option<u32>,
        timestamp: i64,
    ) -> Option<RmspServerRecord> {
        let parsed = parse_rmsp_app_data(app_data_hex)?;
        let coverage = parsed
            .get("c")
            .and_then(serde_json::Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(|entry| entry.as_str().map(ToOwned::to_owned))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let zoom_range = parsed
            .get("z")
            .and_then(serde_json::Value::as_array)
            .map(|values| values.iter().filter_map(json_u32).collect::<Vec<_>>())
            .unwrap_or_default();
        let formats = parsed
            .get("f")
            .and_then(serde_json::Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(|entry| entry.as_str().map(ToOwned::to_owned))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let layers = parsed
            .get("l")
            .and_then(serde_json::Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(|entry| entry.as_str().map(ToOwned::to_owned))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let record = RmspServerRecord {
            destination_hash: destination_hash.clone(),
            identity_hash,
            public_key,
            name: parsed
                .get("n")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            version: parsed
                .get("v")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            coverage,
            zoom_range,
            formats,
            layers,
            updated: parsed.get("u").and_then(|value| {
                value
                    .as_i64()
                    .or_else(|| value.as_u64().map(|raw| raw as i64))
            }),
            size: parsed.get("s").and_then(|value| {
                value.as_u64().or_else(|| {
                    value
                        .as_i64()
                        .and_then(|raw| (raw >= 0).then_some(raw as u64))
                })
            }),
            hops,
            timestamp,
        };
        self.rmsp_servers
            .lock()
            .expect("rmsp server mutex poisoned")
            .insert(destination_hash, record.clone());
        Some(record)
    }

    pub fn handle_framed_request(&self, bytes: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        let request: RpcRequest = codec::decode_frame(bytes)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
        let response = self.handle_rpc(request)?;
        codec::encode_frame(&response).map_err(std::io::Error::other)
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<RpcEvent> {
        self.events.subscribe()
    }

    pub fn take_event(&self) -> Option<RpcEvent> {
        let mut guard = self.event_queue.lock().expect("event_queue mutex poisoned");
        guard.pop_front()
    }

    pub fn push_event(&self, event: RpcEvent) {
        let mut guard = self.event_queue.lock().expect("event_queue mutex poisoned");
        if guard.len() >= 32 {
            guard.pop_front();
        }
        guard.push_back(event);
    }

    pub fn emit_event(&self, event: RpcEvent) {
        self.push_event(event.clone());
        let _ = self.events.send(event);
    }

    pub fn schedule_announce_for_test(&self, id: u64) {
        let timestamp = now_i64();
        let event = RpcEvent {
            event_type: "announce_sent".into(),
            payload: json!({
                "timestamp": timestamp,
                "timestamp_ms": timestamp_to_ms(timestamp),
                "announce_id": id
            }),
        };
        self.push_event(event.clone());
        let _ = self.events.send(event);
    }

    pub fn start_announce_scheduler(
        self: std::rc::Rc<Self>,
        interval_secs: u64,
    ) -> tokio::task::JoinHandle<()> {
        tokio::task::spawn_local(async move {
            if interval_secs == 0 {
                return;
            }

            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
            loop {
                // First tick is immediate, so we announce once at scheduler start.
                interval.tick().await;
                let id = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|value| value.as_secs())
                    .unwrap_or(0);

                if let Some(bridge) = &self.announce_bridge {
                    let _ = bridge.announce_now();
                }

                let timestamp = now_i64();
                let event = RpcEvent {
                    event_type: "announce_sent".into(),
                    payload: json!({
                        "timestamp": timestamp,
                        "timestamp_ms": timestamp_to_ms(timestamp),
                        "announce_id": id
                    }),
                };
                self.push_event(event.clone());
                let _ = self.events.send(event);
            }
        })
    }

    pub fn inject_inbound_test_message(&self, content: &str) {
        let timestamp = now_i64();
        let record = crate::storage::messages::MessageRecord {
            id: format!("test-{}", timestamp),
            source: "test-peer".into(),
            destination: "local".into(),
            title: "".into(),
            content: content.into(),
            timestamp,
            direction: "in".into(),
            fields: None,
            receipt_status: None,
        };
        let _ = self.store.insert_message(&record);
        let event = RpcEvent {
            event_type: "inbound".into(),
            payload: json!({ "message": record }),
        };
        self.push_event(event.clone());
        let _ = self.events.send(event);
    }

    pub fn emit_link_event_for_test(&self) {
        let event = RpcEvent {
            event_type: "link_activated".into(),
            payload: json!({ "link_id": "test-link" }),
        };
        self.push_event(event.clone());
        let _ = self.events.send(event);
    }
}

fn parse_announce_cursor(cursor: Option<&str>) -> Option<(Option<i64>, Option<String>)> {
    let raw = cursor?.trim();
    if raw.is_empty() {
        return None;
    }
    if let Some((timestamp_raw, id)) = raw.split_once(':') {
        let timestamp = timestamp_raw.parse::<i64>().ok()?;
        let before_id = if id.is_empty() {
            None
        } else {
            Some(id.to_string())
        };
        return Some((Some(timestamp), before_id));
    }
    raw.parse::<i64>()
        .ok()
        .map(|timestamp| (Some(timestamp), None))
}

fn delivery_reason_code(status: &str) -> Option<&'static str> {
    let normalized = status.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    if normalized.contains("receipt timeout") {
        return Some("receipt_timeout");
    }
    if normalized.contains("timeout") {
        return Some("timeout");
    }
    if normalized.contains("no route")
        || normalized.contains("no path")
        || normalized.contains("no known path")
    {
        return Some("no_path");
    }
    if normalized.contains("no propagation relay selected") {
        return Some("relay_unset");
    }
    if normalized.contains("retry budget exhausted") {
        return Some("retry_budget_exhausted");
    }
    None
}

fn timestamp_to_ms(timestamp: i64) -> i64 {
    // Preserve existing ms values while upgrading second-based values for
    // interop with clients that consume millisecond timestamps.
    if timestamp.unsigned_abs() >= 1_000_000_000_000_u64 {
        timestamp
    } else {
        timestamp.saturating_mul(1000)
    }
}

fn parse_propagation_costs_from_app_data(
    app_data_hex: Option<&str>,
) -> (Option<u32>, Option<u32>, Option<u32>) {
    let Some(raw_hex) = app_data_hex
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return (None, None, None);
    };
    let Ok(app_data) = hex::decode(raw_hex) else {
        return (None, None, None);
    };
    let Ok(value) = rmp_serde::from_slice::<serde_json::Value>(&app_data) else {
        return (None, None, None);
    };
    let Some(entries) = value.as_array() else {
        return (None, None, None);
    };
    let Some(costs) = entries.get(5).and_then(serde_json::Value::as_array) else {
        return (None, None, None);
    };

    (
        costs.first().and_then(json_u32),
        costs.get(1).and_then(json_u32),
        costs.get(2).and_then(json_u32),
    )
}

fn json_u32(value: &serde_json::Value) -> Option<u32> {
    value
        .as_u64()
        .and_then(|raw| u32::try_from(raw).ok())
        .or_else(|| {
            value
                .as_i64()
                .and_then(|raw| (raw >= 0).then_some(raw as u64))
                .and_then(|raw| u32::try_from(raw).ok())
        })
}

fn generated_message_id(prefix: &str) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or(0);
    format!("{prefix}-{ts}")
}

fn normalize_hash_hex(input: &str) -> Option<String> {
    let cleaned = input.trim().to_ascii_lowercase();
    if cleaned.is_empty() || cleaned.len() % 2 != 0 {
        return None;
    }
    let bytes = hex::decode(cleaned.as_bytes()).ok()?;
    match bytes.len() {
        16 => Some(hex::encode(bytes)),
        32 => Some(hex::encode(&bytes[..16])),
        _ => None,
    }
}

fn source_hash_from_private_key_hex(private_key_hex: &str) -> Result<String, std::io::Error> {
    let key_bytes = hex::decode(private_key_hex.trim()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "source_private_key must be hex-encoded",
        )
    })?;
    let identity =
        crate::identity::PrivateIdentity::from_private_key_bytes(&key_bytes).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "source_private_key is invalid",
            )
        })?;
    Ok(hex::encode(identity.address_hash().as_slice()))
}

fn normalize_public_key_material(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.len() % 2 == 0 && trimmed.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Some(trimmed.to_ascii_lowercase());
    }

    use base64::Engine as _;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(trimmed)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(trimmed))
        .ok()?;
    Some(hex::encode(decoded))
}

fn transport_fields_json(fields: rmpv::Value) -> Result<JsonValue, std::io::Error> {
    use base64::Engine as _;
    let encoded =
        rmp_serde::to_vec(&fields).map_err(|err| std::io::Error::other(err.to_string()))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(encoded);
    Ok(json!({ "_lxmf_fields_msgpack_b64": b64 }))
}

fn reaction_fields_json(
    target_message_id: String,
    emoji: String,
    sender_hash: String,
) -> Result<JsonValue, std::io::Error> {
    const FIELD_APP_EXTENSIONS: i64 = 16;
    let fields = rmpv::Value::Map(vec![(
        rmpv::Value::Integer(FIELD_APP_EXTENSIONS.into()),
        rmpv::Value::Map(vec![
            (
                rmpv::Value::String("reaction_to".into()),
                rmpv::Value::String(target_message_id.into()),
            ),
            (
                rmpv::Value::String("emoji".into()),
                rmpv::Value::String(emoji.into()),
            ),
            (
                rmpv::Value::String("sender".into()),
                rmpv::Value::String(sender_hash.into()),
            ),
        ]),
    )]);
    transport_fields_json(fields)
}

fn location_telemetry_fields_json(
    location: &serde_json::Value,
    icon_name: Option<String>,
    icon_fg_color: Option<String>,
    icon_bg_color: Option<String>,
) -> Result<JsonValue, std::io::Error> {
    const FIELD_TELEMETRY: i64 = 0x02;
    const FIELD_ICON_APPEARANCE: i64 = 0x04;
    const FIELD_COLUMBA_META: i64 = 0x70;

    let cease = location
        .get("cease")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let mut entries = Vec::new();

    if cease {
        let meta = serde_json::json!({ "cease": true }).to_string();
        entries.push((
            rmpv::Value::Integer(FIELD_COLUMBA_META.into()),
            rmpv::Value::String(meta.into()),
        ));
    } else {
        let telemetry = pack_location_telemetry(location)?;
        entries.push((
            rmpv::Value::Integer(FIELD_TELEMETRY.into()),
            rmpv::Value::Binary(telemetry),
        ));

        let mut meta = serde_json::Map::new();
        if let Some(expires) = location.get("expires") {
            meta.insert("expires".to_string(), expires.clone());
        }
        if let Some(approx_radius) = location.get("approxRadius") {
            meta.insert("approxRadius".to_string(), approx_radius.clone());
        }
        if !meta.is_empty() {
            entries.push((
                rmpv::Value::Integer(FIELD_COLUMBA_META.into()),
                rmpv::Value::String(serde_json::Value::Object(meta).to_string().into()),
            ));
        }
    }

    if let (Some(name), Some(fg), Some(bg)) = (icon_name, icon_fg_color, icon_bg_color) {
        if let (Some(fg_bytes), Some(bg_bytes)) = (parse_rgb_hex(&fg), parse_rgb_hex(&bg)) {
            entries.push((
                rmpv::Value::Integer(FIELD_ICON_APPEARANCE.into()),
                rmpv::Value::Array(vec![
                    rmpv::Value::String(name.into()),
                    rmpv::Value::Binary(fg_bytes),
                    rmpv::Value::Binary(bg_bytes),
                ]),
            ));
        }
    }

    transport_fields_json(rmpv::Value::Map(entries))
}

fn telemetry_request_fields_json(
    timebase: Option<f64>,
    is_collector_request: bool,
) -> Result<JsonValue, std::io::Error> {
    const FIELD_COMMANDS: i64 = 0x09;
    const COMMAND_TELEMETRY_REQUEST: i64 = 0x01;

    let command = rmpv::Value::Map(vec![(
        rmpv::Value::Integer(COMMAND_TELEMETRY_REQUEST.into()),
        rmpv::Value::Array(vec![
            rmpv::Value::F64(timebase.unwrap_or(0.0)),
            rmpv::Value::Boolean(is_collector_request),
        ]),
    )]);
    transport_fields_json(rmpv::Value::Map(vec![(
        rmpv::Value::Integer(FIELD_COMMANDS.into()),
        rmpv::Value::Array(vec![command]),
    )]))
}

fn parse_rgb_hex(color: &str) -> Option<Vec<u8>> {
    let raw = color.trim().trim_start_matches('#');
    if raw.len() != 6 || !raw.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    let decoded = hex::decode(raw).ok()?;
    (decoded.len() == 3).then_some(decoded)
}

fn pack_location_telemetry(location: &serde_json::Value) -> Result<Vec<u8>, std::io::Error> {
    fn i32_be(value: f64, scale: f64) -> Vec<u8> {
        ((value * scale).round() as i32).to_be_bytes().to_vec()
    }
    fn u32_be(value: f64, scale: f64) -> Vec<u8> {
        ((value * scale).max(0.0).round() as u32)
            .to_be_bytes()
            .to_vec()
    }
    fn u16_be(value: f64, scale: f64) -> Vec<u8> {
        ((value * scale).max(0.0).round() as u16)
            .to_be_bytes()
            .to_vec()
    }

    let lat = location
        .get("lat")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let lon = location
        .get("lng")
        .or_else(|| location.get("lon"))
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let altitude = location
        .get("altitude")
        .or_else(|| location.get("alt"))
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let speed = location
        .get("speed")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let bearing = location
        .get("bearing")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let accuracy = location
        .get("acc")
        .or_else(|| location.get("accuracy"))
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let timestamp = location
        .get("ts")
        .and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_u64().map(|raw| raw as i64))
        })
        .unwrap_or_else(|| timestamp_to_ms(now_i64()));

    let packed = rmpv::Value::Map(vec![(
        rmpv::Value::Integer(0x02_i64.into()),
        rmpv::Value::Array(vec![
            rmpv::Value::Binary(i32_be(lat, 1e6)),
            rmpv::Value::Binary(i32_be(lon, 1e6)),
            rmpv::Value::Binary(i32_be(altitude, 1e2)),
            rmpv::Value::Binary(u32_be(speed, 1e2)),
            rmpv::Value::Binary(i32_be(bearing, 1e2)),
            rmpv::Value::Binary(u16_be(accuracy, 1e2)),
            rmpv::Value::Integer(timestamp.into()),
        ]),
    )]);
    rmp_serde::to_vec(&packed).map_err(|err| std::io::Error::other(err.to_string()))
}

fn parse_rmsp_app_data(
    app_data_hex: Option<&str>,
) -> Option<serde_json::Map<String, serde_json::Value>> {
    let raw_hex = app_data_hex?.trim();
    if raw_hex.is_empty() {
        return None;
    }
    let bytes = hex::decode(raw_hex).ok()?;
    let value = rmp_serde::from_slice::<serde_json::Value>(&bytes).ok()?;
    value.as_object().cloned()
}

fn derive_lxmf_delivery_destination_hash_from_identity_hash(
    identity_hash_hex: &str,
) -> Option<String> {
    let identity_hash = hex::decode(identity_hash_hex).ok()?;
    if identity_hash.len() != 16 {
        return None;
    }
    let name_hash = crate::destination::DestinationName::new("lxmf", "delivery");
    let mut hasher = Sha256::new();
    hasher.update(name_hash.as_name_hash_slice());
    hasher.update(identity_hash);
    let digest = hasher.finalize();
    Some(hex::encode(&digest[..16]))
}
