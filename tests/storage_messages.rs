use reticulum::storage::messages::{MessageRecord, MessagesStore};

#[test]
fn stores_and_reads_message() {
    let db = MessagesStore::in_memory().unwrap();
    db.insert_message(&MessageRecord {
        id: "m1".into(),
        source: "a".into(),
        destination: "b".into(),
        content: "hi".into(),
        timestamp: 1,
        direction: "in".into(),
    })
    .unwrap();
    let items = db.list_messages(10, None).unwrap();
    assert_eq!(items.len(), 1);
}
