use std::collections::BTreeMap;

use serde::Serialize;

use crate::{nodes::{BlockStructuredTrace, TraceSource}, AggregatorResult};

#[derive(Clone, Debug, Serialize)]
pub struct BlockTraceAggregatorReport {
    height: usize,
    block_hash: String,
    node: String,
    receive_latency: f64,
    block_application: f64,
    structured_trace: BlockStructuredTrace,
}

pub fn aggregate_block_traces(height: usize, state_hash: &str, traces: BTreeMap<String, BlockStructuredTrace>) -> AggregatorResult<Vec<BlockTraceAggregatorReport>> {
    // TODO: unrwap
    let (producer, producer_trace) = traces.clone().into_iter()
        .find(|(_, trace)| matches!(trace.source, TraceSource::Internal)).unwrap();

    let mut external_receivers: Vec<(String, BlockStructuredTrace)> = traces.into_iter()
        .filter(|(_, trace)| matches!(trace.source, TraceSource::External))
        .collect();

    external_receivers.sort_by(|(_, a_trace), (_, b_trace)| a_trace.sections[0].checkpoints[0].started_at.total_cmp(&b_trace.sections[0].checkpoints[0].started_at));

    // let block_production_started_at = producer_trace.sections[0].checkpoints[0].started_at;

    let producer_report = BlockTraceAggregatorReport {
        block_hash: state_hash.to_string(),
        height,
        node: producer,
        receive_latency: 0.0,
        block_application: producer_trace.total_time,
        structured_trace: producer_trace,
    };

    let first_received = external_receivers[0].1.sections[0].checkpoints[0].started_at;

    let mut report: Vec<BlockTraceAggregatorReport> = external_receivers.into_iter().map(|(node, trace)| {
        BlockTraceAggregatorReport {
            block_hash: state_hash.to_string(),
            height,
            node,
            receive_latency: trace.sections[0].checkpoints[0].started_at - first_received,
            block_application: trace.total_time,
            structured_trace: trace.clone(),
        }
    })
    .collect();

    report.push(producer_report);

    Ok(report)

}