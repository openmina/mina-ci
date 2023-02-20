use std::collections::{HashSet, BTreeMap};

use tokio::time::sleep;
use tracing::{error, info, instrument, warn};

use crate::{
    AggregatorResult,
    config::{AggregatorEnvironment, CLUSTER_NODE_LIST_URL},
    debugger_data::{DebuggerCpnpResponse, NodeAddressCluster},
    nodes::{ get_most_recent_produced_blocks, get_block_trace_from_cluster, BlockStructuredTrace, get_node_info_from_cluster, collect_all_urls, ComponentType},
    IpcAggregatorStorage, aggregators::{aggregate_block_traces, BlockTraceAggregatorReport}, BlockTraceAggregatorStorage,
};

#[instrument]
async fn pull_debugger_data_cpnp(
    height: Option<usize>,
    environment: &AggregatorEnvironment,
) -> AggregatorResult<Vec<DebuggerCpnpResponse>> {
    let mut collected: Vec<DebuggerCpnpResponse> = vec![];

    let urls = collect_all_urls(environment, ComponentType::Debugger);

    for url in urls.iter() {
        info!("Pulling {url}");

        match get_height_data_cpnp(height, url, environment).await {
            Ok(data) => collected.push(data),
            Err(e) => warn!("{url} failed to provide data, reson: {}", e),
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

pub async fn poll_debuggers(ipc_storage: &mut IpcAggregatorStorage, block_trace_storage: &mut BlockTraceAggregatorStorage, environment: &AggregatorEnvironment) {
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

        // Catch the case that the block producers have different height for their most recent blocks
        if blocks_on_most_recent_height.len() > 1 && !blocks_on_most_recent_height.windows(2).all(|w| w[0].0 == w[1].0) {
            info!("Height missmatch on producers! Using highest block_height");
            // With this check we can eliminate the scenraio when a block producer lags behind, retaining only the highest block_height
            let highest = blocks_on_most_recent_height.iter().max().unwrap().0;
            blocks_on_most_recent_height.retain(|(height, _)| height == &highest);
        }

        let height = blocks_on_most_recent_height[0].0;
        let mut block_traces: BTreeMap<String, Vec<BlockTraceAggregatorReport>> = BTreeMap::new();

        info!("Height: {height}");

        // collect node info
        info!("Collecting cluster nodes information");
        let node_infos = get_node_info_from_cluster(environment).await;
        info!("Information collected");

        for (_, state_hash) in blocks_on_most_recent_height {
            info!("Collecting node traces for block {state_hash}");
            let trace = get_block_trace_from_cluster(environment, &state_hash).await;
            info!("Traces collected for block {state_hash}");
            info!("Aggregating data and traces for block {state_hash}");
            match aggregate_block_traces(height, &state_hash, &node_infos, trace) {
                Ok(data) => {
                    block_traces.insert(state_hash.clone(), data);
                },
                Err(e) => warn!("{}", e),
            }
            info!("Aggregation finished for block {state_hash}");
        }

        let _ = block_trace_storage.insert(height, block_traces);

        // match pull_debugger_data_cpnp(None, environment).await {
        //     Ok(data) => {
        //         let (height, aggregated_data) =
        //             match aggregate_first_receive(data, nodes_in_cluster) {
        //                 Ok((height, aggregate_data)) => (height, aggregate_data),
        //                 Err(e) => {
        //                     warn!("{}", e);
        //                     continue;
        //                 }
        //             };

        //         // TODO: this is not a very good idea, capture the error!
        //         let _ = storage.insert(height, aggregated_data);
        //     }
        //     Err(e) => error!("Error in pulling data: {}", e),
        // }
    }
}
