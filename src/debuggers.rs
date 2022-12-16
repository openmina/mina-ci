use tokio::time::sleep;
use tracing::{error, info, instrument, warn};

use crate::{
    aggregator::{aggregate_first_receive, AggregatorResult},
    config::AggregatorEnvironment,
    debugger_data::DebuggerCpnpResponse,
    AggregatorStorage,
};

#[instrument]
async fn pull_debugger_data_cpnp(
    height: Option<usize>,
    environment: &AggregatorEnvironment,
) -> AggregatorResult<Vec<DebuggerCpnpResponse>> {
    let mut collected: Vec<DebuggerCpnpResponse> = vec![];

    for debugger_label in 1..=environment.debugger_count {
        let data = get_height_data_cpnp(height, debugger_label, environment).await?;
        collected.push(data);
        info!("Pulling dbg{}", debugger_label);
    }

    Ok(collected)
}

async fn get_height_data_cpnp(
    height: Option<usize>,
    debugger_label: usize,
    environment: &AggregatorEnvironment,
) -> AggregatorResult<DebuggerCpnpResponse> {
    let url = if let Some(height) = height {
        format!(
            "{}{}/{}/{}",
            environment.debugger_base_url, debugger_label, environment.libp2p_ipc_encpoint, height
        )
    } else {
        format!(
            "{}{}/{}/{}",
            environment.debugger_base_url,
            debugger_label,
            environment.libp2p_ipc_encpoint,
            "latest"
        )
    };
    reqwest::get(url).await?.json().await.map_err(|e| e.into())
}

pub async fn poll_debuggers(storage: &mut AggregatorStorage, environment: &AggregatorEnvironment) {
    sleep(environment.data_pull_interval).await;
    loop {
        match pull_debugger_data_cpnp(None, environment).await {
            Ok(data) => {
                let (height, aggregated_data) = match aggregate_first_receive(data) {
                    Ok((height, aggregate_data)) => (height, aggregate_data),
                    Err(e) => {
                        warn!("{}", e);
                        continue;
                    }
                };

                // TODO: this is not a very good idea, capture the error!
                let _ = storage.insert(height, aggregated_data);
            }
            Err(e) => error!("Error in pulling data: {}", e),
        }
        info!("Sleeping");
        sleep(environment.data_pull_interval).await;
    }
}
