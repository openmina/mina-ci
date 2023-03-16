use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use crate::error::AggregatorError;

#[derive(Clone, Debug)]
pub struct LockedBTreeMap<K: Ord + Clone, V: Clone> {
    inner: Arc<RwLock<BTreeMap<K, V>>>,
}

impl<K: Ord + Clone, V: Clone> LockedBTreeMap<K, V> {
    pub fn new(inner: BTreeMap<K, V>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(inner)),
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

    pub fn get_latest_value(&self) -> Result<Option<V>, AggregatorError> {
        self.inner
            .read()
            .map(|read_locked_storage| read_locked_storage.last_key_value().map(|(_, v)| v.clone()))
            .map_err(|e| AggregatorError::StorageError {
                reason: e.to_string(),
            })
    }

    pub fn get_latest_n_values(&self, n: usize) -> Result<Vec<V>, AggregatorError> {
        self.inner
            .read()
            .map(|read_locked_storage| {
                read_locked_storage
                    .values()
                    .cloned()
                    .rev()
                    .take(n)
                    .collect()
            })
            .map_err(|e| AggregatorError::StorageError {
                reason: e.to_string(),
            })
    }

    pub fn get_count(&self) -> Result<usize, AggregatorError> {
        self.inner
            .read()
            .map(|read_locked_storage| read_locked_storage.len())
            .map_err(|e| AggregatorError::StorageError {
                reason: e.to_string(),
            })
    }

    pub fn get_latest_key(&self) -> Result<Option<K>, AggregatorError> {
        self.inner
            .read()
            .map(|read_locked_storage| {
                read_locked_storage
                    .last_key_value()
                    .map(|(k, _)| k)
                    .cloned()
            })
            .map_err(|e| AggregatorError::StorageError {
                reason: e.to_string(),
            })
    }

    pub fn get_values(&self) -> Result<Vec<V>, AggregatorError> {
        self.inner
            .read()
            .map(|read_locked_storage| read_locked_storage.values().cloned().collect())
            .map_err(|e| AggregatorError::StorageError {
                reason: e.to_string(),
            })
    }

    pub fn to_btreemap(&self) -> Result<BTreeMap<K, V>, AggregatorError> {
        self.inner
            .read()
            .map(|read_locked_storage| read_locked_storage.clone())
            .map_err(|e| AggregatorError::StorageError {
                reason: e.to_string(),
            })
    }
}

impl<K: Ord + Clone, V: Clone> Default for LockedBTreeMap<K, V> {
    fn default() -> Self {
        Self {
            inner: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
}
