use reticulum::packet::{Packet, PACKET_MDU, LXMF_MAX_PAYLOAD};

#[test]
fn lxmf_payload_caps_match_packet_mdu() {
    // LXMF_MAX_PAYLOAD must always be less than PACKET_MDU
    assert!(LXMF_MAX_PAYLOAD < PACKET_MDU);

    // Ensure we can fragment exactly at boundary
    let payload = vec![0u8; LXMF_MAX_PAYLOAD * 2 + 1];
    let packets = Packet::fragment_for_lxmf(&payload).expect("fragment");
    assert_eq!(packets.len(), 3);
    assert!(packets.iter().all(|p| p.data.len() <= LXMF_MAX_PAYLOAD));
}

#[test]
fn lxmf_payload_caps_are_consistent_with_packet_overhead() {
    // Expect fernet overhead and max padding subtraction from PACKET_MDU.
    let expected = PACKET_MDU
        - reticulum::crypt::fernet::FERNET_OVERHEAD_SIZE
        - reticulum::crypt::fernet::FERNET_MAX_PADDING_SIZE;
    assert_eq!(LXMF_MAX_PAYLOAD, expected);
}
