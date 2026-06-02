use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(0);

/// Unique identifier for a submitted task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskId {
    pub fn new() -> Self {
        Self(NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// A unit of work to be executed by the thread pool.
pub struct Task {
    pub id: TaskId,
    pub work: Box<dyn FnOnce() + Send + 'static>,
}

impl Task {
    pub fn new<F: FnOnce() + Send + 'static>(work: F) -> Self {
        Self {
            id: TaskId::new(),
            work: Box::new(work),
        }
    }
}
