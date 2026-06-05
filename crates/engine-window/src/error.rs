use thiserror::Error;

/// Errors that can occur in the window module.
///
/// Covers creation failures, invalid configurations, and platform-specific errors.
/// All variants implement [`Display`](std::fmt::Display) and [`Debug`](std::fmt::Debug)
/// via thiserror.
#[derive(Error, Debug)]
pub enum WindowError {
    /// Window creation failed at the platform level.
    ///
    /// This typically means the OS rejected the request (e.g. no display
    /// server available, resource exhaustion, or unsupported attributes).
    #[error("Failed to create window: {reason}")]
    CreationFailed { reason: String },

    /// The requested window was not found.
    ///
    /// Returned when looking up a window by ID or reference that no
    /// longer exists (e.g. after the user closed it).
    #[error("Window not found")]
    NotFound,

    /// The specified window dimensions are invalid (e.g. zero size).
    ///
    /// Both width and height must be greater than zero.
    #[error("Invalid window size: {width}x{height}")]
    InvalidSize { width: u32, height: u32 },

    /// A platform-specific error occurred.
    ///
    /// Wraps errors from the underlying windowing system that don't
    /// fit into the other categories.
    #[error("Platform error: {0}")]
    Platform(String),
}
