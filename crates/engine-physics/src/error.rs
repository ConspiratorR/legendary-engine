//! Physics simulation error types.

use thiserror::Error;

/// Errors that can occur in the physics simulation.
#[derive(Error, Debug)]
pub enum PhysicsError {
    #[error("invalid rigid body: {0}")]
    InvalidRigidBody(String),

    #[error("invalid collider: {0}")]
    InvalidCollider(String),

    #[error("collision error: {0}")]
    CollisionError(String),

    #[error("solver failed to converge after {0} iterations")]
    SolverConvergence(u32),

    #[error("invalid joint: {0}")]
    InvalidJoint(String),
}
