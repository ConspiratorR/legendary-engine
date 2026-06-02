use crate::pool::ThreadPool;
use crate::task::Task;
use crossbeam_deque::Injector;
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A handle to a job submitted to a [`JobGraph`].
///
/// Can be used as a dependency for other jobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobHandle(pub(crate) usize);

/// A directed acyclic graph of jobs with dependency tracking.
///
/// Jobs are submitted with explicit dependencies. The graph executes
/// independent jobs in parallel via the provided [`ThreadPool`],
/// respecting dependency ordering.
///
/// # Example
///
/// ```
/// use engine_jobs::{ThreadPool, JobGraph};
///
/// let pool = ThreadPool::new(4);
/// let mut graph = JobGraph::new();
///
/// let a = graph.add(|| println!("A"));
/// let b = graph.add(|| println!("B"));
/// let c = graph.add_after(&[a, b], || println!("C depends on A+B"));
///
/// graph.execute(&pool);
/// ```
pub struct JobGraph {
    jobs: Vec<JobNode>,
}

struct JobNode {
    work: JobFn,
    deps: Vec<usize>,
}

impl Default for JobGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl JobGraph {
    /// Create an empty job graph.
    pub fn new() -> Self {
        Self { jobs: Vec::new() }
    }

    /// Add a job with no dependencies. Returns a [`JobHandle`].
    pub fn add<F: FnOnce() + Send + 'static>(&mut self, work: F) -> JobHandle {
        let idx = self.jobs.len();
        self.jobs.push(JobNode {
            work: Box::new(work),
            deps: Vec::new(),
        });
        JobHandle(idx)
    }

    /// Add a job that depends on the given handles.
    ///
    /// The job will not execute until all its dependencies have completed.
    pub fn add_after<F: FnOnce() + Send + 'static>(
        &mut self,
        deps: &[JobHandle],
        work: F,
    ) -> JobHandle {
        let idx = self.jobs.len();
        self.jobs.push(JobNode {
            work: Box::new(work),
            deps: deps.iter().map(|h| h.0).collect(),
        });
        JobHandle(idx)
    }

    /// Return the number of jobs in the graph.
    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    /// Return true if the graph has no jobs.
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    /// Execute all jobs in dependency order, running independent jobs in parallel.
    ///
    /// Blocks until all jobs have completed.
    pub fn execute(self, pool: &ThreadPool) {
        let n = self.jobs.len();
        if n == 0 {
            return;
        }

        // Build reverse adjacency: for each job, which jobs depend on it
        let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut in_degree = vec![0u32; n];

        for (i, node) in self.jobs.iter().enumerate() {
            in_degree[i] = node.deps.len() as u32;
            for &dep in &node.deps {
                dependents[dep].push(i);
            }
        }

        // Separate work from metadata
        let jobs: Vec<Option<JobFn>> = self.jobs.into_iter().map(|n| Some(n.work)).collect();

        let state = Arc::new(GraphState {
            jobs: Mutex::new(jobs),
            dependents,
            in_degree: Mutex::new(in_degree),
            completed: AtomicUsize::new(0),
            done_signal: parking_lot::Condvar::new(),
        });

        // Seed: submit all jobs with in_degree == 0
        let ready: Vec<usize> = {
            let degrees = state.in_degree.lock();
            (0..n).filter(|&i| degrees[i] == 0).collect()
        };

        for idx in ready {
            Self::submit_to_pool(idx, pool, &state);
        }

        // Wait for all jobs to complete
        let mut lock = state.in_degree.lock();
        while state.completed.load(Ordering::Relaxed) < n {
            state.done_signal.wait(&mut lock);
        }
    }

    fn submit_to_pool(idx: usize, pool: &ThreadPool, state: &Arc<GraphState>) {
        let state = Arc::clone(state);
        let injector = Arc::clone(pool.injector());
        let injector_for_closure = Arc::clone(&injector);

        injector.push(Task::new(move || {
            // Take and execute the job
            let work = {
                let mut jobs = state.jobs.lock();
                jobs[idx].take()
            };
            if let Some(work) = work {
                work();
            }

            // Signal completion
            state.completed.fetch_add(1, Ordering::Relaxed);
            state.done_signal.notify_all();

            // Decrement in-degree of dependents and submit newly-ready jobs
            let newly_ready: Vec<usize> = {
                let mut degrees = state.in_degree.lock();
                let mut ready = Vec::new();
                for &dep_idx in state.dependents[idx].iter() {
                    degrees[dep_idx] -= 1;
                    if degrees[dep_idx] == 0 {
                        ready.push(dep_idx);
                    }
                }
                ready
            };

            // Submit newly ready dependents back to the pool
            for dep_idx in newly_ready {
                Self::submit_to_pool_inner(dep_idx, &injector_for_closure, &state);
            }
        }));
    }

    fn submit_to_pool_inner(idx: usize, injector: &Arc<Injector<Task>>, state: &Arc<GraphState>) {
        let state = Arc::clone(state);
        let injector_for_closure = Arc::clone(injector);

        injector.push(Task::new(move || {
            let work = {
                let mut jobs = state.jobs.lock();
                jobs[idx].take()
            };
            if let Some(work) = work {
                work();
            }

            state.completed.fetch_add(1, Ordering::Relaxed);
            state.done_signal.notify_all();

            let newly_ready: Vec<usize> = {
                let mut degrees = state.in_degree.lock();
                let mut ready = Vec::new();
                for &dep_idx in state.dependents[idx].iter() {
                    degrees[dep_idx] -= 1;
                    if degrees[dep_idx] == 0 {
                        ready.push(dep_idx);
                    }
                }
                ready
            };

            for dep_idx in newly_ready {
                Self::submit_to_pool_inner(dep_idx, &injector_for_closure, &state);
            }
        }));
    }
}

/// A boxed, sendable job function.
type JobFn = Box<dyn FnOnce() + Send + 'static>;

struct GraphState {
    jobs: Mutex<Vec<Option<JobFn>>>,
    dependents: Vec<Vec<usize>>,
    in_degree: Mutex<Vec<u32>>,
    completed: AtomicUsize,
    done_signal: parking_lot::Condvar,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;

    #[test]
    fn test_empty_graph() {
        let pool = ThreadPool::new(2);
        let graph = JobGraph::new();
        graph.execute(&pool);
    }

    #[test]
    fn test_single_job() {
        let pool = ThreadPool::new(2);
        let mut graph = JobGraph::new();
        let counter = Arc::new(AtomicU64::new(0));
        let c = Arc::clone(&counter);
        graph.add(move || {
            c.fetch_add(1, Ordering::Relaxed);
        });
        graph.execute(&pool);
        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_independent_jobs_parallel() {
        let pool = ThreadPool::new(4);
        let mut graph = JobGraph::new();
        let counter = Arc::new(AtomicU64::new(0));

        for _ in 0..10 {
            let c = Arc::clone(&counter);
            graph.add(move || {
                c.fetch_add(1, Ordering::Relaxed);
            });
        }

        graph.execute(&pool);
        assert_eq!(counter.load(Ordering::Relaxed), 10);
    }

    #[test]
    fn test_dependency_ordering() {
        let pool = ThreadPool::new(4);
        let mut graph = JobGraph::new();
        let order = Arc::new(Mutex::new(Vec::new()));

        let o1 = Arc::clone(&order);
        let a = graph.add(move || {
            o1.lock().push("A");
        });

        let o2 = Arc::clone(&order);
        let b = graph.add(move || {
            o2.lock().push("B");
        });

        let o3 = Arc::clone(&order);
        graph.add_after(&[a, b], move || {
            o3.lock().push("C");
        });

        graph.execute(&pool);

        let order = order.lock();
        assert_eq!(order.len(), 3);
        // C must be last
        assert_eq!(order[2], "C");
        // A and B can be in any order
        assert!(
            (order[0] == "A" && order[1] == "B") || (order[0] == "B" && order[1] == "A"),
            "A and B should run before C, got {:?}",
            *order
        );
    }

    #[test]
    fn test_diamond_dependency() {
        let pool = ThreadPool::new(4);
        let mut graph = JobGraph::new();
        let order = Arc::new(Mutex::new(Vec::new()));

        let o = Arc::clone(&order);
        let a = graph.add(move || o.lock().push("A"));

        let o1 = Arc::clone(&order);
        let b = graph.add_after(&[a], move || o1.lock().push("B"));

        let o2 = Arc::clone(&order);
        let c = graph.add_after(&[a], move || o2.lock().push("C"));

        let o3 = Arc::clone(&order);
        graph.add_after(&[b, c], move || o3.lock().push("D"));

        graph.execute(&pool);

        let order = order.lock();
        assert_eq!(order.len(), 4);
        assert_eq!(order[0], "A");
        assert_eq!(order[3], "D");
        // B and C can be in either order
        let mid = &order[1..3];
        assert!(
            mid.contains(&"B") && mid.contains(&"C"),
            "B and C should be between A and D, got {:?}",
            *order
        );
    }

    #[test]
    fn test_chained_dependencies() {
        let pool = ThreadPool::new(4);
        let mut graph = JobGraph::new();
        let order = Arc::new(Mutex::new(Vec::new()));

        let o1 = Arc::clone(&order);
        let a = graph.add(move || o1.lock().push("A"));

        let o2 = Arc::clone(&order);
        let b = graph.add_after(&[a], move || o2.lock().push("B"));

        let o3 = Arc::clone(&order);
        let c = graph.add_after(&[b], move || o3.lock().push("C"));

        let o4 = Arc::clone(&order);
        graph.add_after(&[c], move || o4.lock().push("D"));

        graph.execute(&pool);

        let order = order.lock();
        assert_eq!(*order, vec!["A", "B", "C", "D"]);
    }
}
