//! Unified error types for the script bridge layer.

use thiserror::Error;

/// Errors that can occur during script bridge operations.
#[derive(Error, Debug)]
pub enum BridgeError {
    /// A Lua runtime error.
    #[error("Lua error: {0}")]
    Lua(#[from] mlua::Error),

    /// A WASM runtime error.
    #[error("WASM error: {0}")]
    Wasm(#[from] anyhow::Error),

    /// A type conversion failed between Rust and script values.
    #[error("type conversion failed: expected {expected}, got {actual}")]
    TypeMismatch {
        expected: &'static str,
        actual: &'static str,
    },

    /// A requested type is not registered in the type registry.
    #[error("type not registered: {0}")]
    TypeNotRegistered(String),

    /// A callback with the given name was not found.
    #[error("callback not found: {0}")]
    CallbackNotFound(String),

    /// An event channel with the given name was not found.
    #[error("event channel not found: {0}")]
    EventChannelNotFound(String),

    /// An entity does not exist or has been despawned.
    #[error("entity not found: index {0}")]
    EntityNotFound(u32),

    /// A component does not exist on the given entity.
    #[error("component '{component}' not found on entity {entity}")]
    ComponentNotFound { entity: u32, component: String },

    /// A buffer overflow occurred during data transfer.
    #[error("buffer overflow: needed {needed} bytes, only {available} available")]
    BufferOverflow { needed: usize, available: usize },
}

/// Result type alias for bridge operations.
pub type BridgeResult<T> = Result<T, BridgeError>;
