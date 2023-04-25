use warp::Filter;

use crate::storage::AggregatorStorage;

use super::handlers::{
    aggregate_cross_validations_handler, cross_validate_ipc_with_traces_handler,
    get_aggregated_block_receive_data, get_aggregated_block_receive_data_latest,
    get_aggregated_block_trace_data, get_aggregated_block_trace_data_latest,
    get_aggregated_block_trace_data_latest_height, get_block_summaries, get_build_summaries,
    get_build_summary, get_cross_validations_count_handler, BuildsQueryOptions, QueryOptions,
};

pub fn filters(
    storage: AggregatorStorage,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    // Allow cors from any origin
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type"])
        .allow_methods(vec!["GET"]);

    block_receive_aggregation(storage.clone())
        .or(block_receive_aggregation_latest(storage.clone()))
        .or(block_traces_aggregation(storage.clone()))
        .or(block_traces_aggregation_latest(storage.clone()))
        .or(block_traces_aggregation_latest_height(storage.clone()))
        .or(cross_validate_ipc_with_traces(storage.clone()))
        .or(cross_validation_counts(storage.clone()))
        .or(aggregate_cross_validations_filter(storage.clone()))
        .or(build_summaries(storage.clone()))
        .or(build_summary(storage.clone()))
        .or(block_summaries(storage))
        .with(cors)
}

fn build_summaries(
    storage: AggregatorStorage,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("builds")
        .and(warp::get())
        .and(warp::query::<BuildsQueryOptions>())
        .and(with_storage(storage))
        .and_then(get_build_summaries)
}

fn build_summary(
    storage: AggregatorStorage,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("builds" / usize)
        .and(warp::get())
        .and(warp::query::<BuildsQueryOptions>())
        .and(with_storage(storage))
        .and_then(get_build_summary)
}

fn block_summaries(
    storage: AggregatorStorage,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("builds" / usize / "blocks")
        .and(warp::get())
        .and(with_storage(storage))
        .and_then(get_block_summaries)
}

fn block_receive_aggregation(
    storage: AggregatorStorage,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("builds" / usize / "ipc_blocks" / usize)
        .and(warp::get())
        .and(with_storage(storage))
        .and_then(get_aggregated_block_receive_data)
}

fn block_receive_aggregation_latest(
    storage: AggregatorStorage,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("builds" / usize / "ipc_blocks" / "latest")
        .and(warp::get())
        .and(with_storage(storage))
        .and_then(get_aggregated_block_receive_data_latest)
}

fn block_traces_aggregation_latest(
    storage: AggregatorStorage,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("builds" / usize / "traces" / "latest")
        .and(warp::get())
        .and(with_storage(storage))
        .and_then(get_aggregated_block_trace_data_latest)
}

fn block_traces_aggregation_latest_height(
    storage: AggregatorStorage,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("builds" / usize / "traces" / "latest" / "height")
        .and(warp::get())
        .and(with_storage(storage))
        .and_then(get_aggregated_block_trace_data_latest_height)
}

fn block_traces_aggregation(
    storage: AggregatorStorage,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("builds" / usize / "traces" / usize)
        .and(warp::get())
        .and(with_storage(storage))
        .and_then(get_aggregated_block_trace_data)
}

fn cross_validate_ipc_with_traces(
    storage: AggregatorStorage,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("builds" / usize / "validate" / "ipc" / usize)
        .and(warp::get())
        .and(with_storage(storage))
        .and_then(cross_validate_ipc_with_traces_handler)
}

fn aggregate_cross_validations_filter(
    storage: AggregatorStorage,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("builds" / usize / "validate" / "ipc")
        .and(warp::get())
        .and(warp::query::<QueryOptions>())
        .and(with_storage(storage))
        .and_then(aggregate_cross_validations_handler)
}

fn cross_validation_counts(
    storage: AggregatorStorage,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("builds" / usize / "validate" / "ipc" / "count")
        .and(warp::get())
        .and(with_storage(storage))
        .and_then(get_cross_validations_count_handler)
}

fn with_storage(
    storage: AggregatorStorage,
) -> impl Filter<Extract = (AggregatorStorage,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || storage.clone())
}

// fn with_ipc_storage(
//     ipc_storage: IpcAggregatorStorage,
// ) -> impl Filter<Extract = (IpcAggregatorStorage,), Error = std::convert::Infallible> + Clone {
//     warp::any().map(move || ipc_storage.clone())
// }

// fn with_block_trace_storage(
//     block_trace_storage: BlockTraceAggregatorStorage,
// ) -> impl Filter<Extract = (BlockTraceAggregatorStorage,), Error = std::convert::Infallible> + Clone
// {
//     warp::any().map(move || block_trace_storage.clone())
// }

// fn with_cross_validation_storage(
//     cross_validation_storage: CrossValidationStorage,
// ) -> impl Filter<Extract = (CrossValidationStorage,), Error = std::convert::Infallible> + Clone {
//     warp::any().map(move || cross_validation_storage.clone())
// }

// cross_validation_storage
