use futures::{stream, StreamExt};
use std::collections::HashSet;

use reqwest::Response;
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument, log::warn};

use crate::{aggregator::AggregatorResult, config::AggregatorEnvironment};

const CLUSTER_BASE_URL: &str = "http://1.k8.openmina.com:31308";
const PLAIN_NODE_COMPONENT: &str = "node";
const SEED_NODE_COMPONENT: &str = "seed";
const PRODUCER_NODE_COMPONENT: &str = "prod";
const SNARKER_NODE_COMPONENT: &str = "snarker";
const TRANSACTION_GENERTOR_NODE_COMPONENT: &str = "transaction-generator";

const GRAPHQL_COMPONENT: &str = "graphql";

const NODE_IPS_PAYLOAD: &str = r#"{"query": "{ daemonStatus { addrsAndPorts { externalIp }}}" }"#;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GraphqlResponse<T> {
    pub data: T,
}

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

async fn query_node(
    client: reqwest::Client,
    url: &str,
    payload: &'static str,
) -> AggregatorResult<Response> {
    Ok(client
        .post(url)
        .body(payload)
        .header("Content-Type", "application/json")
        .send()
        .await?)
}

async fn query_node_ip(client: reqwest::Client, url: &str) -> AggregatorResult<String> {
    let res: GraphqlResponse<DaemonStatusData> = query_node(client, url, NODE_IPS_PAYLOAD)
        .await?
        .json()
        .await?;
    Ok(res.data.daemon_status.addrs_and_ports.external_ip)
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
                    info!("{url} OK");
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

fn collect_all_urls(environment: &AggregatorEnvironment) -> Vec<String> {
    let mut res: Vec<String> = vec![];

    for seed_label in 1..=environment.seed_node_count {
        let url = format!(
            "{}/{}{}/{}",
            CLUSTER_BASE_URL, SEED_NODE_COMPONENT, seed_label, GRAPHQL_COMPONENT
        );
        res.push(url);
    }

    // producer nodes
    for producer_label in 1..=environment.producer_node_count {
        let url = format!(
            "{}/{}{}/{}",
            CLUSTER_BASE_URL, PRODUCER_NODE_COMPONENT, producer_label, GRAPHQL_COMPONENT
        );
        res.push(url);
    }

    // snarker nodes
    for snarker_label in 1..=environment.snarker_node_count {
        let url = format!(
            "{}/{}{}/{}",
            CLUSTER_BASE_URL, SNARKER_NODE_COMPONENT, snarker_label, GRAPHQL_COMPONENT
        );
        res.push(url);
    }

    // transaction generator nodes
    // NOTE: No labels yet, only one transaction-generator
    // for transaction_generator_label in 1..=environment.snarker_node_count {
    //     let url = format!("{}/{}{}/{}", CLUSTER_BASE_URL, TRANSACTION_GENERTOR_NODE_COMPONENT, transaction_generator_label, GRAPHQL_COMPONENT);
    //     match query_node_ip(&url).await {
    //         Ok(ip) => collected.push(ip),
    //         Err(e) => warn!("Seed node failed to respond, reason: {}", e),
    //     }
    // }
    {
        let url = format!(
            "{}/{}/{}",
            CLUSTER_BASE_URL, TRANSACTION_GENERTOR_NODE_COMPONENT, GRAPHQL_COMPONENT
        );
        res.push(url);
    }

    // plain nodes
    for plain_node_label in 1..=environment.plain_node_count {
        let url = if plain_node_label < 10 {
            format!(
                "{}/{}0{}/{}",
                CLUSTER_BASE_URL, PLAIN_NODE_COMPONENT, plain_node_label, GRAPHQL_COMPONENT
            )
        } else {
            format!(
                "{}/{}{}/{}",
                CLUSTER_BASE_URL, PLAIN_NODE_COMPONENT, plain_node_label, GRAPHQL_COMPONENT
            )
        };
        res.push(url);
    }

    res
}
