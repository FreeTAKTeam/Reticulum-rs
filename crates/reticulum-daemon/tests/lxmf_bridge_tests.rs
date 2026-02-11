use reticulum::identity::PrivateIdentity;
use reticulum_daemon::lxmf_bridge::{
    build_wire_message, decode_wire_message, json_to_rmpv, rmpv_to_json,
};

#[test]
fn wire_roundtrip_preserves_content_title_fields() {
    let identity = PrivateIdentity::new_from_rand(rand_core::OsRng);
    let mut source = [0u8; 16];
    source.copy_from_slice(identity.address_hash().as_slice());
    let dest = [42u8; 16];
    let fields = serde_json::json!({"k": "v", "n": 2});

    let wire = build_wire_message(
        source,
        dest,
        "Hello",
        "World",
        Some(fields.clone()),
        &identity,
    )
    .expect("wire");

    let message = decode_wire_message(&wire).expect("decode");
    assert_eq!(message.title_as_string().as_deref(), Some("Hello"));
    assert_eq!(message.content_as_string().as_deref(), Some("World"));

    let roundtrip = message.fields.and_then(|value| rmpv_to_json(&value));
    assert_eq!(roundtrip, Some(fields));
}

#[test]
fn json_to_rmpv_roundtrip() {
    let input = serde_json::json!({"arr": [1, true, "ok"], "n": 9});
    let value = json_to_rmpv(&input).expect("to rmpv");
    let output = rmpv_to_json(&value).expect("to json");
    assert_eq!(output, input);
}
