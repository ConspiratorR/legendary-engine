use crossbeam_deque::{Injector, Stealer, Worker};
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use crate::task::Task;

/// A work-stealing thread pool.
///
/// Spawns `num_threads` worker threads. Tasks submitted via [`submit`](Self::submit)
/// are distributed across workers using a central injector with work-stealing.
pub struct ThreadPool {
    injector: Arc<Injector<Task>>,
    threads: Vec<thread::JoinHandle<()>>,
    active_count: Arc<AtomicUsize>,
    shutdown: Arc<Mutex<bool>>,
}

impl ThreadPool {
    /// Create a new thread pool with the given number of worker threads.
    ///
    /// If `num_threads` is 0, uses the number of available CPU cores.
    pub fn new(num_threads: usize) -> Self {
        let num_threads = if num_threads == 0 {
            thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
        } else {
            num_threads
        };

        let injector = Arc::new(Injector::new());
        let mut workers = Vec::with_capacity(num_threads);
        let mut stealers = Vec::with_capacity(num_threads);

        for _ in 0..num_threads {
            let worker = Worker::new_fifo();
            stealers.push(worker.stealer());
            workers.push(worker);
        }

        let stealers = Arc::new(stealers);
        let active_count = Arc::new(AtomicUsize::new(0));
        let shutdown = Arc::new(Mutex::new(false));

        let mut threads = Vec::with_capacity(num_threads);
        for worker in workers.drain(..) {
            let injector = Arc::clone(&injector);
            let stealers = Arc::clone(&stealers);
            let active = Arc::clone(&active_count);
            let shutdown = Arc::clone(&shutdown);

            let handle = thread::spawn(move || {
                Self::worker_loop(worker, injector, stealers, active, shutdown);
            });
            threads.push(handle);
        }

        Self {
            injector,
            threads,
            active_count,
            shutdown,
        }
    }

    fn worker_loop(
        worker: Worker<Task>,
        injector: Arc<Injector<Task>>,
        stealers: Arc<Vec<Stealer<Task>>>,
        active_count: Arc<AtomicUsize>,
        shutdown: Arc<Mutex<bool>>,
    ) {
        loop {
            let task = worker.pop().or_else(|| {
                injector
                    .steal()
                    .success()
                    .or_else(|| stealers.iter().find_map(|s| s.steal().success()))
            });

            match task {
                Some(task) => {
                    active_count.fetch_add(1, Ordering::Relaxed);
                    (task.work)();
                    active_count.fetch_sub(1, Ordering::Relaxed);
                }
                None => {
                    if *shutdown.lock() {
                        break;
                    }
                    std::thread::yield_now();
                }
            }
        }
    }

    /// Submit a task for execution. Returns a [`TaskId`].
    pub fn submit<F: FnOnce() + Send + 'static>(&self, work: F) -> crate::task::TaskId {
        let task = Task::new(work);
        let id = task.id;
        self.injector.push(task);
        id
    }

    /// Submit a batch of tasks and block until all complete.
    pub fn submit_and_join<F>(&self, tasks: Vec<F>)
    where
        F: FnOnce() + Send + 'static,
    {
        let count = tasks.len();
        if count == 0 {
            return;
        }

        let done = Arc::new(AtomicUsize::new(0));

        for work in tasks {
            let done = Arc::clone(&done);
            self.injector.push(Task::new(move || {
                work();
                done.fetch_add(1, Ordering::Relaxed);
            }));
        }

        while done.load(Ordering::Relaxed) < count {
            std::thread::yield_now();
        }
    }

    /// Return the number of worker threads.
    pub fn thread_count(&self) -> usize {
        self.threads.len()
    }

    /// Return the number of currently executing tasks.
    pub fn active_count(&self) -> usize {
        self.active_count.load(Ordering::Relaxed)
    }

    /// Access the central injector for direct task submission.
    ///
    /// Used by [`JobGraph`](crate::graph::JobGraph) to submit dependent
    /// tasks without holding a `&ThreadPool` reference.
    pub fn injector(&self) -> &Arc<Injector<Task>> {
        &self.injector
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        *self.shutdown.lock() = true;
        for _ in 0..self.threads.len() {
            self.injector.push(Task::new(|| {}));
        }
        for handle in self.threads.drain(..) {
            let _ = handle.join();
        }
    }
}
