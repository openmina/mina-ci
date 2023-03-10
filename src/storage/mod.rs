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

pub type BlockHeight = usize;

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
    pub tx_count: usize,
    pub block_production_min: f64,
    pub block_production_avg: f64,
    pub block_production_max: f64,
    pub block_application_min: f64,
    pub block_application_avg: f64,
    pub block_application_max: f64,
    pub receive_latency_min: f64,
    pub receive_latency_avg: f64,
    pub receive_latency_max: f64,
    pub application_times: Vec<f64>,
    pub production_times: Vec<f64>,
    pub receive_latencies: Vec<f64>,
    #[serde(skip)]
    pub helpers: BuildSummaryHelpers,
    // #[serde(skip)]
    // pub avg_total_count: usize,
    // #[serde(skip)]
    // pub avg_total: f64,
    // pub block_production: Statistics,
    // pub block_application: Statistics,
    // pub latency: Statistics,
    // pub latency
}

#[derive(Debug, Default, Clone)]
pub struct BuildSummaryHelpers {
    /// Total occurences, to calculate the avg
    pub application_avg_total_count: BTreeMap<BlockHeight, usize>,
    /// Total, to calculate average online
    pub application_total: BTreeMap<BlockHeight, f64>,
    /// Total occurences, to calculate the avg
    pub production_avg_total_count: BTreeMap<BlockHeight, usize>,
    /// Total, to calculate average online
    pub production_total: BTreeMap<BlockHeight, f64>,
    /// Total occurences, to calculate the avg
    pub receive_latencies_avg_total_count: BTreeMap<BlockHeight, usize>,
    /// Total, to calculate average online
    pub receive_latencies_total: BTreeMap<BlockHeight, f64>,

    // times for histograms
    pub application_times: BTreeMap<BlockHeight, Vec<f64>>,
    pub production_times: BTreeMap<BlockHeight, Vec<f64>>,
    pub block_count_per_height: BTreeMap<BlockHeight, usize>,
    pub receive_latencies: BTreeMap<BlockHeight, Vec<f64>>,

    /// tx counts at each height
    pub tx_count_per_height: BTreeMap<BlockHeight, usize>,
}

impl BuildSummaryHelpers {
    pub fn get_application_average(&self) -> f64 {
        let count: usize = self.application_avg_total_count.values().sum();
        let total: f64 = self.application_total.values().sum();

        total / (count as f64)
    }

    pub fn get_production_average(&self) -> f64 {
        let count: usize = self.production_avg_total_count.values().sum();
        let total: f64 = self.production_total.values().sum();

        total / (count as f64)
    }

    pub fn get_latencies_average(&self) -> f64 {
        let count: usize = self.receive_latencies_avg_total_count.values().sum();
        let total: f64 = self.receive_latencies_total.values().sum();

        total / (count as f64)
    }
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
pub struct StepInfo {
    pub name: String,
    pub number: usize,
    pub status: String,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct StageInfo {
    pub name: String,
    pub number: usize,
    pub status: String,
    pub steps: Option<Vec<StepInfo>>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct BuildInfoExpanded {
    pub status: String,
    #[serde(skip_serializing)]
    pub stages: Vec<StageInfo>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct BuildInfo {
    #[serde(skip_serializing)]
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
