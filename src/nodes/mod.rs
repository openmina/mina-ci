use reqwest::Response;
use serde::{Deserialize, Serialize};

use crate::{config::AggregatorEnvironment, AggregatorResult};

pub mod node_info;
pub use node_info::*;

pub mod producer_traces;
pub use producer_traces::*;

pub mod node_block_traces;
pub use node_block_traces::*;

const CLUSTER_BASE_URL: &str = "http://1.k8.openmina.com:31311";
const PLAIN_NODE_COMPONENT: &str = "node";
const SEED_NODE_COMPONENT: &str = "seed";
const PRODUCER_NODE_COMPONENT: &str = "prod";
const SNARKER_NODE_COMPONENT: &str = "snarker";
const TRANSACTION_GENERTOR_NODE_COMPONENT: &str = "transaction-generator";

const DEBUGGER_COMPONENT: &str = "bpf-debugger";
const GRAPHQL_COMPONENT: &str = "graphql";

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GraphqlResponse<T> {
    pub data: T,
}

pub enum ComponentType {
    Graphql,
    Debugger,
}

async fn query_node(
    client: reqwest::Client,
    url: &str,
    payload: String,
) -> AggregatorResult<Response> {
    Ok(client
        .post(url)
        .body(payload)
        .header("Content-Type", "application/json")
        .send()
        .await?)
}

pub fn collect_all_urls(environment: &AggregatorEnvironment, component: ComponentType) -> Vec<String> {
    let mut res: Vec<String> = vec![];

    let component = match component {
        ComponentType::Graphql => GRAPHQL_COMPONENT,
        ComponentType::Debugger => DEBUGGER_COMPONENT,
    };

    for seed_label in 1..=environment.seed_node_count {
        let url = format!(
            "{}/{}{}/{}",
            CLUSTER_BASE_URL, SEED_NODE_COMPONENT, seed_label, component
        );
        res.push(url);
    }

    // producer nodes
    let producers = collect_producer_urls(environment);
    res.extend(producers);

    // snarker nodes
    for snarker_label in 1..=environment.snarker_node_count {
        let url = format!(
            "{}/{}{}/{}",
            CLUSTER_BASE_URL, SNARKER_NODE_COMPONENT, snarker_label, component
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
            CLUSTER_BASE_URL, TRANSACTION_GENERTOR_NODE_COMPONENT, component
        );
        res.push(url);
    }

    // plain nodes
    for plain_node_label in 1..=environment.plain_node_count {
        let url = if plain_node_label < 10 {
            format!(
                "{}/{}0{}/{}",
                CLUSTER_BASE_URL, PLAIN_NODE_COMPONENT, plain_node_label, component
            )
        } else {
            format!(
                "{}/{}{}/{}",
                CLUSTER_BASE_URL, PLAIN_NODE_COMPONENT, plain_node_label, component
            )
        };
        res.push(url);
    }

    res
}

fn collect_producer_urls(environment: &AggregatorEnvironment) -> Vec<String> {
    let mut res = vec![];
    for producer_label in 1..=environment.producer_node_count {
        let url = format!(
            "{}/{}{}/{}",
            CLUSTER_BASE_URL, PRODUCER_NODE_COMPONENT, producer_label, GRAPHQL_COMPONENT
        );
        res.push(url);
    }
    res
}
