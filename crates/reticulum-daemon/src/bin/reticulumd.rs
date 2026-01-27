use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use clap::Parser;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::LocalSet;

use reticulum::destination::{DestinationName, SingleInputDestination};
use reticulum::hash::AddressHash;
use reticulum::identity::{Identity, PrivateIdentity};
use reticulum::iface::tcp_client::TcpClient;
use reticulum::iface::tcp_server::TcpServer;
use reticulum::rpc::{http, AnnounceBridge, OutboundBridge, RpcDaemon};
use reticulum::storage::messages::MessagesStore;
use reticulum::transport::{Transport, TransportConfig};
use tokio::sync::mpsc::unbounded_channel;

use reticulum_daemon::config::DaemonConfig;
use reticulum_daemon::direct_delivery::send_via_link;
use reticulum_daemon::identity_store::load_or_create_identity;
use reticulum_daemon::inbound_delivery::decode_inbound_payload;
use reticulum_daemon::lxmf_bridge::build_wire_message;
use reticulum_daemon::receipt_bridge::{handle_receipt_event, track_receipt_mapping, ReceiptBridge};
use reticulum_daemon::rns_crypto::decrypt_with_identity;
use lxmf::constants::DESTINATION_LENGTH;

#[derive(Parser, Debug)]
#[command(name = "reticulumd")]
struct Args {
    #[arg(long, default_value = "127.0.0.1:4243")]
    rpc: String,
    #[arg(long, default_value = "reticulum.db")]
    db: PathBuf,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    identity: Option<PathBuf>,
    #[arg(long, default_value_t = 0)]
    announce_interval_secs: u64,
    #[arg(long)]
    transport: Option<String>,
}

struct TransportBridge {
    transport: Arc<Transport>,
    signer: PrivateIdentity,
    announce_destination: Arc<tokio::sync::Mutex<SingleInputDestination>>,
    peer_crypto: Arc<std::sync::Mutex<HashMap<String, PeerCrypto>>>,
    receipt_map: Arc<std::sync::Mutex<HashMap<String, String>>>,
}

#[derive(Clone, Copy)]
struct PeerCrypto {
    identity_hash: [u8; 16],
    identity: Identity,
    public_key: [u8; 32],
    ratchet: Option<[u8; 32]>,
}

impl TransportBridge {
    fn new(
        transport: Arc<Transport>,
        signer: PrivateIdentity,
        announce_destination: Arc<tokio::sync::Mutex<SingleInputDestination>>,
        peer_crypto: Arc<std::sync::Mutex<HashMap<String, PeerCrypto>>>,
        receipt_map: Arc<std::sync::Mutex<HashMap<String, String>>>,
    ) -> Self {
        Self {
            transport,
            signer,
            announce_destination,
            peer_crypto,
            receipt_map,
        }
    }
}

impl OutboundBridge for TransportBridge {
    fn deliver(
        &self,
        record: &reticulum::storage::messages::MessageRecord,
    ) -> Result<(), std::io::Error> {
        let destination = parse_destination_hex(&record.destination);
        let Some(destination) = destination else {
            return Ok(());
        };
        let peer_info = self
            .peer_crypto
            .lock()
            .expect("peer map")
            .get(&record.destination)
            .copied();
        let Some(peer_info) = peer_info else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "peer not announced",
            ));
        };

        let source_hash = self.signer.address_hash();
        let mut source = [0u8; 16];
        source.copy_from_slice(source_hash.as_slice());

        let wire = build_wire_message(
            source,
            destination,
            &record.title,
            &record.content,
            record.fields.clone(),
            &self.signer,
        )
        .map_err(std::io::Error::other)?;

        let payload = wire;

        let destination_desc = reticulum::destination::DestinationDesc {
            identity: peer_info.identity,
            address_hash: AddressHash::new(destination),
            name: DestinationName::new("lxmf", "delivery"),
        };
        let transport = self.transport.clone();
        let receipt_map = self.receipt_map.clone();
        let message_id = record.id.clone();
        tokio::spawn(async move {
            let result = send_via_link(
                transport.as_ref(),
                destination_desc,
                &payload,
                std::time::Duration::from_secs(8),
            )
            .await;
            if let Ok(packet) = result {
                let packet_hash = hex::encode(packet.hash().to_bytes());
                track_receipt_mapping(&receipt_map, &packet_hash, &message_id);
            }
        });
        Ok(())
    }
}

impl AnnounceBridge for TransportBridge {
    fn announce_now(&self) -> Result<(), std::io::Error> {
        let transport = self.transport.clone();
        let destination = self.announce_destination.clone();
        tokio::spawn(async move {
            transport.send_announce(&destination, None).await;
        });
        Ok(())
    }
}

fn parse_destination_hex(input: &str) -> Option<[u8; 16]> {
    let bytes = hex::decode(input).ok()?;
    if bytes.len() != 16 {
        return None;
    }
    let mut out = [0u8; 16];
    out.copy_from_slice(&bytes);
    Some(out)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let local = LocalSet::new();
    local
        .run_until(async {
            let args = Args::parse();
            let addr: SocketAddr = args.rpc.parse().expect("invalid rpc address");
            let store = MessagesStore::open(&args.db).expect("open sqlite");

            let identity_path = args.identity.clone().unwrap_or_else(|| {
                let mut path = args.db.clone();
                path.set_extension("identity");
                path
            });
            let identity = load_or_create_identity(&identity_path).expect("load identity");
            let identity_hash = hex::encode(identity.address_hash().as_slice());

            let mut transport: Option<Arc<Transport>> = None;
            let peer_crypto: Arc<std::sync::Mutex<HashMap<String, PeerCrypto>>> =
                Arc::new(std::sync::Mutex::new(HashMap::new()));
            let mut announce_destination: Option<Arc<tokio::sync::Mutex<SingleInputDestination>>> =
                None;
            let receipt_map: Arc<std::sync::Mutex<HashMap<String, String>>> =
                Arc::new(std::sync::Mutex::new(HashMap::new()));
            let (receipt_tx, mut receipt_rx) = unbounded_channel();

            if let Some(addr) = args.transport.clone() {
                let config = TransportConfig::new("daemon", &identity, true);
                let mut transport_instance = Transport::new(config);
                transport_instance
                    .set_receipt_handler(Box::new(ReceiptBridge::new(
                        receipt_map.clone(),
                        receipt_tx,
                    )))
                    .await;
                let iface_manager = transport_instance.iface_manager();
                iface_manager
                    .lock()
                    .await
                    .spawn(TcpServer::new(addr, iface_manager.clone()), TcpServer::spawn);
                if let Some(config_path) = args.config.as_ref() {
                    if let Ok(config) = DaemonConfig::from_path(config_path) {
                        for (host, port) in config.tcp_client_endpoints() {
                            let addr = format!("{}:{}", host, port);
                            iface_manager
                                .lock()
                                .await
                                .spawn(TcpClient::new(addr), TcpClient::spawn);
                            eprintln!(
                                "[daemon] tcp_client enabled name={} host={} port={}",
                                host,
                                host,
                                port
                            );
                        }
                    } else {
                        eprintln!(
                            "[daemon] failed to load config: {}",
                            config_path.display()
                        );
                    }
                }
                eprintln!("[daemon] transport enabled");

                let destination = transport_instance
                    .add_destination(identity.clone(), DestinationName::new("lxmf", "delivery"))
                    .await;
                {
                    let dest = destination.lock().await;
                    eprintln!(
                        "[daemon] delivery destination hash={}",
                        hex::encode(dest.desc.address_hash.as_slice())
                    );
                }
                announce_destination = Some(destination);
                transport = Some(Arc::new(transport_instance));
            }

            let bridge: Option<Arc<TransportBridge>> = transport
                .as_ref()
                .zip(announce_destination.as_ref())
                .map(|(transport, destination)| {
                    Arc::new(TransportBridge::new(
                        transport.clone(),
                        identity.clone(),
                        destination.clone(),
                        peer_crypto.clone(),
                        receipt_map.clone(),
                    ))
                });

            let outbound_bridge: Option<Arc<dyn OutboundBridge>> = bridge
                .as_ref()
                .map(|bridge| bridge.clone() as Arc<dyn OutboundBridge>);
            let announce_bridge: Option<Arc<dyn AnnounceBridge>> = bridge
                .as_ref()
                .map(|bridge| bridge.clone() as Arc<dyn AnnounceBridge>);

            let daemon = Rc::new(RpcDaemon::with_store_and_bridges(
                store,
                identity_hash,
                outbound_bridge,
                announce_bridge,
            ));

            if transport.is_some() {
                let daemon_receipts = daemon.clone();
                tokio::task::spawn_local(async move {
                    while let Some(event) = receipt_rx.recv().await {
                        let _ = handle_receipt_event(&daemon_receipts, event);
                    }
                });
            }

            if args.announce_interval_secs > 0 {
                let _handle = daemon.clone().start_announce_scheduler(args.announce_interval_secs);
            }

            if let Some(transport) = transport.clone() {
                let daemon_inbound = daemon.clone();
                let inbound_transport = transport.clone();
                let inbound_identity = identity.clone();
                tokio::task::spawn_local(async move {
                    let mut rx = inbound_transport.received_data_events();
                    loop {
                        if let Ok(event) = rx.recv().await {
                            let data = event.data.as_slice();
                            eprintln!(
                                "[daemon] rx data len={} dst={}",
                                data.len(),
                                hex::encode(event.destination.as_slice())
                            );
                            let payload = match decrypt_with_identity(
                                &inbound_identity,
                                inbound_identity.address_hash().as_slice(),
                                data,
                            ) {
                                Ok(plain) => plain,
                                Err(err) => {
                                    eprintln!("[daemon] decrypt failed: {:?}", err);
                                    data.to_vec()
                                }
                            };
                            let mut destination = [0u8; 16];
                            destination.copy_from_slice(event.destination.as_slice());
                            if let Some(record) = decode_inbound_payload(destination, &payload) {
                                let _ = daemon_inbound.accept_inbound(record);
                            }
                        }
                    }
                });

                let daemon_announce = daemon.clone();
                let peer_crypto = peer_crypto.clone();
                let announce_transport = transport.clone();
                tokio::task::spawn_local(async move {
                    let mut rx = announce_transport.recv_announces().await;
                    loop {
                        if let Ok(event) = rx.recv().await {
                            let dest = event.destination.lock().await;
                            let peer = hex::encode(dest.desc.address_hash.as_slice());
                            let public_key = *dest.desc.identity.public_key.as_bytes();
                            let mut identity_hash = [0u8; 16];
                            identity_hash.copy_from_slice(dest.desc.identity.address_hash.as_slice());
                            let identity = dest.desc.identity;
                            let ratchet = event.ratchet;
                            peer_crypto
                                .lock()
                                .expect("peer map")
                                .insert(
                                    peer.clone(),
                                    PeerCrypto {
                                        identity_hash,
                                        identity,
                                        public_key,
                                        ratchet,
                                    },
                                );
                            eprintln!("[daemon] rx announce peer={}", peer);
                            let timestamp = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|value| value.as_secs() as i64)
                                .unwrap_or(0);
                            let _ = daemon_announce.accept_announce(peer, timestamp);
                        }
                    }
                });
            }

            let listener = TcpListener::bind(addr).await.unwrap();
            println!("reticulumd listening on http://{}", addr);

            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buffer = Vec::new();
                loop {
                    let mut chunk = [0u8; 4096];
                    let read = stream.read(&mut chunk).await.unwrap();
                    if read == 0 {
                        break;
                    }
                    buffer.extend_from_slice(&chunk[..read]);
                    if let Some(header_end) = http::find_header_end(&buffer) {
                        let headers = &buffer[..header_end];
                        if let Some(length) = http::parse_content_length(headers) {
                            let body_start = header_end + 4;
                            if buffer.len() >= body_start + length {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }

                if buffer.is_empty() {
                    continue;
                }

                let response = http::handle_http_request(&daemon, &buffer)
                    .unwrap_or_else(|err| http::build_error_response(&format!("rpc error: {}", err)));
                let _ = stream.write_all(&response).await;
                let _ = stream.shutdown().await;
            }
        })
        .await;
}
