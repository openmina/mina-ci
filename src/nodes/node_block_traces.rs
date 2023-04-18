use std::{collections::BTreeMap, time::Duration};

use futures::{stream, StreamExt};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::{error, info, warn};

use crate::{error::AggregatorError, AggregatorResult};

use super::{query_node, GraphqlResponse, Nodes, TraceSource, TraceStatus};

const STRUCTURED_TRACE_PAYLOAD: &str =
    r#"{"query": "{ blockStructuredTrace(block_identifier: \"{STATE_HASH}\" ) }" }"#;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlockStructuredTraceData {
    pub block_structured_trace: BlockStructuredTrace,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlockStructuredTrace {
    pub source: TraceSource,
    pub blockchain_length: i64,
    // pub global_slot: String,
    pub status: TraceStatus,
    pub total_time: f64,
    pub sections: Vec<BlockStructuredTraceSection>,
    pub metadata: BlockStructuredTraceMetadata,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlockStructuredTraceSection {
    pub title: String,
    pub checkpoints: Vec<BlockStructuredTraceCheckpoint>,
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
    pub global_slot: Option<String>,
    pub creator: Option<String>,
    pub winner: Option<String>,
    pub coinbase_receiver: Option<String>,
    pub diff_log: Option<Vec<DiffLogPart>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DiffLogPart {
    pub discarded_commands: DiscardedCommands,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DiscardedCommands {
    pub insufficient_work: usize,
    pub insufficient_space: usize,
}

impl DiscardedCommands {
    pub fn total(&self) -> usize {
        self.insufficient_space + self.insufficient_work
    }
}

async fn query_block_traces(
    client: reqwest::Client,
    url: &str,
    state_hash: &str,
) -> AggregatorResult<BlockStructuredTrace> {
    // let res: GraphqlResponse<BlockStructuredTraceData> = query_node(client, url, STRUCTURED_TRACE_PAYLOAD.replace("{STATE_HASH}", state_hash))
    //     .await?
    //     .json()
    //     .await?;

    // hack until we remove duplicate fields from the trace response
    let res = query_node(
        client,
        url,
        STRUCTURED_TRACE_PAYLOAD.replace("{STATE_HASH}", state_hash),
    )
    .await?;

    let status = res.status();

    if status != StatusCode::OK {
        return Err(AggregatorError::RpcServerError { status });
    }

    let response = res.text().await?;

    let raw: serde_json::Value = serde_json::from_str(&response)?;
    let res: GraphqlResponse<BlockStructuredTraceData> = serde_json::from_value(raw)?;

    Ok(res.data.block_structured_trace)
}

pub async fn get_block_trace_from_cluster(
    nodes: Nodes,
    state_hash: &str,
) -> (BTreeMap<String, BlockStructuredTrace>, usize) {
    let client = reqwest::Client::new();

    const MAX_RETRIES: usize = 5;
    let mut retries: usize = 0;
    let nodes_count = nodes.len();
    let mut nodes_to_query = nodes;
    let mut final_res: BTreeMap<String, BlockStructuredTrace> = BTreeMap::new();
    let mut total_timeouts: usize = 0;

    while retries < MAX_RETRIES {
        let bodies = stream::iter(nodes_to_query.clone())
            .map(|(tag, url)| {
                let client = client.clone();
                let state_hash = state_hash.to_string();
                tokio::spawn(async move {
                    (
                        tag.clone(),
                        query_block_traces(client, &url, &state_hash).await,
                    )
                })
            })
            .buffer_unordered(150);

        let (collected, timeouts): (BTreeMap<String, BlockStructuredTrace>, usize) = bodies
            .fold(
                (BTreeMap::<String, BlockStructuredTrace>::new(), 0),
                |(mut collected, mut timeouts), b| async move {
                    match b {
                        Ok((tag, Ok(res))) => {
                            collected.insert(tag, res);
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

    // info!("Timeouts: {total_timeouts}");
    info!(
        "Collected {}/{} traces - Timeouts: {}",
        final_res.len(),
        nodes_count,
        total_timeouts
    );

    (final_res, total_timeouts)
}
