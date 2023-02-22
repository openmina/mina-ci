use std::collections::{HashSet, BTreeMap};

use futures::{stream, StreamExt};
use serde::{Serialize, Deserialize};
use tracing::{instrument, debug, warn, error, info};

use crate::{config::AggregatorEnvironment, nodes::{collect_all_urls, ComponentType}, AggregatorResult};

use super::{GraphqlResponse, query_node};

const NODE_INFO_PAYLOAD: &str = r#"{"query": "{ daemonStatus { addrsAndPorts { externalIp, peer { peerId } } syncStatus metrics { transactionPoolSize transactionsAddedToPool transactionPoolDiffReceived transactionPoolDiffBroadcasted } } snarkPool { prover } }" }"#;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DaemonStatusData {
    pub daemon_status: DaemonStatus,
    pub snark_pool: Vec<SnarkPoolElement>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddrsAndPorts {
    pub external_ip: String,
    pub peer: Peer,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Peer {
    pub peer_id: String,
} 

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SnarkPoolElement {
    pub prover: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DaemonMetrics {
    pub transaction_pool_size: usize,
    pub transactions_added_to_pool: usize,
    pub transaction_pool_diff_broadcasted: usize,
    pub transaction_pool_diff_received: usize,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DaemonStatus {
    pub addrs_and_ports: AddrsAndPorts,
    pub sync_status: String,
    pub metrics: DaemonMetrics,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DaemonStatusDataSlim {
    pub daemon_status: DaemonStatus,
    pub snark_pool: usize,
}

impl From<DaemonStatusData> for DaemonStatusDataSlim {
    fn from(value: DaemonStatusData) -> Self {
        Self {
            daemon_status: value.daemon_status,
            snark_pool: value.snark_pool.len(),
        }
    }
}

#[instrument]
/// Fires requests to all the nodes and collects their IPs (requests are parallel)
pub async fn get_node_info_from_cluster(environment: &AggregatorEnvironment) -> BTreeMap<String, DaemonStatusDataSlim> {
    let client = reqwest::Client::new();

    let urls = collect_all_urls(environment, ComponentType::Graphql);
    println!("URLS: {:#?}", urls);
    let bodies = stream::iter(urls)
        .map(|(tag, url)| {
            let client = client.clone();
            tokio::spawn(async move { (tag.clone(), query_node_info(client, &url).await) })
        })
        .buffer_unordered(150);

    let collected = bodies
        .fold(BTreeMap::new(), |mut collected, b| async {
            match b {
                Ok((tag, Ok(res))) => {
                    // info!("{tag} OK");
                    collected.insert(tag, res.into());
                }
                Ok((tag, Err(e))) => warn!("Error requestig {tag}, reason: {}", e),
                Err(e) => error!("Tokio join error: {e}"),
            }
            collected
        })
        .await;

    info!("Collected {} nodes", collected.len());

    println!("TAGS COLLECTED: {:#?}", collected.keys());
    println!("IPS COLLECTED: {:#?}", collected.values());

    collected
}

async fn query_node_info(client: reqwest::Client, url: &str) -> AggregatorResult<DaemonStatusData> {
    let res: GraphqlResponse<DaemonStatusData> = query_node(client, url, NODE_INFO_PAYLOAD.to_string())
        .await?
        .json()
        .await?;
    Ok(res.data)
}