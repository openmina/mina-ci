use warp::Filter;

use crate::{BlockTraceAggregatorStorage, CrossValidationStorage, IpcAggregatorStorage};

use super::handlers::{
    aggregate_cross_validations_handler, cross_validate_ipc_with_traces_handler,
    get_aggregated_block_receive_data, get_aggregated_block_receive_data_latest,
    get_aggregated_block_trace_data, get_aggregated_block_trace_data_latest,
    get_aggregated_block_trace_data_latest_height, QueryOptions, get_cross_validations_count_handler,
};

pub fn filters(
    ipc_storage: IpcAggregatorStorage,
    block_trace_storage: BlockTraceAggregatorStorage,
    cross_validation_storage: CrossValidationStorage,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    // Allow cors from any origin
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type"])
        .allow_methods(vec!["GET"]);

    block_receive_aggregation(ipc_storage.clone())
        .or(block_receive_aggregation_latest(ipc_storage))
        .or(block_traces_aggregation(block_trace_storage.clone()))
        .or(block_traces_aggregation_latest(block_trace_storage.clone()))
        .or(block_traces_aggregation_latest_height(block_trace_storage))
        .or(cross_validate_ipc_with_traces(
            cross_validation_storage.clone(),
        ))
        .or(cross_validation_counts(cross_validation_storage.clone()))
        .or(aggregate_cross_validations_filter(cross_validation_storage))
        .with(cors)
}

fn block_receive_aggregation(
    ipc_storage: IpcAggregatorStorage,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("blocks" / usize)
        .and(warp::get())
        .and(with_ipc_storage(ipc_storage))
        .and_then(get_aggregated_block_receive_data)
}

fn block_receive_aggregation_latest(
    ipc_storage: IpcAggregatorStorage,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("blocks" / "latest")
        .and(warp::get())
        .and(with_ipc_storage(ipc_storage))
        .and_then(get_aggregated_block_receive_data_latest)
}

fn block_traces_aggregation_latest(
    block_trace_storage: BlockTraceAggregatorStorage,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("traces" / "latest")
        .and(warp::get())
        .and(with_block_trace_storage(block_trace_storage))
        .and_then(get_aggregated_block_trace_data_latest)
}

fn block_traces_aggregation_latest_height(
    block_trace_storage: BlockTraceAggregatorStorage,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("traces" / "latest" / "height")
        .and(warp::get())
        .and(with_block_trace_storage(block_trace_storage))
        .and_then(get_aggregated_block_trace_data_latest_height)
}

fn block_traces_aggregation(
    block_trace_storage: BlockTraceAggregatorStorage,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("traces" / usize)
        .and(warp::get())
        .and(with_block_trace_storage(block_trace_storage))
        .and_then(get_aggregated_block_trace_data)
}

fn cross_validate_ipc_with_traces(
    cross_validation_storage: CrossValidationStorage,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("validate" / "ipc" / usize)
        .and(warp::get())
        .and(with_cross_validation_storage(cross_validation_storage))
        .and_then(cross_validate_ipc_with_traces_handler)
}

fn aggregate_cross_validations_filter(
    cross_validation_storage: CrossValidationStorage,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("validate" / "ipc")
        .and(warp::get())
        .and(warp::query::<QueryOptions>())
        .and(with_cross_validation_storage(cross_validation_storage))
        .and_then(aggregate_cross_validations_handler)
}

fn cross_validation_counts(
    cross_validation_storage: CrossValidationStorage,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("validate" / "ipc" / "count")
        .and(warp::get())
        .and(with_cross_validation_storage(cross_validation_storage))
        .and_then(get_cross_validations_count_handler)
}

fn with_ipc_storage(
    ipc_storage: IpcAggregatorStorage,
) -> impl Filter<Extract = (IpcAggregatorStorage,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || ipc_storage.clone())
}

fn with_block_trace_storage(
    block_trace_storage: BlockTraceAggregatorStorage,
) -> impl Filter<Extract = (BlockTraceAggregatorStorage,), Error = std::convert::Infallible> + Clone
{
    warp::any().map(move || block_trace_storage.clone())
}

fn with_cross_validation_storage(
    cross_validation_storage: CrossValidationStorage,
) -> impl Filter<Extract = (CrossValidationStorage,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || cross_validation_storage.clone())
}

// cross_validation_storage
