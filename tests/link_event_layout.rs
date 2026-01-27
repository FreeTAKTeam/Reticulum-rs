use reticulum::destination::link::{LinkEvent, LinkPayload};

#[test]
fn link_event_handles_boxed_payload() {
    let payload = LinkPayload::default();
    let event = LinkEvent::Data(Box::new(payload));
    match event {
        LinkEvent::Data(_) => {}
        _ => panic!("expected data event"),
    }
}
