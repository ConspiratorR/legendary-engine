use thiserror::Error;

/// Errors that can occur in the window module.
///
/// Covers creation failures, invalid configurations, and platform-specific errors.
#[derive(Error, Debug)]
pub enum WindowError {
    /// Window creation failed at the platform level.
    #[error("Failed to create window: {reason}")]
    CreationFailed { reason: String },

    /// The requested window was not found.
    #[error("Window not found")]
    NotFound,

    /// The specified window dimensions are invalid (e.g. zero size).
    #[error("Invalid window size: {width}x{height}")]
    InvalidSize { width: u32, height: u32 },

    /// A platform-specific error occurred.
    #[error("Platform error: {0}")]
    Platform(String),
}
