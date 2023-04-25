use std::{env, fmt::Display, time::Duration};

const LIBP2P_IPC_URL_COMPONENT_DEFAULT: &str = "libp2p_ipc/block";
// const OUTPUT_PATH: &str = "output";
const RPC_PORT_DEFAULT: u16 = 8000;
pub const CLUSTER_NODE_LIST_URL: &str = "http://1.k8.openmina.com:31311/nodes";
const CLUSTER_BASE_URL: &str = "http://1.k8.openmina.com:31308";
const CI_API_URL: &str = "https://ci.openmina.com/api";
const CI_REPO: &str = "mina";
const REMOTE_STORAGE_URL: &str = "ci.openmina.com:22";
const REMOTE_STORAGE_PATH: &str = "/home/aggregator/storage.json";

const DATA_PULL_INTERVAL_DEFAULT: u64 = 10;

#[derive(Clone, Debug)]
pub struct AggregatorEnvironment {
    pub plain_node_count: usize,
    pub seed_node_count: usize,
    pub producer_node_count: usize,
    pub snarker_node_count: usize,
    pub transaction_generator_node_count: usize,
    pub libp2p_ipc_encpoint: String,
    pub data_pull_interval: Duration,
    pub rpc_port: u16,
    pub cluster_base_url: String,
    pub ci_api_url: String,
    pub ci_repo: String,
    pub remote_storage_url: String,
    pub remote_storage_user: String,
    pub remote_storage_password: String,
    pub remote_storage_path: String,
    pub use_internal_endpoints: bool,
    pub disable_aggregation: bool,
}

impl Display for AggregatorEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\n\tplain_node_count: {}", self.plain_node_count)?;
        writeln!(f, "\tseed_node_count: {}", self.seed_node_count)?;
        writeln!(f, "\tproducer_node_count: {}", self.producer_node_count)?;
        writeln!(f, "\tsnarker_node_count: {}", self.snarker_node_count)?;
        writeln!(
            f,
            "\tdata_pull_interval: {}",
            self.data_pull_interval.as_secs()
        )?;
        writeln!(f, "\trpc_port: {}", self.rpc_port)?;
        writeln!(f, "\tcluster_base_url: {}", self.cluster_base_url)?;
        writeln!(f, "\tci_api_url: {}", self.ci_api_url)?;
        writeln!(f, "\tremote_storage_url: {}", self.remote_storage_url)?;
        writeln!(f, "\tremote_storage_path: {}", self.remote_storage_path)?;
        writeln!(
            f,
            "\tuse_internal_endpoints: {}",
            self.use_internal_endpoints
        )?;
        writeln!(f, "\tdisable_aggregation: {}", self.disable_aggregation)
    }
}

impl AggregatorEnvironment {
    pub fn total_node_count(&self) -> usize {
        self.seed_node_count
            + self.plain_node_count
            + self.producer_node_count
            + self.snarker_node_count
            + self.transaction_generator_node_count
    }
}

pub fn set_environment() -> AggregatorEnvironment {
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

    let cluster_base_url =
        env::var("CLUSTER_BASE_URL").unwrap_or_else(|_| CLUSTER_BASE_URL.to_string());

    let ci_api_url = env::var("CI_API_URL").unwrap_or_else(|_| CI_API_URL.to_string());
    let ci_repo = env::var("CI_REPO").unwrap_or_else(|_| CI_REPO.to_string());

    let remote_storage_url =
        env::var("REMOTE_STORAGE_URL").unwrap_or_else(|_| REMOTE_STORAGE_URL.to_string());
    let remote_storage_user =
        env::var("REMOTE_STORAGE_USER").expect("REMOTE_STORAGE_USER environment var must be set!");
    let remote_storage_password = env::var("REMOTE_STORAGE_PASSWORD")
        .expect("REMOTE_STORAGE_PASSWORD environment var must be set!");
    let remote_storage_path =
        env::var("REMOTE_STORAGE_PATH").unwrap_or_else(|_| REMOTE_STORAGE_PATH.to_string());

    let use_internal_endpoints = env::var("USE_INTERNAL_ENDPOINTS").is_ok();
    let disable_aggregation = env::var("DISABLE_AGGREGATION").is_ok();

    AggregatorEnvironment {
        plain_node_count,
        seed_node_count,
        producer_node_count,
        snarker_node_count,
        transaction_generator_node_count,
        libp2p_ipc_encpoint,
        data_pull_interval,
        rpc_port,
        cluster_base_url,
        ci_api_url,
        ci_repo,
        remote_storage_url,
        remote_storage_user,
        remote_storage_password,
        remote_storage_path,
        use_internal_endpoints,
        disable_aggregation,
    }
}
