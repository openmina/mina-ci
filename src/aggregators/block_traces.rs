use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    nodes::{BlockStructuredTrace, DaemonMetrics, DaemonStatusDataSlim, TraceSource, TraceStatus},
    storage::{BlockSummary, PeerTiming},
    AggregatorResult,
};

type BlockHash = String;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AggregatedBlockTraces {
    #[serde(flatten)]
    inner: BTreeMap<String, Vec<BlockTraceAggregatorReport>>,
}

impl AggregatedBlockTraces {
    pub fn insert(&mut self, hash: BlockHash, traces: Vec<BlockTraceAggregatorReport>) {
        self.inner.insert(hash, traces);
    }

    // TODO: remove this ideally
    pub fn inner(&self) -> BTreeMap<String, Vec<BlockTraceAggregatorReport>> {
        self.inner.clone()
    }

    pub fn block_summaries(&self, height: usize) -> BTreeMap<BlockHash, BlockSummary> {
        let mut block_summaries = BTreeMap::new();

        for (block_hash, block_traces_per_node) in self.inner.iter() {
            let global_slot = block_traces_per_node
                .iter()
                .find_map(|t| t.global_slot.clone());
            let tx_count = block_traces_per_node
                .iter()
                .find_map(|t| t.included_tranasction_count);
            let date_time = block_traces_per_node.iter().find_map(|t| t.date_time);
            let block_producer = block_traces_per_node
                .iter()
                .find_map(|t| t.block_producer.clone());
            let block_producer_nodes = block_traces_per_node
                .iter()
                .filter(|t| t.is_producer)
                .map(|t| t.node.clone())
                .collect();
            let max_receive_latency = block_traces_per_node
                .iter()
                .filter(|t| !t.is_producer)
                .filter_map(|t| t.receive_latency)
                .reduce(|a, b| a.max(b))
                .unwrap_or_default();
            let peer_timings = block_traces_per_node
                .iter()
                .map(|t| PeerTiming {
                    node: t.node.clone(),
                    block_processing_time: t.block_application,
                    receive_latency: t.receive_latency,
                })
                .collect();
            let summary = BlockSummary {
                block_hash: block_hash.clone(),
                global_slot,
                tx_count,
                date_time,
                block_producer,
                block_producer_nodes,
                max_receive_latency,
                peer_timings,
                height,
            };
            block_summaries.insert(block_hash.to_string(), summary);
        }
        block_summaries
    }

    // TODO: rework, so only the best chain is inlcuded, probably needs to be moved
    pub fn transaction_count(&self) -> usize {
        self.inner
            .values()
            .flat_map(|traces| {
                traces
                    .iter()
                    .filter(|val| val.is_producer)
                    .filter_map(|val| val.included_tranasction_count)
                    .max()
            })
            .max()
            .unwrap_or_default()
    }

    pub fn application_times(&self) -> Vec<f64> {
        self.inner
            .values()
            .flat_map(|traces| {
                traces
                    .iter()
                    .filter(|val| !val.is_producer)
                    .filter_map(|val| val.block_application)
                    .collect::<Vec<f64>>()
            })
            .collect()
    }

    pub fn production_times(&self) -> Vec<f64> {
        self.inner
            .values()
            .flat_map(|traces| {
                traces
                    .iter()
                    .filter(|val| val.is_producer)
                    .filter_map(|val| val.block_application)
                    .collect::<Vec<f64>>()
            })
            .collect()
    }

    pub fn receive_latencies(&self) -> Vec<f64> {
        self.inner
            .values()
            .flat_map(|traces| {
                traces
                    .iter()
                    .filter(|val| !val.is_producer)
                    .filter_map(|val| val.receive_latency)
                    .collect::<Vec<f64>>()
            })
            .collect()
    }

    pub fn appliaction_count(&self) -> usize {
        self.inner
            .values()
            .map(|traces| traces.iter().filter(|t| !t.is_producer).count())
            .sum()
    }

    pub fn production_count(&self) -> usize {
        self.inner
            .values()
            .map(|traces| traces.iter().filter(|t| t.is_producer).count())
            .sum()
    }

    pub fn unique_block_count(&self) -> usize {
        self.inner.values().count()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
    pub block_producer: Option<String>,
    pub global_slot: Option<String>,
    #[serde(skip)]
    pub included_tranasction_count: Option<usize>,
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
        (a_trace.sections[0].checkpoints[0].started_at + a_trace.total_time)
            .total_cmp(&(b_trace.sections[0].checkpoints[0].started_at + b_trace.total_time))
    });

    // println!(
    //     "IT: {:#?}",
    //     internal_receivers
    //         .iter()
    //         .map(|v| v.1.sections[0].checkpoints[0].started_at)
    //         .collect::<Vec<_>>()
    // );

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
            let tx_count = trace.and_then(|t| t.metadata.txn_count);
            BlockTraceAggregatorReport {
                block_hash: state_hash.to_string(),
                is_producer: producer_nodes.contains(node),
                global_slot: trace.and_then(|t| t.metadata.global_slot.clone()),
                block_producer: trace.and_then(|t| t.metadata.creator.clone()),
                included_tranasction_count: tx_count, // TODO
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

// #[cfg(test)]
// mod tests {
//     use std::{
//         collections::BTreeMap,
//         io::BufReader,
//         sync::{Arc, RwLock},
//     };

//     use crate::{
//         aggregators::AggregatedBlockTraces,
//         executor::state::{AggregatorState, AggregatorStateInner},
//         storage::{AggregatorStorage, BuildStorage, BuildStorageDump, LockedBTreeMap},
//     };

//     fn load_storage() -> (AggregatorState, AggregatorStorage) {
//         let file = std::fs::File::open("test-storage-170.json").unwrap();
//         let reader = BufReader::new(file);

//         match serde_json::from_reader::<_, BTreeMap<usize, BuildStorageDump>>(reader) {
//             Ok(storage) => {
//                 // println!("{:#?}", storage);
//                 let storage = storage
//                     .iter()
//                     .map(|(k, v)| (*k, v.clone().into()))
//                     .collect::<BTreeMap<usize, BuildStorage>>();
//                 // println!("{:#?}", storage);
//                 let state_inner = AggregatorStateInner {
//                     build_number: storage
//                         .last_key_value()
//                         .map(|(k, _)| *k)
//                         .unwrap_or_default(),
//                     ..Default::default()
//                 };
//                 let state = Arc::new(RwLock::new(state_inner));
//                 (state, LockedBTreeMap::new(storage))
//             }
//             Err(_) => (AggregatorState::default(), LockedBTreeMap::default()),
//         }
//     }

//     // fn simple_storage() -> AggregatedBlockTraces {
//     //     let trace_storage = BTreeMap::<usize, AggregatedBlockTraces>::new();

//     // }

//     #[test]
//     fn test_applciation_time_aggregations() {
//         // let (_, storage) = load_storage();
//         // let build_storage = storage.get(170).unwrap().unwrap();

//         let build_storage = BuildStorage::default();

//         // max
//         let expected = 47.741026878356934;
//         let reported_max = build_storage.build_summary.block_application_max;
//         assert_eq!(expected, reported_max);

//         // min
//         let expected = 0.024409055709838867;
//         let reported_min = build_storage.build_summary.block_application_min;
//         assert_eq!(expected, reported_min);

//         println!("Avg: {}", build_storage.build_summary.block_application_avg)
//     }
// }
