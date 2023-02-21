use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Write;

use petgraph::algo::{connected_components, is_cyclic_directed};
use petgraph::dot::Dot;
use petgraph::prelude::*;
use petgraph::{Directed, Graph};
use serde::Serialize;

use crate::debugger_data::{CpnpCapturedData, DebuggerCpnpResponse, NodeAddressCluster};

use crate::nodes::DaemonStatusDataSlim;
use crate::{AggregatorError, AggregatorResult};

pub type NodeIP = String;
pub type BlockHash = String;
pub type MessageGraph = Graph<String, u64, Directed>;

#[derive(Debug, Default, Serialize, Clone)]
pub struct CpnpLatencyAggregationData {
    pub message_source: NodeIP,
    pub message_source_tag: String,
    pub node_address: NodeIP,
    pub node_tag: String,
    pub receive_time: u64,
    pub latency_since_sent: Option<u64>,
    pub latency_since_block_publication: u64,
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct CpnpBlockPublication {
    #[serde(flatten)]
    pub node_latencies: BTreeMap<NodeIP, CpnpLatencyAggregationData>,
    #[serde(skip)]
    pub graph: MessageGraph,
    pub publish_time: u64,
    pub block_hash: String,
    pub height: usize,
    pub graph_info: Option<MessageGraphInfo>,
    #[serde(skip)]
    unique_nodes: HashMap<NodeIP, NodeIndex>,
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct CpnpBlockPublicationFlattened {
    pub height: usize,
    pub block_hash: String,
    pub publish_time: u64,
    pub node_latencies: Vec<CpnpLatencyAggregationData>,
    pub graph_info: Option<MessageGraphInfo>,
    #[serde(skip)]
    pub graph: MessageGraph,
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct MessageGraphInfo {
    pub node_count: usize,
    pub source_node_count: usize,
    pub source_nodes: Vec<String>,
    pub graph_count: usize,
    pub is_graph_cyclic: bool,
    pub missing_nodes: Vec<String>,
}

impl From<CpnpBlockPublication> for CpnpBlockPublicationFlattened {
    fn from(value: CpnpBlockPublication) -> Self {
        let node_latencies = value.node_latencies.values().cloned().collect();

        Self {
            node_latencies,
            graph: value.graph.clone(),
            publish_time: value.publish_time,
            block_hash: value.block_hash,
            height: value.height,
            graph_info: value.graph_info,
        }
    }
}

impl CpnpBlockPublication {
    pub fn init_with_source(
        source_node: NodeIP,
        source_tag: &str,
        publish_time: u64,
        block_hash: String,
        height: usize,
    ) -> Self {
        let mut graph: MessageGraph = Graph::new();
        let mut unique_nodes: HashMap<NodeIP, NodeIndex> = HashMap::new();
        let mut node_latencies: BTreeMap<NodeIP, CpnpLatencyAggregationData> = BTreeMap::new();

        let source_data = CpnpLatencyAggregationData {
            message_source: source_node.clone(),
            message_source_tag: source_tag.to_string(),
            node_tag: source_tag.to_string(),
            node_address: source_node.clone(),
            receive_time: publish_time,
            latency_since_block_publication: 0,
            latency_since_sent: Some(0),
        };

        node_latencies.insert(source_node.clone(), source_data);

        unique_nodes.insert(source_node.clone(), graph.add_node(source_node));

        Self {
            block_hash,
            publish_time,
            graph,
            unique_nodes,
            node_latencies,
            height,
            graph_info: None,
        }
    }
}

pub fn aggregate_first_receive(
    data: Vec<DebuggerCpnpResponse>,
    node_ip_to_tag_map: &BTreeMap<String, String>,
) -> AggregatorResult<(usize, BTreeMap<BlockHash, CpnpBlockPublication>)> {
    // println!("Data: {:#?}", data);
    // there could be multiple blocks for a specific height, so we need to differentioat by block hash
    let mut by_block: BTreeMap<BlockHash, CpnpBlockPublication> = BTreeMap::new();

    // collect all the events, flatten
    let mut events: Vec<CpnpCapturedData> = data.into_iter().flatten().collect();

    // and sort by time
    // events.sort_by(|a, b| a.better_time.to_nanoseconds().cmp(&b.better_time.to_nanoseconds()));
    events.sort_by_key(|e| e.real_time_microseconds);

    // TODO: try to remove this additional iteration somehow as an optimization
    // Note: When we are sure the debugger timestamps are correct, we can remove this as by sorting by timestamp, the pusblish event will always
    //       preceed the receive events
    let publish_messages: Vec<CpnpCapturedData> = events
        .clone()
        .into_iter()
        .filter(|e| {
            e.events
                .iter()
                .filter(|cpnp_event| cpnp_event.r#type == *"publish_gossip")
                .peekable()
                .peek()
                .is_some()
        })
        .collect();

    println!("Publish messages: {:#?}", publish_messages);

    // FIXME
    let height = if !publish_messages.is_empty() {
        publish_messages[0].events[0].msg.height
    } else {
        println!("Error: Height");
        return Err(AggregatorError::SourceNotReady);
    };

    for publish_message in publish_messages {
        let block_hash = publish_message.events[0].hash.clone();
        let publication_data = CpnpBlockPublication::init_with_source(
            publish_message.node_address.ip(),
            &publish_message.node_tag,
            publish_message.real_time_microseconds,
            block_hash,
            height,
        );
        by_block.insert(publish_message.events[0].hash.clone(), publication_data);
    }

    for event in events.clone() {
        let block_data = if let Some(block_data) = by_block.get_mut(&event.events[0].hash) {
            println!("Found matching event with hash: {}", event.events[0].hash);
            block_data
        } else {
            println!("Ignoring event with hash: {}", event.events[0].hash);
            // return Err(AggregatorError::SourceNotReady);
            continue;
        };

        // ignore other event types
        // TODO: enum...
        if event.events[0].r#type == "received_gossip" {
            let source_node = event.events[0].peer_address.as_ref().unwrap();
            let current_node = event.node_address;
            let current_node_tag = event.node_tag;

            // this is a hack due to wrong timestamps...
            let source_receive_time =
                if let Some(data) = block_data.node_latencies.get(&source_node.ip()) {
                    data.receive_time
                } else {
                    u64::MAX
                };

            let node_data = CpnpLatencyAggregationData {
                message_source: source_node.ip(),
                message_source_tag: node_ip_to_tag_map.get(&source_node.ip()).unwrap_or(&"".to_string()).to_string(),
                node_address: current_node.ip(),
                node_tag: current_node_tag,
                receive_time: event.real_time_microseconds,
                latency_since_sent: Some(
                    event
                        .real_time_microseconds
                        .saturating_sub(source_receive_time),
                ),
                latency_since_block_publication: event
                    .real_time_microseconds
                    .saturating_sub(block_data.publish_time),
            };

            let source_graph_vertex = *block_data
                .unique_nodes
                .entry(source_node.ip())
                .or_insert_with(|| block_data.graph.add_node(source_node.ip()));
            let destination_graph_vertex = block_data
                .unique_nodes
                .entry(current_node.ip())
                .or_insert_with(|| block_data.graph.add_node(current_node.ip()));

            block_data.graph.update_edge(
                source_graph_vertex,
                *destination_graph_vertex,
                event
                    .real_time_microseconds
                    .saturating_sub(source_receive_time),
            );
            block_data
                .node_latencies
                .insert(current_node.ip(), node_data);
        }
    }

    // Add info by running algos on the constructed graph
    for (_, block_data) in by_block.iter_mut() {
        let source_nodes = get_source_nodes(&block_data.graph);
        let node_count = block_data.graph.node_count();
        let graph_count = connected_components(&block_data.graph);
        let is_graph_cyclic = is_cyclic_directed(&block_data.graph);

        // let missing_nodes = nodes_in_cluster
        //     .clone()
        //     .into_iter()
        //     .filter(|node| !block_data.unique_nodes.contains_key(node))
        //     .collect();

        let info = MessageGraphInfo {
            node_count,
            source_node_count: source_nodes.len(),
            source_nodes,
            graph_count,
            is_graph_cyclic,
            // TODO
            missing_nodes: vec![],
        };

        block_data.graph_info = Some(info);
    }

    Ok((height, by_block))
    // for (hash, data) in by_block {
    //     let dot = Dot::new(&data.graph);
    //     let file_path = format!("output/{}", &hash[..5]);
    //     let mut f = std::fs::File::create(file_path).unwrap();
    //     f.write_all(format!("{}", dot).as_bytes()).unwrap();
    // }

    // TODO: move to tests
    // SANITY CHECK: first receive time < first send time
    // for block in by_block.values() {
    //     for node in &block.pairs {
    //         if let (Some(rec), Some(send)) = (node.1.first_receive.clone(), node.1.first_send.clone()) {
    //             if rec.to_nanoseconds() > send.to_nanoseconds() {
    //                 println!("WRONG!");
    //                 println!("{} !< {}", rec.to_nanoseconds(), send.to_nanoseconds());
    //                 let node_events: Vec<CapturedEvent> = events.iter().filter(|e| &e.sender_addr.ip() == node.0 || &e.receiver_addr.ip() == node.0).cloned().collect();
    //                 println!("Events: {:#?}", node_events);
    //             }
    //         }
    //     }
    // }
}

fn get_source_nodes(graph: &MessageGraph) -> Vec<String> {
    let mut sources: Vec<String> = vec![];
    for node in graph.node_indices() {
        if graph
            .edges_directed(node, Incoming)
            .peekable()
            .peek()
            .is_none()
        {
            sources.push(graph.node_weight(node).unwrap().to_string());
        }
    }
    sources
}

// #[cfg(test)]
// mod tests {
//     use crate::debugger_data::{DebuggerCpnpResponse, NodeAddressCluster};

//     fn test_data() -> (Vec<DebuggerCpnpResponse>, Vec<NodeAddressCluster>) {

//         (vec![], vec![])
//     }
// }
