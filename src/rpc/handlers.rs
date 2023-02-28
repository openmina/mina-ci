use std::collections::BTreeMap;

use crate::{
    aggregators::{
        BlockTraceAggregatorReport, CpnpBlockPublication, CpnpBlockPublicationFlattened,
    },
    cross_validation::{cross_validate_ipc_with_traces, ValidationReport},
    BlockTraceAggregatorStorage, IpcAggregatorStorage,
};
use reqwest::StatusCode;

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
    block_trace_storage: BlockTraceAggregatorStorage,
    ipc_storage: IpcAggregatorStorage,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let traces = if let Ok(Some(data)) = block_trace_storage.get(height) {
        data
    } else {
        return Ok(warp::reply::with_status(
            warp::reply::json(&BTreeMap::<String, ValidationReport>::new()),
            StatusCode::OK,
        ));
    };

    let ipc_data = if let Ok(Some(data)) = ipc_storage.get(height) {
        data
    } else {
        return Ok(warp::reply::with_status(
            warp::reply::json(&BTreeMap::<String, ValidationReport>::new()),
            StatusCode::OK,
        ));
    };

    let res = cross_validate_ipc_with_traces(traces, ipc_data);

    Ok(warp::reply::with_status(
        warp::reply::json(&res),
        StatusCode::OK,
    ))
}

// fn empty_json<T: Serialize + Default>(res: T) -> impl warp::Reply {
//     warp::reply::with_status(
//         warp::reply::json(&T::default()),
//         StatusCode::OK,
//     )
// }
