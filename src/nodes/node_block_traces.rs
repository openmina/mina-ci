use std::collections::BTreeMap;

use futures::{stream, StreamExt};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

use crate::{config::AggregatorEnvironment, error::AggregatorError, AggregatorResult};

use super::{
    collect_all_urls, query_node, ComponentType, GraphqlResponse, TraceSource, TraceStatus,
};

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
    environment: &AggregatorEnvironment,
    state_hash: &str,
) -> BTreeMap<String, BlockStructuredTrace> {
    let client = reqwest::Client::new();

    let nodes = collect_all_urls(environment, ComponentType::InternalTracing);
    let bodies = stream::iter(nodes)
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

    let collected: BTreeMap<String, BlockStructuredTrace> = bodies
        .fold(
            BTreeMap::<String, BlockStructuredTrace>::new(),
            |mut collected, b| async {
                match b {
                    Ok((tag, Ok(res))) => {
                        // info!("{url} OK");
                        collected.insert(tag, res);
                    }
                    Ok((tag, Err(e))) => warn!("Error requestig {tag}, reason: {}", e),
                    Err(e) => error!("Tokio join error: {e}"),
                }
                collected
            },
        )
        .await;

    collected
}
