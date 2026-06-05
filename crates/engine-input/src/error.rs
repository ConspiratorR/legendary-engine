use thiserror::Error;

/// Errors that can occur in the input module.
#[derive(Error, Debug)]
pub enum InputError {
    #[error("Action not found: {0}")]
    ActionNotFound(String),

    #[error("Invalid binding: {0}")]
    InvalidBinding(String),

    #[error("Duplicate action: {0}")]
    DuplicateAction(String),
}
