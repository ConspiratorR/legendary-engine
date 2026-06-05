use thiserror::Error;

/// Errors that can occur in the jobs/tasks module.
#[derive(Error, Debug)]
pub enum JobsError {
    /// The thread pool has been shut down and cannot accept new tasks.
    #[error("Task pool shutdown")]
    Shutdown,

    /// A task panicked during execution. Contains the panic message.
    #[error("Task panicked: {0}")]
    TaskPanicked(String),

    /// An invalid thread count was specified (e.g., in a context where 0 is not allowed).
    #[error("Invalid thread count: {0}")]
    InvalidThreadCount(usize),

    /// A task exceeded its allotted execution time.
    #[error("Task timeout after {0}ms")]
    Timeout(u64),
}
