use std::collections::{BTreeMap, HashSet, HashMap};
use std::io::Write;

use petgraph::dot::Dot;
use petgraph::{Graph, Directed};
use petgraph::prelude::*;
use serde::Serialize;

use crate::debugger_data::{DebuggerBlockResponse, DebuggerTime, CapturedEvent, DebuggerCpnpResponse, CpnpCapturedData};

use crate::AggregatorError;

pub type AggregatorResult<T> = Result<T, AggregatorError>;

#[derive(Debug, Default)]
pub struct FirstReceiveSendPair {
    pub first_receive: Option<DebuggerTime>,
    pub first_send: Option<DebuggerTime>,
    pub source_node: Option<String>,
    pub destination_node: Option<String>,
}

#[derive(Debug, Default)]
pub struct BlockFirstReceiveSendData {
    pub pairs: BlockFirstReceiveSendPairs,
    pub graph: Graph<String, u64, Directed>,
}

pub type NodeIP = String;
pub type BlockHash = String;

pub type BlockFirstReceiveSendPairs = BTreeMap<NodeIP, FirstReceiveSendPair>;

#[derive(Debug, Default, Serialize, Clone)]
pub struct CpnpLatencyAggregationData {
    pub message_source: NodeIP,
    pub node_address: NodeIP,
    pub receive_time: u64,
    pub latency_since_sent: Option<u64>,
    pub latency_since_block_publication: u64,
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct CpnpBlockPublication {
    pub node_latencies: BTreeMap<NodeIP, CpnpLatencyAggregationData>,
    #[serde(skip)]
    pub graph: Graph<String, u64, Directed>,
    pub publish_time: u64,
    #[serde(skip)]
    unique_nodes: HashMap<NodeIP, NodeIndex>,
}

impl CpnpBlockPublication {
    pub fn init_with_source(source_node: NodeIP, publish_time: u64) -> Self {
        let mut graph: Graph<String, u64, Directed> = Graph::new();
        let mut unique_nodes: HashMap<NodeIP, NodeIndex> = HashMap::new();
        let mut node_latencies: BTreeMap<NodeIP, CpnpLatencyAggregationData> = BTreeMap::new();

        let source_data = CpnpLatencyAggregationData {
            message_source: source_node.clone(),
            node_address: source_node.clone(),
            receive_time: publish_time,
            latency_since_block_publication: 0,
            latency_since_sent: Some(0),
        };

        node_latencies.insert(source_node.clone(), source_data);

        unique_nodes.insert(source_node.clone(), graph.add_node(source_node));

        Self {
            publish_time,
            graph,
            unique_nodes,
            node_latencies,
        }
    }
}

pub fn aggregate_first_receive_send(data: Vec<DebuggerBlockResponse>) {
    // there could be multiple blocks for a specific height, so we need to differentioat by block hash

    let mut by_block: BTreeMap<BlockHash, BlockFirstReceiveSendData> = BTreeMap::new();
    // let mut pairs: BlockFirstReceiveSendPairs = BTreeMap::new();

    // collect all the events, flatten
    let mut events: Vec<CapturedEvent> = data.iter().flat_map(|dbg| {
        dbg.events.clone()
    }).collect();

    // and sort by time
    events.sort_by(|a, b| a.better_time.to_nanoseconds().cmp(&b.better_time.to_nanoseconds()));

    // DBEUG: quick sanity check for sorted
    // {
    //     for window in events.windows(2) {
    //         let time0 = window[0].better_time.to_nanoseconds();
    //         let time1 = window[1].better_time.to_nanoseconds();
    //         if window[0].better_time.to_nanoseconds() > window[1].better_time.to_nanoseconds() {
    //             println!("BAD!");
    //             println!("{} >! {}", time0, time1);
    //             break;
    //         }
    //     }
    // }

    for event in events.clone() {
        if let "publish_new_state" = event.message_kind.as_str() {
            let block = by_block.entry(event.hash).or_default();

            if event.incoming {
                let node_addr = event.receiver_addr;
                let node = block.pairs.entry(node_addr.ip()).or_default();

                if node.first_receive.is_none() {
                    node.first_receive = Some(event.time);
                    node.source_node = Some(event.sender_addr.ip());
                }
            } else {
                let node_addr = event.sender_addr;
                let node = block.pairs.entry(node_addr.ip()).or_default();

                if node.first_send.is_none() {
                    node.first_send = Some(event.time);
                    node.destination_node = Some(event.receiver_addr.ip());
                }
            }
        }
    }

    // println!("{:#?}", by_block);

    // TODO: move to tests
    // SANITY CHECK: first receive time < first send time
    for block in by_block.values() {
        for node in &block.pairs {
            if let (Some(rec), Some(send)) = (node.1.first_receive.clone(), node.1.first_send.clone()) {
                if rec.to_nanoseconds() > send.to_nanoseconds() {
                    println!("WRONG!");
                    println!("{} !< {}", rec.to_nanoseconds(), send.to_nanoseconds());
                    let node_events: Vec<CapturedEvent> = events.iter().filter(|e| &e.sender_addr.ip() == node.0 || &e.receiver_addr.ip() == node.0).cloned().collect();
                    println!("Events: {:#?}", node_events);
                }
            }
        }
    }

}

pub fn aggregate_first_receive(data: Vec<DebuggerCpnpResponse>) -> AggregatorResult<(usize, BTreeMap<BlockHash, CpnpBlockPublication>)> {
    // there could be multiple blocks for a specific height, so we need to differentioat by block hash

    let mut by_block: BTreeMap<BlockHash, CpnpBlockPublication> = BTreeMap::new();
    // let mut pairs: BlockFirstReceiveSendPairs = BTreeMap::new();

    // collect all the events, flatten
    let mut events: Vec<CpnpCapturedData> = data.into_iter().flatten().collect();

    // and sort by time
    // events.sort_by(|a, b| a.better_time.to_nanoseconds().cmp(&b.better_time.to_nanoseconds()));
    events.sort_by_key(|e| e.real_time_microseconds);

    // let json = serde_json::to_string_pretty(&events).unwrap();
    // println!("{json}");

    // DBEUG: quick sanity check for sorted
    // {
    //     for window in events.windows(2) {
    //         let time0 = window[0].better_time.to_nanoseconds();
    //         let time1 = window[1].better_time.to_nanoseconds();
    //         if window[0].better_time.to_nanoseconds() > window[1].better_time.to_nanoseconds() {
    //             println!("BAD!");
    //             println!("{} >! {}", time0, time1);
    //             break;
    //         }
    //     }
    // }

    // TODO: remove this additional iteration somehow as an optimization
    let publish_messages: Vec<CpnpCapturedData> = events.clone()
        .into_iter()
        .filter(|e| e.events.iter().filter(|cpnp_event| cpnp_event.r#type == *"publish_gossip").peekable().peek().is_some())
        .collect();

    let height = publish_messages[0].events[0].msg.height;
    for publish_message in publish_messages {
        let publication_data = CpnpBlockPublication::init_with_source(publish_message.node_address.ip(), publish_message.real_time_microseconds);
        by_block.insert(publish_message.events[0].hash.clone(), publication_data);
    }

    for event in events.clone() {
        let block_data = if let Some(block_data) = by_block.get_mut(&event.events[0].hash) {
            block_data
        } else {
            return Err(AggregatorError::SourceNotReady);
        };

        // ignore other event types
        // TODO: enum...
        if event.events[0].r#type == "received_gossip" {
            let source_node = event.events[0].peer_address.as_ref().unwrap();
            let current_node = event.node_address;

            // this is a hack due to wrong timestamps...
            let source_receive_time = if let Some (data) = block_data.node_latencies.get(&source_node.ip()) {
                data.receive_time
            } else {
                u64::MAX
            };
            
            let node_data = CpnpLatencyAggregationData {
                message_source: source_node.ip(),
                node_address: current_node.ip(),
                receive_time: event.real_time_microseconds,
                latency_since_sent: Some(event.real_time_microseconds.saturating_sub(source_receive_time)),
                latency_since_block_publication: event.real_time_microseconds.saturating_sub(block_data.publish_time),
            };

            let source_graph_vertex = *block_data.unique_nodes.entry(source_node.ip()).or_insert_with(|| block_data.graph.add_node(source_node.ip()));
            let destination_graph_vertex = block_data.unique_nodes.entry(current_node.ip()).or_insert_with(|| block_data.graph.add_node(current_node.ip()));

            block_data.graph.update_edge(source_graph_vertex, *destination_graph_vertex, event.real_time_microseconds.saturating_sub(source_receive_time));
            block_data.node_latencies.insert(current_node.ip(), node_data);
        }
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