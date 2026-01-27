use rand_core::OsRng;
use reticulum::destination::link::Link;
use reticulum::destination::{DestinationDesc, DestinationName};
use reticulum::identity::PrivateIdentity;
use reticulum::packet::{DestinationType, PacketType};
use reticulum::transport::{Transport, TransportConfig};
use reticulum_daemon::direct_delivery::send_via_link;
use tokio::time::Duration;

#[tokio::test]
async fn direct_send_uses_link_payloads() {
    let sender = PrivateIdentity::new_from_rand(OsRng);
    let receiver = PrivateIdentity::new_from_rand(OsRng);

    let transport = Transport::new(TransportConfig::new("test", &sender, true));

    let destination = DestinationDesc {
        identity: receiver.as_identity().clone(),
        address_hash: *receiver.address_hash(),
        name: DestinationName::new("lxmf", "delivery"),
    };

    let link = transport.link(destination).await;
    let request = link.lock().await.request();

    let (event_tx, _) = tokio::sync::broadcast::channel(16);
    let mut input_link = Link::new_from_request(
        &request,
        receiver.sign_key().clone(),
        destination,
        event_tx,
    )
    .expect("input link");
    let proof = input_link.prove();

    link.lock().await.handle_packet(&proof);

    let packet = send_via_link(
        &transport,
        destination,
        b"hello link",
        Duration::from_secs(1),
    )
    .await
    .expect("send via link");

    assert_eq!(packet.header.destination_type, DestinationType::Link);
    assert_eq!(packet.header.packet_type, PacketType::Data);
}
