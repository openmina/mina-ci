use std::collections::BTreeMap;

use error::AggregatorError;
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

pub type IpcAggregatorStorage = LockedBTreeMap<usize, BTreeMap<BlockHash, CpnpBlockPublication>>;
pub type BlockTraceAggregatorStorage =
    LockedBTreeMap<usize, BTreeMap<BlockHash, Vec<BlockTraceAggregatorReport>>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let environment = config::set_environment();

    info!("Creating debugger pulling thread");
    let ipc_storage: IpcAggregatorStorage = LockedBTreeMap::new();
    let block_trace_storage: BlockTraceAggregatorStorage = LockedBTreeMap::new();

    let mut t_ipc_storage = ipc_storage.clone();
    let mut t_block_trace_storage = block_trace_storage.clone();
    let t_environment = environment.clone();
    let handle = tokio::spawn(async move {
        poll_node_traces(
            &mut t_ipc_storage,
            &mut t_block_trace_storage,
            &t_environment,
        )
        .await
    });

    info!("Creating rpc server");
    let rpc_server_handle = rpc::spawn_rpc_server(
        environment.rpc_port,
        ipc_storage.clone(),
        block_trace_storage.clone(),
    );

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
