use tokio::time::sleep;
use tracing::{instrument, info, error, warn};

use crate::{aggregator::{AggregatorResult, aggregate_first_receive}, debugger_data::DebuggerCpnpResponse, DEBUGGER_COUNT, DEBUGGER_BASE_URL, CPNP_URL_COMPONENT, AggregatorStorage, DEBUGGER_PULL_INTERVAL};

#[instrument]
async fn pull_debugger_data_cpnp(height: Option<usize>) -> AggregatorResult<Vec<DebuggerCpnpResponse>> {
    let mut collected: Vec<DebuggerCpnpResponse> = vec![];

    for debugger_label in 1..=DEBUGGER_COUNT {
        let data = get_height_data_cpnp(height, debugger_label).await?;
        collected.push(data);
        info!("Pulling dbg{}", debugger_label);
    }

    Ok(collected)
}

async fn get_height_data_cpnp(height: Option<usize>, debugger_label: usize) -> AggregatorResult<DebuggerCpnpResponse> {
    let url = if let Some(height) = height {
        format!("{}{}/{}/{}", DEBUGGER_BASE_URL, debugger_label, CPNP_URL_COMPONENT, height)
    } else {
        format!("{}{}/{}/{}", DEBUGGER_BASE_URL, debugger_label, CPNP_URL_COMPONENT, "latest")
    };
    reqwest::get(url).await?.json().await.map_err(|e| e.into())
}

// #[instrument]
pub async fn poll_debuggers(storage: &mut AggregatorStorage) {
    sleep(DEBUGGER_PULL_INTERVAL).await;
    loop {
        // if let Err(e) = pull_debugger_data_cpnp(None).await {
        //     error!("Error in pulling data: {}", e)
        // }
        match pull_debugger_data_cpnp(None).await {
            Ok(data) => {
                let (height, aggregated_data) = match aggregate_first_receive(data) {
                    Ok((height, aggregate_data)) => (height, aggregate_data),
                    Err(e) => {
                        warn!("{}", e);
                        continue;
                    },
                };

                // TODO: this is not a very good idea, capture the error!
                let _ = storage.insert(height, aggregated_data);
            },
            Err(e) => error!("Error in pulling data: {}", e),
        }
        info!("Sleeping");
        sleep(DEBUGGER_PULL_INTERVAL).await;
    }
}