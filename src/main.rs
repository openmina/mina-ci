use std::collections::BTreeMap;

use error::AggregatorError;
use tokio::{time::{sleep, Duration}, signal};
use tracing::{info, error, instrument};

use aggregator::{aggregate_first_receive_send, aggregate_first_receive, BlockHash, BlockFirstReceiveSendData, CpnpBlockPublication};
use debugger_data::{DebuggerBlockResponse, DebuggerCpnpResponse};

use crate::{storage::LockedBTreeMap, debuggers::poll_debuggers};

pub mod debugger_data;
pub mod aggregator;
pub mod storage;
pub mod error;
pub mod debuggers;
pub mod rpc;

// the debugger naming the cluster follows is dbg1, dbg2, ..., dbgN, where N is the number of debuggers running
// full URL for reference http://1.k8.openmina.com:31308/dbg1/block/111
const DEBUGGER_COUNT: usize = 7;
const DEBUGGER_BASE_URL: &str = "http://1.k8.openmina.com:31308/dbg";
const BLOCK_URL_COMPONENT: &str = "block";
const CPNP_URL_COMPONENT: &str = "capnp/block";
const OUTPUT_PATH: &str = "output";

const DEBUGGER_PULL_INTERVAL: Duration = Duration::from_secs(10);

pub type AggregatorStorage = LockedBTreeMap<usize, BTreeMap<BlockHash, CpnpBlockPublication>>;

#[tokio::main]
async fn main() {
    std::fs::remove_dir_all(OUTPUT_PATH).unwrap();
    std::fs::create_dir(OUTPUT_PATH).unwrap();

    tracing_subscriber::fmt::init();

    // let collected_data = pull_debugger_data_cpnp(5);
    // aggregate_first_receive(collected_data);

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

fn get_height_data(height: usize, debugger_label: usize) -> DebuggerBlockResponse {
    let url = format!("{}{}/{}/{}", DEBUGGER_BASE_URL, debugger_label, BLOCK_URL_COMPONENT, height);
    reqwest::blocking::get(url).unwrap().json().unwrap()
}

// fn get_height_data_cpnp(height: usize, debugger_label: usize) -> DebuggerCpnpResponse {
//     let url = format!("{}{}/{}/{}", DEBUGGER_BASE_URL, debugger_label, CPNP_URL_COMPONENT, height);
//     reqwest::blocking::get(url).unwrap().json().unwrap()
// }

fn pull_debugger_data(height: usize) -> Vec<DebuggerBlockResponse> {
    let mut collected: Vec<DebuggerBlockResponse> = vec![];
    for debugger_label in 1..=DEBUGGER_COUNT {
        let data = get_height_data(height, debugger_label);
        collected.push(data);
    }

    // println!("{:?}", collected);
    collected
}

// fn pull_debugger_data_cpnp(height: usize) -> Vec<DebuggerCpnpResponse> {
//     let mut collected: Vec<DebuggerCpnpResponse> = vec![];

//     for debugger_label in 1..=DEBUGGER_COUNT {
//         let data = get_height_data_cpnp(height, debugger_label);
//         collected.push(data);
//     }

//     collected
// }

