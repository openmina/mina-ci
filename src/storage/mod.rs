use std::collections::BTreeMap;

pub mod locked_btree_map;
pub use locked_btree_map::*;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize)]
pub struct BuildStorage {
    #[serde(skip)]
    pub ipc_storage: IpcAggregatorStorage,
    #[serde(skip)]
    pub trace_storage: BlockTraceAggregatorStorage,
    #[serde(skip)]
    pub cross_validation_storage: CrossValidationStorage,
    #[serde(flatten)]
    pub build_info: BuildInfo,
    #[serde(flatten)]
    pub build_summary: BuildSummary,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct BuildSummary {
    pub block_count: usize,
    pub cannonical_block_count: usize,
    pub block_production_min: f64,
    pub block_production_avg: f64,
    pub block_production_max: f64,
    pub block_application_min: f64,
    pub block_application_avg: f64,
    pub block_application_max: f64,
    pub latency_min: f64,
    pub latency_avg: f64,
    pub latency_max: f64,
    // pub block_production: Statistics,
    // pub block_application: Statistics,
    // pub latency: Statistics,
    // pub latency
}

// #[derive(Debug, Default, Clone, Serialize)]
// pub struct Statistics {
//     pub average: f64,
//     pub min: f64,
//     pub max: f64,
//     // pub distribution: Vec<>,
// }

// pub struct

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct BuildInfo {
    #[serde()]
    pub id: usize,
    pub number: usize,
    #[serde(rename(serialize = "commit"))]
    pub after: String,
    pub status: String,
    pub started: u64,
    pub message: String,
    #[serde(rename(serialize = "branch"))]
    pub source: String,
}

impl BuildStorage {
    pub fn new() -> Self {
        Self {
            ipc_storage: BTreeMap::new(),
            trace_storage: BTreeMap::new(),
            cross_validation_storage: BTreeMap::new(),
            build_info: Default::default(),
            build_summary: Default::default(),
        }
    }

    // pub fn ipc_storage(&self) -> IpcAggregatorStorage {
    //     self.ipc_storage.clone()
    // }

    // pub fn trace_storage(&self) -> BlockTraceAggregatorStorage {
    //     self.trace_storage.clone()
    // }

    // pub fn cross_validation_storage(&self) -> CrossValidationStorage {
    //     self.cross_validation_storage.clone()
    // }
}

impl Default for BuildStorage {
    fn default() -> Self {
        Self::new()
    }
}
