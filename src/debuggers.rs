use std::collections::HashSet;

use tokio::time::sleep;
use tracing::{error, info, instrument, warn};

use crate::{
    aggregator::{aggregate_first_receive, AggregatorResult},
    config::{AggregatorEnvironment, CLUSTER_NODE_LIST_URL},
    debugger_data::{DebuggerCpnpResponse, NodeAddressCluster},
    AggregatorStorage,
};

#[instrument]
async fn pull_debugger_data_cpnp(
    height: Option<usize>,
    environment: &AggregatorEnvironment,
) -> AggregatorResult<Vec<DebuggerCpnpResponse>> {
    let mut collected: Vec<DebuggerCpnpResponse> = vec![];

    for debugger_label in 1..=environment.debugger_count {
        let data = get_height_data_cpnp(height, debugger_label, environment).await?;
        collected.push(data);
        info!("Pulling dbg{}", debugger_label);
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

async fn get_node_list_from_cluster() -> AggregatorResult<Vec<NodeAddressCluster>> {
    reqwest::get(CLUSTER_NODE_LIST_URL)
        .await?
        .json()
        .await
        .map_err(|e| e.into())
}

pub async fn poll_debuggers(storage: &mut AggregatorStorage, environment: &AggregatorEnvironment) {
    sleep(environment.data_pull_interval).await;
    loop {
        let nodes_in_cluster: HashSet<NodeAddressCluster> = match get_node_list_from_cluster().await
        {
            Ok(node_list) => node_list.into_iter().collect(),
            Err(e) => {
                warn!("Error getting node list from cluster: {}", e);
                HashSet::new()
            }
        };
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
        info!("Sleeping");
        sleep(environment.data_pull_interval).await;
    }
}
