use std::collections::BTreeMap;

use futures::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn, error, info};

use crate::{config::AggregatorEnvironment, AggregatorResult};

use super::{query_node, GraphqlResponse, TraceSource, TraceStatus, collect_all_urls, AddrsAndPorts, ComponentType};

const STRUCTURED_TRACE_PAYLOAD: &str = r#"{"query": "{ blockStructuredTrace(block_identifier: \"{STATE_HASH}\" ) }" }"#;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlockStructuredTraceData {
    pub block_structured_trace: BlockStructuredTrace,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlockStructuredTrace {
    pub source: TraceSource,
    pub blockchain_length_int: i64,
    // pub global_slot: String,
    pub status: TraceStatus,
    pub total_time: f64,
    pub sections: Vec<BlockStructuredTraceSection>,
    pub metadata: BlockStructuredTraceMetadata,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlockStructuredTraceSection {
    pub title: String,
    pub checkpoints: Vec<BlockStructuredTraceCheckpoint>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlockStructuredTraceCheckpoint {
    pub checkpoint: String,
    pub started_at: f64,
    pub duration: f64,
    pub metadata: serde_json::Value,
    pub checkpoints: Vec<BlockStructuredTraceCheckpoint>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlockStructuredTraceMetadata {
    pub proof_count: Option<usize>,
    pub txn_count: Option<usize>,
    pub global_slot: String,
    pub creator: String,
    pub winner: String,
    pub coinbase_receiver: String,
}

async fn query_block_traces(client: reqwest::Client, url: &str, state_hash: &str) -> AggregatorResult<BlockStructuredTrace> {
    let res: GraphqlResponse<BlockStructuredTraceData> = query_node(client, url, STRUCTURED_TRACE_PAYLOAD.replace("{STATE_HASH}", state_hash))
        .await?
        .json()
        .await?;

    Ok(res.data.block_structured_trace)
}

pub async fn get_block_trace_from_cluster(environment: &AggregatorEnvironment, state_hash: &str) -> BTreeMap<String, BlockStructuredTrace> {
    let client = reqwest::Client::new();

    let nodes = collect_all_urls(environment, ComponentType::Graphql);
    let bodies = stream::iter(nodes)
        .map(|(_, url)| {
            let client = client.clone();
            let state_hash = state_hash.to_string();
            tokio::spawn(async move { (url.clone(), query_block_traces(client, &url, &state_hash).await) })
        })
        .buffer_unordered(150);

    let collected: BTreeMap<String, BlockStructuredTrace> = bodies
        .fold(BTreeMap::<String, BlockStructuredTrace>::new(), |mut collected, b| async {
            match b {
                Ok((url, Ok(res))) => {
                    // info!("{url} OK");
                    collected.insert(url, res);
                }
                Ok((url, Err(e))) => warn!("Error requestig {url}, reason: {}", e),
                Err(e) => error!("Tokio join error: {e}"),
            }
            collected
        })
        .await;

    collected
}