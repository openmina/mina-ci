use std::{collections::BTreeMap, time::Duration};

use reqwest::Response;
use serde::{Deserialize, Serialize};

use crate::{config::AggregatorEnvironment, AggregatorResult};

pub mod node_info;
pub use node_info::*;

pub mod producer_traces;
pub use producer_traces::*;

pub mod node_block_traces;
pub use node_block_traces::*;

pub mod best_chain;
pub use best_chain::*;

const PLAIN_NODE_COMPONENT: &str = "node";
const SEED_NODE_COMPONENT: &str = "seed";
const PRODUCER_NODE_COMPONENT: &str = "prod";
const SNARKER_NODE_COMPONENT: &str = "snarker";
// const TRANSACTION_GENERTOR_NODE_COMPONENT: &str = "transaction-generator";

const DEBUGGER_COMPONENT: &str = "bpf-debugger";
const GRAPHQL_COMPONENT: &str = "graphql";
const INTERNAL_TRACING_COMPONENT: &str = "internal-trace/graphql";

pub type BuildNodes = BTreeMap<String, DaemonStatusDataSlim>;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GraphqlResponse<T> {
    pub data: T,
}

pub enum ComponentType {
    Graphql,
    Debugger,
    InternalTracing,
}

/// <tag, URL>
pub type Nodes = BTreeMap<String, String>;

async fn query_node(
    client: reqwest::Client,
    url: &str,
    payload: String,
) -> AggregatorResult<Response> {
    Ok(client
        .post(url)
        .body(payload)
        .header("Content-Type", "application/json")
        .timeout(Duration::from_secs(5))
        .send()
        .await?)
}

pub fn collect_all_urls(
    environment: &AggregatorEnvironment,
    component_type: ComponentType,
) -> Nodes {
    let mut res: Nodes = BTreeMap::new();

    let cluster_base_url = environment.cluster_base_url.to_string();

    let component = match component_type {
        ComponentType::Graphql => GRAPHQL_COMPONENT,
        ComponentType::Debugger => DEBUGGER_COMPONENT,
        ComponentType::InternalTracing => INTERNAL_TRACING_COMPONENT,
    };

    for seed_index in 1..=environment.seed_node_count {
        let seed_label = format!("{}{}", SEED_NODE_COMPONENT, seed_index);
        let url = format!("{}/{}/{}", cluster_base_url, seed_label, component);
        res.insert(seed_label, url);
    }

    // producer nodes
    let producers = collect_producer_urls(environment, component_type);
    res.extend(producers);

    // snarker nodes
    for snarker_index in 1..=environment.snarker_node_count {
        let snarker_label = format!("{}{snarker_index:0>3}", SNARKER_NODE_COMPONENT);
        let url = format!("{}/{}/{}", cluster_base_url, snarker_label, component);
        res.insert(snarker_label, url);
    }

    // transaction generator nodes
    // NOTE: No indexs yet, only one transaction-generator
    // for transaction_generator_index in 1..=environment.snarker_node_count {
    //     let url = format!("{}/{}{}/{}", CLUSTER_BASE_URL, TRANSACTION_GENERTOR_NODE_COMPONENT, transaction_generator_index, GRAPHQL_COMPONENT);
    //     match query_node_ip(&url).await {
    //         Ok(ip) => collected.push(ip),
    //         Err(e) => warn!("Seed node failed to respond, reason: {}", e),
    //     }
    // }
    // {
    //     let url = format!(
    //         "{}/{}/{}",
    //         CLUSTER_BASE_URL, TRANSACTION_GENERTOR_NODE_COMPONENT, component
    //     );
    //     res.push(url);
    // }

    // plain nodes
    for plain_node_index in 1..=environment.plain_node_count {
        let plain_node_label = format!("{}{}", PLAIN_NODE_COMPONENT, plain_node_index);
        let url = format!("{}/{}/{}", cluster_base_url, plain_node_label, component);
        res.insert(plain_node_label, url);
    }

    res
}

pub fn get_seed_url(environment: &AggregatorEnvironment, component_type: ComponentType) -> String {
    let component = match component_type {
        ComponentType::Graphql => GRAPHQL_COMPONENT,
        ComponentType::Debugger => DEBUGGER_COMPONENT,
        ComponentType::InternalTracing => INTERNAL_TRACING_COMPONENT,
    };

    let cluster_base_url = environment.cluster_base_url.to_string();
    let seed_label = format!("{}{}", SEED_NODE_COMPONENT, 1);
    format!("{}/{}/{}", cluster_base_url, seed_label, component)
}

pub fn collect_producer_urls(
    environment: &AggregatorEnvironment,
    component: ComponentType,
) -> Nodes {
    let component = match component {
        ComponentType::Graphql => GRAPHQL_COMPONENT,
        ComponentType::Debugger => DEBUGGER_COMPONENT,
        ComponentType::InternalTracing => INTERNAL_TRACING_COMPONENT,
    };

    let cluster_base_url = environment.cluster_base_url.to_string();

    let mut res = BTreeMap::new();
    // TODO: special case, when the producers with prefix 0 have the same key...
    let producers = ["01", "02", "03", "2", "3"];
    for producer_index in producers {
        let producer_label = format!("{}{}", PRODUCER_NODE_COMPONENT, producer_index);
        let url = format!("{}/{}/{}", cluster_base_url, producer_label, component);
        res.insert(producer_label, url);
    }
    res
}

pub fn collect_producer_urls_cluster_ip(
    build_nodes: &BuildNodes,
    component_type: ComponentType,
) -> Nodes {
    let (component, port) = match component_type {
        // TODO: rework
        ComponentType::Graphql => ("/graphql", "3085"),
        ComponentType::Debugger => ("", "80"),
        ComponentType::InternalTracing => ("/graphql", "8000"),
    };

    build_nodes
        .iter()
        .filter(|(tag, _)| tag.contains("prod"))
        .map(|(tag, data)| {
            // TODO: get the port form the environmnet(inlcude in chart?)
            let url = format!(
                "http://{}:{port}{component}",
                data.daemon_status.addrs_and_ports.external_ip
            );
            (tag.clone(), url)
        })
        .collect()
}

pub fn get_seed_url_cluster_ip(build_nodes: &BuildNodes, component_type: ComponentType) -> String {
    let (component, port) = match component_type {
        // TODO: rework
        ComponentType::Graphql => ("/graphql", "3085"),
        ComponentType::Debugger => ("", "80"),
        ComponentType::InternalTracing => ("/graphql", "8000"),
    };

    build_nodes
        .iter()
        .find(|(tag, _)| tag.contains("seed1"))
        .map(|(_, data)| {
            format!(
                "http://{}:{port}{component}",
                data.daemon_status.addrs_and_ports.external_ip
            )
        })
        .unwrap_or_default()
}

pub fn collect_all_urls_cluster_ip(
    build_nodes: &BuildNodes,
    component_type: ComponentType,
) -> Nodes {
    let (component, port) = match component_type {
        // TODO: rework
        ComponentType::Graphql => ("/graphql", "3085"),
        ComponentType::Debugger => ("", "80"),
        ComponentType::InternalTracing => ("/graphql", "8000"),
    };

    build_nodes
        .iter()
        .map(|(tag, data)| {
            // TODO: get the port form the environmnet(inlcude in chart?)
            let url = format!(
                "http://{}:{port}{component}",
                data.daemon_status.addrs_and_ports.external_ip
            );
            (tag.clone(), url)
        })
        .collect()
}
