use std::collections::HashSet;

use tokio::time::sleep;
use tracing::{error, info, instrument, warn};

use crate::{
    aggregator::{aggregate_first_receive, AggregatorResult},
    config::{AggregatorEnvironment, CLUSTER_NODE_LIST_URL},
    debugger_data::{DebuggerCpnpResponse, NodeAddressCluster},
    nodes::get_node_list_from_cluster,
    AggregatorStorage,
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

pub async fn poll_debuggers(storage: &mut AggregatorStorage, environment: &AggregatorEnvironment) {
    loop {
        info!("Sleeping");
        sleep(environment.data_pull_interval).await;

        let nodes_in_cluster: HashSet<String> = get_node_list_from_cluster(environment).await;
        match pull_debugger_data_cpnp(None, environment).await {
            Ok(data) => {
                let (height, aggregated_data) =
                    match aggregate_first_receive(data, nodes_in_cluster) {
                        Ok((height, aggregate_data)) => (height, aggregate_data),
                        Err(e) => {
                            warn!("{}", e);
                            continue;
                        }
                    };

                // TODO: this is not a very good idea, capture the error!
                let _ = storage.insert(height, aggregated_data);
            }
            Err(e) => error!("Error in pulling data: {}", e),
        }
    }
}
