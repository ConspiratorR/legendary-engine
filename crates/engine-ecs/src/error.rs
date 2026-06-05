use thiserror::Error;

/// Errors that can occur in the ECS module.
#[derive(Error, Debug)]
pub enum EcsError {
    #[error("Entity not found: {0:?}")]
    EntityNotFound(crate::entity::Entity),

    #[error("Component not registered: {0}")]
    ComponentNotRegistered(String),

    #[error("Duplicate component")]
    DuplicateComponent,

    #[error("World is locked for modification")]
    WorldLocked,

    #[error("Invalid archetype")]
    InvalidArchetype,

    #[error("Query error: {0}")]
    QueryError(String),
}
