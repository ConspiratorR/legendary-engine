use thiserror::Error;

/// Errors that can occur in the asset module.
#[derive(Error, Debug)]
pub enum AssetError {
    #[error("Asset not found: {path}")]
    NotFound { path: String },

    #[error("Failed to load asset: {path}, reason: {reason}")]
    LoadFailed { path: String, reason: String },

    #[error("Unsupported asset type: {0}")]
    UnsupportedType(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Invalid asset handle")]
    InvalidHandle,
}
