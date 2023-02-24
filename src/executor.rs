use std::collections::{HashSet, BTreeMap};

use tokio::time::sleep;
use tracing::{error, info, instrument, warn};

use crate::{
    AggregatorResult,
    config::{AggregatorEnvironment, CLUSTER_NODE_LIST_URL},
    debugger_data::{DebuggerCpnpResponse, NodeAddressCluster, NodeAddress, CpnpCapturedData},
    nodes::{ get_most_recent_produced_blocks, get_block_trace_from_cluster, BlockStructuredTrace, get_node_info_from_cluster, collect_all_urls, ComponentType, DaemonStatusDataSlim},
    IpcAggregatorStorage, aggregators::{aggregate_block_traces, BlockTraceAggregatorReport, aggregate_first_receive}, BlockTraceAggregatorStorage, cross_validation::cross_validate_ipc_with_traces,
};

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
                let modified_data = data.into_iter().map(|mut e| {
                    e.node_address = NodeAddress(node_infos.get(tag).unwrap().daemon_status.addrs_and_ports.external_ip.clone());
                    e.node_tag = tag.to_string();
                    e
                }).collect::<Vec<CpnpCapturedData>>();
                collected.push(modified_data);
            },
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
            base_url,
            environment.libp2p_ipc_encpoint,
            "latest"
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

pub async fn poll_node_traces(ipc_storage: &mut IpcAggregatorStorage, block_trace_storage: &mut BlockTraceAggregatorStorage, environment: &AggregatorEnvironment) {
    loop {
        info!("Sleeping");
        sleep(environment.data_pull_interval).await;

        info!("Collecting produced blocks...");
        let mut blocks_on_most_recent_height = get_most_recent_produced_blocks(environment).await;
        info!("Produced blocks collected");

        if blocks_on_most_recent_height.is_empty() {
            info!("No blocks yet");
            continue;
        }

        // blocks_on_most_recent_height.sort_unstable();
        // blocks_on_most_recent_height.dedup();

        // Catch the case that the block producers have different height for their most recent blocks
        if blocks_on_most_recent_height.len() > 1 && !blocks_on_most_recent_height.windows(2).all(|w| w[0].0 == w[1].0) {
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

        // build a map that maps NodeIp to the node tag
        // let node_ip_to_tag_map: BTreeMap<String, String> = node_infos.iter().map(|(k, v)| {
        //     (v.daemon_status.addrs_and_ports.external_ip.to_string(), k.to_string())
        // })
        // .collect();

        // build a map that maps peer_id to tag
        let peer_id_to_tag_map: BTreeMap<String, String> = node_infos.iter().map(|(k, v)| {
            (v.daemon_status.addrs_and_ports.peer.peer_id.clone(), k.to_string())
        })
        .collect();

        // let tag_to_peer_id_map: BTreeMap<String, String> = node_infos.iter().map(|(k, v)| {
        //     (k.to_string(), v.daemon_status.addrs_and_ports.peer.peer_id.clone())
        // })
        // .collect();

        // let peer_id_to_state_hash_map: BTreeMap<String, String> = blocks_on_most_recent_height.iter().map(|(_, state_hash, tag)| {
        //     let peer_id = tag_to_peer_id_map.get(tag).unwrap();
        //     (peer_id.to_string(), state_hash.to_string())
        // }).collect();

        let tag_to_block_hash_map: BTreeMap<String, String> = blocks_on_most_recent_height.iter().map(|(_, state_hash, tag)| {
            // let peer_id = tag_to_peer_id_map.get(tag).unwrap();
            (tag.to_string(), state_hash.to_string())
        }).collect();

        for (_, state_hash, _) in blocks_on_most_recent_height {
            info!("Collecting node traces for block {state_hash}");
            let trace = get_block_trace_from_cluster(environment, &state_hash).await;
            info!("Traces collected for block {state_hash}");
            info!("Aggregating trace data and traces for block {state_hash}");
            // println!("TRACES KEYS: {:#?}", trace.keys());
            match aggregate_block_traces(height, &state_hash, &node_infos, trace) {
                Ok(data) => {
                    block_traces.insert(state_hash.clone(), data);
                },
                Err(e) => warn!("{}", e),
            }
            info!("Trace aggregation finished for block {state_hash}");
        }

        let _ = block_trace_storage.insert(height, block_traces);

        // TODO: move this to a separate thread?
        info!("Polling debuggers for height {height}");

        match pull_debugger_data_cpnp(Some(height), environment, &node_infos).await {
            Ok(data) => {
                let (height, aggregated_data) =
                    match aggregate_first_receive(data, &peer_id_to_tag_map, &tag_to_block_hash_map) {
                        Ok((height, aggregate_data)) => (height, aggregate_data),
                        Err(e) => {
                            warn!("{}", e);
                            continue;
                        }
                    };

                // TODO: this is not a very good idea, capture the error!
                let _ = ipc_storage.insert(height, aggregated_data);
            }
            Err(e) => error!("Error in pulling data: {}", e),
        }
    }
}
