//! Work-stealing job system for parallel task execution.
//!
//! Provides a [`ThreadPool`] with work-stealing scheduling,
//! a [`JobGraph`] for dependency-driven parallel execution,
//! and utilities for parallelizing engine workloads.

pub mod error;
pub mod graph;
pub mod pool;
pub mod task;

pub use error::JobsError;
pub use graph::{JobGraph, JobHandle};
pub use pool::ThreadPool;
pub use task::{Task, TaskId};
