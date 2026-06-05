use thiserror::Error;

/// Errors that can occur in the window module.
#[derive(Error, Debug)]
pub enum WindowError {
    #[error("Failed to create window: {reason}")]
    CreationFailed { reason: String },

    #[error("Window not found")]
    NotFound,

    #[error("Invalid window size: {width}x{height}")]
    InvalidSize { width: u32, height: u32 },

    #[error("Platform error: {0}")]
    Platform(String),
}
