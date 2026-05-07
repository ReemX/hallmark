//! Domain error types. Each module owns a thiserror enum below.
//! Application code uses `anyhow::Result`; library boundaries return these typed errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PathDiscoveryError {
    #[error("registry read failed: {0}")]
    Registry(#[from] std::io::Error),
    #[error("VDF parse failed at {path}: {message}")]
    Vdf {
        path: std::path::PathBuf,
        message: String,
    },
    #[error("path does not exist: {0}")]
    NotFound(std::path::PathBuf),
}

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json parse: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid path layout: expected <root>/<appid>/achievements.json, got {0}")]
    InvalidLayout(std::path::PathBuf),
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("system time: {0}")]
    SystemTime(#[from] std::time::SystemTimeError),
}
