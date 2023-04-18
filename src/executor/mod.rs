use std::{collections::BTreeMap, time::Duration};

use tokio::time::sleep;
use tracing::{info, instrument, warn};

use crate::{
    aggregators::{aggregate_block_traces, aggregate_first_receive},
    config::AggregatorEnvironment,
    cross_validation::cross_validate_ipc_with_traces,
    debugger_data::{CpnpCapturedData, DebuggerCpnpResponse},
    executor::state::AggregatorStateInner,
    nodes::{
        collect_all_urls, collect_all_urls_cluster_ip, collect_producer_urls,
        collect_producer_urls_cluster_ip, get_best_chain, get_block_trace_from_cluster,
        get_most_recent_produced_blocks, get_node_info_from_cluster, get_seed_url,
        get_seed_url_cluster_ip, ComponentType, Nodes, RequestStats,
    },
    storage::AggregatorStorage,
    AggregatorResult,
};

use self::state::AggregatorState;

pub mod state;

#[instrument(skip(environment, nodes))]
async fn pull_debugger_data_cpnp(
    height: Option<usize>,
    environment: &AggregatorEnvironment,
    nodes: Nodes,
) -> Vec<DebuggerCpnpResponse> {
    let mut collected: Vec<DebuggerCpnpResponse> = vec![];

    for (tag, url) in nodes.iter() {
        // info!("Pulling {}", url);
        match get_height_data_cpnp(height, url, environment).await {
            Ok(data) => {
                let modified_data = data
                    .into_iter()
                    .map(|mut e| {
                        // e.node_address = NodeAddress(node_infos.get(tag).unwrap().daemon_status.addrs_and_ports.external_ip.clone());
                        e.node_tag = tag.to_string();
                        e
                    })
                    .collect::<Vec<CpnpCapturedData>>();
                collected.push(modified_data);
            }
            Err(e) => warn!("{} failed to provide data, reson: {}", url, e),
        }
    }

    collected
}

async fn get_height_data_cpnp(
    height: Option<usize>,
    base_url: &str,
    environment: &AggregatorEnvironment,
) -> AggregatorResult<DebuggerCpnpResponse> {
    let url = if let Some(height) = height {
        format!(
            "{}/{}/{}",
            base_url, environment.libp2p_ipc_encpoint, height
        )
    } else {
        format!(
            "{}/{}/{}",
            base_url, environment.libp2p_ipc_encpoint, "latest"
        )
    };
    // reqwest::get(url).await?.json().await.map_err(|e| e.into())
    let client = reqwest::Client::new();
    client
        .get(url)
        .timeout(Duration::from_secs(5))
        .send()
        .await?
        .json()
        .await
        .map_err(|e| e.into())
}

pub async fn poll_node_traces(
    state: &AggregatorState,
    storage: &mut AggregatorStorage,
    environment: &AggregatorEnvironment,
) {
    loop {
        info!("Sleeping");
        sleep(environment.data_pull_interval).await;

        let current_state = if let Ok(read_state) = state.read() {
            read_state.clone()
        } else {
            info!("No CI build yet!");
            continue;
        };

        let AggregatorStateInner {
            build_number,
            build_nodes,
            enable_aggregation,
            ..
        } = current_state;

        if !enable_aggregation {
            info!("Build {build_number} locked, waiting for new testnet start");
            continue;
        }

        let mut build_storage = if let Ok(Some(build_storage)) = storage.get(build_number) {
            build_storage
        } else {
            info!("NO BUILD STORAGE?");
            continue;
        };

        let mut total_request_stats = RequestStats::default();

        // Collect urls based on wether we want to access the nodes directly (only when aggregator is running inside the cluster) or trough the proxy
        let (graphql_urls, tracing_urls, debugger_urls, producer_tracing_urls, seed_url) =
            if environment.use_internal_endpoints {
                (
                    collect_all_urls_cluster_ip(&build_nodes, ComponentType::Graphql),
                    collect_all_urls_cluster_ip(&build_nodes, ComponentType::InternalTracing),
                    collect_all_urls_cluster_ip(&build_nodes, ComponentType::Debugger),
                    collect_producer_urls_cluster_ip(&build_nodes, ComponentType::InternalTracing),
                    get_seed_url_cluster_ip(&build_nodes, ComponentType::Graphql),
                )
            } else {
                (
                    collect_all_urls(environment, ComponentType::Graphql),
                    collect_all_urls(environment, ComponentType::InternalTracing),
                    collect_all_urls(environment, ComponentType::Debugger),
                    collect_producer_urls(environment, ComponentType::InternalTracing),
                    get_seed_url(environment, ComponentType::Graphql),
                )
            };

        info!("Collecting produced blocks...");
        let (mut blocks_on_most_recent_height, producer_trace_timeouts) =
            get_most_recent_produced_blocks(environment, producer_tracing_urls).await;
        info!("Produced blocks collected");

        total_request_stats += producer_trace_timeouts;

        // Catch the case that the block producers have different height for their most recent blocks
        let height = if blocks_on_most_recent_height.is_empty() {
            info!("No blocks yet");
            continue;
        } else {
            let highest = blocks_on_most_recent_height
                .values()
                .map(|v| v.height)
                .max()
                .unwrap_or_default();
            blocks_on_most_recent_height.retain(|_, v| v.height == highest);
            highest
        };

        // let mut block_traces: BTreeMap<String, Vec<BlockTraceAggregatorReport>> = BTreeMap::new();
        // let mut block_traces = AggregatedBlockTraces::default();
        let mut block_traces = build_storage
            .trace_storage
            .get(&height)
            .cloned()
            .unwrap_or_default();

        info!("Height: {height}");

        // collect node info
        info!("Collecting cluster nodes information");
        let (node_infos, node_info_timeouts) = get_node_info_from_cluster(graphql_urls).await;
        // println!("INF: {:#?}", node_infos);
        info!("Information collected");
        total_request_stats += node_info_timeouts;

        // build a map that maps peer_id to tag
        let peer_id_to_tag_map: BTreeMap<String, String> = node_infos
            .iter()
            .map(|(k, v)| {
                (
                    v.daemon_status.addrs_and_ports.peer.peer_id.clone(),
                    k.to_string(),
                )
            })
            .collect();

        let tag_to_block_hash_map: BTreeMap<String, String> = blocks_on_most_recent_height
            .iter()
            .map(|(tag, produced_block)| {
                // let peer_id = tag_to_peer_id_map.get(tag).unwrap();
                (tag.to_string(), produced_block.state_hash.clone())
            })
            .collect();

        // Optimization, only grab traces once for the same block hash
        // blocks_on_most_recent_height.sort_unstable();
        // blocks_on_most_recent_height.dedup();

        for (_, produced_block) in blocks_on_most_recent_height.clone() {
            info!(
                "Collecting node traces for block {}",
                produced_block.state_hash
            );
            let (trace, timeouts) =
                get_block_trace_from_cluster(tracing_urls.clone(), &produced_block.state_hash)
                    .await;
            info!("Traces collected");
            info!("Aggregating trace data");
            // println!("TRACES KEYS: {:#?}", trace.keys());
            match aggregate_block_traces(height, &produced_block.state_hash, &node_infos, trace) {
                Ok(data) => {
                    block_traces.insert(produced_block.state_hash.clone(), data);
                }
                Err(e) => warn!("{}", e),
            }
            total_request_stats += timeouts;
            info!("Trace aggregation finished");
        }

        // TODO: move this to a separate thread?
        info!("Polling debuggers for height {height}");

        let ipc_data = pull_debugger_data_cpnp(Some(height), environment, debugger_urls).await;

        let (_, aggregated_ipc_data) =
            match aggregate_first_receive(ipc_data, &peer_id_to_tag_map, &tag_to_block_hash_map) {
                Ok((height, aggregate_data)) => (height, aggregate_data),
                Err(e) => {
                    warn!("{}", e);
                    (height, Default::default())
                }
            };

        // also do the cross_validation
        let cross_validation_report = cross_validate_ipc_with_traces(
            block_traces.clone(),
            aggregated_ipc_data.clone(),
            height,
        );

        info!("Updating best chain");
        match get_best_chain(&seed_url).await {
            Ok(best_chain) => {
                best_chain.into_iter().for_each(|best_chain_block| {
                    let height = best_chain_block
                        .protocol_state
                        .consensus_state
                        .block_height
                        .parse()
                        .unwrap_or_default();
                    build_storage
                        .best_chain
                        .insert(height, best_chain_block.state_hash);
                });
            }
            Err(e) => {
                warn!("{e}")
            }
        }

        build_storage.update_summary(height, &block_traces, total_request_stats);

        // store aggregated data
        build_storage.store_data(
            height,
            block_traces,
            aggregated_ipc_data,
            cross_validation_report,
        );
        let _ = storage.insert(build_number, build_storage);
    }
}
