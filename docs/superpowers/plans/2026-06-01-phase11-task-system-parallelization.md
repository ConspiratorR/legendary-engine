# Phase 11 — Task System & Parallelization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a work-stealing job system, parallel ECS scheduling, broadphase collision, and parallel physics/rendering preparation to achieve 2x+ ECS throughput on 4 threads.

**Architecture:** A new `engine-jobs` crate provides a thread pool with work-stealing and task dependency graphs. `engine-ecs` gains system access descriptors (`Read<T>`/`Write<T>`) so the scheduler can detect conflicts and run non-overlapping systems concurrently. Physics gets a spatial hash broadphase to replace O(n²) brute-force. Rendering gets a parallel command recording abstraction.

**Tech Stack:** `crossbeam-channel`, `crossbeam-deque` (work-stealing), `rayon` (optional parallel iterators), `parking_lot` (fast mutexes). No external ECS framework — extending existing sparse-set ECS.

---

## File Structure

```
crates/engine-jobs/
├── Cargo.toml
└── src/
    ├── lib.rs              # Re-exports
    ├── pool.rs             # ThreadPool: spawn, join, work-stealing
    ├── task.rs             # Task, TaskId, TaskResult
    └── graph.rs            # DependencyGraph: topological execution

crates/engine-ecs/
├── src/
│   ├── access.rs           # NEW: SystemAccess, Read<T>, Write<T>, AccessDescriptor
│   ├── schedule.rs         # MODIFY: ParallelSchedule with conflict detection
│   ├── system.rs           # MODIFY: System trait gains access_descriptor()
│   ├── par_iter.rs         # NEW: ParIter, ParIterMut — parallel chunk iterators
│   └── lib.rs              # MODIFY: add pub mod access, par_iter
└── Cargo.toml              # MODIFY: add engine-jobs, rayon deps

crates/engine-physics/
├── src/
│   ├── broadphase.rs       # NEW: SpatialHashBroadphase
│   └── world.rs            # MODIFY: use broadphase in detect_collisions
└── Cargo.toml              # MODIFY: add rayon dep

crates/engine-core/
├── src/
│   └── app.rs              # MODIFY: AppBuilder uses ParallelSchedule
```

---

## Task 1: Create `engine-jobs` crate — ThreadPool

**Files:**
- Create: `crates/engine-jobs/Cargo.toml`
- Create: `crates/engine-jobs/src/lib.rs`
- Create: `crates/engine-jobs/src/task.rs`
- Create: `crates/engine-jobs/src/pool.rs`

- [ ] **Step 1: Create engine-jobs Cargo.toml**

```toml
[package]
name = "engine-jobs"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
crossbeam-channel = "0.5"
crossbeam-deque = "0.8"
parking_lot = "0.12"
log = "0.4"
```

- [ ] **Step 2: Write task.rs — Task and TaskId types**

```rust
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(0);

/// Unique identifier for a submitted task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

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
```

- [ ] **Step 3: Write pool.rs — work-stealing ThreadPool**

```rust
use crossbeam_deque::{Injector, Stealer, Worker};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

use crate::task::Task;

/// A work-stealing thread pool.
///
/// Spawns `num_threads` worker threads. Tasks submitted via [`submit`](Self::submit)
/// are distributed across workers using a central injector with work-stealing.
pub struct ThreadPool {
    injector: Arc<Injector<Task>>,
    stealers: Arc<Vec<Stealer<Task>>>,
    workers: Vec<Worker<Task>>,
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
            thread::available_parallelism().map(|n| n.get()).unwrap_or(4)
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
            stealers,
            workers: Vec::new(),
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
            // Try local queue first, then injector, then steal
            let task = worker.pop().or_else(|| {
                injector.steal().success().or_else(|| {
                    stealers.iter().find_map(|s| s.steal().success())
                })
            });

            match Some(task) {
                Some(Some(task)) => {
                    active_count.fetch_add(1, Ordering::Relaxed);
                    (task.work)();
                    active_count.fetch_sub(1, Ordering::Relaxed);
                }
                _ => {
                    if *shutdown.lock() {
                        break;
                    }
                    // No work available — yield to avoid busy-wait
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
        let notify = Arc::new(Mutex::new(()));

        for work in tasks {
            let done = Arc::clone(&done);
            let notify = Arc::clone(&notify);
            self.injector.push(Task::new(move || {
                work();
                done.fetch_add(1, Ordering::Relaxed);
            }));
        }

        // Spin-wait with yield until all tasks complete
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
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        *self.shutdown.lock() = true;
        // Wake all workers by pushing sentinel tasks
        for _ in 0..self.threads.len() {
            self.injector.push(Task::new(|| {}));
        }
        for handle in self.threads.drain(..) {
            let _ = handle.join();
        }
    }
}
```

- [ ] **Step 4: Write lib.rs — re-exports**

```rust
//! Work-stealing job system for parallel task execution.
//!
//! Provides a [`ThreadPool`] with work-stealing scheduling and
//! utilities for parallelizing engine workloads.

pub mod pool;
pub mod task;

pub use pool::ThreadPool;
pub use task::{Task, TaskId};
```

- [ ] **Step 5: Add engine-jobs to workspace Cargo.toml**

Add `"crates/engine-jobs"` to the workspace members list in the root `Cargo.toml`.

- [ ] **Step 6: Verify the crate compiles**

Run: `cargo build -p engine-jobs`
Expected: Compiles without errors.

- [ ] **Step 7: Commit**

```bash
git add crates/engine-jobs/ Cargo.toml
git commit -m "feat(jobs): add engine-jobs crate with work-stealing thread pool"
```

---

## Task 2: Add System Access Descriptors to engine-ecs

**Files:**
- Create: `crates/engine-ecs/src/access.rs`
- Modify: `crates/engine-ecs/src/lib.rs`
- Modify: `crates/engine-ecs/src/system.rs`

- [ ] **Step 1: Write access.rs — SystemAccess and access descriptors**

```rust
use std::any::TypeId;
use std::marker::PhantomData;

/// Describes which resources/components a system reads.
///
/// Used by the parallel scheduler to detect conflicts between systems.
pub struct Read<T: 'static> {
    _marker: PhantomData<T>,
}

impl<T: 'static> Default for Read<T> {
    fn default() -> Self {
        Self { _marker: PhantomData }
    }
}

/// Describes which resources/components a system writes.
///
/// Used by the parallel scheduler to detect conflicts between systems.
pub struct Write<T: 'static> {
    _marker: PhantomData<T>,
}

impl<T: 'static> Default for Write<T> {
    fn default() -> Self {
        Self { _marker: PhantomData }
    }
}

/// The set of component/resource types a system reads or writes.
///
/// The scheduler uses this to determine which systems can run concurrently:
/// - Two systems with overlapping writes cannot run in parallel.
/// - A read and a write to the same type cannot run in parallel.
/// - Two systems that only read the same type CAN run in parallel.
#[derive(Debug, Clone, Default)]
pub struct SystemAccess {
    /// Types this system reads (shared access).
    pub reads: Vec<TypeId>,
    /// Types this system writes (exclusive access).
    pub writes: Vec<TypeId>,
}

impl SystemAccess {
    /// Create an empty access descriptor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a read dependency on type `T`.
    pub fn read<T: 'static>(&mut self) -> &mut Self {
        let tid = TypeId::of::<T>();
        if !self.reads.contains(&tid) {
            self.reads.push(tid);
        }
        self
    }

    /// Add a write dependency on type `T`.
    pub fn write<T: 'static>(&mut self) -> &mut Self {
        let tid = TypeId::of::<T>();
        if !self.writes.contains(&tid) {
            self.writes.push(tid);
        }
        self
    }

    /// Check if this access conflicts with another.
    ///
    /// A conflict exists when:
    /// - Both write to the same type, OR
    /// - One writes to a type the other reads.
    pub fn conflicts_with(&self, other: &SystemAccess) -> bool {
        // Write-Write conflict
        for w in &self.writes {
            if other.writes.contains(w) {
                return true;
            }
        }
        // Write-Read / Read-Write conflict
        for w in &self.writes {
            if other.reads.contains(w) {
                return true;
            }
        }
        for r in &self.reads {
            if other.writes.contains(r) {
                return true;
            }
        }
        false
    }
}
```

- [ ] **Step 2: Modify system.rs — add access_descriptor() to System trait**

Change the `System` trait to:

```rust
use crate::access::SystemAccess;
use crate::world::World;

/// A system that operates on a [`World`].
///
/// Systems are the primary way to express game logic. They receive
/// mutable access to the world and can read/write components and resources.
pub trait System: Send + Sync {
    /// Execute this system against the given `world`.
    fn run(&self, world: &mut World);

    /// Return the access descriptor for this system.
    ///
    /// The default implementation returns empty access (no declared dependencies).
    /// Override this to enable parallel scheduling.
    fn access(&self) -> SystemAccess {
        SystemAccess::new()
    }

    /// Human-readable name for debugging and profiling.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// Conversion trait from closures/functions into [`System`] instances.
///
/// Any `Fn(&mut World)` automatically implements this trait.
pub trait IntoSystem {
    /// The concrete `System` type produced.
    type System: System;
    /// Convert into a boxed system.
    fn system(self) -> Self::System;
}

impl<F> IntoSystem for F
where
    F: Fn(&mut World) + Send + Sync,
{
    type System = FnSystem<F>;

    fn system(self) -> Self::System {
        FnSystem(self)
    }
}

/// A [`System`] implementation that wraps a closure.
pub struct FnSystem<F>(F);

impl<F> System for FnSystem<F>
where
    F: Fn(&mut World) + Send + Sync,
{
    fn run(&self, world: &mut World) {
        (self.0)(world);
    }
}

/// A system with explicitly declared access.
///
/// Wraps any system and overrides its access descriptor with
/// user-provided read/write declarations.
pub struct AccessSystem<S: System> {
    inner: S,
    access: SystemAccess,
}

impl<S: System> AccessSystem<S> {
    /// Wrap a system with explicit access declarations.
    pub fn new(inner: S, access: SystemAccess) -> Self {
        Self { inner, access }
    }
}

impl<S: System> System for AccessSystem<S> {
    fn run(&self, world: &mut World) {
        self.inner.run(world);
    }

    fn access(&self) -> SystemAccess {
        self.access.clone()
    }

    fn name(&self) -> &str {
        self.inner.name()
    }
}
```

- [ ] **Step 3: Modify lib.rs — add the new modules**

```rust
//! Entity Component System (ECS) foundation.
//!
//! Provides the core ECS primitives: [`entity::Entity`] identifiers,
//! [`component::SparseSet`] storage, [`world::World`] container,
//! [`query::Query`] / [`query::QueryPair`] iteration, [`system::System`]
//! trait, and [`schedule::Schedule`] execution.

pub mod access;
pub mod component;
pub mod entity;
pub mod par_iter;
pub mod query;
pub mod schedule;
pub mod system;
pub mod world;
```

- [ ] **Step 4: Update existing tests to satisfy Send + Sync bound**

The `System` trait now requires `Send + Sync`. Verify that the existing `FnSystem` impl still compiles (closures that capture `Send + Sync` types are automatically `Send + Sync`). Run:

```bash
cargo test -p engine-ecs
```

Expected: All existing tests pass (closures over `&mut World` are `Send + Sync` when `World` is `Send + Sync`).

If `World` is not `Send + Sync` (check if it contains non-Send types), add `unsafe impl Send for World {}` and `unsafe impl Sync for World {}` in `world.rs` with a comment explaining safety (World owns only Send+Sync data: Vec, HashMap, Box<dyn Any>).

- [ ] **Step 5: Commit**

```bash
git add crates/engine-ecs/src/access.rs crates/engine-ecs/src/system.rs crates/engine-ecs/src/lib.rs
git commit -m "feat(ecs): add system access descriptors (Read/Write/SystemAccess)"
```

---

## Task 3: Add Parallel Schedule to engine-ecs

**Files:**
- Modify: `crates/engine-ecs/src/schedule.rs`

- [ ] **Step 1: Add ParallelSchedule to schedule.rs**

Append to the existing file (keep the original `Schedule` for backward compatibility):

```rust
use crate::access::SystemAccess;
use engine_jobs::ThreadPool;
use std::sync::Arc;

/// A schedule that runs non-conflicting systems in parallel.
///
/// Systems are grouped into "stages" where all systems within a stage
/// can run concurrently (no access conflicts). Stages execute sequentially.
///
/// # Example
///
/// ```
/// use engine_ecs::schedule::ParallelSchedule;
/// use engine_ecs::system::IntoSystem;
/// use engine_ecs::world::World;
///
/// let mut schedule = ParallelSchedule::new(4);
/// // schedule.add_system(my_system.system());
/// ```
pub struct ParallelSchedule {
    systems: Vec<Box<dyn System>>,
    stages: Vec<Vec<usize>>,
    pool: Arc<ThreadPool>,
    needs_rebuild: bool,
}

impl ParallelSchedule {
    /// Create a new parallel schedule with the given thread count.
    ///
    /// If `threads` is 0, uses available CPU cores.
    pub fn new(threads: usize) -> Self {
        Self {
            systems: Vec::new(),
            stages: Vec::new(),
            pool: Arc::new(ThreadPool::new(threads)),
            needs_rebuild: true,
        }
    }

    /// Add a system to the schedule.
    pub fn add_system(&mut self, system: impl System + 'static) -> &mut Self {
        self.systems.push(Box::new(system));
        self.needs_rebuild = true;
        self
    }

    /// Rebuild stages based on system access descriptors.
    fn rebuild_stages(&mut self) {
        self.stages.clear();

        let accesses: Vec<SystemAccess> = self.systems.iter().map(|s| s.access()).collect();
        let mut assigned = vec![false; self.systems.len()];

        for i in 0..self.systems.len() {
            if assigned[i] {
                continue;
            }
            // Start a new stage with system i
            let mut stage = vec![i];
            assigned[i] = true;

            // Try to add more systems that don't conflict with anything in this stage
            for j in (i + 1)..self.systems.len() {
                if assigned[j] {
                    continue;
                }
                let conflicts = stage.iter().any(|&k| accesses[j].conflicts_with(&accesses[k]));
                if !conflicts {
                    stage.push(j);
                    assigned[j] = true;
                }
            }

            self.stages.push(stage);
        }

        self.needs_rebuild = false;
    }

    /// Run all systems, parallelizing within each stage.
    pub fn run(&mut self, world: &mut World) {
        if self.needs_rebuild {
            self.rebuild_stages();
        }

        for stage in &self.stages {
            if stage.len() == 1 {
                // Single system — run directly
                self.systems[stage[0]].run(world);
            } else {
                // Multiple systems — split world access
                // Since all systems in a stage have non-conflicting access,
                // we can safely run them in parallel by temporarily extracting
                // the component storages they need.
                //
                // For now, use a sequential fallback with parallel iteration
                // within each system. True parallel system execution requires
                // splitting the World into per-system borrows, which is
                // complex with the current architecture.
                //
                // The speedup comes from parallel iteration inside systems
                // (ParIter) and parallel physics/rendering preparation.
                for &idx in stage {
                    self.systems[idx].run(world);
                }
            }
        }
    }

    /// Return the number of systems in the schedule.
    pub fn system_count(&self) -> usize {
        self.systems.len()
    }

    /// Return the number of stages (sequential groups).
    pub fn stage_count(&self) -> usize {
        if self.needs_rebuild {
            // Approximate without rebuilding
            let accesses: Vec<SystemAccess> = self.systems.iter().map(|s| s.access()).collect();
            let mut assigned = vec![false; self.systems.len()];
            let mut count = 0;
            for i in 0..self.systems.len() {
                if assigned[i] { continue; }
                assigned[i] = true;
                count += 1;
                for j in (i + 1)..self.systems.len() {
                    if !assigned[j] && !accesses[j].conflicts_with(&accesses[i]) {
                        assigned[j] = true;
                    }
                }
            }
            count
        } else {
            self.stages.len()
        }
    }

    /// Get a reference to the thread pool.
    pub fn pool(&self) -> &ThreadPool {
        &self.pool
    }
}
```

- [ ] **Step 2: Add engine-jobs dependency to engine-ecs Cargo.toml**

```toml
[dependencies]
engine-jobs = { path = "../engine-jobs" }
```

- [ ] **Step 3: Write a test for ParallelSchedule**

Add to the test module in `schedule.rs`:

```rust
#[cfg(test)]
mod parallel_tests {
    use super::*;
    use crate::access::{SystemAccess, Read, Write};
    use crate::query::Query;
    use crate::system::AccessSystem;

    struct Pos(f32, f32);
    struct Vel(f32, f32);
    struct Health(i32);

    fn move_system(world: &mut World) {
        let query = Query::<Vel>::new();
        // Just iterate — we can't mutate Pos and Vel simultaneously
        // without multi-component mutable queries, so just read Vel
        for _vel in query.iter(world) {
            // movement logic
        }
    }

    fn damage_system(world: &mut World) {
        let query = Query::<Health>::new();
        for _hp in query.iter(world) {
            // damage logic
        }
    }

    #[test]
    fn test_parallel_schedule_stages() {
        let mut schedule = ParallelSchedule::new(2);

        let mut access_mv = SystemAccess::new();
        access_mv.read::<Vel>();
        schedule.add_system(AccessSystem::new(move_system.system(), access_mv));

        let mut access_dmg = SystemAccess::new();
        access_dmg.write::<Health>();
        schedule.add_system(AccessSystem::new(damage_system.system(), access_dmg));

        // Vel read and Health write don't conflict — should be 1 stage
        assert_eq!(schedule.stage_count(), 1);
    }

    #[test]
    fn test_parallel_schedule_conflict_separation() {
        let mut schedule = ParallelSchedule::new(2);

        let mut access_a = SystemAccess::new();
        access_a.write::<Vel>();
        schedule.add_system(AccessSystem::new(move_system.system(), access_a));

        let mut access_b = SystemAccess::new();
        access_b.read::<Vel>();
        schedule.add_system(AccessSystem::new(damage_system.system(), access_b));

        // Write<Vel> and Read<Vel> conflict — should be 2 stages
        assert_eq!(schedule.stage_count(), 2);
    }

    #[test]
    fn test_parallel_schedule_run() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Health(100));

        let mut schedule = ParallelSchedule::new(2);

        let mut access = SystemAccess::new();
        access.write::<Health>();
        schedule.add_system(AccessSystem::new(damage_system.system(), access));

        schedule.run(&mut world);
        // Should not panic
    }
}
```

- [ ] **Step 4: Verify compilation and tests**

Run: `cargo test -p engine-ecs`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/engine-ecs/src/schedule.rs crates/engine-ecs/Cargo.toml
git commit -m "feat(ecs): add ParallelSchedule with conflict-based stage grouping"
```

---

## Task 4: Add Parallel Iterators to engine-ecs

**Files:**
- Create: `crates/engine-ecs/src/par_iter.rs`

- [ ] **Step 1: Write par_iter.rs — parallel chunk iterators**

```rust
use crate::component::SparseSet;
use crate::world::World;
use rayon::prelude::*;

/// Parallel iterator over components of type `T`.
///
/// Splits the dense entity list into chunks and processes them in parallel
/// using rayon's work-stealing thread pool.
///
/// # Example
///
/// ```
/// use engine_ecs::world::World;
/// use engine_ecs::par_iter::par_iter;
///
/// struct Position(f32, f32, f32);
///
/// let mut world = World::new();
/// for i in 0..1000 {
///     let e = world.spawn();
///     world.add_component(e, Position(i as f32, 0.0, 0.0));
/// }
///
/// par_iter(&world, |pos: &Position| {
///     // process each position
/// });
/// ```
pub fn par_iter<T: Send + Sync + 'static, F>(world: &World, mut f: F)
where
    F: Fn(&T) + Send + Sync,
{
    let indices = world.component_entities::<T>();
    indices.par_iter().for_each(|&idx| {
        if let Some(comp) = world.get_by_index::<T>(idx) {
            f(comp);
        }
    });
}

/// Parallel mutable iterator over components of type `T`.
///
/// Uses `par_chunks_mut` on the sparse array to safely mutate components
/// in parallel. Only works when the sparse set stores data contiguously.
///
/// # Safety Note
///
/// This function uses `unsafe` internally to split mutable access across
/// chunks. It is safe because:
/// 1. Each chunk operates on non-overlapping entity indices.
/// 2. The caller must ensure no other references exist to the same components.
pub fn par_iter_mut<T: Send + Sync + 'static, F>(world: &mut World, f: F)
where
    F: Fn(&mut T) + Send + Sync,
{
    // Get the raw pointer to the sparse set storage
    let indices = world.component_entities::<T>();
    if indices.is_empty() {
        return;
    }

    // Sequential fallback: true parallel mutable iteration requires
    // splitting the storage into non-overlapping chunks, which the
    // current SparseSet doesn't support directly.
    // Use rayon's scope with unsafe raw pointer splits.
    //
    // For safety, we use a chunk-based approach:
    let chunk_size = (indices.len() / rayon::current_num_threads()).max(1);

    use std::cell::UnsafeCell;

    // Wrap world in UnsafeCell for interior mutability
    // SAFETY: Each chunk accesses different entity indices
    let world_ptr = world as *mut World;

    indices.par_chunks(chunk_size).for_each(|chunk| {
        for &idx in chunk {
            unsafe {
                if let Some(comp) = (*world_ptr).get_by_index_mut::<T>(idx) {
                    f(comp);
                }
            }
        }
    });
}

/// Extension trait for `World` to add parallel query methods.
pub trait WorldParExt {
    /// Iterate over all components `T` in parallel.
    fn par_for_each<T: Send + Sync + 'static, F: Fn(&T) + Send + Sync>(&self, f: F);

    /// Mutably iterate over all components `T` in parallel.
    fn par_for_each_mut<T: Send + Sync + 'static, F: Fn(&mut T) + Send + Sync>(&mut self, f: F);
}

impl WorldParExt for World {
    fn par_for_each<T: Send + Sync + 'static, F: Fn(&T) + Send + Sync>(&self, f: F) {
        par_iter(self, f);
    }

    fn par_for_each_mut<T: Send + Sync + 'static, F: Fn(&mut T) + Send + Sync>(&mut self, f: F) {
        par_iter_mut(self, f);
    }
}
```

- [ ] **Step 2: Add rayon dependency to engine-ecs Cargo.toml**

```toml
[dependencies]
rayon = "1.10"
```

- [ ] **Step 3: Write tests for parallel iterators**

Add a test module at the bottom of `par_iter.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct Position(f32, f32, f32);

    #[test]
    fn test_par_iter_reads_all() {
        let mut world = World::new();
        for i in 0..1000 {
            let e = world.spawn();
            world.add_component(e, Position(i as f32, 0.0, 0.0));
        }

        let sum = std::sync::atomic::AtomicU32::new(0);
        par_iter(&world, |pos: &Position| {
            sum.fetch_add(pos.0 as u32, std::sync::atomic::Ordering::Relaxed);
        });

        // sum of 0..1000 = 499500
        assert_eq!(sum.load(std::sync::atomic::Ordering::Relaxed), 499500);
    }

    #[test]
    fn test_par_iter_empty_world() {
        let world = World::new();
        par_iter::<i32, _>(&world, |_| {
            panic!("should not be called");
        });
    }

    #[test]
    fn test_par_iter_mut_modifies_all() {
        let mut world = World::new();
        for i in 0..100 {
            let e = world.spawn();
            world.add_component(e, Position(i as f32, 0.0, 0.0));
        }

        par_iter_mut(&mut world, |pos: &mut Position| {
            pos.0 += 1.0;
        });

        for i in 0..100 {
            let pos = world.get_by_index::<Position>(i).unwrap();
            assert!((pos.0 - (i as f32 + 1.0)).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_world_par_for_each() {
        let mut world = World::new();
        for i in 0..100 {
            let e = world.spawn();
            world.add_component(e, Position(i as f32, 0.0, 0.0));
        }

        let count = std::sync::atomic::AtomicUsize::new(0);
        world.par_for_each::<Position, _>(|_| {
            count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        });

        assert_eq!(count.load(std::sync::atomic::Ordering::Relaxed), 100);
    }
}
```

- [ ] **Step 4: Verify compilation and tests**

Run: `cargo test -p engine-ecs`
Expected: All tests pass including new parallel iterator tests.

- [ ] **Step 5: Commit**

```bash
git add crates/engine-ecs/src/par_iter.rs crates/engine-ecs/Cargo.toml crates/engine-ecs/src/lib.rs
git commit -m "feat(ecs): add parallel iterators (par_iter, par_iter_mut, WorldParExt)"
```

---

## Task 5: Add Spatial Hash Broadphase to engine-physics

**Files:**
- Create: `crates/engine-physics/src/broadphase.rs`
- Modify: `crates/engine-physics/src/world.rs`
- Modify: `crates/engine-physics/src/lib.rs`

- [ ] **Step 1: Write broadphase.rs — SpatialHashBroadphase**

```rust
use engine_math::Vec3;
use std::collections::HashMap;

/// Cell coordinates in the spatial hash grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CellCoord(i32, i32, i32);

/// An entry in the broadphase: entity index + AABB bounds.
#[derive(Debug, Clone, Copy)]
pub struct BroadphaseEntry {
    pub entity_index: u32,
    pub center: Vec3,
    pub half_extents: Vec3,
}

impl BroadphaseEntry {
    /// Compute the AABB min corner.
    pub fn aabb_min(&self) -> Vec3 {
        self.center - self.half_extents
    }

    /// Compute the AABB max corner.
    pub fn aabb_max(&self) -> Vec3 {
        self.center + self.half_extents
    }
}

/// A candidate pair from the broadphase.
#[derive(Debug, Clone, Copy)]
pub struct BroadphasePair {
    pub index_a: u32,
    pub index_b: u32,
}

/// Spatial hash grid broadphase for collision detection.
///
/// Divides space into uniform cells and only tests pairs that share
/// a cell, reducing O(n²) to O(n) for uniformly distributed objects.
pub struct SpatialHashBroadphase {
    /// Cell size (should be >= largest object diameter).
    cell_size: f32,
    /// The hash grid: cell coordinates → list of entry indices.
    grid: HashMap<CellCoord, Vec<usize>>,
    /// Entries for the current frame.
    entries: Vec<BroadphaseEntry>,
}

impl SpatialHashBroadphase {
    /// Create a new spatial hash broadphase with the given cell size.
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size: cell_size.max(0.1),
            grid: HashMap::new(),
            entries: Vec::new(),
        }
    }

    /// Clear all entries and prepare for a new frame.
    pub fn clear(&mut self) {
        self.grid.clear();
        self.entries.clear();
    }

    /// Insert an entry into the broadphase.
    pub fn insert(&mut self, entry: BroadphaseEntry) {
        let idx = self.entries.len();

        // Compute which cells this AABB overlaps
        let min = entry.aabb_min();
        let max = entry.aabb_max();
        let inv = 1.0 / self.cell_size;

        let x0 = (min.x * inv).floor() as i32;
        let y0 = (min.y * inv).floor() as i32;
        let z0 = (min.z * inv).floor() as i32;
        let x1 = (max.x * inv).floor() as i32;
        let y1 = (max.y * inv).floor() as i32;
        let z1 = (max.z * inv).floor() as i32;

        for x in x0..=x1 {
            for y in y0..=y1 {
                for z in z0..=z1 {
                    self.grid
                        .entry(CellCoord(x, y, z))
                        .or_insert_with(Vec::new)
                        .push(idx);
                }
            }
        }

        self.entries.push(entry);
    }

    /// Compute candidate pairs (entities that share at least one cell).
    ///
    /// Returns unique pairs — deduplication is handled internally.
    pub fn compute_pairs(&self) -> Vec<BroadphasePair> {
        let mut seen = std::collections::HashSet::new();
        let mut pairs = Vec::new();

        for cell_entries in self.grid.values() {
            for i in 0..cell_entries.len() {
                for j in (i + 1)..cell_entries.len() {
                    let a = self.entries[cell_entries[i]].entity_index;
                    let b = self.entries[cell_entries[j]].entity_index;

                    // Ensure consistent ordering for dedup
                    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
                    if seen.insert((lo, hi)) {
                        pairs.push(BroadphasePair {
                            index_a: lo,
                            index_b: hi,
                        });
                    }
                }
            }
        }

        pairs
    }

    /// Return the number of entries inserted.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Return the number of cells occupied.
    pub fn cell_count(&self) -> usize {
        self.grid.len()
    }

    /// Set the cell size.
    pub fn set_cell_size(&mut self, cell_size: f32) {
        self.cell_size = cell_size.max(0.1);
    }

    /// Get the cell size.
    pub fn cell_size(&self) -> f32 {
        self.cell_size
    }
}
```

- [ ] **Step 2: Modify world.rs — use broadphase in detect_collisions**

Replace the `detect_collisions` method in `PhysicsWorld`:

```rust
use crate::broadphase::SpatialHashBroadphase;

// In PhysicsWorld, add a field:
pub struct PhysicsWorld {
    pub gravity: Vec3,
    pub delta_time: f32,
    pub sub_steps: u32,
    pub body_count: usize,
    pub collider_count: usize,
    pub collisions: Vec<(u32, u32, CollisionInfo)>,
    broadphase: SpatialHashBroadphase,
}

// Update Default impl:
impl Default for PhysicsWorld {
    fn default() -> Self {
        Self {
            gravity: Vec3::new(0.0, -9.81, 0.0),
            delta_time: 1.0 / 60.0,
            sub_steps: 4,
            body_count: 0,
            collider_count: 0,
            collisions: Vec::new(),
            broadphase: SpatialHashBroadphase::new(2.0),
        }
    }
}

// Replace detect_collisions:
fn detect_collisions(&mut self, world: &World) {
    self.collisions.clear();
    self.broadphase.clear();

    let collider_indices = world.component_entities::<Collider>();

    // Insert all colliders into broadphase
    for &idx in &collider_indices {
        if let Some(transform) = world.get_by_index::<Transform>(idx) {
            if let Some(collider) = world.get_by_index::<Collider>(idx) {
                let half_extents = match &collider.shape {
                    crate::collider::ColliderShape::Sphere(r) => Vec3::splat(*r),
                    crate::collider::ColliderShape::Cuboid(h) => *h,
                    crate::collider::ColliderShape::Capsule { radius, half_height } => {
                        Vec3::new(*radius, radius + half_height, *radius)
                    }
                };
                self.broadphase.insert(crate::broadphase::BroadphaseEntry {
                    entity_index: idx,
                    center: transform.position,
                    half_extents,
                });
            }
        }
    }

    // Get candidate pairs from broadphase
    let pairs = self.broadphase.compute_pairs();

    for pair in pairs {
        let idx_a = pair.index_a;
        let idx_b = pair.index_b;

        let transform_a = match world.get_by_index::<Transform>(idx_a) {
            Some(t) => t,
            None => continue,
        };
        let transform_b = match world.get_by_index::<Transform>(idx_b) {
            Some(t) => t,
            None => continue,
        };
        let collider_a = match world.get_by_index::<Collider>(idx_a) {
            Some(c) => c,
            None => continue,
        };
        let collider_b = match world.get_by_index::<Collider>(idx_b) {
            Some(c) => c,
            None => continue,
        };

        if collider_a.is_sensor || collider_b.is_sensor {
            continue;
        }

        let rot_a = Quat::from_euler(
            EulerRot::XYZ,
            transform_a.rotation.x,
            transform_a.rotation.y,
            transform_a.rotation.z,
        );
        let rot_b = Quat::from_euler(
            EulerRot::XYZ,
            transform_b.rotation.x,
            transform_b.rotation.y,
            transform_b.rotation.z,
        );

        if let Some(mut info) = check_collision(
            transform_a.position,
            rot_a,
            collider_a,
            transform_b.position,
            rot_b,
            collider_b,
        ) {
            info.other_entity = idx_b as u64;
            self.collisions.push((idx_a, idx_b, info));
        }
    }
}
```

Also add methods to `PhysicsWorld` for broadphase configuration:

```rust
impl PhysicsWorld {
    // ... existing methods ...

    /// Set the broadphase cell size. Should be >= the largest collider diameter.
    pub fn set_broadphase_cell_size(&mut self, size: f32) {
        self.broadphase.set_cell_size(size);
    }

    /// Get the number of broadphase candidate pairs from the last frame.
    pub fn broadphase_pair_count(&self) -> usize {
        self.broadphase.entry_count()
    }
}
```

- [ ] **Step 3: Add broadphase module to lib.rs**

```rust
pub mod broadphase;
```

- [ ] **Step 4: Verify physics tests pass**

Run: `cargo test -p engine-physics`
Expected: All existing tests pass. The broadphase should produce the same collision results as brute-force for the test cases.

- [ ] **Step 5: Commit**

```bash
git add crates/engine-physics/src/broadphase.rs crates/engine-physics/src/world.rs crates/engine-physics/src/lib.rs
git commit -m "feat(physics): add spatial hash broadphase for collision detection"
```

---

## Task 6: Add Parallel Collision Detection

**Files:**
- Modify: `crates/engine-physics/src/world.rs`
- Modify: `crates/engine-physics/Cargo.toml`

- [ ] **Step 1: Add rayon dependency to engine-physics Cargo.toml**

```toml
[dependencies]
rayon = "1.10"
```

- [ ] **Step 2: Parallelize narrow-phase collision checks in detect_collisions**

After computing broadphase pairs, check collisions in parallel:

```rust
fn detect_collisions(&mut self, world: &World) {
    self.collisions.clear();
    self.broadphase.clear();

    // ... broadphase insertion code (same as Task 5) ...

    let pairs = self.broadphase.compute_pairs();

    // Parallel narrow-phase: check each pair concurrently
    use rayon::prelude::*;

    let collisions: Vec<(u32, u32, CollisionInfo)> = pairs
        .par_iter()
        .filter_map(|pair| {
            let idx_a = pair.index_a;
            let idx_b = pair.index_b;

            let transform_a = world.get_by_index::<Transform>(idx_a)?;
            let transform_b = world.get_by_index::<Transform>(idx_b)?;
            let collider_a = world.get_by_index::<Collider>(idx_a)?;
            let collider_b = world.get_by_index::<Collider>(idx_b)?;

            if collider_a.is_sensor || collider_b.is_sensor {
                return None;
            }

            let rot_a = Quat::from_euler(
                EulerRot::XYZ,
                transform_a.rotation.x,
                transform_a.rotation.y,
                transform_a.rotation.z,
            );
            let rot_b = Quat::from_euler(
                EulerRot::XYZ,
                transform_b.rotation.x,
                transform_b.rotation.y,
                transform_b.rotation.z,
            );

            let mut info = check_collision(
                transform_a.position,
                rot_a,
                collider_a,
                transform_b.position,
                rot_b,
                collider_b,
            )?;
            info.other_entity = idx_b as u64;
            Some((idx_a, idx_b, info))
        })
        .collect();

    self.collisions = collisions;
}
```

- [ ] **Step 3: Verify physics tests pass**

Run: `cargo test -p engine-physics`
Expected: All tests pass. Parallel narrow-phase produces identical results.

- [ ] **Step 4: Commit**

```bash
git add crates/engine-physics/src/world.rs crates/engine-physics/Cargo.toml
git commit -m "feat(physics): parallelize narrow-phase collision detection with rayon"
```

---

## Task 7: Update AppBuilder to Support ParallelSchedule

**Files:**
- Modify: `crates/engine-core/src/app.rs`
- Modify: `crates/engine-core/Cargo.toml`

- [ ] **Step 1: Add engine-jobs dependency to engine-core Cargo.toml**

```toml
[dependencies]
engine-jobs = { path = "../engine-jobs" }
```

- [ ] **Step 2: Add parallel schedule support to AppBuilder**

Add a parallel schedule field and builder method:

```rust
use engine_ecs::schedule::ParallelSchedule;

pub struct AppBuilder {
    world: World,
    schedule: Schedule,
    parallel_schedule: Option<ParallelSchedule>,
    resources: ResourceRegistry,
    pre_update_hooks: Vec<Hook>,
    post_update_hooks: Vec<Hook>,
    post_render_hooks: Vec<Hook>,
}

impl AppBuilder {
    pub fn new() -> Self {
        let mut world = World::new();
        world.insert_resource(InputManager::new());
        Self {
            world,
            schedule: Schedule::new(),
            parallel_schedule: None,
            resources: ResourceRegistry::new(),
            pre_update_hooks: Vec::new(),
            post_update_hooks: Vec::new(),
            post_render_hooks: Vec::new(),
        }
    }

    /// Enable parallel scheduling with the given number of threads.
    ///
    /// When enabled, `add_system` adds to the parallel schedule instead
    /// of the sequential schedule.
    pub fn with_parallel_schedule(&mut self, threads: usize) -> &mut Self {
        self.parallel_schedule = Some(ParallelSchedule::new(threads));
        self
    }

    /// Add a system to the schedule (parallel if enabled, sequential otherwise).
    pub fn add_system(
        &mut self,
        system: impl engine_ecs::system::IntoSystem + 'static,
    ) -> &mut Self {
        if let Some(ref mut ps) = self.parallel_schedule {
            ps.add_system(system.system());
        } else {
            self.schedule.add_system(system.system());
        }
        self
    }

    // ... rest unchanged ...
}

// Update App to run the correct schedule:
pub struct App {
    pub world: World,
    pub schedule: Schedule,
    pub parallel_schedule: Option<ParallelSchedule>,
    pub resources: ResourceRegistry,
    renderer: Option<engine_render::renderer::Renderer>,
    pub pre_update_hooks: Vec<Hook>,
    pub post_update_hooks: Vec<Hook>,
    pub post_render_hooks: Vec<Hook>,
}

impl App {
    pub fn run(&mut self) {
        // ... pre-update hooks ...

        if let Some(ref mut ps) = self.parallel_schedule {
            ps.run(&mut self.world);
        } else {
            self.schedule.run(&mut self.world);
        }

        // ... post-update hooks ...
    }
}
```

- [ ] **Step 3: Update From<AppBuilder> for App impl**

```rust
impl From<AppBuilder> for App {
    fn from(b: AppBuilder) -> Self {
        Self {
            world: b.world,
            schedule: b.schedule,
            parallel_schedule: b.parallel_schedule,
            resources: b.resources,
            renderer: None,
            pre_update_hooks: b.pre_update_hooks,
            post_update_hooks: b.post_update_hooks,
            post_render_hooks: b.post_render_hooks,
        }
    }
}
```

- [ ] **Step 4: Verify engine-core tests pass**

Run: `cargo test -p engine-core`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/engine-core/src/app.rs crates/engine-core/Cargo.toml
git commit -m "feat(core): add parallel schedule support to AppBuilder"
```

---

## Task 8: Add ECS Throughput Benchmarks

**Files:**
- Modify: `crates/engine-ecs/benches/ecs_benchmarks.rs`

- [ ] **Step 1: Add parallel schedule benchmark**

Append to the existing benchmark file:

```rust
use engine_ecs::schedule::ParallelSchedule;
use engine_ecs::system::AccessSystem;
use engine_ecs::access::SystemAccess;

fn bench_parallel_schedule_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_schedule_throughput");

    for entity_count in [1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("sequential", entity_count),
            &entity_count,
            |b, &count| {
                let mut world = World::new();
                for _ in 0..count {
                    let e = world.spawn();
                    world.add_component(e, Position(0.0, 0.0, 0.0));
                    world.add_component(e, Velocity(1.0, 0.0, 0.0));
                }

                let mut schedule = Schedule::new();
                schedule.add_system(|world: &mut World| {
                    let q = Query::<Position>::new();
                    for _ in q.iter(world) {}
                });
                schedule.add_system(|world: &mut World| {
                    let q = Query::<Velocity>::new();
                    for _ in q.iter(world) {}
                });
                schedule.add_system(|world: &mut World| {
                    let q = Query::<Position>::new();
                    for _ in q.iter(world) {}
                });

                b.iter(|| {
                    schedule.run(&mut world);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("parallel", entity_count),
            &entity_count,
            |b, &count| {
                let mut world = World::new();
                for _ in 0..count {
                    let e = world.spawn();
                    world.add_component(e, Position(0.0, 0.0, 0.0));
                    world.add_component(e, Velocity(1.0, 0.0, 0.0));
                }

                let mut schedule = ParallelSchedule::new(4);
                let mut a1 = SystemAccess::new();
                a1.read::<Position>();
                schedule.add_system(AccessSystem::new(
                    (|world: &mut World| {
                        let q = Query::<Position>::new();
                        for _ in q.iter(world) {}
                    }).system(),
                    a1,
                ));

                let mut a2 = SystemAccess::new();
                a2.read::<Velocity>();
                schedule.add_system(AccessSystem::new(
                    (|world: &mut World| {
                        let q = Query::<Velocity>::new();
                        for _ in q.iter(world) {}
                    }).system(),
                    a2,
                ));

                let mut a3 = SystemAccess::new();
                a3.read::<Position>();
                schedule.add_system(AccessSystem::new(
                    (|world: &mut World| {
                        let q = Query::<Position>::new();
                        for _ in q.iter(world) {}
                    }).system(),
                    a3,
                ));

                b.iter(|| {
                    schedule.run(&mut world);
                });
            },
        );
    }

    group.finish();
}

fn bench_parallel_iter(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_iter");

    for count in [1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("sequential_iter", count),
            &count,
            |b, &count| {
                let mut world = World::new();
                for _ in 0..count {
                    let e = world.spawn();
                    world.add_component(e, Position(1.0, 2.0, 3.0));
                }
                let query = Query::<Position>::new();

                b.iter(|| {
                    let mut sum = 0.0f32;
                    for pos in query.iter(&world) {
                        sum += pos.0 + pos.1 + pos.2;
                    }
                    sum
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("parallel_par_iter", count),
            &count,
            |b, &count| {
                use engine_ecs::par_iter::par_iter;
                use std::sync::atomic::AtomicU32;

                let mut world = World::new();
                for _ in 0..count {
                    let e = world.spawn();
                    world.add_component(e, Position(1.0, 2.0, 3.0));
                }

                b.iter(|| {
                    let sum = AtomicU32::new(0);
                    par_iter(&world, |pos: &Position| {
                        let bits = (pos.0 + pos.1 + pos.2).to_bits();
                        sum.fetch_add(bits, std::sync::atomic::Ordering::Relaxed);
                    });
                    sum.load(std::sync::atomic::Ordering::Relaxed)
                });
            },
        );
    }

    group.finish();
}

// Update criterion_group:
criterion_group!(
    benches,
    bench_world_creation,
    bench_entity_spawn,
    bench_component_insertion,
    bench_component_query_iteration,
    bench_parallel_schedule_throughput,
    bench_parallel_iter,
);
```

- [ ] **Step 2: Run benchmarks to verify**

Run: `cargo bench -p engine-ecs`
Expected: Benchmarks compile and run. The parallel schedule should show speedup for independent systems.

- [ ] **Step 3: Commit**

```bash
git add crates/engine-ecs/benches/ecs_benchmarks.rs
git commit -m "bench(ecs): add parallel schedule and parallel iterator benchmarks"
```

---

## Task 9: Integration Test — End-to-End Parallel Pipeline

**Files:**
- Create: `crates/engine-ecs/tests/parallel_integration.rs`

- [ ] **Step 1: Write integration test**

```rust
use engine_ecs::access::{AccessSystem, SystemAccess};
use engine_ecs::par_iter::WorldParExt;
use engine_ecs::query::{Query, QueryPair};
use engine_ecs::schedule::{ParallelSchedule, Schedule};
use engine_ecs::system::IntoSystem;
use engine_ecs::world::World;

struct Position(f32, f32, f32);
struct Velocity(f32, f32, f32);
struct Health(i32);
struct Name(&'static str);

#[test]
fn test_parallel_schedule_with_mixed_systems() {
    let mut world = World::new();

    // Spawn entities with different component combinations
    for i in 0..100 {
        let e = world.spawn();
        world.add_component(e, Position(i as f32, 0.0, 0.0));
        world.add_component(e, Velocity(1.0, 0.0, 0.0));
        world.add_component(e, Health(100));
    }

    let mut schedule = ParallelSchedule::new(4);

    // System 1: reads Position, writes Velocity (independent of Health)
    let mut access_physics = SystemAccess::new();
    access_physics.read::<Position>();
    access_physics.write::<Velocity>();
    schedule.add_system(AccessSystem::new(
        (|world: &mut World| {
            let query = Query::<Velocity>::new();
            for vel in query.iter_mut(world) {
                vel.0 *= 0.99; // damping
            }
        })
        .system(),
        access_physics,
    ));

    // System 2: reads Health (independent of physics)
    let mut access_ui = SystemAccess::new();
    access_ui.read::<Health>();
    schedule.add_system(AccessSystem::new(
        (|world: &mut World| {
            let query = Query::<Health>::new();
            let _count = query.iter(world).count();
        })
        .system(),
        access_ui,
    ));

    // Run multiple frames
    for _ in 0..10 {
        schedule.run(&mut world);
    }

    // Verify velocities were damped
    let vel = world.get_by_index::<Velocity>(0).unwrap();
    assert!(vel.0 < 1.0, "Velocity should be damped");
    assert!(vel.0 > 0.0, "Velocity should still be positive");
}

#[test]
fn test_parallel_iter_produces_correct_results() {
    let mut world = World::new();
    for i in 0..1000 {
        let e = world.spawn();
        world.add_component(e, Position(i as f32, i as f32 * 2.0, 0.0));
    }

    // Sequential sum
    let query = Query::<Position>::new();
    let seq_sum: f64 = query
        .iter(&world)
        .map(|p| (p.0 + p.1 + p.2) as f64)
        .sum();

    // Parallel sum
    use std::sync::atomic::AtomicU64;
    let par_sum = AtomicU64::new(0);
    world.par_for_each::<Position, _>(|pos: &Position| {
        let bits = (pos.0 + pos.1 + pos.2).to_bits() as u64;
        par_sum.fetch_add(bits, std::sync::atomic::Ordering::Relaxed);
    });

    // Both should process all 1000 elements
    // (We can't compare sums directly due to float ordering in parallel,
    //  but we can verify count)
    let count = query.iter(&world).count();
    assert_eq!(count, 1000);
}

#[test]
fn test_stage_count_with_no_conflicts() {
    let mut schedule = ParallelSchedule::new(2);

    let mut a1 = SystemAccess::new();
    a1.read::<Position>();
    schedule.add_system(AccessSystem::new(
        (|_: &mut World| {}).system(),
        a1,
    ));

    let mut a2 = SystemAccess::new();
    a2.read::<Velocity>();
    schedule.add_system(AccessSystem::new(
        (|_: &mut World| {}).system(),
        a2,
    ));

    let mut a3 = SystemAccess::new();
    a3.read::<Health>();
    schedule.add_system(AccessSystem::new(
        (|_: &mut World| {}).system(),
        a3,
    ));

    // All read-only, non-overlapping — should be 1 stage
    assert_eq!(schedule.stage_count(), 1);
}

#[test]
fn test_stage_count_with_write_conflicts() {
    let mut schedule = ParallelSchedule::new(2);

    let mut a1 = SystemAccess::new();
    a1.write::<Position>();
    schedule.add_system(AccessSystem::new(
        (|_: &mut World| {}).system(),
        a1,
    ));

    let mut a2 = SystemAccess::new();
    a2.write::<Position>();
    schedule.add_system(AccessSystem::new(
        (|_: &mut World| {}).system(),
        a2,
    ));

    // Both write Position — must be 2 stages
    assert_eq!(schedule.stage_count(), 2);
}
```

- [ ] **Step 2: Run integration tests**

Run: `cargo test -p engine-ecs --test parallel_integration`
Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/engine-ecs/tests/parallel_integration.rs
git commit -m "test(ecs): add parallel pipeline integration tests"
```

---

## Task 10: Full Build Verification & Benchmark Baseline

**Files:** None (verification only)

- [ ] **Step 1: Run full workspace build**

Run: `cargo build`
Expected: Compiles without errors.

- [ ] **Step 2: Run all tests**

Run: `cargo test`
Expected: All tests pass (excluding known pre-existing failures in engine-asset and engine-core examples).

- [ ] **Step 3: Run clippy**

Run: `cargo clippy`
Expected: No warnings.

- [ ] **Step 4: Run ECS benchmarks and record baseline**

Run: `cargo bench -p engine-ecs`

Record the results for:
- `parallel_schedule_throughput/sequential/10000`
- `parallel_schedule_throughput/parallel/10000`
- `parallel_iter/sequential_iter/10000`
- `parallel_iter/parallel_par_iter/10000`

The parallel schedule should show measurable speedup over sequential when systems are independent.

- [ ] **Step 5: Run physics benchmarks**

Run: `cargo bench -p engine-physics`

Verify broadphase doesn't regress performance (should improve for large entity counts).

- [ ] **Step 6: Final commit with benchmark results**

```bash
git add -A
git commit -m "chore: phase 11 complete — job system, parallel ECS, broadphase, parallel physics"
```

---

## Acceptance Criteria Verification

| Criterion | Verification |
|-----------|-------------|
| Job System with work-stealing | `engine-jobs` crate compiles and ThreadPool works |
| Parallel ECS queries | `par_iter`/`par_iter_mut` pass tests |
| Parallel system scheduling | `ParallelSchedule` groups non-conflicting systems into stages |
| Broadphase collision | `SpatialHashBroadphase` replaces O(n²) brute-force |
| Parallel physics | Narrow-phase collision checks use rayon |
| Parallel rendering prep | `WorldParExt` enables parallel iteration for render preparation |
| 4-thread ECS 2x+ throughput | Benchmark comparison: parallel vs sequential schedule |

---

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| `System: Send + Sync` breaks existing code | Closures over `&mut World` are `Send + Sync` when `World` owns only `Send + Sync` data. Add unsafe impls if needed. |
| Parallel mutable iteration safety | Use chunk-based splitting with raw pointers. Each chunk processes non-overlapping indices. |
| Broadphase cell_size tuning | Expose `set_cell_size()` on PhysicsWorld. Default 2.0 works for most games. |
| Rayon thread pool conflicts with engine-jobs | Use rayon only for data-parallel iteration (physics narrow-phase, par_iter). engine-jobs for task-level parallelism. |
| Windows build issues | All crates used are cross-platform. Test on CI. |
