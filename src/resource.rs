#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceReceipt {
    Delivered,
}

#[derive(Debug, Clone)]
pub struct Resource {
    data: Vec<u8>,
}

impl Resource {
    pub fn from_bytes(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub async fn send_for_test(&self) -> ResourceReceipt {
        ResourceReceipt::Delivered
    }
}
