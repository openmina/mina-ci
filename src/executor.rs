use std::collections::{HashSet, BTreeMap};

use tokio::time::sleep;
use tracing::{error, info, instrument, warn};

use crate::{
    AggregatorResult,
    config::{AggregatorEnvironment, CLUSTER_NODE_LIST_URL},
    debugger_data::{DebuggerCpnpResponse, NodeAddressCluster},
    nodes::{get_node_list_from_cluster, get_most_recent_produced_blocks, get_block_trace_from_cluster, BlockStructuredTrace},
    IpcAggregatorStorage, aggregators::{aggregate_block_traces, BlockTraceAggregatorReport}, BlockTraceAggregatorStorage,
};

#[instrument]
async fn pull_debugger_data_cpnp(
    height: Option<usize>,
    environment: &AggregatorEnvironment,
) -> AggregatorResult<Vec<DebuggerCpnpResponse>> {
    let mut collected: Vec<DebuggerCpnpResponse> = vec![];

    for debugger_label in 1..=environment.debugger_count {
        info!("Pulling dbg{}", debugger_label);

        match get_height_data_cpnp(height, debugger_label, environment).await {
            Ok(data) => collected.push(data),
            Err(e) => warn!("dbg{} failed to provide data, reson: {}", debugger_label, e),
        }
    }

    Ok(collected)
}

async fn get_height_data_cpnp(
    height: Option<usize>,
    debugger_label: usize,
    environment: &AggregatorEnvironment,
) -> AggregatorResult<DebuggerCpnpResponse> {
    let url = if let Some(height) = height {
        format!(
            "{}{}/{}/{}",
            environment.debugger_base_url, debugger_label, environment.libp2p_ipc_encpoint, height
        )
    } else {
        format!(
            "{}{}/{}/{}",
            environment.debugger_base_url,
            debugger_label,
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

        // let nodes_in_cluster: HashSet<String> = get_node_list_from_cluster(environment).await;
        let blocks_on_most_recent_height = get_most_recent_produced_blocks(environment).await;

        if blocks_on_most_recent_height.is_empty() {
            info!("No blocks yet");
            continue;
        }

        // Catch the case that the block producers have different height for their most recent blocks
        if blocks_on_most_recent_height.len() > 1 && !blocks_on_most_recent_height.windows(2).all(|w| w[0].0 == w[1].0) {
            info!("Height missmatch!");
            continue;
        }

        // this should be ok, but lets change the type in the trace to a number...
        let height = blocks_on_most_recent_height[0].0.parse::<usize>().unwrap();
        let mut block_traces: BTreeMap<String, Vec<BlockTraceAggregatorReport>> = BTreeMap::new();

        info!("Height: {height}");

        for (_, state_hash) in blocks_on_most_recent_height {
            info!("Collecting node traces and data for block {state_hash}...");
            let trace = get_block_trace_from_cluster(environment, &state_hash).await;
            info!("Traces and data collected for block {state_hash}");
            info!("Aggregating data and traces for block {state_hash}");
            match aggregate_block_traces(height, &state_hash, trace) {
                Ok(data) => {
                    block_traces.insert(state_hash.clone(), data);
                },
                Err(e) => warn!("{}", e),
            }
            info!("Aggregation finished for block {state_hash}...");
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
