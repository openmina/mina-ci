use reqwest::StatusCode;

use crate::{AggregatorStorage, aggregator::CpnpBlockPublication};


pub async fn get_aggregated_block_receive_data(height: usize, storage: AggregatorStorage) -> Result<impl warp::Reply, warp::reject::Rejection> {
    if let Ok(Some(data)) = storage.get(height) {
        let res: Vec<CpnpBlockPublication> = data.values().cloned().into_iter().collect();
        Ok(warp::reply::with_status(warp::reply::json(&res), StatusCode::OK))
    } else {
        Ok(warp::reply::with_status(warp::reply::json(&Vec::<CpnpBlockPublication>::new()), StatusCode::OK))
    }
}