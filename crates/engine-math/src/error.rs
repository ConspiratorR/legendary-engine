use thiserror::Error;

/// Errors that can occur in the math module.
///
/// Each variant represents a distinct class of math error that callers
/// can match on to provide domain-specific recovery logic.
#[derive(Error, Debug)]
pub enum MathError {
    /// A vector or slice has an unexpected length (e.g., constructing a
    /// matrix from a flat slice with the wrong element count).
    #[error("Invalid vector length: {0}")]
    InvalidLength(usize),

    /// An operation attempted to divide by zero (e.g., normalizing a
    /// zero-length vector with a checked API).
    #[error("Division by zero")]
    DivisionByZero,

    /// A quaternion fails a validity check (e.g., NaN components or
    /// non-unit quaternion where one was required).
    #[error("Invalid quaternion: {reason}")]
    InvalidQuaternion { reason: String },

    /// A matrix inversion was attempted on a singular (non-invertible)
    /// matrix, such as a scale matrix with a zero scale factor.
    #[error("Matrix is not invertible")]
    NotInvertible,
}
