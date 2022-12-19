use std::collections::HashSet;

use reqwest::Response;
use serde::{Deserialize, Serialize};
use tracing::log::warn;

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

async fn query_node(url: &str, payload: &'static str) -> AggregatorResult<Response> {
    let client = reqwest::Client::new();

    Ok(client
        .post(url)
        .body(payload)
        .header("Content-Type", "application/json")
        .send()
        .await?)
}

async fn query_node_ip(url: &str) -> AggregatorResult<String> {
    let res: GraphqlResponse<DaemonStatusData> =
        query_node(url, NODE_IPS_PAYLOAD).await?.json().await?;
    Ok(res.data.daemon_status.addrs_and_ports.external_ip)
}

pub async fn get_node_list_from_cluster(environment: &AggregatorEnvironment) -> HashSet<String> {
    let mut collected: HashSet<String> = HashSet::new();
    // seed nodes
    for seed_label in 1..=environment.seed_node_count {
        let url = format!(
            "{}/{}{}/{}",
            CLUSTER_BASE_URL, SEED_NODE_COMPONENT, seed_label, GRAPHQL_COMPONENT
        );
        match query_node_ip(&url).await {
            Ok(ip) => {
                collected.insert(ip);
            }
            Err(e) => warn!("Seed node {seed_label} failed to respond, reason: {}", e),
        }
    }

    // producer nodes
    for producer_label in 1..=environment.producer_node_count {
        let url = format!(
            "{}/{}{}/{}",
            CLUSTER_BASE_URL, PRODUCER_NODE_COMPONENT, producer_label, GRAPHQL_COMPONENT
        );
        match query_node_ip(&url).await {
            Ok(ip) => {
                collected.insert(ip);
            }
            Err(e) => warn!(
                "Producer node {producer_label} failed to respond, reason: {}",
                e
            ),
        }
    }

    // snarker nodes
    for snarker_label in 1..=environment.snarker_node_count {
        let url = format!(
            "{}/{}{}/{}",
            CLUSTER_BASE_URL, SNARKER_NODE_COMPONENT, snarker_label, GRAPHQL_COMPONENT
        );
        match query_node_ip(&url).await {
            Ok(ip) => {
                collected.insert(ip);
            }
            Err(e) => warn!(
                "Snarker node {snarker_label} failed to respond, reason: {}",
                e
            ),
        }
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
        match query_node_ip(&url).await {
            Ok(ip) => {
                collected.insert(ip);
            }
            Err(e) => warn!(
                "Transaction generator node failed to respond, reason: {}",
                e
            ),
        }
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

        println!("Url: {}", url);

        match query_node_ip(&url).await {
            Ok(ip) => {
                collected.insert(ip);
            }
            Err(e) => warn!(
                "Plain node {plain_node_label} failed to respond, reason: {}",
                e
            ),
        }
    }

    collected
}
