use std::{env, time::Duration};

// the debugger naming the cluster follows is dbg1, dbg2, ..., dbgN, where N is the number of debuggers running
// full URL for reference http://1.k8.openmina.com:31308/dbg1/block/111
// const DEBUGGER_COUNT: usize = 6;
const DEBUGGER_BASE_URL_DEFAULT: &str = "http://1.k8.openmina.com:31308/dbg";
const LIBP2P_IPC_URL_COMPONENT_DEFAULT: &str = "libp2p_ipc/block";
// const OUTPUT_PATH: &str = "output";
const RPC_PORT_DEFAULT: u16 = 8000;
pub const CLUSTER_NODE_LIST_URL: &str = "http://1.k8.openmina.com:31308/nodes";

const DATA_PULL_INTERVAL_DEFAULT: u64 = 10;

#[derive(Clone, Debug)]
pub struct AggregatorEnvironment {
    pub plain_node_count: usize,
    pub seed_node_count: usize,
    pub producer_node_count: usize,
    pub snarker_node_count: usize,
    pub transaction_generator_node_count: usize,
    pub debugger_count: usize,
    pub debugger_base_url: String,
    pub libp2p_ipc_encpoint: String,
    pub data_pull_interval: Duration,
    pub rpc_port: u16,
}

pub fn set_environment() -> AggregatorEnvironment {
    let debugger_count = env::var("DEBUGGER_COUNT")
        .expect("DEBUGGER_COUNT environment var must be set!")
        .parse::<usize>()
        .expect("DEBUGGER_COUNT should be a positve number (usize)");

    let plain_node_count = env::var("PLAIN_NODE_COUNT")
        .expect("PLAIN_NODE_COUNT environment var must be set!")
        .parse::<usize>()
        .expect("PLAIN_NODE_COUNT should be a positve number (usize)");

    let seed_node_count = env::var("SEED_NODE_COUNT")
        .expect("SEED_NODE_COUNT environment var must be set!")
        .parse::<usize>()
        .expect("SEED_NODE_COUNT should be a positve number (usize)");

    let producer_node_count = env::var("PRODUCER_NODE_COUNT")
        .expect("PRODUCER_NODE_COUNT environment var must be set!")
        .parse::<usize>()
        .expect("PRODUCER_NODE_COUNT should be a positve number (usize)");

    let transaction_generator_node_count = env::var("TRANSACTION_GENERATOR_NODE_COUNT")
        .expect("TRANSACTION_GENERATOR_NODE_COUNT environment var must be set!")
        .parse::<usize>()
        .expect("TRANSACTION_GENERATOR_NODE_COUNT should be a positve number (usize)");

    let snarker_node_count = env::var("SNARKER_NODE_COUNT")
        .expect("SNARKER_NODE_COUNT environment var must be set!")
        .parse::<usize>()
        .expect("SNARKER_NODE_COUNT should be a positve number (usize)");

    // TODO: parse as URL
    let debugger_base_url =
        env::var("DEBUGGER_BASE_URL").unwrap_or_else(|_| DEBUGGER_BASE_URL_DEFAULT.to_string());

    let libp2p_ipc_encpoint = env::var("LIBP2P_IPC_URL_COMPONENT")
        .unwrap_or_else(|_| LIBP2P_IPC_URL_COMPONENT_DEFAULT.to_string());

    let data_pull_interval = env::var("DATA_PULL_INTERVAL")
        .unwrap_or_else(|_| DATA_PULL_INTERVAL_DEFAULT.to_string())
        .parse::<u64>()
        .expect("DATA_PULL_INTERVAL should be a positve number representing seconds");
    let data_pull_interval = Duration::from_secs(data_pull_interval);

    let rpc_port = env::var("RPC_PORT")
        .unwrap_or_else(|_| RPC_PORT_DEFAULT.to_string())
        .parse::<u16>()
        .expect("RPC_PORT should be a valid port number");

    AggregatorEnvironment {
        plain_node_count,
        seed_node_count,
        producer_node_count,
        snarker_node_count,
        transaction_generator_node_count,
        debugger_count,
        debugger_base_url,
        libp2p_ipc_encpoint,
        data_pull_interval,
        rpc_port,
    }
}
