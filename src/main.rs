use error::AggregatorError;
use executor::state::AggregatorStateInner;
use std::{
    collections::BTreeMap,
    io::BufReader,
    sync::{Arc, RwLock},
};
use storage::{AggregatorStorage, BuildStorage, BuildStorageDump};
use tokio::signal;
use tracing::{info, log::warn};

use crate::{
    executor::{
        poll_node_traces,
        state::{poll_drone, AggregatorState},
    },
    storage::LockedBTreeMap,
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

    info!("Creating debugger pulling thread");
    // let aggregator_storage = LockedBTreeMap::new();
    let (state, aggregator_storage) = load_storage("storage.json");

    // let state = AggregatorState::default();

    let mut t_aggregator_storage = aggregator_storage.clone();
    let t_state = state.clone();
    let t_environment = environment.clone();
    let drone_handle = tokio::spawn(async move {
        poll_drone(&t_state, &t_environment, &mut t_aggregator_storage).await
    });

    let mut t_aggregator_storage = aggregator_storage.clone();
    let t_environment = environment.clone();
    let aggregator_handle = tokio::spawn(async move {
        poll_node_traces(&state, &mut t_aggregator_storage, &t_environment).await
    });

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

    info!("Dumping storage to file");
    // TODO: move to fn
    {
        let raw: BTreeMap<usize, BuildStorageDump> = aggregator_storage
            .to_btreemap()
            .expect("Cannot convert to btreemap")
            .into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect();
        // let raw_string = serde_json::to_string(&raw).expect("Cannot convert to json string");
        let file = std::fs::File::create("storage.json").expect("Failed to create file");
        serde_json::to_writer(file, &raw).expect("Failed to write storage to file");
    }

    drop(drone_handle);
    drop(aggregator_handle);
    drop(rpc_server_handle);
}

fn load_storage(path: &str) -> (AggregatorState, AggregatorStorage) {
    match std::fs::File::open(path) {
        Ok(file) => {
            match serde_json::from_reader::<_, BTreeMap<usize, BuildStorageDump>>(BufReader::new(
                file,
            )) {
                Ok(storage) => {
                    // println!("{:#?}", storage);
                    let storage = storage
                        .iter()
                        .map(|(k, v)| (*k, v.clone().into()))
                        .collect::<BTreeMap<usize, BuildStorage>>();
                    // println!("{:#?}", storage);
                    let state_inner = AggregatorStateInner {
                        build_number: storage
                            .last_key_value()
                            .map(|(k, _)| *k)
                            .unwrap_or_default(),
                        ..Default::default()
                    };
                    let state = Arc::new(RwLock::new(state_inner));
                    (state, LockedBTreeMap::new(storage))
                }
                Err(e) => {
                    warn!("{e}");
                    warn!("Failed to deserialize storage json, creating new storage");
                    (AggregatorState::default(), LockedBTreeMap::default())
                }
            }
        }
        Err(_) => {
            warn!("Failed to open storage file at {path}, creating new storage");
            (AggregatorState::default(), LockedBTreeMap::default())
        }
    }
}
