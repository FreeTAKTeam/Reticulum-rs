use reticulum::resource::{Resource, ResourceReceipt};

#[tokio::test]
async fn resource_send_receives_receipt() {
    let data = vec![1u8; 2048];
    let resource = Resource::from_bytes(data.clone());
    let receipt = resource.send_for_test().await;
    assert!(matches!(receipt, ResourceReceipt::Delivered));
}
