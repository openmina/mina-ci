use std::collections::BTreeMap;

pub mod locked_btree_map;
pub use locked_btree_map::*;

use crate::{
    aggregators::{BlockHash, BlockTraceAggregatorReport, CpnpBlockPublication},
    cross_validation::ValidationReport,
};

pub type IpcAggregatorStorage = BTreeMap<usize, BTreeMap<BlockHash, CpnpBlockPublication>>;
pub type BlockTraceAggregatorStorage =
    BTreeMap<usize, BTreeMap<BlockHash, Vec<BlockTraceAggregatorReport>>>;

pub type CrossValidationStorage = BTreeMap<usize, BTreeMap<BlockHash, ValidationReport>>;

pub type BuildNumber = usize;

pub type AggregatorStorage = LockedBTreeMap<BuildNumber, BuildStorage>;

#[derive(Debug, Clone)]
pub struct BuildStorage {
    ipc_storage: IpcAggregatorStorage,
    trace_storage: BlockTraceAggregatorStorage,
    cross_validation_storage: CrossValidationStorage,
}

impl BuildStorage {
    pub fn new() -> Self {
        Self {
            ipc_storage: BTreeMap::new(),
            trace_storage: BTreeMap::new(),
            cross_validation_storage: BTreeMap::new(),
        }
    }

    pub fn ipc_storage(&self) -> IpcAggregatorStorage {
        self.ipc_storage.clone()
    }

    pub fn trace_storage(&self) -> BlockTraceAggregatorStorage {
        self.trace_storage.clone()
    }

    pub fn cross_validation_storage(&self) -> CrossValidationStorage {
        self.cross_validation_storage.clone()
    }
}

impl Default for BuildStorage {
    fn default() -> Self {
        Self::new()
    }
}
