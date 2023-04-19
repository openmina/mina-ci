use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::aggregators::{AggregatedBlockTraces, BlockHash, CpnpBlockPublication};

pub type ComparisonDeltas = BTreeMap<String, f64>;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ValidationReport {
    // pub measured_latency_comparison: BTreeMap<String, f64>,
    pub received_time_comparison: ComparisonDeltas,
    pub all_nodes_present: bool,
    pub missing_nodes: Vec<String>,
    pub producer_nodes: Vec<String>,
    pub height: usize,
}

impl ValidationReport {
    fn aggregatge_min(&self, other: &Self) -> Self {
        let mut res = Self::default();

        for (k, v) in &self.received_time_comparison {
            if let Some(other_data) = other.received_time_comparison.get(k) {
                res.received_time_comparison
                    .insert(k.to_string(), v.min(*other_data));
            } else {
                res.received_time_comparison.insert(k.to_string(), *v);
            }
        }

        for (k, v) in &other.received_time_comparison {
            if let Some(self_data) = self.received_time_comparison.get(k) {
                res.received_time_comparison
                    .insert(k.to_string(), v.min(*self_data));
            } else {
                res.received_time_comparison.insert(k.to_string(), *v);
            }
        }

        res
    }

    fn aggregatge_max(&self, other: &Self) -> Self {
        let mut res = Self::default();

        for (k, v) in &self.received_time_comparison {
            if let Some(other_data) = other.received_time_comparison.get(k) {
                res.received_time_comparison
                    .insert(k.to_string(), v.max(*other_data));
            } else {
                res.received_time_comparison.insert(k.to_string(), *v);
            }
        }

        for (k, v) in &other.received_time_comparison {
            if let Some(self_data) = self.received_time_comparison.get(k) {
                res.received_time_comparison
                    .insert(k.to_string(), v.max(*self_data));
            } else {
                res.received_time_comparison.insert(k.to_string(), *v);
            }
        }

        res
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct AggregateValidationReport {
    pub min_recevied_time_comparison: Option<ComparisonDeltas>,
    // pub avg_recevied_time_comparison: ComparisonDeltas,
    pub max_recevied_time_comparison: Option<ComparisonDeltas>,
    pub height_start: Option<usize>,
    pub height_end: Option<usize>,
    pub total_heights_checked: usize,
    pub total_blocks_checked: usize,
    pub all_nodes_present: Option<bool>,
}

// TODO: unwraps
pub fn _cross_validate_ipc_with_traces(
    traces: AggregatedBlockTraces,
    ipc_reports: BTreeMap<BlockHash, CpnpBlockPublication>,
    height: usize,
) -> BTreeMap<String, ValidationReport> {
    let mut by_block: BTreeMap<String, ValidationReport> = BTreeMap::new();

    // TODO: move this to AggregatedBlockTraces as a method?
    for (block_hash, block_traces) in traces.inner().iter() {
        let mut report = ValidationReport {
            height,
            ..Default::default()
        };

        let ipc_report = ipc_reports.get(block_hash).cloned().unwrap_or_default();
        let ipc_node_latencies = ipc_report.node_latencies.clone();

        report.all_nodes_present = block_traces.len() == ipc_node_latencies.len();

        for trace in block_traces {
            let ipc_latencies = if let Some(ipc_latencies) = ipc_node_latencies.get(&trace.node) {
                ipc_latencies
            } else {
                // println!("No ipc data for {}", trace.node);
                report.missing_nodes.push(trace.node.clone());
                continue;
            };

            if trace.is_producer {
                report.producer_nodes.push(trace.node.to_string());
                continue;
            }

            // let latency_difference = trace.receive_latency.unwrap() - ipc_latencies.latency_since_block_publication_seconds;
            // report.measured_latency_comparison.insert(trace.node.clone(), latency_difference);

            let receive_time_difference = trace
                .date_time
                .map(|trace_time| trace_time - ((ipc_latencies.receive_time as f64) / 1_000_000.0));
            report.received_time_comparison.insert(
                trace.node.clone(),
                receive_time_difference.unwrap_or_default(),
            );
        }

        by_block.insert(block_hash.clone(), report);
    }

    by_block
}

pub fn aggregate_cross_validations(
    cross_validations: Vec<BTreeMap<String, ValidationReport>>,
) -> AggregateValidationReport {
    let total_heights_checked = cross_validations.len();
    let height_start = cross_validations
        .first()
        .and_then(|first| first.first_key_value().map(|(_, v)| v.height));
    let height_end = cross_validations
        .last()
        .and_then(|first| first.first_key_value().map(|(_, v)| v.height));
    let reports: Vec<ValidationReport> = cross_validations
        .into_iter()
        .flat_map(|v| v.values().cloned().collect::<Vec<_>>())
        .collect();

    let total_blocks_checked = reports.len();

    let all_nodes_present = reports
        .clone()
        .into_iter()
        .map(|v| v.all_nodes_present)
        .reduce(|acc, current| acc && current);

    let min_agg = reports
        .clone()
        .into_iter()
        .reduce(|acc, current| acc.aggregatge_min(&current));
    let max_agg = reports
        .into_iter()
        .reduce(|acc, current| acc.aggregatge_max(&current));

    AggregateValidationReport {
        max_recevied_time_comparison: max_agg.map(|r| r.received_time_comparison),
        min_recevied_time_comparison: min_agg.map(|r| r.received_time_comparison),
        total_heights_checked,
        height_start,
        height_end,
        total_blocks_checked,
        all_nodes_present,
    }
}
