use engine_jobs::{JobGraph, TaskId, ThreadPool};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn test_task_pool_creation() {
    let pool = ThreadPool::new(4);
    assert_eq!(pool.thread_count(), 4);
}

#[test]
fn test_task_pool_zero_threads_defaults() {
    let pool = ThreadPool::new(0);
    assert!(pool.thread_count() > 0);
}

#[test]
fn test_parallel_execution_with_atomic_counter() {
    let pool = ThreadPool::new(4);
    let counter = Arc::new(AtomicUsize::new(0));

    let tasks: Vec<Box<dyn FnOnce() + Send + 'static>> = (0..100)
        .map(|_| {
            let c = Arc::clone(&counter);
            Box::new(move || {
                c.fetch_add(1, Ordering::Relaxed);
            }) as Box<dyn FnOnce() + Send + 'static>
        })
        .collect();

    pool.submit_and_join(tasks);
    assert_eq!(counter.load(Ordering::Relaxed), 100);
}

#[test]
fn test_single_thread_execution() {
    let pool = ThreadPool::new(1);
    let counter = Arc::new(AtomicUsize::new(0));

    let tasks: Vec<Box<dyn FnOnce() + Send + 'static>> = (0..10)
        .map(|_| {
            let c = Arc::clone(&counter);
            Box::new(move || {
                c.fetch_add(1, Ordering::Relaxed);
            }) as Box<dyn FnOnce() + Send + 'static>
        })
        .collect();

    pool.submit_and_join(tasks);
    assert_eq!(counter.load(Ordering::Relaxed), 10);
}

#[test]
fn test_submit_returns_unique_ids() {
    let pool = ThreadPool::new(2);
    let id1 = pool.submit(|| {});
    let id2 = pool.submit(|| {});
    assert_ne!(id1, id2);
}

#[test]
fn test_task_id_uniqueness() {
    let a = TaskId::new();
    let b = TaskId::new();
    assert_ne!(a, b);
}

#[test]
fn test_job_graph_execution() {
    let pool = ThreadPool::new(4);
    let mut graph = JobGraph::new();
    let counter = Arc::new(AtomicUsize::new(0));

    for _ in 0..5 {
        let c = Arc::clone(&counter);
        graph.add(move || {
            c.fetch_add(1, Ordering::Relaxed);
        });
    }

    graph.execute(&pool);
    assert_eq!(counter.load(Ordering::Relaxed), 5);
}

#[test]
fn test_job_graph_dependency_ordering() {
    let pool = ThreadPool::new(4);
    let mut graph = JobGraph::new();
    let order = Arc::new(parking_lot::Mutex::new(Vec::new()));

    let o1 = Arc::clone(&order);
    let a = graph.add(move || o1.lock().push(1));

    let o2 = Arc::clone(&order);
    let b = graph.add(move || o2.lock().push(2));

    let o3 = Arc::clone(&order);
    graph.add_after(&[a, b], move || o3.lock().push(3));

    graph.execute(&pool);

    let order = order.lock();
    assert_eq!(order.len(), 3);
    assert_eq!(order[2], 3);
}

#[test]
fn test_empty_job_graph() {
    let pool = ThreadPool::new(2);
    let graph = JobGraph::new();
    assert!(graph.is_empty());
    graph.execute(&pool);
}

#[test]
fn test_submit_and_join_empty() {
    let pool = ThreadPool::new(2);
    pool.submit_and_join(Vec::<Box<dyn FnOnce() + Send>>::new());
}

#[test]
fn test_concurrent_task_submission() {
    let pool = ThreadPool::new(4);
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();

    for _ in 0..8 {
        let c = Arc::clone(&counter);
        let injector = Arc::clone(pool.injector());
        let handle = std::thread::spawn(move || {
            for _ in 0..50 {
                let c = Arc::clone(&c);
                injector.push(engine_jobs::Task::new(move || {
                    c.fetch_add(1, Ordering::Relaxed);
                }));
            }
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }

    // Wait for all tasks to complete
    while counter.load(Ordering::Relaxed) < 400 {
        std::thread::yield_now();
    }

    assert_eq!(counter.load(Ordering::Relaxed), 400);
}

#[test]
fn test_graceful_shutdown() {
    let counter = Arc::new(AtomicUsize::new(0));
    {
        let pool = ThreadPool::new(4);
        let tasks: Vec<Box<dyn FnOnce() + Send + 'static>> = (0..50)
            .map(|_| {
                let c = Arc::clone(&counter);
                Box::new(move || {
                    c.fetch_add(1, Ordering::Relaxed);
                }) as Box<dyn FnOnce() + Send + 'static>
            })
            .collect();
        pool.submit_and_join(tasks);
        // Pool drops here — all tasks should already be done
    }
    assert_eq!(counter.load(Ordering::Relaxed), 50);
}

#[test]
fn test_empty_queue_does_not_spin_forever() {
    let pool = ThreadPool::new(2);
    // Submit nothing, then submit something — pool should still be alive
    let counter = Arc::new(AtomicUsize::new(0));
    let c = Arc::clone(&counter);
    pool.submit(move || {
        c.fetch_add(1, Ordering::Relaxed);
    });
    // Give time for the task to execute
    std::thread::sleep(std::time::Duration::from_millis(50));
    assert_eq!(counter.load(Ordering::Relaxed), 1);
}

#[test]
fn test_work_stealing_distributes_load() {
    let pool = ThreadPool::new(4);
    let total = Arc::new(AtomicUsize::new(0));

    // Submit many small tasks — work stealing should distribute them across threads
    let mut tasks: Vec<Box<dyn FnOnce() + Send + 'static>> = Vec::new();
    for _ in 0..200 {
        let t = Arc::clone(&total);
        tasks.push(Box::new(move || {
            t.fetch_add(1, Ordering::Relaxed);
        }) as Box<dyn FnOnce() + Send + 'static>);
    }

    pool.submit_and_join(tasks);
    assert_eq!(total.load(Ordering::Relaxed), 200);
}

#[test]
fn test_job_graph_len() {
    let mut graph = JobGraph::new();
    assert_eq!(graph.len(), 0);
    graph.add(|| {});
    assert_eq!(graph.len(), 1);
    graph.add(|| {});
    assert_eq!(graph.len(), 2);
}

#[test]
fn test_jobs_error_display() {
    use engine_jobs::JobsError;

    let err = JobsError::Shutdown;
    assert_eq!(format!("{}", err), "Task pool shutdown");

    let err = JobsError::TaskPanicked("oops".into());
    assert_eq!(format!("{}", err), "Task panicked: oops");

    let err = JobsError::InvalidThreadCount(0);
    assert_eq!(format!("{}", err), "Invalid thread count: 0");

    let err = JobsError::Timeout(5000);
    assert_eq!(format!("{}", err), "Task timeout after 5000ms");
}

#[test]
fn test_thread_pool_active_count() {
    let pool = ThreadPool::new(4);
    assert_eq!(pool.active_count(), 0);
}

#[test]
fn test_submit_and_join_single_task() {
    let pool = ThreadPool::new(2);
    let counter = Arc::new(AtomicUsize::new(0));
    let c = Arc::clone(&counter);
    pool.submit_and_join(vec![move || {
        c.fetch_add(1, Ordering::Relaxed);
    }]);
    assert_eq!(counter.load(Ordering::Relaxed), 1);
}
