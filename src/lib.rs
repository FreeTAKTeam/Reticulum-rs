#[cfg(feature = "alloc")]
extern crate alloc;

pub mod buffer;
pub mod channel;
pub mod crypt;
pub mod config;
pub mod destination;
pub mod e2e_harness;
pub mod error;
pub mod hash;
pub mod identity;
pub mod iface;
pub mod packet;
pub mod resource;
pub mod rpc;
pub mod storage;
pub mod transport;

pub use crate::destination::{group_decrypt, group_encrypt};
pub use crate::hash::lxmf_address_hash;
pub use crate::identity::{lxmf_sign, lxmf_verify};
pub use crate::packet::{LXMF_MAX_PAYLOAD, Packet};
pub use crate::transport::{DeliveryReceipt, ReceiptHandler};

pub mod utils;
mod serde;
