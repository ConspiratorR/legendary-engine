use thiserror::Error;

/// Errors that can occur in the scene module.
#[derive(Error, Debug)]
pub enum SceneError {
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Circular dependency detected")]
    CircularDependency,

    #[error("Invalid parent: {0}")]
    InvalidParent(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),
}
