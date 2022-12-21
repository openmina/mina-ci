use std::collections::BTreeMap;

use serde::Serialize;

use crate::{nodes::{BlockStructuredTrace, TraceSource, DaemonMetrics, DaemonStatusDataSlim}, AggregatorResult, error::AggregatorError};

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
    // structured_trace: BlockStructuredTrace,
}

pub fn aggregate_block_traces(height: usize, state_hash: &str, node_infos: &BTreeMap<String, DaemonStatusDataSlim>, traces: BTreeMap<String, BlockStructuredTrace>) -> AggregatorResult<Vec<BlockTraceAggregatorReport>> {
    let mut external_receivers: Vec<(String, BlockStructuredTrace)> = traces.clone().into_iter()
        .filter(|(_, trace)| matches!(trace.source, TraceSource::External))
        .collect();

    if external_receivers.is_empty() {
        return Err(AggregatorError::NoTracesYet);
    }

    external_receivers.sort_by(|(_, a_trace), (_, b_trace)| a_trace.sections[0].checkpoints[0].started_at.total_cmp(&b_trace.sections[0].checkpoints[0].started_at));

    let first_received = external_receivers[0].1.sections[0].checkpoints[0].started_at;
    // let block_production_started_at = producer_trace.sections[0].checkpoints[0].started_at;

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
            receive_latency: trace.map(|t| t.sections[0].checkpoints[0].started_at - first_received),
            block_application: trace.map(|t| t.total_time),
            // structured_trace: trace.block_structured_trace.clone(),
        }
    })
    .collect();

    // let producer_report = BlockTraceAggregatorReport {
    //     block_hash: state_hash.to_string(),
    //     height,
    //     node: producer,
    //     node_address: producer_trace.daemon_status.addrs_and_ports.external_ip,
    //     source: producer_trace.block_structured_trace.source.clone(),
    //     sync_status: producer_trace.daemon_status.sync_status,
    //     snark_pool_size: producer_trace.snark_pool,
    //     transaction_pool_size: producer_trace.daemon_status.metrics.transaction_pool_size,
    //     metrics: producer_trace.daemon_status.metrics,
    //     date_time: producer_trace.block_structured_trace.sections[0].checkpoints[0].started_at,
    //     receive_latency: 0.0,
    //     block_application: producer_trace.total_time,
    //     // structured_trace: producer_trace.block_structured_trace,
    // };

    Ok(report)

}