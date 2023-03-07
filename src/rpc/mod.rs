pub mod filters;
pub mod handlers;

use tokio::task::JoinHandle;

use crate::{
    storage::AggregatorStorage, BlockTraceAggregatorStorage, CrossValidationStorage,
    IpcAggregatorStorage,
};

pub fn spawn_rpc_server(rpc_port: u16, storage: AggregatorStorage) -> JoinHandle<()> {
    tokio::spawn(async move {
        let api = filters::filters(storage.clone());

        warp::serve(api).run(([0, 0, 0, 0], rpc_port)).await;
    })
}
