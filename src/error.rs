use std::num::TryFromIntError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PyeCliError {
    #[error("ReqwestError: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("SerdeJsonError: {0}")]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("TryFromIntError: {0}")]
    TryFromIntError(#[from] TryFromIntError),
    #[error("SolanaClientError: {0}")]
    SolanaClientError(#[from] solana_rpc_client_api::client_error::Error),
    #[error("DialoguerError: {0}")]
    DialoguerError(#[from] dialoguer::Error),
    #[error("AuthFailed: {0}")]
    AuthFailed(String),
    #[error("ReadKeypairError: {0}")]
    ReadKeypairError(String),
}
