use reqwest::StatusCode;

use crate::{AggregatorStorage, aggregator::{CpnpBlockPublication, CpnpBlockPublicationFlattened}};


pub async fn get_aggregated_block_receive_data(height: usize, storage: AggregatorStorage) -> Result<impl warp::Reply, warp::reject::Rejection> {
    if let Ok(Some(data)) = storage.get(height) {
        let res: Vec<CpnpBlockPublicationFlattened> = data.values().cloned().into_iter().map(|p| p.into()).collect();
        Ok(warp::reply::with_status(warp::reply::json(&res), StatusCode::OK))
    } else {
        Ok(warp::reply::with_status(warp::reply::json(&Vec::<CpnpBlockPublication>::new()), StatusCode::OK))
    }
}

pub async fn get_aggregated_block_receive_data_latest(storage: AggregatorStorage) -> Result<impl warp::Reply, warp::reject::Rejection> {
    if let Ok(Some(data)) = storage.get_latest() {
        let res: Vec<CpnpBlockPublicationFlattened> = data.values().cloned().into_iter().map(|p| p.into()).collect();
        Ok(warp::reply::with_status(warp::reply::json(&res), StatusCode::OK))
    } else {
        Ok(warp::reply::with_status(warp::reply::json(&Vec::<CpnpBlockPublication>::new()), StatusCode::OK))
    }
}