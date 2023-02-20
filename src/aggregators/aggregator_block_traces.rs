use std::collections::BTreeMap;

use serde::Serialize;

use crate::{nodes::{BlockStructuredTrace, TraceSource, DaemonMetrics, DaemonStatusDataSlim, TraceStatus}, AggregatorResult};

#[derive(Clone, Debug, Serialize)]
pub struct BlockTraceAggregatorReport {
    height: usize,
    block_hash: String,
    node: String,
    node_address: String,
    date_time: Option<f64>,
    source: Option<TraceSource>,
    sync_status: String,
    snark_pool_size: usize,
    transaction_pool_size: usize,
    metrics: DaemonMetrics,
    receive_latency: Option<f64>,
    block_application: Option<f64>,
}

pub fn aggregate_block_traces(height: usize, state_hash: &str, node_infos: &BTreeMap<String, DaemonStatusDataSlim>, traces: BTreeMap<String, BlockStructuredTrace>) -> AggregatorResult<Vec<BlockTraceAggregatorReport>> {
    let mut external_receivers: Vec<(String, BlockStructuredTrace)> = traces.clone().into_iter()
        .filter(|(_, trace)| matches!(trace.source, TraceSource::External) || matches!(trace.source, TraceSource::Catchup))
        .collect();

    external_receivers.sort_by(|(_, a_trace), (_, b_trace)| a_trace.sections[0].checkpoints[0].started_at.total_cmp(&b_trace.sections[0].checkpoints[0].started_at));

    let first_received = external_receivers.get(0);

    let report: Vec<BlockTraceAggregatorReport> = node_infos.iter().map(|(node, node_info)| {
        let trace = traces.get(node);
        BlockTraceAggregatorReport {
            block_hash: state_hash.to_string(),
            height,
            node: node.to_string(),
            node_address: node_info.daemon_status.addrs_and_ports.external_ip.clone(),
            source: trace.map(|t| t.source.clone()),
            sync_status: node_info.daemon_status.sync_status.clone(),
            snark_pool_size: node_info.snark_pool,
            transaction_pool_size: node_info.daemon_status.metrics.transaction_pool_size,
            metrics: node_info.daemon_status.metrics.clone(),
            date_time: trace.map(|t| t.sections[0].checkpoints[0].started_at),
            receive_latency: trace.and_then(|t| first_received.map(|first_received| t.sections[0].checkpoints[0].started_at - first_received.1.sections[0].checkpoints[0].started_at)),
            block_application: trace.filter(|t| !matches!(t.status, TraceStatus::Pending)).map(|t| t.total_time),
        }
    })
    .collect();
    Ok(report)

}