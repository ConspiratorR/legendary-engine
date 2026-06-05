use std::sync::atomic::{AtomicU64, Ordering};

/// Global atomic counter for generating unique task IDs.
static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(0);

/// Unique identifier for a submitted task.
///
/// IDs are monotonically increasing and globally unique across all
/// thread pool instances within a process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskId {
    /// Create a new unique task identifier.
    pub fn new() -> Self {
        Self(NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// A unit of work to be executed by the thread pool.
///
/// Wraps a `FnOnce() + Send + 'static` closure with a unique [`TaskId`].
pub struct Task {
    /// Unique task identifier.
    pub id: TaskId,
    /// The work closure to execute.
    pub work: Box<dyn FnOnce() + Send + 'static>,
}

impl Task {
    /// Create a new task with the given work closure.
    pub fn new<F: FnOnce() + Send + 'static>(work: F) -> Self {
        Self {
            id: TaskId::new(),
            work: Box::new(work),
        }
    }
}
