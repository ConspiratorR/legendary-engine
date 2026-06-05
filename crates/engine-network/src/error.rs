//! Networking error types.

use thiserror::Error;

/// Errors that can occur in the networking layer.
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    #[error("connection timed out")]
    Timeout,

    #[error("disconnected: {0}")]
    Disconnected(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("deserialization error: {0}")]
    Deserialization(String),

    #[error("protocol error: {0}")]
    ProtocolError(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
