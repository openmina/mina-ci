use std::collections::HashSet;

use futures::{stream, StreamExt};
use serde::{Serialize, Deserialize};
use tracing::{instrument, debug, warn, error, info};

use crate::{config::AggregatorEnvironment, nodes::collect_all_urls, AggregatorResult};

use super::{GraphqlResponse, query_node};

const NODE_IPS_PAYLOAD: &str = r#"{"query": "{ daemonStatus { addrsAndPorts { externalIp }}}" }"#;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DaemonStatusData {
    pub daemon_status: DaemonStatus,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DaemonStatus {
    pub addrs_and_ports: AddrsAndPorts,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddrsAndPorts {
    pub external_ip: String,
}

#[instrument]
/// Fires requests to all the nodes and collects their IPs (requests are parallel)
pub async fn get_node_list_from_cluster(environment: &AggregatorEnvironment) -> HashSet<String> {
    let client = reqwest::Client::new();

    let urls = collect_all_urls(environment);
    let bodies = stream::iter(urls)
        .map(|url| {
            let client = client.clone();
            tokio::spawn(async move { (url.clone(), query_node_ip(client, &url).await) })
        })
        .buffer_unordered(20);

    let collected = bodies
        .fold(HashSet::new(), |mut collected, b| async {
            match b {
                Ok((url, Ok(res))) => {
                    debug!("{url} OK");
                    collected.insert(res);
                }
                Ok((url, Err(e))) => warn!("Error requestig {url}, reason: {}", e),
                Err(e) => error!("Tokio join error: {e}"),
            }
            collected
        })
        .await;

    info!("Collected {} nodes", collected.len());

    collected
}

async fn query_node_ip(client: reqwest::Client, url: &str) -> AggregatorResult<String> {
    let res: GraphqlResponse<DaemonStatusData> = query_node(client, url, NODE_IPS_PAYLOAD.to_string())
        .await?
        .json()
        .await?;
    Ok(res.data.daemon_status.addrs_and_ports.external_ip)
}