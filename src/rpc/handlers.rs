use reqwest::StatusCode;

use crate::{
    IpcAggregatorStorage, aggregators::{CpnpBlockPublicationFlattened, CpnpBlockPublication, BlockTraceAggregatorReport}, BlockTraceAggregatorStorage,
};

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
        let res: Vec<BlockTraceAggregatorReport> = data
            .values()
            .cloned()
            .into_iter()
            .flatten()
            .collect();
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
        let res: Vec<BlockTraceAggregatorReport> = data
            .values()
            .cloned()
            .into_iter()
            .flatten()
            .collect();
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
