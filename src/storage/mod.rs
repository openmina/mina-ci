use std::collections::BTreeMap;

pub mod locked_btree_map;
pub use locked_btree_map::*;

pub mod remote;
pub use remote::*;

use serde::{Deserialize, Serialize};

use crate::{
    aggregators::{AggregatedBlockTraces, BlockHash, CpnpBlockPublication},
    cross_validation::ValidationReport,
    nodes::RequestStats,
};

pub type IpcAggregatorStorage = BTreeMap<usize, BTreeMap<BlockHash, CpnpBlockPublication>>;
pub type BlockTraceAggregatorStorage = BTreeMap<usize, AggregatedBlockTraces>;

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
    #[serde(skip)]
    pub helpers: BuildSummaryHelpers,
    #[serde(skip)]
    pub block_summaries: BTreeMap<BlockHash, BlockSummary>,
    #[serde(skip)]
    pub best_chain: BTreeMap<BlockHeight, BlockHash>,
}

impl From<BuildStorage> for BuildStorageDump {
    fn from(value: BuildStorage) -> Self {
        Self {
            ipc_storage: value.ipc_storage,
            trace_storage: value.trace_storage,
            cross_validation_storage: value.cross_validation_storage,
            build_info: value.build_info,
            build_summary: value.build_summary,
            block_summaries: value.block_summaries,
            helpers: value.helpers,
            best_chain: value.best_chain,
        }
    }
}

impl From<BuildStorageDump> for BuildStorage {
    fn from(value: BuildStorageDump) -> Self {
        Self {
            ipc_storage: value.ipc_storage,
            trace_storage: value.trace_storage,
            cross_validation_storage: value.cross_validation_storage,
            build_info: value.build_info,
            build_summary: value.build_summary,
            block_summaries: value.block_summaries,
            helpers: value.helpers,
            best_chain: value.best_chain,
        }
    }
}

impl BuildStorage {
    pub fn store_data(
        &mut self,
        height: usize,
        traces: AggregatedBlockTraces,
        ipc_data: BTreeMap<BlockHash, CpnpBlockPublication>,
        cross_validation_report: BTreeMap<BlockHash, ValidationReport>,
    ) {
        let _ = self.trace_storage.insert(height, traces);
        let _ = self.ipc_storage.insert(height, ipc_data);
        let _ = self
            .cross_validation_storage
            .insert(height, cross_validation_report);
    }

    pub fn update_summary(
        &mut self,
        height: usize,
        block_traces: &AggregatedBlockTraces,
        request_stats: RequestStats,
    ) {
        // per block summaries
        self.block_summaries
            .extend(block_traces.block_summaries(height));

        for (block_hash, tx_count) in block_traces.transaction_count_per_block() {
            self.helpers.tx_count_per_block.insert(block_hash, tx_count);
        }

        self.build_summary.request_timeout_count += request_stats.request_timeout_count;
        self.build_summary.request_count += request_stats.request_count;

        self.build_summary.tx_count = self
            .helpers
            .tx_count_per_block
            .iter()
            .filter(|(k, _)| self.best_chain.iter().any(|(_, hash)| &hash == k))
            .map(|(_, v)| v)
            .sum();

        let application_times: Vec<f64> = block_traces.application_times();

        // TODO: get from producer_traces
        let production_times: Vec<f64> = block_traces.production_times();

        let receive_latencies: Vec<f64> = block_traces.receive_latencies();

        let application_time_sum: f64 = application_times.iter().sum();
        let application_min = f64_min(&application_times);
        let application_max = f64_max(&application_times);

        let production_time_sum: f64 = production_times.iter().sum();
        let production_min = f64_min(&production_times);
        let production_max = f64_max(&production_times);

        let receive_latencies_sum: f64 = receive_latencies.iter().sum();
        let receive_latencies_min = f64_min(&receive_latencies);
        let receive_latencies_max = f64_max(&receive_latencies);

        let unique_block_count = block_traces.unique_block_count();
        let application_measurement_count: usize = block_traces.appliaction_count();
        let production_measurement_count: usize = block_traces.production_count();

        // store the aggregated values
        self.helpers
            .application_times
            .insert(height, application_times);
        self.helpers
            .production_times
            .insert(height, production_times);
        self.helpers
            .receive_latencies
            .insert(height, receive_latencies);

        self.helpers
            .application_avg_total_count
            .insert(height, application_measurement_count);
        self.helpers
            .application_total
            .insert(height, application_time_sum);
        self.helpers
            .production_avg_total_count
            .insert(height, production_measurement_count);
        self.helpers
            .production_total
            .insert(height, production_time_sum);
        // the count is same as the application ones (optimization: remove this helper map and use the application map?)
        self.helpers
            .receive_latencies_avg_total_count
            .insert(height, application_measurement_count);
        self.helpers
            .receive_latencies_total
            .insert(height, receive_latencies_sum);

        self.helpers
            .block_count_per_height
            .insert(height, unique_block_count);
        self.build_summary.block_count = self.helpers.block_count_per_height.values().sum();
        // We don't have traces for blocks on height 0 and 1, so add them in
        self.build_summary.cannonical_block_count = self.helpers.application_total.len() + 2;

        if self.build_summary.block_application_min == 0.0 {
            self.build_summary.block_application_min = application_min;
        } else {
            self.build_summary.block_application_min =
                application_min.min(self.build_summary.block_application_min);
        }
        self.build_summary.block_application_max =
            application_max.max(self.build_summary.block_application_max);
        self.build_summary.block_application_avg = self.helpers.get_application_average();

        if self.build_summary.block_production_min == 0.0 {
            self.build_summary.block_production_min = production_min;
        } else {
            self.build_summary.block_production_min =
                production_min.min(self.build_summary.block_production_min);
        }
        self.build_summary.block_production_max =
            production_max.max(self.build_summary.block_production_max);
        self.build_summary.block_production_avg = self.helpers.get_production_average();

        if self.build_summary.receive_latency_min == 0.0 {
            self.build_summary.receive_latency_min = receive_latencies_min;
        } else {
            self.build_summary.receive_latency_min =
                receive_latencies_min.min(self.build_summary.receive_latency_min);
        }
        self.build_summary.receive_latency_max =
            receive_latencies_max.max(self.build_summary.receive_latency_max);
        self.build_summary.receive_latency_avg = self.helpers.get_latencies_average();
    }

    pub fn calculate_deltas(&mut self, other: &Self) {
        self.build_summary.block_application_avg_delta =
            other.build_summary.block_application_avg - self.build_summary.block_application_avg;
        self.build_summary.block_application_max_delta =
            other.build_summary.block_application_max - self.build_summary.block_application_max;
        self.build_summary.block_application_min_delta =
            other.build_summary.block_application_min - self.build_summary.block_application_min;
        self.build_summary.block_application_regression = self
            .build_summary
            .block_application_max_delta
            .is_sign_positive();

        self.build_summary.block_production_avg_delta =
            other.build_summary.block_production_avg - self.build_summary.block_production_avg;
        self.build_summary.block_production_max_delta =
            other.build_summary.block_production_max - self.build_summary.block_production_max;
        self.build_summary.block_production_min_delta =
            other.build_summary.block_production_min - self.build_summary.block_production_min;
        self.build_summary.block_production_regression = self
            .build_summary
            .block_production_max_delta
            .is_sign_positive();

        self.build_summary.receive_latency_avg_delta =
            other.build_summary.receive_latency_avg - self.build_summary.receive_latency_avg;
        self.build_summary.receive_latency_max_delta =
            other.build_summary.receive_latency_max - self.build_summary.receive_latency_max;
        self.build_summary.receive_latency_min_delta =
            other.build_summary.receive_latency_min - self.build_summary.receive_latency_min;
        self.build_summary.receive_latency_regression = self
            .build_summary
            .receive_latency_max_delta
            .is_sign_positive();

        self.build_summary.application_times_previous =
            other.build_summary.application_times.clone();
        self.build_summary.production_times_previous = other.build_summary.production_times.clone();
        self.build_summary.receive_latencies_previous =
            other.build_summary.receive_latencies.clone();
    }

    pub fn include_times(&mut self) {
        self.build_summary.application_times = self
            .helpers
            .application_times
            .values()
            .flatten()
            .cloned()
            .collect();
        self.build_summary.production_times = self
            .helpers
            .production_times
            .values()
            .flatten()
            .cloned()
            .collect();
        self.build_summary.receive_latencies = self
            .helpers
            .receive_latencies
            .values()
            .flatten()
            .cloned()
            .collect();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildStorageDump {
    pub ipc_storage: IpcAggregatorStorage,
    pub trace_storage: BlockTraceAggregatorStorage,
    pub cross_validation_storage: CrossValidationStorage,
    pub build_info: BuildInfo,
    pub build_summary: BuildSummary,
    pub block_summaries: BTreeMap<BlockHash, BlockSummary>,
    pub helpers: BuildSummaryHelpers,
    pub best_chain: BTreeMap<BlockHeight, BlockHash>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockSummary {
    pub height: usize,
    pub block_hash: String,
    pub global_slot: Option<String>,
    pub tx_count: Option<usize>,
    pub max_receive_latency: f64,
    pub date_time: Option<f64>,
    pub block_producer: Option<String>,
    pub block_producer_nodes: Vec<String>,
    pub peer_timings: Vec<PeerTiming>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerTiming {
    pub node: String,
    pub block_processing_time: Option<f64>,
    pub receive_latency: Option<f64>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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
    pub block_production_min_delta: f64,
    pub block_production_avg_delta: f64,
    pub block_production_max_delta: f64,
    pub block_application_min_delta: f64,
    pub block_application_avg_delta: f64,
    pub block_application_max_delta: f64,
    pub receive_latency_min_delta: f64,
    pub receive_latency_avg_delta: f64,
    pub receive_latency_max_delta: f64,
    pub application_times_previous: Vec<f64>,
    pub production_times_previous: Vec<f64>,
    pub receive_latencies_previous: Vec<f64>,
    pub block_production_regression: bool,
    pub block_application_regression: bool,
    pub receive_latency_regression: bool,
    pub request_count: usize,
    pub request_timeout_count: usize,
    // #[serde(skip)]
    // pub avg_total_count: usize,
    // #[serde(skip)]
    // pub avg_total: f64,
    // pub block_production: Statistics,
    // pub block_application: Statistics,
    // pub latency: Statistics,
    // pub latency
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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

    /// tx counts at each block
    pub tx_count_per_block: BTreeMap<BlockHash, usize>,
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
    pub number: usize,
    #[serde(rename(serialize = "commit"), alias = "commit")]
    pub after: String,
    pub status: String,
    pub started: u64,
    pub message: String,
    #[serde(rename(serialize = "branch"), alias = "branch")]
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
            block_summaries: Default::default(),
            helpers: Default::default(),
            best_chain: Default::default(),
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

fn f64_min(values: &[f64]) -> f64 {
    values
        .iter()
        .copied()
        .reduce(|a, b| a.min(b))
        .unwrap_or(f64::MAX)
}

fn f64_max(values: &[f64]) -> f64 {
    values
        .iter()
        .copied()
        .reduce(|a, b| a.max(b))
        .unwrap_or(f64::MIN)
}

#[cfg(test)]
mod tests {
    use crate::{
        aggregators::{AggregatedBlockTraces, BlockTraceAggregatorReport},
        nodes::RequestStats,
        storage::BuildStorage,
    };

    #[test]
    fn test_summary_update_one_block() {
        // create empty storage
        let mut storage = BuildStorage::default();

        // set height
        let height = 1;

        // set block hash
        let block_hash = "Height1Block1".to_string();

        // create dummy traces
        let trace_1 = BlockTraceAggregatorReport {
            height,
            node: "prod1".to_string(),
            block_hash: block_hash.clone(),
            is_producer: true,
            receive_latency: Some(-20.40),
            block_application: Some(20.40),
            included_tranasction_count: Some(128),
            ..Default::default()
        };

        let trace_2 = BlockTraceAggregatorReport {
            height,
            node: "node1".to_string(),
            block_hash: block_hash.clone(),
            is_producer: false,
            receive_latency: Some(1.20),
            block_application: Some(12.40),
            included_tranasction_count: None,
            ..Default::default()
        };

        let trace_3 = BlockTraceAggregatorReport {
            height,
            node: "node2".to_string(),
            block_hash: block_hash.clone(),
            is_producer: false,
            receive_latency: Some(2.20),
            block_application: Some(8.40),
            included_tranasction_count: None,
            ..Default::default()
        };

        let raw_traces: Vec<BlockTraceAggregatorReport> = vec![trace_1, trace_2, trace_3];
        let mut block_traces = AggregatedBlockTraces::default();

        block_traces.insert(block_hash, raw_traces);

        storage.best_chain.insert(1, "Height1Block1".to_string());

        storage.update_summary(height, &block_traces, RequestStats::default());

        // check min, max and average calculations

        // production times
        // only one block produced, should be the same
        let expected = 20.40;
        assert_eq!(expected, storage.build_summary.block_production_min);
        assert_eq!(expected, storage.build_summary.block_production_max);
        assert_eq!(expected, storage.build_summary.block_production_avg);

        // aplication times
        // min
        let expected = 8.40;
        assert_eq!(expected, storage.build_summary.block_application_min);
        // max
        // We separate application and production times, so the production time does not influence this
        let expected = 12.40;
        assert_eq!(expected, storage.build_summary.block_application_max);
        // avg
        let expected = 10.4;
        assert_eq!(expected, storage.build_summary.block_application_avg);

        // receive_latencies
        // We ignore the latency on producer as it is negative (it is the origin of the block)
        // min
        let expected = 1.20;
        assert_eq!(expected, storage.build_summary.receive_latency_min);
        // max
        let expected = 2.20;
        assert_eq!(expected, storage.build_summary.receive_latency_max);
        // avg
        let expected = 1.7;
        assert!(almost::equal(
            expected,
            storage.build_summary.receive_latency_avg
        ));

        // transaction count
        let expected = 128;
        assert_eq!(expected, storage.build_summary.tx_count);

        // block counts
        let expected = 1;
        assert_eq!(expected, storage.build_summary.block_count);
        // +2 to include real node's blocks at height 0 and 1
        assert_eq!(expected + 2, storage.build_summary.cannonical_block_count);

        storage.store_data(height, block_traces, Default::default(), Default::default());
    }

    #[test]
    fn test_summary_update_multiple_blocks_one_per_height() {
        // create empty storage
        let mut storage = BuildStorage::default();

        // height 1 block 1
        // set height
        let height = 1;

        // set block hash
        let block_hash = "Height1Block1".to_string();

        // create dummy traces
        let trace_1 = BlockTraceAggregatorReport {
            height,
            node: "prod1".to_string(),
            block_hash: block_hash.clone(),
            is_producer: true,
            receive_latency: Some(-20.40),
            block_application: Some(20.40),
            included_tranasction_count: Some(128),
            ..Default::default()
        };

        let trace_2 = BlockTraceAggregatorReport {
            height,
            node: "node1".to_string(),
            block_hash: block_hash.clone(),
            is_producer: false,
            receive_latency: Some(1.20),
            block_application: Some(12.40),
            included_tranasction_count: None,
            ..Default::default()
        };

        let trace_3 = BlockTraceAggregatorReport {
            height,
            node: "node2".to_string(),
            block_hash: block_hash.clone(),
            is_producer: false,
            receive_latency: Some(2.20),
            block_application: Some(8.40),
            included_tranasction_count: None,
            ..Default::default()
        };

        let raw_traces: Vec<BlockTraceAggregatorReport> = vec![trace_1, trace_2, trace_3];
        let mut block_traces = AggregatedBlockTraces::default();

        block_traces.insert(block_hash, raw_traces);

        storage.update_summary(height, &block_traces, RequestStats::default());
        storage.store_data(height, block_traces, Default::default(), Default::default());

        // height 2 block 1
        // set height
        let height = 2;

        // set block hash
        let block_hash = "Height2Block1".to_string();

        // create dummy traces
        let trace_1 = BlockTraceAggregatorReport {
            height,
            node: "prod1".to_string(),
            block_hash: block_hash.clone(),
            is_producer: true,
            receive_latency: Some(-21.60),
            block_application: Some(21.60),
            included_tranasction_count: Some(55),
            ..Default::default()
        };

        let trace_2 = BlockTraceAggregatorReport {
            height,
            node: "node1".to_string(),
            block_hash: block_hash.clone(),
            is_producer: false,
            receive_latency: Some(0.20),
            block_application: Some(1.40),
            included_tranasction_count: None,
            ..Default::default()
        };

        let trace_3 = BlockTraceAggregatorReport {
            height,
            node: "node2".to_string(),
            block_hash: block_hash.clone(),
            is_producer: false,
            receive_latency: Some(0.234),
            block_application: Some(3.0),
            included_tranasction_count: None,
            ..Default::default()
        };

        // storage.best_chain = vec!["Height1Block1".to_string(), "Height2Block1".to_string()];
        storage.best_chain.insert(1, "Height1Block1".to_string());
        storage.best_chain.insert(2, "Height2Block1".to_string());

        let raw_traces: Vec<BlockTraceAggregatorReport> = vec![trace_1, trace_2, trace_3];
        let mut block_traces = AggregatedBlockTraces::default();

        block_traces.insert(block_hash, raw_traces);

        storage.update_summary(height, &block_traces, RequestStats::default());

        // check min, max and average calculations
        // production times
        // min
        let expected = 20.40;
        assert_eq!(expected, storage.build_summary.block_production_min);
        // max
        let expected = 21.60;
        assert_eq!(expected, storage.build_summary.block_production_max);
        // avg
        let expected = 21.0;
        assert!(almost::equal(
            expected,
            storage.build_summary.block_production_avg
        ));

        // aplication times
        // min
        let expected = 1.40;
        assert_eq!(expected, storage.build_summary.block_application_min);
        // max
        // We separate application and production times, so the production time does not influence this
        let expected = 12.40;
        assert_eq!(expected, storage.build_summary.block_application_max);
        // avg
        let expected = 6.3;
        assert!(almost::equal(
            expected,
            storage.build_summary.block_application_avg
        ));

        // receive_latencies
        // We ignore the latency on producer as it is negative (it is the origin of the block)
        // min
        let expected = 0.20;
        assert_eq!(expected, storage.build_summary.receive_latency_min);
        // max
        let expected = 2.20;
        assert_eq!(expected, storage.build_summary.receive_latency_max);
        // avg
        let expected = 0.9585;
        assert!(almost::equal(
            expected,
            storage.build_summary.receive_latency_avg
        ));

        // transaction count
        let expected = 183;
        assert_eq!(expected, storage.build_summary.tx_count);

        // block counts
        let expected = 2;
        assert_eq!(expected, storage.build_summary.block_count);
        assert_eq!(expected + 2, storage.build_summary.cannonical_block_count);

        storage.store_data(height, block_traces, Default::default(), Default::default());
    }

    #[test]
    fn test_summary_update_multiple_blocks_multiple_per_height() {
        // create empty storage
        let mut storage = BuildStorage::default();

        // height 1 block 1
        // set height
        let height = 1;

        // set block hash
        let block_hash = "Height1Block1".to_string();

        // create dummy traces
        let trace_1 = BlockTraceAggregatorReport {
            height,
            node: "prod1".to_string(),
            block_hash: block_hash.clone(),
            is_producer: true,
            receive_latency: Some(-20.40),
            block_application: Some(20.40),
            included_tranasction_count: Some(128),
            ..Default::default()
        };

        let trace_2 = BlockTraceAggregatorReport {
            height,
            node: "node1".to_string(),
            block_hash: block_hash.clone(),
            is_producer: false,
            receive_latency: Some(1.20),
            block_application: Some(12.40),
            included_tranasction_count: None,
            ..Default::default()
        };

        let trace_3 = BlockTraceAggregatorReport {
            height,
            node: "node2".to_string(),
            block_hash: block_hash.clone(),
            is_producer: false,
            receive_latency: Some(2.20),
            block_application: Some(8.40),
            included_tranasction_count: None,
            ..Default::default()
        };

        let trace_4 = BlockTraceAggregatorReport {
            height,
            node: "prod2".to_string(),
            block_hash: block_hash.clone(),
            is_producer: false,
            receive_latency: Some(2.60),
            block_application: Some(12.0),
            included_tranasction_count: None,
            ..Default::default()
        };

        let raw_traces: Vec<BlockTraceAggregatorReport> = vec![trace_1, trace_2, trace_3, trace_4];
        let mut block_traces = AggregatedBlockTraces::default();

        block_traces.insert(block_hash, raw_traces);

        storage.update_summary(height, &block_traces, RequestStats::default());
        storage.store_data(height, block_traces, Default::default(), Default::default());

        // height 2 block 1
        // set height
        let height = 2;

        // set block hash
        let block_hash = "Height2Block1".to_string();

        // create dummy traces
        let trace_1 = BlockTraceAggregatorReport {
            height,
            node: "prod1".to_string(),
            block_hash: block_hash.clone(),
            is_producer: true,
            receive_latency: Some(-21.60),
            block_application: Some(21.60),
            included_tranasction_count: Some(120),
            ..Default::default()
        };

        let trace_2 = BlockTraceAggregatorReport {
            height,
            node: "node1".to_string(),
            block_hash: block_hash.clone(),
            is_producer: false,
            receive_latency: Some(0.20),
            block_application: Some(1.40),
            included_tranasction_count: None,
            ..Default::default()
        };

        let trace_3 = BlockTraceAggregatorReport {
            height,
            node: "node2".to_string(),
            block_hash: block_hash.clone(),
            is_producer: false,
            receive_latency: Some(0.234),
            block_application: Some(3.0),
            included_tranasction_count: None,
            ..Default::default()
        };

        let trace_4 = BlockTraceAggregatorReport {
            height,
            node: "prod2".to_string(),
            block_hash: block_hash.clone(),
            is_producer: false,
            receive_latency: Some(1.60),
            block_application: Some(10.0),
            included_tranasction_count: None,
            ..Default::default()
        };

        let raw_traces: Vec<BlockTraceAggregatorReport> = vec![trace_1, trace_2, trace_3, trace_4];
        let mut block_traces = AggregatedBlockTraces::default();

        block_traces.insert(block_hash, raw_traces);

        // height 2 block 2
        // set block hash
        let block_hash = "Height2Block2".to_string();

        // create dummy traces
        let trace_1 = BlockTraceAggregatorReport {
            height,
            node: "prod1".to_string(),
            block_hash: block_hash.clone(),
            is_producer: false,
            receive_latency: Some(0.5),
            block_application: Some(1.50),
            included_tranasction_count: None,
            ..Default::default()
        };

        let trace_2 = BlockTraceAggregatorReport {
            height,
            node: "node1".to_string(),
            block_hash: block_hash.clone(),
            is_producer: false,
            receive_latency: Some(0.80),
            block_application: Some(1.90),
            included_tranasction_count: None,
            ..Default::default()
        };

        let trace_3 = BlockTraceAggregatorReport {
            height,
            node: "node2".to_string(),
            block_hash: block_hash.clone(),
            is_producer: false,
            receive_latency: Some(0.444),
            block_application: Some(3.9),
            included_tranasction_count: None,
            ..Default::default()
        };

        let trace_4 = BlockTraceAggregatorReport {
            height,
            node: "prod2".to_string(),
            block_hash: block_hash.clone(),
            is_producer: true,
            receive_latency: Some(-32.1),
            block_application: Some(32.1),
            included_tranasction_count: Some(55),
            ..Default::default()
        };

        let raw_traces: Vec<BlockTraceAggregatorReport> = vec![trace_1, trace_2, trace_3, trace_4];
        block_traces.insert(block_hash, raw_traces);

        // storage.best_chain = vec!["Height1Block1".to_string(), "Height2Block2".to_string()];
        storage.best_chain.insert(1, "Height1Block1".to_string());
        storage.best_chain.insert(2, "Height2Block2".to_string());

        storage.update_summary(height, &block_traces, RequestStats::default());

        // check min, max and average calculations
        // production times
        // min
        let expected = 20.40;
        assert_eq!(expected, storage.build_summary.block_production_min);
        // max
        let expected = 32.1;
        assert_eq!(expected, storage.build_summary.block_production_max);
        // avg
        let expected = 24.7;
        assert!(almost::equal(
            expected,
            storage.build_summary.block_production_avg
        ));

        // aplication times
        // min
        let expected = 1.40;
        assert_eq!(expected, storage.build_summary.block_application_min);
        // max
        // We separate application and production times, so the production time does not influence this
        let expected = 12.40;
        assert_eq!(expected, storage.build_summary.block_application_max);
        // avg
        let expected = 6.055555556;
        assert!(almost::equal(
            expected,
            storage.build_summary.block_application_avg
        ));

        // receive_latencies
        // We ignore the latency on producer as it is negative (it is the origin of the block)
        // min
        let expected = 0.20;
        assert_eq!(expected, storage.build_summary.receive_latency_min);
        // max
        let expected = 2.60;
        assert_eq!(expected, storage.build_summary.receive_latency_max);
        // avg
        let expected = 1.086444444;
        assert!(almost::equal(
            expected,
            storage.build_summary.receive_latency_avg
        ));

        // transaction count
        let expected = 183;
        assert_eq!(expected, storage.build_summary.tx_count);

        // block counts
        let expected = 3;
        assert_eq!(expected, storage.build_summary.block_count);

        // cannonical_block_count should always equal height
        // TODO: rework test to mimic real traces, where there are non on height 0 and 1, for now, just add 2
        assert_eq!(height + 2, storage.build_summary.cannonical_block_count);

        storage.store_data(height, block_traces, Default::default(), Default::default());
    }
}
