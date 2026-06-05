//! Asset error types.

use thiserror::Error;

/// Errors that can occur in the asset module.
#[derive(Error, Debug)]
pub enum AssetError {
    /// The requested asset file was not found.
    #[error("Asset not found: {path}")]
    NotFound { path: String },

    /// An asset failed to load.
    #[error("Failed to load asset: {path}, reason: {reason}")]
    LoadFailed { path: String, reason: String },

    /// The asset type is not supported.
    #[error("Unsupported asset type: {0}")]
    UnsupportedType(String),

    /// An I/O error occurred.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// A serialization/deserialization error occurred.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// The asset handle is invalid (e.g., the asset was dropped).
    #[error("Invalid asset handle")]
    InvalidHandle,
}
