use std::{sync::{Arc, RwLock}, collections::BTreeMap};

use crate::error::AggregatorError;

#[derive(Clone, Debug)]
pub struct LockedBTreeMap<K: Ord, V: Clone> {
    inner: Arc<RwLock<BTreeMap<K, V>>>,
}

impl<K: Ord, V: Clone> LockedBTreeMap<K, V> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Result<(), AggregatorError> {
        self.inner
            .write()
            .map(|mut write_locked_storage| {
                write_locked_storage.insert(key, value);
            })
            .map_err(|e| AggregatorError::StorageError {
                reason: e.to_string(),
            })
    }

    pub fn get(&self, key: K) -> Result<Option<V>, AggregatorError> {
        self.inner
            .read()
            .map(|read_locked_storage| read_locked_storage.get(&key).cloned())
            .map_err(|e| AggregatorError::StorageError {
                reason: e.to_string(),
            })
    }
}

impl<K: Ord, V: Clone> Default for LockedBTreeMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}