use std::sync::{Arc, RwLock};

use tokio::time::sleep;

use crate::{
    config::AggregatorEnvironment,
    storage::{AggregatorStorage, BuildInfo, BuildStorage},
    AggregatorResult,
};

pub type AggregatorState = Arc<RwLock<AggregatorStateInner>>;

#[derive(Debug, Default, Clone)]
pub struct AggregatorStateInner {
    pub build_number: usize,
    pub enable_aggregation: bool,
    // pub current_height: usize,
}

pub async fn poll_drone(
    state: &AggregatorState,
    environment: &AggregatorEnvironment,
    storage: &mut AggregatorStorage,
) {
    loop {
        sleep(environment.data_pull_interval).await;

        match query_latest_build().await {
            Ok(build) => {
                // println!("BUILD: {:#?}", build);
                if let Ok(mut write_locked_state) = state.write() {
                    if write_locked_state.build_number != build.number {
                        write_locked_state.build_number = build.number;

                        let build_storage = BuildStorage {
                            build_info: build.clone(),
                            ..Default::default()
                        };
                        let _ = storage.insert(build.number, build_storage);
                    }
                }
            }
            Err(e) => {
                println!("Can't query drone CI: {}", e);
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
