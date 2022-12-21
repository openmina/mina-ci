use std::collections::BTreeMap;

use serde::Serialize;

use crate::{nodes::{BlockStructuredTrace, TraceSource, BlockStructuredTraceDataSlim, TraceStatus, DaemonMetrics}, AggregatorResult, error::AggregatorError};

#[derive(Clone, Debug, Serialize)]
pub struct BlockTraceAggregatorReport {
    height: usize,
    block_hash: String,
    node: String,
    node_address: String,
    date_time: f64,
    source: TraceSource,
    sync_status: String,
    snark_pool_size: usize,
    transaction_pool_size: usize,
    metrics: DaemonMetrics,
    receive_latency: f64,
    block_application: f64,
    // structured_trace: BlockStructuredTrace,
}

pub fn aggregate_block_traces(height: usize, state_hash: &str, traces: BTreeMap<String, BlockStructuredTraceDataSlim>) -> AggregatorResult<Vec<BlockTraceAggregatorReport>> {
    // TODO: unrwap
    let (producer, producer_trace) = traces.clone().into_iter()
        .find(|(_, trace)| matches!(trace.block_structured_trace.source, TraceSource::Internal)).unwrap();

    let mut external_receivers: Vec<(String, BlockStructuredTraceDataSlim)> = traces.into_iter()
        .filter(|(_, trace)| matches!(trace.block_structured_trace.source, TraceSource::External))
        .collect();

    if external_receivers.is_empty() {
        return Err(AggregatorError::NoTracesYet);
    }

    external_receivers.sort_by(|(_, a_trace), (_, b_trace)| a_trace.block_structured_trace.sections[0].checkpoints[0].started_at.total_cmp(&b_trace.block_structured_trace.sections[0].checkpoints[0].started_at));

    // let block_production_started_at = producer_trace.sections[0].checkpoints[0].started_at;

    let producer_report = BlockTraceAggregatorReport {
        block_hash: state_hash.to_string(),
        height,
        node: producer,
        node_address: producer_trace.daemon_status.addrs_and_ports.external_ip,
        source: producer_trace.block_structured_trace.source.clone(),
        sync_status: producer_trace.daemon_status.sync_status,
        snark_pool_size: producer_trace.snark_pool,
        transaction_pool_size: producer_trace.daemon_status.metrics.transaction_pool_size,
        metrics: producer_trace.daemon_status.metrics,
        date_time: producer_trace.block_structured_trace.sections[0].checkpoints[0].started_at,
        receive_latency: 0.0,
        block_application: producer_trace.block_structured_trace.total_time,
        // structured_trace: producer_trace.block_structured_trace,
    };

    let first_received = external_receivers[0].1.block_structured_trace.sections[0].checkpoints[0].started_at;

    let mut report: Vec<BlockTraceAggregatorReport> = external_receivers.into_iter().map(|(node, trace)| {
        BlockTraceAggregatorReport {
            block_hash: state_hash.to_string(),
            height,
            node,
            node_address: trace.daemon_status.addrs_and_ports.external_ip,
            source: trace.block_structured_trace.source.clone(),
            sync_status: trace.daemon_status.sync_status,
            snark_pool_size: trace.snark_pool,
            transaction_pool_size: trace.daemon_status.metrics.transaction_pool_size,
            metrics: trace.daemon_status.metrics,
            date_time: trace.block_structured_trace.sections[0].checkpoints[0].started_at,
            receive_latency: trace.block_structured_trace.sections[0].checkpoints[0].started_at - first_received,
            block_application: trace.block_structured_trace.total_time,
            // structured_trace: trace.block_structured_trace.clone(),
        }
    })
    .collect();

    report.push(producer_report);

    Ok(report)

}