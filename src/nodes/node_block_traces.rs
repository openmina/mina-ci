use std::collections::BTreeMap;

use futures::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn, error};

use crate::{config::AggregatorEnvironment, AggregatorResult};

use super::{query_node, GraphqlResponse, TraceSource, TraceStatus, collect_all_urls, AddrsAndPorts};

const STRUCTURED_TRACE_PAYLOAD: &str = r#"{"query": "{ blockStructuredTrace(block_identifier: \"{STATE_HASH}\" ) daemonStatus { addrsAndPorts { externalIp } syncStatus metrics { transactionPoolSize transactionsAddedToPool transactionPoolDiffReceived transactionPoolDiffBroadcasted } } snarkPool { prover } }" }"#;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlockStructuredTraceData {
    pub block_structured_trace: BlockStructuredTrace,
    pub daemon_status: DaemonStatusForTraces,
    pub snark_pool: Vec<SnarkPoolElement>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlockStructuredTraceDataSlim {
    pub block_structured_trace: BlockStructuredTrace,
    pub daemon_status: DaemonStatusForTraces,
    pub snark_pool: usize,
}

impl From<BlockStructuredTraceData> for BlockStructuredTraceDataSlim {
    fn from(value: BlockStructuredTraceData) -> Self {
        Self {
            block_structured_trace: value.block_structured_trace,
            daemon_status: value.daemon_status,
            snark_pool: value.snark_pool.len(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SnarkPoolElement {
    pub prover: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DaemonMetrics {
    pub transaction_pool_size: usize,
    pub transactions_added_to_pool: usize,
    pub transaction_pool_diff_broadcasted: usize,
    pub transaction_pool_diff_received: usize,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DaemonStatusForTraces {
    pub addrs_and_ports: AddrsAndPorts,
    pub sync_status: String,
    pub metrics: DaemonMetrics,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlockStructuredTrace {
    pub source: TraceSource,
    pub global_slot: String,
    pub status: TraceStatus,
    pub total_time: f64,
    pub sections: Vec<BlockStructuredTraceSection>,
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
    pub metadata: String,
    pub checkpoints: Vec<BlockStructuredTraceCheckpoint>,
}

async fn query_block_traces(client: reqwest::Client, url: &str, state_hash: &str) -> AggregatorResult<BlockStructuredTraceDataSlim> {
    let res: GraphqlResponse<BlockStructuredTraceData> = query_node(client, url, STRUCTURED_TRACE_PAYLOAD.replace("{STATE_HASH}", state_hash))
        .await?
        .json()
        .await?;

    Ok(res.data.into())
}

pub async fn get_block_trace_from_cluster(environment: &AggregatorEnvironment, state_hash: &str) -> BTreeMap<String, BlockStructuredTraceDataSlim> {
    let client = reqwest::Client::new();

    let urls = collect_all_urls(environment);
    let bodies = stream::iter(urls)
        .map(|url| {
            let client = client.clone();
            let state_hash = state_hash.to_string();
            tokio::spawn(async move { (url.clone(), query_block_traces(client, &url, &state_hash).await) })
        })
        .buffer_unordered(150);

    let collected: BTreeMap<String, BlockStructuredTraceDataSlim> = bodies
        .fold(BTreeMap::<String, BlockStructuredTraceDataSlim>::new(), |mut collected, b| async {
            match b {
                Ok((url, Ok(res))) => {
                    debug!("{url} OK");
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