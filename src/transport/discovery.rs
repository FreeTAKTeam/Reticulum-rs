use std::collections::{HashSet, VecDeque};

use crate::hash::AddressHash;

pub struct DiscoveryCache {
    max_size: usize,
    order: VecDeque<AddressHash>,
    set: HashSet<AddressHash>,
}

impl DiscoveryCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            order: VecDeque::new(),
            set: HashSet::new(),
        }
    }

    pub fn seen(&self, hash: &AddressHash) -> bool {
        self.set.contains(hash)
    }

    pub fn mark_seen(&mut self, hash: AddressHash) -> bool {
        if self.set.contains(&hash) {
            return false;
        }

        self.set.insert(hash);
        self.order.push_back(hash);

        if self.order.len() > self.max_size {
            if let Some(old) = self.order.pop_front() {
                self.set.remove(&old);
            }
        }

        true
    }

    pub fn len(&self) -> usize {
        self.set.len()
    }
}
