use std::collections::BTreeMap;

use crate::{
    aggregators::{
        BlockTraceAggregatorReport, CpnpBlockPublication, CpnpBlockPublicationFlattened,
    },
    cross_validation::{aggregate_cross_validations, AggregateValidationReport, ValidationReport},
    BlockTraceAggregatorStorage, CrossValidationStorage, IpcAggregatorStorage,
};
use reqwest::StatusCode;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct QueryOptions {
    count: Option<usize>,
}

pub async fn get_aggregated_block_receive_data(
    height: usize,
    ipc_storage: IpcAggregatorStorage,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    if let Ok(Some(data)) = ipc_storage.get(height) {
        let res: Vec<CpnpBlockPublicationFlattened> = data
            .values()
            .cloned()
            .into_iter()
            .map(|p| p.into())
            .collect();
        Ok(warp::reply::with_status(
            warp::reply::json(&res),
            StatusCode::OK,
        ))
    } else {
        Ok(warp::reply::with_status(
            warp::reply::json(&Vec::<CpnpBlockPublication>::new()),
            StatusCode::OK,
        ))
    }
}

pub async fn get_aggregated_block_receive_data_latest(
    ipc_storage: IpcAggregatorStorage,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    if let Ok(Some(data)) = ipc_storage.get_latest_value() {
        let res: Vec<CpnpBlockPublicationFlattened> = data
            .values()
            .cloned()
            .into_iter()
            .map(|p| p.into())
            .collect();
        Ok(warp::reply::with_status(
            warp::reply::json(&res),
            StatusCode::OK,
        ))
    } else {
        Ok(warp::reply::with_status(
            warp::reply::json(&Vec::<CpnpBlockPublication>::new()),
            StatusCode::OK,
        ))
    }
}

pub async fn get_aggregated_block_trace_data(
    height: usize,
    block_trace_storage: BlockTraceAggregatorStorage,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    if let Ok(Some(data)) = block_trace_storage.get(height) {
        let res: Vec<BlockTraceAggregatorReport> =
            data.values().cloned().into_iter().flatten().collect();
        Ok(warp::reply::with_status(
            warp::reply::json(&res),
            StatusCode::OK,
        ))
    } else {
        Ok(warp::reply::with_status(
            warp::reply::json(&Vec::<BlockTraceAggregatorReport>::new()),
            StatusCode::OK,
        ))
    }
}

pub async fn get_aggregated_block_trace_data_latest(
    block_trace_storage: BlockTraceAggregatorStorage,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    if let Ok(Some(data)) = block_trace_storage.get_latest_value() {
        let res: Vec<BlockTraceAggregatorReport> =
            data.values().cloned().into_iter().flatten().collect();
        Ok(warp::reply::with_status(
            warp::reply::json(&res),
            StatusCode::OK,
        ))
    } else {
        Ok(warp::reply::with_status(
            warp::reply::json(&Vec::<BlockTraceAggregatorReport>::new()),
            StatusCode::OK,
        ))
    }
}

pub async fn get_aggregated_block_trace_data_latest_height(
    block_trace_storage: BlockTraceAggregatorStorage,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    if let Ok(Some(data)) = block_trace_storage.get_latest_key() {
        Ok(warp::reply::with_status(
            warp::reply::json(&data),
            StatusCode::OK,
        ))
    } else {
        Ok(warp::reply::with_status(
            warp::reply::json(&0),
            StatusCode::OK,
        ))
    }
}

pub async fn cross_validate_ipc_with_traces_handler(
    height: usize,
    cross_validation_storage: CrossValidationStorage,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let res = if let Ok(Some(data)) = cross_validation_storage.get(height) {
        data
    } else {
        return Ok(warp::reply::with_status(
            warp::reply::json(&BTreeMap::<String, ValidationReport>::new()),
            StatusCode::OK,
        ));
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&res),
        StatusCode::OK,
    ))
}

pub async fn aggregate_cross_validations_handler(
    options: QueryOptions,
    cross_validation_storage: CrossValidationStorage,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let count = options.count.unwrap_or(10);

    let n_validations = if let Ok(data) = cross_validation_storage.get_latest_n_values(count) {
        data
    } else {
        return Ok(warp::reply::with_status(
            warp::reply::json(&AggregateValidationReport::default()),
            StatusCode::OK,
        ));
    };

    let res = aggregate_cross_validations(n_validations);

    Ok(warp::reply::with_status(
        warp::reply::json(&res),
        StatusCode::OK,
    ))
}

pub async fn get_cross_validations_count_handler(
    cross_validation_storage: CrossValidationStorage,

) -> Result<impl warp::Reply, warp::reject::Rejection> {
    if let Ok(data) = cross_validation_storage.get_count() {
        Ok(warp::reply::with_status(
            warp::reply::json(&data),
            StatusCode::OK,
        ))
    } else {
        Ok(warp::reply::with_status(
            warp::reply::json(&0),
            StatusCode::OK,
        ))
    }
}

// fn empty_json<T: Serialize + Default>(res: T) -> impl warp::Reply {
//     warp::reply::with_status(
//         warp::reply::json(&T::default()),
//         StatusCode::OK,
//     )
// }
