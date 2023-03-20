use std::collections::BTreeMap;

use itertools::Itertools;
use tokio::time::sleep;
use tracing::{error, info, instrument, warn};

use crate::{
    aggregators::{aggregate_block_traces, aggregate_first_receive, AggregatedBlockTraces},
    config::AggregatorEnvironment,
    cross_validation::cross_validate_ipc_with_traces,
    debugger_data::{CpnpCapturedData, DebuggerCpnpResponse},
    nodes::{
        collect_all_urls, get_block_trace_from_cluster, get_most_recent_produced_blocks,
        get_node_info_from_cluster, ComponentType, DaemonStatusDataSlim,
    },
    storage::AggregatorStorage,
    AggregatorResult,
};

use self::state::AggregatorState;

pub mod state;

#[instrument]
async fn pull_debugger_data_cpnp(
    height: Option<usize>,
    environment: &AggregatorEnvironment,
    node_infos: &BTreeMap<String, DaemonStatusDataSlim>,
) -> AggregatorResult<Vec<DebuggerCpnpResponse>> {
    let mut collected: Vec<DebuggerCpnpResponse> = vec![];

    let nodes = collect_all_urls(environment, ComponentType::Debugger);

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

    Ok(collected)
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
    reqwest::get(url).await?.json().await.map_err(|e| e.into())
}

pub async fn poll_node_traces(
    state: &AggregatorState,
    storage: &mut AggregatorStorage,
    environment: &AggregatorEnvironment,
) {
    loop {
        info!("Sleeping");
        sleep(environment.data_pull_interval).await;

        let build_number = if let Ok(read_state) = state.read() {
            if !read_state.enable_aggregation {
                info!(
                    "Build {} locked, waiting for testnet start",
                    read_state.build_number
                );
                continue;
            }
            read_state.build_number
        } else {
            info!("No CI build yet!");
            continue;
        };

        let mut build_storage = if let Ok(Some(build_storage)) = storage.get(build_number) {
            build_storage
        } else {
            info!("NO BUILD STORAGE?");
            continue;
        };

        info!("Collecting produced blocks...");
        let mut blocks_on_most_recent_height = get_most_recent_produced_blocks(environment).await;
        info!("Produced blocks collected");

        if blocks_on_most_recent_height.is_empty() {
            info!("No blocks yet");
            continue;
        }

        // Catch the case that the block producers have different height for their most recent blocks
        if blocks_on_most_recent_height.len() > 1
            && !blocks_on_most_recent_height
                .windows(2)
                .all(|w| w[0].0 == w[1].0)
        {
            info!("Height missmatch on producers! Using highest block_height");
            // With this check we can eliminate the scenraio when a block producer lags behind, retaining only the highest block_height
            let highest = blocks_on_most_recent_height.iter().max().unwrap().0;
            blocks_on_most_recent_height.retain(|(height, _, _)| height == &highest);
        }

        let height = blocks_on_most_recent_height[0].0;
        // let mut block_traces: BTreeMap<String, Vec<BlockTraceAggregatorReport>> = BTreeMap::new();
        let mut block_traces = AggregatedBlockTraces::default();

        info!("Height: {height}");

        // collect node info
        info!("Collecting cluster nodes information");
        let node_infos = get_node_info_from_cluster(environment).await;
        // println!("INF: {:#?}", node_infos);
        info!("Information collected");

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
            .map(|(_, state_hash, tag)| {
                // let peer_id = tag_to_peer_id_map.get(tag).unwrap();
                (tag.to_string(), state_hash.to_string())
            })
            .collect();

        // Optimization, only grab traces once for the same block hash
        // blocks_on_most_recent_height.sort_unstable();
        // blocks_on_most_recent_height.dedup();

        for (_, state_hash, _) in blocks_on_most_recent_height.clone() {
            info!("Collecting node traces for block {state_hash}");
            let trace = get_block_trace_from_cluster(environment, &state_hash).await;
            info!("Traces collected for block {state_hash}");
            info!("Aggregating trace data and traces for block {state_hash}");
            // println!("TRACES KEYS: {:#?}", trace.keys());
            match aggregate_block_traces(height, &state_hash, &node_infos, trace) {
                Ok(data) => {
                    block_traces.insert(state_hash.clone(), data);
                }
                Err(e) => warn!("{}", e),
            }
            info!("Trace aggregation finished for block {state_hash}");
        }

        // TODO: move this to a separate thread?
        info!("Polling debuggers for height {height}");

        match pull_debugger_data_cpnp(Some(height), environment, &node_infos).await {
            Ok(data) => {
                let (height, aggregated_data) = match aggregate_first_receive(
                    data,
                    &peer_id_to_tag_map,
                    &tag_to_block_hash_map,
                ) {
                    Ok((height, aggregate_data)) => (height, aggregate_data),
                    Err(e) => {
                        warn!("{}", e);
                        continue;
                    }
                };

                // also do the cross_validation
                let report = cross_validate_ipc_with_traces(
                    block_traces.clone(),
                    aggregated_data.clone(),
                    height,
                );
                // TODO!

                let _ = build_storage
                    .trace_storage
                    .insert(height, block_traces.clone());
                let _ = build_storage
                    .ipc_storage
                    .insert(height, aggregated_data.clone());
                let _ = build_storage
                    .cross_validation_storage
                    .insert(height, report);
            }
            Err(e) => error!("Error in pulling data: {}", e),
        }

        // per block summaries
        build_storage.block_summaries = block_traces.block_summaries(height);

        let tx_count = block_traces.transaction_count();

        build_storage
            .helpers
            .tx_count_per_height
            .insert(height, tx_count);
        // TODO: rework!
        build_storage.build_summary.tx_count =
            build_storage.helpers.tx_count_per_height.values().sum();

        let application_times: Vec<f64> = block_traces.application_times();

        // TODO: get from producer_traces
        let production_times: Vec<f64> = block_traces.production_times();

        let receive_latencies: Vec<f64> = block_traces.receive_latencies();

        let application_time_sum: f64 = application_times.iter().sum();
        let application_min = f64_min(&application_times);
        let application_max = f64_max(&application_times);

        let production_time_sum: f64 = production_times.iter().sum();
        let production_min = f64_min(&production_times);
        let production_max = f64_max(&production_times);

        let receive_latencies_sum: f64 = receive_latencies.iter().sum();
        let receive_latencies_min = f64_min(&receive_latencies);
        let receive_latencies_max = f64_max(&receive_latencies);

        let unique_block_count = blocks_on_most_recent_height
            .iter()
            .map(|(_, hash, _)| hash)
            .unique()
            .count();
        let application_measurement_count: usize = block_traces.appliaction_count();
        let production_measurement_count: usize = block_traces.production_count();

        // store the aggregated values
        build_storage
            .helpers
            .application_times
            .insert(height, application_times);
        build_storage
            .helpers
            .production_times
            .insert(height, production_times);
        build_storage
            .helpers
            .receive_latencies
            .insert(height, receive_latencies);

        build_storage
            .helpers
            .application_avg_total_count
            .insert(height, application_measurement_count);
        build_storage
            .helpers
            .application_total
            .insert(height, application_time_sum);
        build_storage
            .helpers
            .production_avg_total_count
            .insert(height, production_measurement_count);
        build_storage
            .helpers
            .production_total
            .insert(height, production_time_sum);
        // the count is same as the application ones (optimization: remove this helper map and use the application map?)
        build_storage
            .helpers
            .receive_latencies_avg_total_count
            .insert(height, application_measurement_count);
        build_storage
            .helpers
            .receive_latencies_total
            .insert(height, receive_latencies_sum);

        build_storage
            .helpers
            .block_count_per_height
            .insert(height, unique_block_count);
        build_storage.build_summary.block_count =
            build_storage.helpers.block_count_per_height.values().sum();
        build_storage.build_summary.cannonical_block_count =
            build_storage.helpers.application_total.len();

        if build_storage.build_summary.block_application_min == 0.0 {
            build_storage.build_summary.block_application_min = application_min;
        } else {
            build_storage.build_summary.block_application_min =
                application_min.min(build_storage.build_summary.block_application_min);
        }
        build_storage.build_summary.block_application_max =
            application_max.max(build_storage.build_summary.block_application_max);
        build_storage.build_summary.block_application_avg =
            build_storage.helpers.get_application_average();

        if build_storage.build_summary.block_production_min == 0.0 {
            build_storage.build_summary.block_production_min = production_min;
        } else {
            build_storage.build_summary.block_production_min =
                production_min.min(build_storage.build_summary.block_production_min);
        }
        build_storage.build_summary.block_production_max =
            production_max.max(build_storage.build_summary.block_production_max);
        build_storage.build_summary.block_production_avg =
            build_storage.helpers.get_production_average();

        if build_storage.build_summary.receive_latency_min == 0.0 {
            build_storage.build_summary.receive_latency_min = receive_latencies_min;
        } else {
            build_storage.build_summary.receive_latency_min =
                production_min.min(build_storage.build_summary.receive_latency_min);
        }
        build_storage.build_summary.receive_latency_max =
            receive_latencies_max.max(build_storage.build_summary.receive_latency_max);
        build_storage.build_summary.receive_latency_avg =
            build_storage.helpers.get_latencies_average();

        let _ = storage.insert(build_number, build_storage);
    }
}

fn f64_min(values: &[f64]) -> f64 {
    values
        .iter()
        .copied()
        .reduce(|a, b| a.min(b))
        .unwrap_or(f64::MAX)
}

fn f64_max(values: &[f64]) -> f64 {
    values
        .iter()
        .copied()
        .reduce(|a, b| a.max(b))
        .unwrap_or(f64::MIN)
}
