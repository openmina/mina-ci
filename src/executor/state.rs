use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
    time::Duration,
};

use tokio::time::sleep;
use tracing::{error, info, instrument};

use crate::{
    config::AggregatorEnvironment,
    nodes::{collect_all_urls, get_node_info_from_cluster, ComponentType, DaemonStatusDataSlim},
    storage::{AggregatorStorage, BuildInfo, BuildInfoExpanded, BuildStorage, RemoteStorage},
    AggregatorResult,
};

pub type AggregatorState = Arc<RwLock<AggregatorStateInner>>;

const DEPLOY_PIPELINE_NAME: &str = "deploy-to-cluster-custom";
const DEPLOY_STEP_NAME: &str = "deploy-nodes";

#[derive(Debug, Default, Clone)]
pub struct AggregatorStateInner {
    pub build_number: usize,
    pub enable_aggregation: bool,
    pub is_cluster_ready: bool,
    pub build_nodes: BTreeMap<String, DaemonStatusDataSlim>,
    // pub current_height: usize,
}

pub async fn poll_drone(
    state: &AggregatorState,
    environment: &AggregatorEnvironment,
    storage: &mut AggregatorStorage,
    remote_storage: &RemoteStorage,
) {
    loop {
        sleep(environment.data_pull_interval).await;

        match query_latest_build().await {
            Ok(build) => {
                // println!("BUILD: {:#?}", build);
                if let Ok(mut write_locked_state) = state.write() {
                    if write_locked_state.build_number != build.number {
                        // save the storage when detecting new build
                        remote_storage.save_storage(storage);
                        write_locked_state.build_number = build.number;
                        write_locked_state.is_cluster_ready = false;
                        write_locked_state.enable_aggregation = false;

                        // clear out the build nodes data, as the netwokr will be restarted
                        // it will be filled in another thread
                        write_locked_state.build_nodes.clear();

                        let build_storage = BuildStorage {
                            build_info: build.clone(),
                            ..Default::default()
                        };
                        let _ = storage.insert(build.number, build_storage);
                    }
                }

                if let Ok(Some(mut build_storage)) = storage.get(build.number) {
                    build_storage.build_info = build.clone();

                    let _ = storage.insert(build.number, build_storage);
                }

                // TODO: optimize this part
                // TODO: REENABLE THIS!!
                let is_cluster_ready = is_deployment_ready(build.number).await;
                // let is_cluster_ready = true;
                if let Ok(mut write_locked_state) = state.write() {
                    write_locked_state.is_cluster_ready = is_cluster_ready;
                    let is_ready = if environment.use_internal_endpoints {
                        is_cluster_ready && !write_locked_state.build_nodes.is_empty()
                    } else {
                        is_cluster_ready
                    };
                    write_locked_state.enable_aggregation = is_ready;
                }
            }
            Err(_) => {
                // println!("Can't query drone CI: {}", e);
                continue;
            }
        }
    }
}

pub async fn query_latest_build() -> AggregatorResult<BuildInfo> {
    let url = "https://ci.openmina.com/api/repos/openmina/mina/builds?per_page=1";
    let client = reqwest::Client::new();

    let res: Vec<BuildInfo> = client
        .get(url)
        .bearer_auth("26e2399a1f9f336286eabe5e2bb6c2ba")
        .send()
        .await?
        .json()
        .await?;

    Ok(res.get(0).cloned().unwrap_or_default())
}

pub async fn is_deployment_ready(build_number: usize) -> bool {
    match query_deploy_step(build_number).await {
        Ok(build_info) => {
            // if the build has any other status than running, return false
            // NOTE: we don't want to collect data once the tests finished (the testnet is only restarted on the next run)
            if build_info.status != *"running" {
                return false;
            }
            let is_ready = build_info
                .stages
                .iter()
                .find(|s| s.name == DEPLOY_PIPELINE_NAME)
                .and_then(|stage| {
                    stage.steps.clone().and_then(|steps| {
                        steps
                            .iter()
                            .find(|step| step.name == DEPLOY_STEP_NAME)
                            .map(|step| step.status == *"success")
                    })
                });
            // println!("Is ready: {:?}", is_ready);
            is_ready.unwrap_or(false)
        }
        Err(_) => false,
    }
}

pub async fn query_deploy_step(build_number: usize) -> AggregatorResult<BuildInfoExpanded> {
    let url = format!(
        "https://ci.openmina.com/api/repos/openmina/mina/builds/{}",
        build_number
    );
    let client = reqwest::Client::new();

    let res: BuildInfoExpanded = client
        .get(url)
        .bearer_auth("26e2399a1f9f336286eabe5e2bb6c2ba")
        .send()
        .await?
        .json()
        .await?;

    Ok(res)
}

#[instrument(skip(environment, state))]
pub async fn poll_info_from_cluster(environment: &AggregatorEnvironment, state: &AggregatorState) {
    const MAX_RETRIES: usize = 90;
    loop {
        sleep(environment.data_pull_interval).await;

        // execute only when the nodes are empty (emptied by the drone pulling thread)
        match state.read() {
            Ok(read_locked_storage) => {
                if !read_locked_storage.build_nodes.is_empty()
                    || !read_locked_storage.is_cluster_ready
                {
                    continue;
                } else {
                    info!("Nodes redeployed pulling new IPs");
                }
            }
            Err(e) => error!("Failed reading state: {e}"),
        }

        let mut node_status_final: BTreeMap<String, DaemonStatusDataSlim> = BTreeMap::new();
        let mut retries: usize = 0;
        let nodes = collect_all_urls(environment, ComponentType::Graphql);
        while retries < MAX_RETRIES {
            let (nodes, _) = get_node_info_from_cluster(nodes.clone()).await;
            node_status_final.extend(nodes);

            if node_status_final.len() == environment.total_node_count() {
                break;
            }
            retries += 1;
            sleep(Duration::from_secs(2)).await;
        }

        info!(
            "Collected {} nodes out of {}",
            node_status_final.len(),
            environment.total_node_count()
        );

        match state.write() {
            Ok(mut write_locked_state) => {
                write_locked_state.build_nodes = node_status_final;
                info!("Build nodes updated");
            }
            Err(e) => error!("Failed to update build nodes: {e}"),
        }
    }
}
