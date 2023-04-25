use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{aggregators::BlockHash, error::AggregatorError, AggregatorResult};

use super::{query_node, GraphqlResponse};

const BEST_CHAIN_PAYLOAD: &str =
    r#"{"query": "{ bestChain { stateHash protocolState { consensusState { blockHeight } } } }" }"#;

pub type BestChain = Vec<BestChainBlock>;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BestChainData {
    pub best_chain: BestChain,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BestChainBlock {
    pub state_hash: BlockHash,
    pub protocol_state: ProtocolState,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolState {
    pub consensus_state: ConsensusState,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConsensusState {
    pub block_height: String,
}

pub async fn get_best_chain(url: &str) -> AggregatorResult<Vec<BestChainBlock>> {
    let client = reqwest::Client::new();

    // let url = "http://1.k8.openmina.com:31355/seed1/graphql";

    let res = query_node(client, url, BEST_CHAIN_PAYLOAD.to_string()).await?;

    let status = res.status();

    if status != StatusCode::OK {
        return Err(AggregatorError::RpcServerError { status });
    }

    let res_json: GraphqlResponse<BestChainData> = res.json().await?;

    Ok(res_json.data.best_chain)
}
