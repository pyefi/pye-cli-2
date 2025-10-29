use thiserror::Error;

#[derive(Debug, Error)]
pub enum PyeCliError {
    #[error("ReqwestError: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("SerdeJsonError: {0}")]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("AuthFailed: {0}")]
    AuthFailed(String),
}
