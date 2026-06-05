//! Framework error types for state machine and game flow.

use thiserror::Error;

/// Errors that can occur in the game framework layer.
#[derive(Error, Debug)]
pub enum FrameworkError {
    #[error("state not found: {0}")]
    StateNotFound(String),

    #[error("state stack is empty")]
    StackEmpty,

    #[error("invalid transition from '{from}' to '{to}'")]
    InvalidTransition { from: String, to: String },

    #[error("save/load error: {0}")]
    SaveLoadError(String),
}
