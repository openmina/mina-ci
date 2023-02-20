use thiserror::Error;

#[derive(Debug, Error)]
pub enum AggregatorError {
    #[error("Error while accessing storage, reason: {reason}")]
    StorageError { reason: String },

    #[error("Error in RPC communication, reason: {0}")]
    OutgoingRpcError(#[from] reqwest::Error),

    #[error("Producer node not seen yet!")]
    SourceNotReady,

    #[error("Nodes have not produced any traces yet!")]
    NoTracesYet,
}
