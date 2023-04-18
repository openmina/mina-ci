use std::{collections::BTreeMap, time::Duration};

use futures::{stream, StreamExt};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::{error, info, instrument, warn};

use crate::{error::AggregatorError, AggregatorResult};

use super::{query_node, GraphqlResponse, Nodes};

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

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
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

#[instrument(skip(nodes))]
/// Fires requests to all the nodes and collects their IPs (requests are parallel)
pub async fn get_node_info_from_cluster(
    nodes: Nodes,
) -> (BTreeMap<String, DaemonStatusDataSlim>, usize) {
    let client = reqwest::Client::new();

    const MAX_RETRIES: usize = 5;
    let mut retries: usize = 0;
    let nodes_count = nodes.len();
    let mut nodes_to_query = nodes;
    let mut final_res = BTreeMap::new();
    let mut total_timeouts = 0;

    while retries < MAX_RETRIES {
        let bodies = stream::iter(nodes_to_query.clone())
            .map(|(tag, url)| {
                let client = client.clone();
                tokio::spawn(async move { (tag.clone(), query_node_info(client, &url).await) })
            })
            .buffer_unordered(150);

        let (collected, timeouts): (_, usize) = bodies
            .fold(
                (BTreeMap::new(), 0),
                |(mut collected, mut timeouts), b| async move {
                    match b {
                        Ok((tag, Ok(res))) => {
                            collected.insert(tag, res.into());
                        }
                        Ok((tag, Err(e))) => {
                            warn!("Error requestig {tag}, reason: {}", e);
                            if let AggregatorError::OutgoingRpcError(reqwest_error) = e {
                                if reqwest_error.is_timeout() {
                                    timeouts += 1;
                                }
                            };
                        }
                        Err(e) => error!("Tokio join error: {e}"),
                    }
                    (collected, timeouts)
                },
            )
            .await;

        final_res.extend(collected);
        nodes_to_query.retain(|k, _| !final_res.contains_key(k));
        total_timeouts += timeouts;

        if nodes_to_query.is_empty() {
            break;
        }

        sleep(Duration::from_secs(1)).await;
        retries += 1;
    }

    info!("Collected {}/{} nodes", final_res.len(), nodes_count);

    (final_res, total_timeouts)

    // let bodies = stream::iter(nodes)
    //     .map(|(tag, url)| {
    //         let client = client.clone();
    //         tokio::spawn(async move { (tag.clone(), query_node_info(client, &url).await) })
    //     })
    //     .buffer_unordered(150);

    // let collected = bodies
    //     .fold(BTreeMap::new(), |mut collected, b| async {
    //         match b {
    //             Ok((tag, Ok(res))) => {
    //                 // info!("{tag} OK");
    //                 collected.insert(tag, res.into());
    //             }
    //             Ok((tag, Err(e))) => warn!("Error requestig {tag}, reason: {}", e),
    //             Err(e) => error!("Tokio join error: {e}"),
    //         }
    //         collected
    //     })
    //     .await;

    // info!("Collected {} nodes", collected.len());

    // collected
}

async fn query_node_info(client: reqwest::Client, url: &str) -> AggregatorResult<DaemonStatusData> {
    let res = query_node(client, url, NODE_INFO_PAYLOAD.to_string()).await?;
    let status = res.status();

    if status != StatusCode::OK {
        return Err(AggregatorError::RpcServerError { status });
    }

    let res_json: GraphqlResponse<DaemonStatusData> = res.json().await?;

    Ok(res_json.data)
}
