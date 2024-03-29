use std::collections::BTreeMap;

use crate::{
    aggregators::{
        BlockTraceAggregatorReport, CpnpBlockPublication, CpnpBlockPublicationFlattened,
    },
    cross_validation::{aggregate_cross_validations, ValidationReport},
    storage::{AggregatorStorage, BlockSummary, BuildStorage},
};
use itertools::Itertools;
use reqwest::StatusCode;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct QueryOptions {
    count: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct BuildsQueryOptions {
    status: Option<String>,
    compare_to: Option<usize>,
}

impl BuildsQueryOptions {
    // fn status_filters(&self) -> Vec<String> {
    //     match &self.status {
    //         Some(s) => s.split(',').map(|s| s.to_string()).collect(),
    //         None => vec![],
    //     }
    // }
    fn status_filters(&self) -> Option<Vec<String>> {
        self.status
            .as_ref()
            .map(|s| s.split(',').map(|s| s.to_string()).collect())
    }
}

pub async fn get_aggregated_block_receive_data(
    build_number: usize,
    height: usize,
    storage: AggregatorStorage,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let ipc_storage = if let Ok(Some(read_storage)) = storage.get(build_number) {
        read_storage.ipc_storage
    } else {
        return Ok(warp::reply::with_status(
            warp::reply::json(&Vec::<CpnpBlockPublication>::new()),
            StatusCode::OK,
        ));
    };

    if let Some(data) = ipc_storage.get(&height) {
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
    build_number: usize,
    storage: AggregatorStorage,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let ipc_storage = if let Ok(Some(read_storage)) = storage.get(build_number) {
        read_storage.ipc_storage
    } else {
        return Ok(warp::reply::with_status(
            warp::reply::json(&Vec::<CpnpBlockPublication>::new()),
            StatusCode::OK,
        ));
    };

    if let Some((_, data)) = ipc_storage.last_key_value() {
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
    build_number: usize,
    height: usize,
    storage: AggregatorStorage,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let block_trace_storage = if let Ok(Some(read_storage)) = storage.get(build_number) {
        read_storage.trace_storage
    } else {
        return Ok(warp::reply::with_status(
            warp::reply::json(&Vec::<BlockTraceAggregatorReport>::new()),
            StatusCode::OK,
        ));
    };

    if let Some(data) = block_trace_storage.get(&height) {
        let res: Vec<BlockTraceAggregatorReport> = data
            .inner()
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
    build_number: usize,
    storage: AggregatorStorage,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let block_trace_storage = if let Ok(Some(read_storage)) = storage.get(build_number) {
        read_storage.trace_storage
    } else {
        return Ok(warp::reply::with_status(
            warp::reply::json(&Vec::<BlockTraceAggregatorReport>::new()),
            StatusCode::OK,
        ));
    };

    if let Some((_, data)) = block_trace_storage.last_key_value() {
        let res: Vec<BlockTraceAggregatorReport> = data
            .inner()
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
    build_number: usize,
    storage: AggregatorStorage,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let block_trace_storage = if let Ok(Some(read_storage)) = storage.get(build_number) {
        read_storage.trace_storage
    } else {
        return Ok(warp::reply::with_status(
            warp::reply::json(&Vec::<BlockTraceAggregatorReport>::new()),
            StatusCode::OK,
        ));
    };

    if let Some((_, data)) = block_trace_storage.last_key_value() {
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
    build_number: usize,
    height: usize,
    storage: AggregatorStorage,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let cross_validation_storage = if let Ok(Some(read_storage)) = storage.get(build_number) {
        read_storage.cross_validation_storage
    } else {
        return Ok(warp::reply::with_status(
            warp::reply::json(&BTreeMap::<String, ValidationReport>::new()),
            StatusCode::OK,
        ));
    };

    let res = if let Some(data) = cross_validation_storage.get(&height) {
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
    build_number: usize,
    options: QueryOptions,
    storage: AggregatorStorage,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let count = options.count.unwrap_or(10);

    let cross_validation_storage = if let Ok(Some(read_storage)) = storage.get(build_number) {
        read_storage.cross_validation_storage
    } else {
        return Ok(warp::reply::with_status(
            warp::reply::json(&BTreeMap::<String, ValidationReport>::new()),
            StatusCode::OK,
        ));
    };

    let n_validations = cross_validation_storage
        .values()
        .cloned()
        .rev()
        .take(count)
        .collect();

    let res = aggregate_cross_validations(n_validations);

    Ok(warp::reply::with_status(
        warp::reply::json(&res),
        StatusCode::OK,
    ))
}

pub async fn get_cross_validations_count_handler(
    build_number: usize,
    storage: AggregatorStorage,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let cross_validation_storage = if let Ok(Some(read_storage)) = storage.get(build_number) {
        read_storage.cross_validation_storage
    } else {
        return Ok(warp::reply::with_status(
            warp::reply::json(&BTreeMap::<String, ValidationReport>::new()),
            StatusCode::OK,
        ));
    };
    Ok(warp::reply::with_status(
        warp::reply::json(&cross_validation_storage.len()),
        StatusCode::OK,
    ))
}

pub async fn get_build_summaries(
    options: BuildsQueryOptions,
    storage: AggregatorStorage,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    match storage.get_values() {
        Ok(values) => {
            let final_res = match options.status_filters() {
                Some(filter) => {
                    let res: Vec<BuildStorage> = values
                        .into_iter()
                        .rev()
                        .filter(|build| filter.contains(&build.build_info.status))
                        .map(|mut build| {
                            build.include_times();
                            build
                        })
                        .collect();
                    let last_value = res.last().cloned().unwrap_or_default();
                    // calculate deltas
                    let mut final_res: Vec<BuildStorage> = res
                        .into_iter()
                        .tuple_windows::<(BuildStorage, BuildStorage)>()
                        .map(|(mut w0, w1)| {
                            w0.calculate_deltas(&w1);
                            w0
                        })
                        .collect();
                    // add the last value as is (nothing to comapre to)
                    final_res.push(last_value);
                    final_res
                }
                // Without status filter, return the two most recent successfull builds
                None => {
                    let res: Vec<BuildStorage> = values
                        .into_iter()
                        .rev()
                        .filter(|build| build.build_info.status == "success")
                        .take(2)
                        .map(|mut build| {
                            build.include_times();
                            build
                        })
                        .collect();
                    let last_value = res.last().cloned().unwrap_or_default();
                    // calculate deltas
                    let mut final_res: Vec<BuildStorage> = res
                        .into_iter()
                        .tuple_windows::<(BuildStorage, BuildStorage)>()
                        .map(|(mut w0, w1)| {
                            w0.calculate_deltas(&w1);
                            w0
                        })
                        .collect();
                    // add the last value as is (nothing to comapre to)
                    final_res.push(last_value);
                    final_res
                }
            };

            Ok(warp::reply::with_status(
                warp::reply::json(&final_res),
                StatusCode::OK,
            ))
        }
        _ => Ok(warp::reply::with_status(
            warp::reply::json(&Vec::<BuildStorage>::new()),
            StatusCode::OK,
        )),
    }
}

pub async fn get_build_summary(
    build_num: usize,
    options: BuildsQueryOptions,
    storage: AggregatorStorage,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    match options.compare_to {
        Some(compare_to) => match (storage.get(build_num), storage.get(compare_to)) {
            (Ok(Some(mut build)), Ok(Some(build_to_compare_to))) => {
                build.include_times();
                build.calculate_deltas(&build_to_compare_to);
                Ok(warp::reply::with_status(
                    warp::reply::json(&vec![build, build_to_compare_to]),
                    StatusCode::OK,
                ))
            }
            _ => Ok(warp::reply::with_status(
                warp::reply::json(&Vec::<BuildStorage>::new()),
                StatusCode::OK,
            )),
        },
        None => match storage.get(build_num) {
            Ok(Some(mut build)) => {
                build.include_times();
                Ok(warp::reply::with_status(
                    warp::reply::json(&vec![build]),
                    StatusCode::OK,
                ))
            }
            _ => Ok(warp::reply::with_status(
                warp::reply::json(&Vec::<BuildStorage>::new()),
                StatusCode::OK,
            )),
        },
    }
}

pub async fn get_block_summaries(
    build_num: usize,
    storage: AggregatorStorage,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    match storage.get(build_num) {
        Ok(Some(build)) => Ok(warp::reply::with_status(
            warp::reply::json(&build.block_summaries.values().collect::<Vec<_>>()),
            StatusCode::OK,
        )),
        _ => Ok(warp::reply::with_status(
            warp::reply::json(&Vec::<BlockSummary>::new()),
            StatusCode::OK,
        )),
    }
}
// fn empty_json<T: Serialize + Default>(res: T) -> impl warp::Reply {
//     warp::reply::with_status(
//         warp::reply::json(&T::default()),
//         StatusCode::OK,
//     )
// }
