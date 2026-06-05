//! # engine-jobs
//!
//! Task scheduling and parallel execution for the RustEngine.
//!
//! Provides a [`ThreadPool`] with work-stealing scheduling,
//! a [`JobGraph`] for dependency-driven parallel execution,
//! and utilities for parallelizing engine workloads.
//!
//! ## Quick Start
//!
//! ```rust
//! use engine_jobs::ThreadPool;
//!
//! let pool = ThreadPool::new(4);
//! pool.submit_and_join(vec![
//!     || println!("Task 1"),
//!     || println!("Task 2"),
//!     || println!("Task 3"),
//! ]);
//! ```

pub mod error;
pub mod graph;
pub mod pool;
pub mod task;

pub use error::JobsError;
pub use graph::{JobGraph, JobHandle};
pub use pool::ThreadPool;
pub use task::{Task, TaskId};
