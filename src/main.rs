use std::collections::BTreeMap;

use error::AggregatorError;
use tokio::{time::Duration, signal};
use tracing::info;

use aggregator::{BlockHash, CpnpBlockPublication};

use crate::{storage::LockedBTreeMap, debuggers::poll_debuggers};

pub mod debugger_data;
pub mod aggregator;
pub mod storage;
pub mod error;
pub mod debuggers;
pub mod rpc;

// the debugger naming the cluster follows is dbg1, dbg2, ..., dbgN, where N is the number of debuggers running
// full URL for reference http://1.k8.openmina.com:31308/dbg1/block/111
const DEBUGGER_COUNT: usize = 6;
const DEBUGGER_BASE_URL: &str = "http://1.k8.openmina.com:31308/dbg";
const CPNP_URL_COMPONENT: &str = "capnp/block";
const OUTPUT_PATH: &str = "output";

const DEBUGGER_PULL_INTERVAL: Duration = Duration::from_secs(10);

pub type AggregatorStorage = LockedBTreeMap<usize, BTreeMap<BlockHash, CpnpBlockPublication>>;

#[tokio::main]
async fn main() {
    std::fs::remove_dir_all(OUTPUT_PATH).unwrap();
    std::fs::create_dir(OUTPUT_PATH).unwrap();

    tracing_subscriber::fmt::init();

    info!("Creating debugger pulling thread");
    let storage: AggregatorStorage = LockedBTreeMap::new();
    
    let mut t_storage = storage.clone();
    let handle = tokio::spawn(async move {
        poll_debuggers(&mut t_storage).await
    });

    info!("Creating rpc server");
    let rpc_server_handle = rpc::spawn_rpc_server(4000, storage.clone());

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
