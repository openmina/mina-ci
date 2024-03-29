use std::{collections::BTreeMap, time::Duration};

use futures::{stream, StreamExt};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::{error, info, instrument, warn};

use crate::{
    config::AggregatorEnvironment, error::AggregatorError, nodes::RequestStats, AggregatorResult,
};

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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProducedBlock {
    pub height: usize,
    pub state_hash: String,
    pub tag: String,
}

#[instrument(skip(client, tag))]
async fn query_producer_internal_blocks(
    client: reqwest::Client,
    url: &str,
    tag: &str,
    // block_count: usize,
) -> AggregatorResult<BTreeMap<String, ProducedBlock>> {
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
        return Ok(BTreeMap::new());
    };

    let produced_blocks: BTreeMap<String, ProducedBlock> = traces
        .into_iter()
        .filter(|trace| {
            trace.blockchain_length == most_recent_height
                && matches!(trace.source, TraceSource::Internal)
        })
        // .map(|trace| (trace.blockchain_length, trace.state_hash, tag.to_string()))
        .map(|trace| {
            let produced_block = ProducedBlock {
                height: trace.blockchain_length,
                state_hash: trace.state_hash,
                tag: tag.to_string(),
            };
            (tag.to_string(), produced_block)
        })
        .collect();

    Ok(produced_blocks)
}

pub async fn get_most_recent_produced_blocks(
    environment: &AggregatorEnvironment,
    nodes: Nodes,
) -> (BTreeMap<String, ProducedBlock>, RequestStats) {
    let client = reqwest::Client::new();
    // let client = reqwest::Client::builder().pool_max_idle_per_host(0).build().unwrap();

    const MAX_RETRIES: usize = 5;
    let mut retries: usize = 0;
    let mut nodes_to_query = nodes;
    let mut final_res = BTreeMap::new();

    let mut total_request_stats = RequestStats::default();

    while retries < MAX_RETRIES {
        let bodies = stream::iter(nodes_to_query.clone())
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

        let (collected, timeouts, requests): (_, usize, usize) = bodies
            .fold(
                (BTreeMap::new(), 0, 0),
                |(mut collected, mut timeouts, mut requests), b| async move {
                    match b {
                        Ok((_, Ok(res))) => {
                            collected.extend(res);
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
                    requests += 1;
                    (collected, timeouts, requests)
                },
            )
            .await;

        final_res.extend(collected);
        nodes_to_query.retain(|k, _| !final_res.contains_key(k));

        let request_stats = RequestStats {
            request_count: requests,
            request_timeout_count: timeouts,
        };

        total_request_stats += request_stats;

        if final_res.len() == environment.producer_node_count {
            break;
        }

        sleep(Duration::from_secs(1)).await;
        retries += 1;
    }

    info!(
        "Collected {} produced blocks - Timeouts: {}",
        final_res.len(),
        total_request_stats.request_timeout_count,
    );

    (final_res, total_request_stats)

    // let bodies = stream::iter(nodes)
    //     .map(|(tag, url)| {
    //         let client = client.clone();
    //         tokio::spawn(async move {
    //             (
    //                 url.clone(),
    //                 query_producer_internal_blocks(client, &url, &tag).await,
    //             )
    //         })
    //     })
    //     .buffer_unordered(environment.producer_node_count);

    // let collected = bodies
    //     .fold(Vec::new(), |mut collected, b| async {
    //         match b {
    //             Ok((url, Ok(res))) => {
    //                 debug!("{url} OK");
    //                 collected.extend(res);
    //             }
    //             Ok((url, Err(e))) => warn!("Error requestig {url}, reason: {}", e),
    //             Err(e) => error!("Tokio join error: {e}"),
    //         }
    //         collected
    //     })
    //     .await;
    // collected
}
