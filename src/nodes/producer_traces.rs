use futures::{stream, StreamExt};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument, warn};

use crate::{config::AggregatorEnvironment, error::AggregatorError, AggregatorResult};

use super::{query_node, GraphqlResponse, Nodes};

const TRACES_PAYLOAD: &str = r#"{"query": "{ blockTraces(maxLength: 50, order: Descending) }" }"#;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlockTracesData {
    block_traces: BlockTraces,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlockTraces {
    traces: Vec<BlockTrace>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlockTrace {
    source: TraceSource,
    blockchain_length: usize,
    state_hash: String,
    status: TraceStatus,
    total_time: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum TraceSource {
    Internal,
    External,
    Unknown,
    Catchup,
    Reconstruct,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum TraceStatus {
    Success,
    Failure,
    Pending,
}

#[instrument(skip(client, tag))]
async fn query_producer_internal_blocks(
    client: reqwest::Client,
    url: &str,
    tag: &str,
    // block_count: usize,
) -> AggregatorResult<Vec<(usize, String, String)>> {
    // let res: GraphqlResponse<BlockTracesData> = query_node(client, url, TRACES_PAYLOAD.to_string())
    //     .await?
    //     .json()
    //     .await?;

    let res = query_node(client, url, TRACES_PAYLOAD.to_string()).await?;
    let status = res.status();

    if status != StatusCode::OK {
        return Err(AggregatorError::RpcServerError { status });
    }

    let res_json: GraphqlResponse<BlockTracesData> = res.json().await?;

    // the traces are sorted by height in asc order, reverse to get the most recent ones on the top
    let traces = res_json.data.block_traces.traces;

    let most_recent_height = if !traces.is_empty() {
        // traces[0].blockchain_length.clone()
        traces[0].blockchain_length
    } else {
        return Ok(vec![]);
    };

    let produced_blocks: Vec<(usize, String, String)> = traces
        .into_iter()
        .filter(|trace| {
            trace.blockchain_length == most_recent_height
                && (matches!(trace.source, TraceSource::Internal)
                    || matches!(trace.source, TraceSource::Unknown))
        })
        .map(|trace| (trace.blockchain_length, trace.state_hash, tag.to_string()))
        .collect();

    Ok(produced_blocks)
}

pub async fn get_most_recent_produced_blocks(
    environment: &AggregatorEnvironment,
    nodes: Nodes,
) -> Vec<(usize, String, String)> {
    let client = reqwest::Client::new();

    let bodies = stream::iter(nodes)
        .map(|(tag, url)| {
            let client = client.clone();
            tokio::spawn(async move {
                (
                    url.clone(),
                    query_producer_internal_blocks(client, &url, &tag).await,
                )
            })
        })
        .buffer_unordered(environment.producer_node_count);

    let collected = bodies
        .fold(Vec::new(), |mut collected, b| async {
            match b {
                Ok((url, Ok(res))) => {
                    debug!("{url} OK");
                    // let res: Vec<(usize, String, String)> = res
                    //     .into_iter()
                    //     .map(|(height, state_hash, tag)| {
                    //         // parsing shold be OK, as it is always a positive number, but lets change the graphql to report a number as height
                    //         (height.parse::<usize>().unwrap(), state_hash, tag)
                    //     })
                    //     .collect();
                    collected.extend(res);
                }
                Ok((url, Err(e))) => warn!("Error requestig {url}, reason: {}", e),
                Err(e) => error!("Tokio join error: {e}"),
            }
            collected
        })
        .await;

    info!("Collected {} produced blocks", collected.len());
    // println!("Blocks: {:#?}", collected);

    collected
}
