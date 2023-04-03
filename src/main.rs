use error::AggregatorError;
use tokio::signal;
use tracing::info;

use crate::{
    executor::{
        poll_node_traces,
        state::{poll_drone, poll_info_from_cluster},
    },
    storage::RemoteStorage,
};

pub mod aggregators;
pub mod config;
mod cross_validation;
pub mod debugger_data;
pub mod error;
pub mod executor;
pub mod nodes;
pub mod rpc;
pub mod storage;

pub type AggregatorResult<T> = Result<T, AggregatorError>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let environment = config::set_environment();
    let remote_storage = RemoteStorage::new(
        &environment.remote_storage_url,
        &environment.remote_storage_user,
        &environment.remote_storage_password,
        &environment.remote_storage_path,
    );

    info!("Starting aggregator with configuration: {environment}");

    let (state, aggregator_storage) = remote_storage.load_storage();

    let (node_info_handle, drone_handle, aggregator_handle) = if !environment.disable_aggregation {
        let node_info_handle = if environment.use_internal_endpoints {
            info!("Creating ip retrieval thread");
            let t_state = state.clone();
            let t_environment = environment.clone();

            let handle =
                tokio::spawn(async move { poll_info_from_cluster(&t_environment, &t_state).await });

            Some(handle)
        } else {
            None
        };

        let mut t_aggregator_storage = aggregator_storage.clone();
        let t_state = state.clone();
        let t_environment = environment.clone();
        let t_remote_storage = remote_storage.clone();
        info!("Creating drone polling thread");
        let drone_handle = tokio::spawn(async move {
            poll_drone(
                &t_state,
                &t_environment,
                &mut t_aggregator_storage,
                &t_remote_storage,
            )
            .await
        });

        let mut t_aggregator_storage = aggregator_storage.clone();
        let t_environment = environment.clone();

        info!("Creating aggregator thread");
        let aggregator_handle = tokio::spawn(async move {
            poll_node_traces(&state, &mut t_aggregator_storage, &t_environment).await
        });
        (
            node_info_handle,
            Some(drone_handle),
            Some(aggregator_handle),
        )
    } else {
        info!("Aggregation dissabled! Only serving data");
        (None, None, None)
    };

    info!("Creating rpc server");
    let t_aggregator_storage = aggregator_storage.clone();
    let rpc_server_handle = rpc::spawn_rpc_server(environment.rpc_port, t_aggregator_storage);

    let mut signal_stream =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Unable to handle SIGTERM");

    tokio::select! {
        s = signal::ctrl_c() => {
            s.expect("Failed to listen for ctrl-c event");
            info!("Ctrl-c or SIGINT received!");
        }
        _ = signal_stream.recv() => {
            info!("SIGTERM received!");
        }
    }

    info!("Dumping storage to remote");
    // remote_storage.upload_storage(&data).unwrap();
    remote_storage.save_storage(&aggregator_storage);

    info!("Destroying threads");
    drop(node_info_handle);
    drop(drone_handle);
    drop(aggregator_handle);
    drop(rpc_server_handle);

    info!("Shutdown successfull!");
}
