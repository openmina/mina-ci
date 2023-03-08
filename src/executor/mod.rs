use std::collections::BTreeMap;

use tokio::time::sleep;
use tracing::{error, info, instrument, warn};

use crate::{
    aggregators::{aggregate_block_traces, aggregate_first_receive, BlockTraceAggregatorReport},
    config::AggregatorEnvironment,
    cross_validation::cross_validate_ipc_with_traces,
    debugger_data::{CpnpCapturedData, DebuggerCpnpResponse},
    nodes::{
        collect_all_urls, get_block_trace_from_cluster, get_most_recent_produced_blocks,
        get_node_info_from_cluster, ComponentType, DaemonStatusDataSlim,
    },
    storage::{AggregatorStorage, BuildStorage},
    AggregatorResult,
};

use self::state::AggregatorState;

pub mod state;

#[instrument]
async fn pull_debugger_data_cpnp(
    height: Option<usize>,
    environment: &AggregatorEnvironment,
    node_infos: &BTreeMap<String, DaemonStatusDataSlim>,
) -> AggregatorResult<Vec<DebuggerCpnpResponse>> {
    let mut collected: Vec<DebuggerCpnpResponse> = vec![];

    let nodes = collect_all_urls(environment, ComponentType::Debugger);

    for (tag, url) in nodes.iter() {
        // info!("Pulling {}", url);
        match get_height_data_cpnp(height, url, environment).await {
            Ok(data) => {
                let modified_data = data
                    .into_iter()
                    .map(|mut e| {
                        // e.node_address = NodeAddress(node_infos.get(tag).unwrap().daemon_status.addrs_and_ports.external_ip.clone());
                        e.node_tag = tag.to_string();
                        e
                    })
                    .collect::<Vec<CpnpCapturedData>>();
                collected.push(modified_data);
            }
            Err(e) => warn!("{} failed to provide data, reson: {}", url, e),
        }
    }

    Ok(collected)
}

async fn get_height_data_cpnp(
    height: Option<usize>,
    base_url: &str,
    environment: &AggregatorEnvironment,
) -> AggregatorResult<DebuggerCpnpResponse> {
    let url = if let Some(height) = height {
        format!(
            "{}/{}/{}",
            base_url, environment.libp2p_ipc_encpoint, height
        )
    } else {
        format!(
            "{}/{}/{}",
            base_url, environment.libp2p_ipc_encpoint, "latest"
        )
    };
    reqwest::get(url).await?.json().await.map_err(|e| e.into())
}

// async fn get_node_list_from_cluster() -> AggregatorResult<Vec<NodeAddressCluster>> {
//     reqwest::get(CLUSTER_NODE_LIST_URL)
//         .await?
//         .json()
//         .await
//         .map_err(|e| e.into())
// }

// pub async fn rehydrate_storage(
//     ipc_storage: &mut IpcAggregatorStorage,
//     block_trace_storage: &mut BlockTraceAggregatorStorage,
//     environment: &AggregatorEnvironment,
// ) {
//     let mut blocks = get_most_recent_produced_blocks(environment, 10).await;

//     blocks.sort_by(|a, b| a.0.cmp(&b.0));
//     let mut highest = blocks[0].0;
//     const MAX_HISTORY: usize = 100;

//     let mut current_height_blocks: Vec<(usize, String, String)> = vec![];

//     while let Some(e) = blocks.get(0) {
//         if e.0 == highest {
//             current_height_blocks.push(e.clone());
//             blocks.remove(0);
//         } else {
//             info!("Rehydratating height: {}", highest);
//             // aggregate here
//             let node_infos = get_node_info_from_cluster(environment).await;

//             let peer_id_to_tag_map: BTreeMap<String, String> = node_infos
//                 .iter()
//                 .map(|(k, v)| {
//                     (
//                         v.daemon_status.addrs_and_ports.peer.peer_id.clone(),
//                         k.to_string(),
//                     )
//                 })
//                 .collect();

//             let tag_to_block_hash_map: BTreeMap<String, String> = current_height_blocks
//                 .iter()
//                 .map(|(_, state_hash, tag)| {
//                     // let peer_id = tag_to_peer_id_map.get(tag).unwrap();
//                     (tag.to_string(), state_hash.to_string())
//                 })
//                 .collect();

//             let mut block_traces: BTreeMap<String, Vec<BlockTraceAggregatorReport>> =
//                 BTreeMap::new();

//             for (_, state_hash, _) in &current_height_blocks {
//                 info!("Collecting node traces for block {state_hash}");
//                 let trace = get_block_trace_from_cluster(environment, state_hash).await;
//                 info!("Traces collected for block {state_hash}");
//                 info!("Aggregating trace data and traces for block {state_hash}");
//                 // println!("TRACES KEYS: {:#?}", trace.keys());
//                 match aggregate_block_traces(highest, state_hash, &node_infos, trace) {
//                     Ok(data) => {
//                         block_traces.insert(state_hash.clone(), data);
//                     }
//                     Err(e) => warn!("{}", e),
//                 }
//                 info!("Trace aggregation finished for block {state_hash}");
//             }

//             let _ = block_trace_storage.insert(highest, block_traces);

//             // TODO: move this to a separate thread?
//             info!("Polling debuggers for height {highest}");

//             match pull_debugger_data_cpnp(Some(highest), environment, &node_infos).await {
//                 Ok(data) => {
//                     let (height, aggregated_data) = match aggregate_first_receive(
//                         data,
//                         &peer_id_to_tag_map,
//                         &tag_to_block_hash_map,
//                     ) {
//                         Ok((height, aggregate_data)) => (height, aggregate_data),
//                         Err(e) => {
//                             warn!("{}", e);
//                             continue;
//                         }
//                     };

//                     // TODO: this is not a very good idea, capture the error!
//                     let _ = ipc_storage.insert(height, aggregated_data);
//                 }
//                 Err(e) => error!("Error in pulling data: {}", e),
//             }
//             current_height_blocks.clear();
//             highest -= 1;
//         }
//     }
//     info!("Rehydratation completed!");
// }

pub async fn poll_node_traces(
    state: &AggregatorState,
    storage: &mut AggregatorStorage,
    environment: &AggregatorEnvironment,
) {
    loop {
        info!("Sleeping");
        sleep(environment.data_pull_interval).await;

        let build_number = if let Ok(read_state) = state.read() {
            read_state.build_number
        } else {
            info!("No CI build yet!");
            continue;
        };

        let mut build_storage = if let Ok(Some(build_storage)) = storage.get(build_number) {
            build_storage
        } else {
            info!("NO BUILD STORAGE?");
            continue;
        };

        info!("Collecting produced blocks...");
        let mut blocks_on_most_recent_height = get_most_recent_produced_blocks(environment).await;
        info!("Produced blocks collected");

        if blocks_on_most_recent_height.is_empty() {
            info!("No blocks yet");
            continue;
        }

        // Catch the case that the block producers have different height for their most recent blocks
        if blocks_on_most_recent_height.len() > 1
            && !blocks_on_most_recent_height
                .windows(2)
                .all(|w| w[0].0 == w[1].0)
        {
            info!("Height missmatch on producers! Using highest block_height");
            // With this check we can eliminate the scenraio when a block producer lags behind, retaining only the highest block_height
            let highest = blocks_on_most_recent_height.iter().max().unwrap().0;
            blocks_on_most_recent_height.retain(|(height, _, _)| height == &highest);
        }

        let height = blocks_on_most_recent_height[0].0;
        let mut block_traces: BTreeMap<String, Vec<BlockTraceAggregatorReport>> = BTreeMap::new();

        info!("Height: {height}");

        // collect node info
        info!("Collecting cluster nodes information");
        let node_infos = get_node_info_from_cluster(environment).await;
        // println!("INF: {:#?}", node_infos);
        info!("Information collected");

        // build a map that maps peer_id to tag
        let peer_id_to_tag_map: BTreeMap<String, String> = node_infos
            .iter()
            .map(|(k, v)| {
                (
                    v.daemon_status.addrs_and_ports.peer.peer_id.clone(),
                    k.to_string(),
                )
            })
            .collect();

        let tag_to_block_hash_map: BTreeMap<String, String> = blocks_on_most_recent_height
            .iter()
            .map(|(_, state_hash, tag)| {
                // let peer_id = tag_to_peer_id_map.get(tag).unwrap();
                (tag.to_string(), state_hash.to_string())
            })
            .collect();

        // Optimization, only grab traces once for the same block hash
        // blocks_on_most_recent_height.sort_unstable();
        // blocks_on_most_recent_height.dedup();

        for (_, state_hash, _) in blocks_on_most_recent_height {
            info!("Collecting node traces for block {state_hash}");
            let trace = get_block_trace_from_cluster(environment, &state_hash).await;
            info!("Traces collected for block {state_hash}");
            info!("Aggregating trace data and traces for block {state_hash}");
            // println!("TRACES KEYS: {:#?}", trace.keys());
            match aggregate_block_traces(height, &state_hash, &node_infos, trace) {
                Ok(data) => {
                    block_traces.insert(state_hash.clone(), data);
                }
                Err(e) => warn!("{}", e),
            }
            info!("Trace aggregation finished for block {state_hash}");
        }

        // TODO: move this to a separate thread?
        info!("Polling debuggers for height {height}");

        match pull_debugger_data_cpnp(Some(height), environment, &node_infos).await {
            Ok(data) => {
                let (height, aggregated_data) = match aggregate_first_receive(
                    data,
                    &peer_id_to_tag_map,
                    &tag_to_block_hash_map,
                ) {
                    Ok((height, aggregate_data)) => (height, aggregate_data),
                    Err(e) => {
                        warn!("{}", e);
                        continue;
                    }
                };

                // TODO: this is not a very good idea, capture the error!

                // also do the cross_validation
                let report = cross_validate_ipc_with_traces(
                    block_traces.clone(),
                    aggregated_data.clone(),
                    height,
                );
                // TODO!

                let _ = build_storage
                    .trace_storage
                    .insert(height, block_traces.clone());
                let _ = build_storage
                    .ipc_storage
                    .insert(height, aggregated_data.clone());
                let _ = build_storage
                    .cross_validation_storage
                    .insert(height, report);
            }
            Err(e) => error!("Error in pulling data: {}", e),
        }

        // TODO: move this around
        let application_min = block_traces
            .values()
            .map(|traces| {
                traces
                    .iter()
                    .map(|val| val.block_application.unwrap_or(f64::MAX))
                    .reduce(|a, b| a.min(b))
                    .unwrap_or(f64::MAX)
            })
            .reduce(|a, b| a.min(b))
            .unwrap_or(f64::MAX);

        let application_max = block_traces
            .values()
            .map(|traces| {
                traces
                    .iter()
                    .map(|val| val.block_application.unwrap_or(f64::MIN))
                    .reduce(|a, b| a.min(b))
                    .unwrap_or(f64::MIN)
            })
            .reduce(|a, b| a.max(b))
            .unwrap_or(f64::MIN);

        if build_storage.build_summary.block_application_min == 0.0 {
            build_storage.build_summary.block_application_min = application_min;
        } else {
            build_storage.build_summary.block_application_min =
                application_min.min(build_storage.build_summary.block_application_min);
        }
        build_storage.build_summary.block_application_max =
            application_max.max(build_storage.build_summary.block_application_max);

        let _ = storage.insert(build_number, build_storage);
    }
}
