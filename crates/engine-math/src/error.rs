use thiserror::Error;

/// Errors that can occur in the math module.
#[derive(Error, Debug)]
pub enum MathError {
    #[error("Invalid vector length: {0}")]
    InvalidLength(usize),

    #[error("Division by zero")]
    DivisionByZero,

    #[error("Invalid quaternion: {reason}")]
    InvalidQuaternion { reason: String },

    #[error("Matrix is not invertible")]
    NotInvertible,
}
