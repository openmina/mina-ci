use std::collections::BTreeMap;

use serde::Serialize;

use crate::{
    nodes::{BlockStructuredTrace, DaemonMetrics, DaemonStatusDataSlim, TraceSource, TraceStatus},
    AggregatorResult,
};

#[derive(Clone, Debug, Serialize)]
pub struct BlockTraceAggregatorReport {
    pub height: usize,
    pub block_hash: String,
    pub node: String,
    pub node_address: String,
    pub date_time: Option<f64>,
    pub source: Option<TraceSource>,
    pub sync_status: String,
    pub snark_pool_size: usize,
    pub transaction_pool_size: usize,
    pub metrics: DaemonMetrics,
    pub receive_latency: Option<f64>,
    pub block_application: Option<f64>,
    pub is_producer: bool,
}

pub fn aggregate_block_traces(
    height: usize,
    state_hash: &str,
    node_infos: &BTreeMap<String, DaemonStatusDataSlim>,
    traces: BTreeMap<String, BlockStructuredTrace>,
) -> AggregatorResult<Vec<BlockTraceAggregatorReport>> {
    // let mut external_receivers: Vec<(String, BlockStructuredTrace)> = traces.clone().into_iter()
    //     .filter(|(_, trace)| matches!(trace.source, TraceSource::External) || matches!(trace.source, TraceSource::Catchup))
    //     .collect();

    // external_receivers.sort_by(|(_, a_trace), (_, b_trace)| a_trace.sections[0].checkpoints[0].started_at.total_cmp(&b_trace.sections[0].checkpoints[0].started_at));

    let mut internal_receivers: Vec<(String, BlockStructuredTrace)> = traces
        .clone()
        .into_iter()
        .filter(|(_, trace)| matches!(trace.source, TraceSource::Internal))
        .collect();

    internal_receivers.sort_by(|(_, a_trace), (_, b_trace)| {
        a_trace.sections[0].checkpoints[0]
            .started_at
            .total_cmp(&b_trace.sections[0].checkpoints[0].started_at)
    });

    // println!("IT: {:#?}", internal_receivers);
    let producer_nodes: Vec<String> = internal_receivers
        .iter()
        .map(|(tag, _)| tag.to_string())
        .collect();

    // TODO: Should be the timestamp the first producer finished?
    // TraceSource::Internal -> sort by ~sent_time=(started_at + total_time)
    let first_received = internal_receivers.get(0);

    let report: Vec<BlockTraceAggregatorReport> = node_infos
        .iter()
        .map(|(node, node_info)| {
            let trace = traces.get(node);
            BlockTraceAggregatorReport {
                block_hash: state_hash.to_string(),
                is_producer: producer_nodes.contains(node), // TODO
                height,
                node: node.to_string(),
                node_address: node_info.daemon_status.addrs_and_ports.external_ip.clone(),
                source: trace.map(|t| t.source.clone()),
                sync_status: node_info.daemon_status.sync_status.clone(),
                snark_pool_size: node_info.snark_pool,
                transaction_pool_size: node_info.daemon_status.metrics.transaction_pool_size,
                metrics: node_info.daemon_status.metrics.clone(),
                date_time: trace.map(|t| t.sections[0].checkpoints[0].started_at),
                receive_latency: trace.and_then(|t| {
                    first_received.map(|first_received| {
                        t.sections[0].checkpoints[0].started_at
                            - (first_received.1.sections[0].checkpoints[0].started_at
                                + first_received.1.total_time)
                    })
                }),
                block_application: trace
                    .filter(|t| !matches!(t.status, TraceStatus::Pending))
                    .map(|t| t.total_time),
            }
        })
        .collect();
    Ok(report)
}
