#[test]
fn decodes_python_packet() {
    let bytes = std::fs::read("tests/fixtures/python/reticulum/packet_basic.bin").unwrap();
    let packet = reticulum::packet::Packet::from_bytes(&bytes).unwrap();
    let encoded = packet.to_bytes().unwrap();
    assert_eq!(bytes, encoded);
}
