use warp::Filter;

use crate::{IpcAggregatorStorage, BlockTraceAggregatorStorage};

use super::handlers::{
    get_aggregated_block_receive_data, get_aggregated_block_receive_data_latest, get_aggregated_block_trace_data_latest, get_aggregated_block_trace_data, get_aggregated_block_trace_data_latest_height, cross_validate_ipc_with_traces_handler,
};

pub fn filters(
    ipc_storage: IpcAggregatorStorage,
    block_trace_storage: BlockTraceAggregatorStorage,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    // Allow cors from any origin
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type"])
        .allow_methods(vec!["GET"]);

    block_receive_aggregation(ipc_storage.clone())
        .or(block_receive_aggregation_latest(ipc_storage.clone()))
        .or(block_traces_aggregation(block_trace_storage.clone()))
        .or(block_traces_aggregation_latest(block_trace_storage.clone()))
        .or(block_traces_aggregation_latest_height(block_trace_storage.clone()))
        .or(cross_validate_ipc_with_traces(block_trace_storage, ipc_storage))
        .with(cors)
}

fn block_receive_aggregation(
    ipc_storage: IpcAggregatorStorage,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("blocks" / usize)
        .and(warp::get())
        .and(with_ipc_storage(ipc_storage))
        .and_then(get_aggregated_block_receive_data)
}

fn block_receive_aggregation_latest(
    ipc_storage: IpcAggregatorStorage,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("blocks" / "latest")
        .and(warp::get())
        .and(with_ipc_storage(ipc_storage))
        .and_then(get_aggregated_block_receive_data_latest)
}

fn block_traces_aggregation_latest(
    block_trace_storage: BlockTraceAggregatorStorage,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("traces" / "latest")
    .and(warp::get())
    .and(with_block_trace_storage(block_trace_storage))
    .and_then(get_aggregated_block_trace_data_latest)
}

fn block_traces_aggregation_latest_height(
    block_trace_storage: BlockTraceAggregatorStorage,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("traces" / "latest" / "height")
    .and(warp::get())
    .and(with_block_trace_storage(block_trace_storage))
    .and_then(get_aggregated_block_trace_data_latest_height)
}

fn block_traces_aggregation(
    block_trace_storage: BlockTraceAggregatorStorage,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("traces" / usize)
    .and(warp::get())
    .and(with_block_trace_storage(block_trace_storage))
    .and_then(get_aggregated_block_trace_data)
}

fn cross_validate_ipc_with_traces(
    block_trace_storage: BlockTraceAggregatorStorage,
    ipc_storage: IpcAggregatorStorage,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("validate" / "ipc" / usize)
    .and(warp::get())
    .and(with_block_trace_storage(block_trace_storage))
    .and(with_ipc_storage(ipc_storage))
    .and_then(cross_validate_ipc_with_traces_handler)
}

fn with_ipc_storage(
    ipc_storage: IpcAggregatorStorage,
) -> impl Filter<Extract = (IpcAggregatorStorage,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || ipc_storage.clone())
}

fn with_block_trace_storage(
    block_trace_storage: BlockTraceAggregatorStorage,
) -> impl Filter<Extract = (BlockTraceAggregatorStorage,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || block_trace_storage.clone())
}