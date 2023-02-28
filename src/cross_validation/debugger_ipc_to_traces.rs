use std::collections::BTreeMap;

use serde::Serialize;

use crate::aggregators::{BlockHash, BlockTraceAggregatorReport, CpnpBlockPublication};

#[derive(Clone, Debug, Default, Serialize)]
pub struct ValidationReport {
    // pub measured_latency_comparison: BTreeMap<String, f64>,
    pub received_time_comparison: BTreeMap<String, f64>,
    pub all_nodes_present: bool,
    pub missing_nodes: Vec<String>,
}

// TODO: unwraps
pub fn cross_validate_ipc_with_traces(
    traces: BTreeMap<String, Vec<BlockTraceAggregatorReport>>,
    ipc_reports: BTreeMap<BlockHash, CpnpBlockPublication>,
) -> BTreeMap<String, ValidationReport> {
    let mut by_block: BTreeMap<String, ValidationReport> = BTreeMap::new();

    for (block_hash, block_traces) in traces.iter() {
        let mut report = ValidationReport::default();

        let ipc_report = ipc_reports.get(block_hash).unwrap();
        let ipc_node_latencies = ipc_report.node_latencies.clone();

        report.all_nodes_present = block_traces.len() == ipc_node_latencies.len();

        for trace in block_traces {
            let ipc_latencies = if let Some(ipc_latencies) = ipc_node_latencies.get(&trace.node) {
                ipc_latencies
            } else {
                println!("No ipc data for {}", trace.node);
                report.missing_nodes.push(trace.node.clone());
                continue;
            };

            // let latency_difference = trace.receive_latency.unwrap() - ipc_latencies.latency_since_block_publication_seconds;
            // report.measured_latency_comparison.insert(trace.node.clone(), latency_difference);

            let receive_time_difference =
                trace.date_time.unwrap() - ((ipc_latencies.receive_time as f64) / 1_000_000.0);
            report
                .received_time_comparison
                .insert(trace.node.clone(), receive_time_difference);
        }

        by_block.insert(block_hash.clone(), report);
    }

    by_block
}
