use std::collections::BTreeMap;

use cross_validation::ValidationReport;
use error::AggregatorError;
use storage::{BlockTraceAggregatorStorage, CrossValidationStorage, IpcAggregatorStorage};
use tokio::signal;
use tracing::info;

use aggregators::{BlockHash, BlockTraceAggregatorReport, CpnpBlockPublication};

use crate::{executor::poll_node_traces, storage::LockedBTreeMap};

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

    info!("Creating debugger pulling thread");
    let aggregator_storage = LockedBTreeMap::new();

    // let mut t_ipc_storage = ipc_storage.clone();
    // let mut t_block_trace_storage = block_trace_storage.clone();
    // let mut t_cross_validation_storage = cross_validation_storage.clone();
    let mut t_aggregator_storage = aggregator_storage.clone();
    let t_environment = environment.clone();
    let handle =
        tokio::spawn(
            async move { poll_node_traces(&mut t_aggregator_storage, &t_environment).await },
        );

    info!("Creating rpc server");
    let rpc_server_handle = rpc::spawn_rpc_server(environment.rpc_port, aggregator_storage);

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

    drop(handle);
    drop(rpc_server_handle);
}
