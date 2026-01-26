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

#[test]
fn opens_disk_store() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("messages.db");
    let db = MessagesStore::open(&path).unwrap();
    db.insert_message(&MessageRecord {
        id: "m2".into(),
        source: "a".into(),
        destination: "b".into(),
        content: "hello".into(),
        timestamp: 2,
        direction: "in".into(),
    })
    .unwrap();
    drop(db);

    let db2 = MessagesStore::open(&path).unwrap();
    let items = db2.list_messages(10, None).unwrap();
    assert_eq!(items.len(), 1);
}
