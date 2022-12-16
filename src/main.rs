use std::collections::BTreeMap;

use error::AggregatorError;
use tokio::{signal, time::Duration};
use tracing::info;

use aggregator::{BlockHash, CpnpBlockPublication};

use crate::{debuggers::poll_debuggers, storage::LockedBTreeMap};

pub mod aggregator;
pub mod config;
pub mod debugger_data;
pub mod debuggers;
pub mod error;
pub mod rpc;
pub mod storage;

pub type AggregatorStorage = LockedBTreeMap<usize, BTreeMap<BlockHash, CpnpBlockPublication>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let environment = config::set_environment();

    info!("Creating debugger pulling thread");
    let storage: AggregatorStorage = LockedBTreeMap::new();

    let mut t_storage = storage.clone();
    let t_environment = environment.clone();
    let handle = tokio::spawn(async move { poll_debuggers(&mut t_storage, &t_environment).await });

    info!("Creating rpc server");
    let rpc_server_handle = rpc::spawn_rpc_server(environment.rpc_port, storage.clone());

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
