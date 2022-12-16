use warp::Filter;

use crate::AggregatorStorage;

use super::handlers::{
    get_aggregated_block_receive_data, get_aggregated_block_receive_data_latest,
};

pub fn filters(
    storage: AggregatorStorage,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    // Allow cors from any origin
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type"])
        .allow_methods(vec!["GET"]);

    block_receive_aggregation(storage.clone())
        .or(block_receive_aggregation_latest(storage))
        .with(cors)
}

fn block_receive_aggregation(
    storage: AggregatorStorage,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("blocks" / usize)
        .and(warp::get())
        .and(with_storage(storage))
        .and_then(get_aggregated_block_receive_data)
}

fn block_receive_aggregation_latest(
    storage: AggregatorStorage,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("blocks" / "latest")
        .and(warp::get())
        .and(with_storage(storage))
        .and_then(get_aggregated_block_receive_data_latest)
}

fn with_storage(
    storage: AggregatorStorage,
) -> impl Filter<Extract = (AggregatorStorage,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || storage.clone())
}
