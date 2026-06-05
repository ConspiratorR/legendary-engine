use thiserror::Error;

/// Errors that can occur in the jobs/tasks module.
#[derive(Error, Debug)]
pub enum JobsError {
    #[error("Task pool shutdown")]
    Shutdown,

    #[error("Task panicked: {0}")]
    TaskPanicked(String),

    #[error("Invalid thread count: {0}")]
    InvalidThreadCount(usize),

    #[error("Task timeout after {0}ms")]
    Timeout(u64),
}
