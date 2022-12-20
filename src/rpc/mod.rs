pub mod filters;
pub mod handlers;

use tokio::task::JoinHandle;

use crate::{IpcAggregatorStorage, BlockTraceAggregatorStorage};

pub fn spawn_rpc_server(rpc_port: u16, ipc_storage: IpcAggregatorStorage, block_trace_storage: BlockTraceAggregatorStorage) -> JoinHandle<()> {
    tokio::spawn(async move {
        let api = filters::filters(ipc_storage.clone(), block_trace_storage.clone());

        warp::serve(api).run(([0, 0, 0, 0], rpc_port)).await;
    })
}
