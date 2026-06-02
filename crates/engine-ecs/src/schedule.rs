use crate::access::SystemAccess;
use crate::system::System;
use crate::world::World;
use rayon::prelude::*;
use std::sync::atomic::AtomicPtr;

/// Strategy trait for executing a batch of systems within a single stage.
///
/// Implementors control *how* systems are run (sequentially, rayon-parallel,
/// job-graph, etc.) while the schedule decides *which* systems go together.
pub trait ScheduleExecutor: Send + Sync {
    /// Execute the given `systems` against `world`.
    ///
    /// All systems in the slice are guaranteed non-conflicting by the
    /// schedule's access analysis.
    fn execute_stage(&self, systems: &[&dyn System], world: &mut World);
}

/// Default executor that runs systems in parallel via rayon.
///
/// Single-system stages skip the rayon overhead and run directly.
pub struct RayonExecutor;

/// Executor that routes stage systems through the engine-jobs [`JobGraph`].
///
/// Each stage invocation creates a fresh `JobGraph` with one node per system.
/// All systems in the stage are independent (no `add_after` edges), so they
/// execute concurrently via the shared [`ThreadPool`](engine_jobs::ThreadPool).
///
/// Requires the `jobs-backend` feature.
#[cfg(feature = "jobs-backend")]
pub struct JobGraphExecutor {
    pool: std::sync::Arc<engine_jobs::ThreadPool>,
}

#[cfg(feature = "jobs-backend")]
impl JobGraphExecutor {
    /// Create a new executor backed by the given thread pool.
    ///
    /// The pool is shared (via `Arc`) — the same pool can back both
    /// ECS scheduling and other engine subsystems (physics, asset loading).
    pub fn new(pool: std::sync::Arc<engine_jobs::ThreadPool>) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "jobs-backend")]
impl ScheduleExecutor for JobGraphExecutor {
    fn execute_stage(&self, systems: &[&dyn System], world: &mut World) {
        if systems.is_empty() {
            return;
        }

        // Sort systems by priority (descending) so higher-priority systems
        // are submitted to the job graph first and execute sooner.
        let mut sorted: Vec<&dyn System> = systems.to_vec();
        sorted.sort_by(|a, b| b.priority().cmp(&a.priority()));

        // SAFETY: Same invariant as RayonExecutor — access analysis guarantees
        // non-overlapping component access within a stage.
        let world_ptr = AtomicPtr::new(world as *mut World);

        if sorted.len() == 1 {
            let world_ref = unsafe { &mut *world_ptr.load(std::sync::atomic::Ordering::Relaxed) };
            sorted[0].run(world_ref);
        } else {
            let mut graph = engine_jobs::JobGraph::new();

            for system in &sorted {
                // SAFETY: see above
                let world_ref =
                    unsafe { &mut *world_ptr.load(std::sync::atomic::Ordering::Relaxed) };
                // SAFETY: the 'static bound on JobGraph::add requires owned data.
                // We transmute the lifetime of the system reference to 'static.
                // This is safe because:
                // 1. The World pointer is valid for the entire execute_stage call
                // 2. graph.execute(&pool) blocks until all jobs complete
                // 3. No job escapes this function scope
                let system_ref: &'static dyn System = unsafe { std::mem::transmute(*system) };
                let world_ref: &'static mut World = unsafe { std::mem::transmute(world_ref) };
                graph.add(move || {
                    system_ref.run(world_ref);
                });
            }

            graph.execute(&self.pool);
        }
    }
}

impl ScheduleExecutor for RayonExecutor {
    fn execute_stage(&self, systems: &[&dyn System], world: &mut World) {
        if systems.len() == 1 {
            systems[0].run(world);
        } else {
            // SAFETY: The access analysis guarantees that systems within a stage
            // do not access the same component types. Each system gets &mut World
            // but they operate on disjoint component sparse-sets. The AtomicPtr
            // allows us to hand out multiple &mut World references to parallel
            // systems without violating Rust's Send+Sync bounds.
            // Soundness depends entirely on the correctness of SystemAccess.
            let world_ptr = AtomicPtr::new(world as *mut World);
            systems.par_iter().for_each(|system| {
                let world_ref =
                    unsafe { &mut *world_ptr.load(std::sync::atomic::Ordering::Relaxed) };
                system.run(world_ref);
            });
        }
    }
}

/// An ordered list of [`System`]s executed sequentially.
///
/// Systems are run in the order they were added via [`add_system`](Self::add_system).
pub struct Schedule {
    systems: Vec<Box<dyn System>>,
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

impl Schedule {
    /// Create an empty schedule.
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    /// Append a system to the end of the schedule.
    pub fn add_system(&mut self, system: impl System + 'static) -> &mut Self {
        self.systems.push(Box::new(system));
        self
    }

    /// Run all systems in order against the given `world`.
    pub fn run(&self, world: &mut World) {
        for system in &self.systems {
            system.run(world);
        }
    }
}

/// A schedule that groups non-conflicting systems into parallel stages.
///
/// Systems are grouped into "stages" where all systems within a stage
/// have non-conflicting access descriptors and can run concurrently.
/// Stages execute sequentially.
///
/// Within each stage, the configured [`ScheduleExecutor`] runs the systems.
/// The default executor ([`RayonExecutor`]) runs them in parallel via rayon.
/// The access analysis ([`SystemAccess`]) guarantees that parallel systems
/// do not read/write the same component types simultaneously.
///
/// # Example
///
/// ```
/// use engine_ecs::schedule::ParallelSchedule;
/// use engine_ecs::system::{AccessSystem, IntoSystem};
/// use engine_ecs::access::SystemAccess;
/// use engine_ecs::world::World;
///
/// let mut schedule = ParallelSchedule::new(4);
///
/// let mut access = SystemAccess::new();
/// access.read::<f32>();
/// schedule.add_system(AccessSystem::new(
///     (|_: &mut World| {}).system(),
///     access,
/// ));
/// ```
pub struct ParallelSchedule {
    systems: Vec<Box<dyn System>>,
    stages: Vec<Vec<usize>>,
    needs_rebuild: bool,
}

impl ParallelSchedule {
    /// Create a new parallel schedule with the given number of threads.
    ///
    /// The `threads` parameter is reserved for future use with true parallel
    /// stage execution. Currently stages run sequentially but systems within
    /// a stage are grouped by access compatibility.
    pub fn new(_threads: usize) -> Self {
        Self {
            systems: Vec::new(),
            stages: Vec::new(),
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
            let mut stage = vec![i];
            assigned[i] = true;

            for j in (i + 1)..self.systems.len() {
                if assigned[j] {
                    continue;
                }
                let conflicts = stage
                    .iter()
                    .any(|&k| accesses[j].conflicts_with(&accesses[k]));
                if !conflicts {
                    stage.push(j);
                    assigned[j] = true;
                }
            }

            self.stages.push(stage);
        }

        self.needs_rebuild = false;
    }

    /// Run all systems, executing non-conflicting systems within each stage in parallel.
    ///
    /// Stages execute sequentially. Within a stage, systems run concurrently
    /// via the default [`RayonExecutor`]. The access analysis guarantees
    /// no data races on component storage.
    pub fn run(&mut self, world: &mut World) {
        self.run_with_executor(world, &RayonExecutor);
    }

    /// Run all systems using a custom [`ScheduleExecutor`].
    ///
    /// This is the generic execution path. Both [`run`](Self::run) (rayon)
    /// and `run_with_jobs` (JobGraph) delegate here.
    pub fn run_with_executor(&mut self, world: &mut World, executor: &dyn ScheduleExecutor) {
        if self.needs_rebuild {
            self.rebuild_stages();
        }

        for stage in &self.stages {
            let refs: Vec<&dyn System> = stage.iter().map(|&idx| &*self.systems[idx]).collect();
            executor.execute_stage(&refs, world);
        }
    }

    /// Run all systems using the engine-jobs [`JobGraph`] backend.
    ///
    /// Requires the `jobs-backend` feature. Each stage is converted into
    /// a `JobGraph` and executed via the provided [`ThreadPool`](engine_jobs::ThreadPool).
    /// The pool is shared — the same instance can back physics, asset loading, etc.
    ///
    /// # Panics
    ///
    /// Panics if the `jobs-backend` feature is not enabled.
    #[cfg(feature = "jobs-backend")]
    pub fn run_with_jobs(
        &mut self,
        world: &mut World,
        pool: &std::sync::Arc<engine_jobs::ThreadPool>,
    ) {
        let executor = JobGraphExecutor::new(std::sync::Arc::clone(pool));
        self.run_with_executor(world, &executor);
    }

    /// Return the number of systems in the schedule.
    pub fn system_count(&self) -> usize {
        self.systems.len()
    }

    /// Return the number of stages (sequential groups).
    pub fn stage_count(&mut self) -> usize {
        if self.needs_rebuild {
            self.rebuild_stages();
        }
        self.stages.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::{AccessSystem, IntoSystem};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn test_sequential_schedule() {
        let mut schedule = Schedule::new();
        schedule.add_system((|_: &mut World| {}).system());
        schedule.add_system((|_: &mut World| {}).system());

        let mut world = World::new();
        schedule.run(&mut world);
    }

    #[test]
    fn test_parallel_schedule_groups_non_conflicting() {
        let mut schedule = ParallelSchedule::new(4);

        // Two systems that read different types - should be in same stage
        let mut a1 = SystemAccess::new();
        a1.read::<f32>();
        schedule.add_system(AccessSystem::new((|_: &mut World| {}).system(), a1));

        let mut a2 = SystemAccess::new();
        a2.read::<i32>();
        schedule.add_system(AccessSystem::new((|_: &mut World| {}).system(), a2));

        assert_eq!(schedule.stage_count(), 1); // same stage
    }

    #[test]
    fn test_parallel_schedule_separates_conflicting() {
        let mut schedule = ParallelSchedule::new(4);

        // Two systems that write the same type - must be in different stages
        let mut a1 = SystemAccess::new();
        a1.write::<f32>();
        schedule.add_system(AccessSystem::new((|_: &mut World| {}).system(), a1));

        let mut a2 = SystemAccess::new();
        a2.write::<f32>();
        schedule.add_system(AccessSystem::new((|_: &mut World| {}).system(), a2));

        assert_eq!(schedule.stage_count(), 2); // different stages
    }

    #[test]
    fn test_parallel_schedule_read_write_conflict() {
        let mut schedule = ParallelSchedule::new(4);

        // One reads, one writes same type - conflict
        let mut a1 = SystemAccess::new();
        a1.read::<f32>();
        schedule.add_system(AccessSystem::new((|_: &mut World| {}).system(), a1));

        let mut a2 = SystemAccess::new();
        a2.write::<f32>();
        schedule.add_system(AccessSystem::new((|_: &mut World| {}).system(), a2));

        assert_eq!(schedule.stage_count(), 2);
    }

    #[test]
    fn test_parallel_schedule_reads_same_type_no_conflict() {
        let mut schedule = ParallelSchedule::new(4);

        // Both read same type - no conflict, same stage
        let mut a1 = SystemAccess::new();
        a1.read::<f32>();
        schedule.add_system(AccessSystem::new((|_: &mut World| {}).system(), a1));

        let mut a2 = SystemAccess::new();
        a2.read::<f32>();
        schedule.add_system(AccessSystem::new((|_: &mut World| {}).system(), a2));

        assert_eq!(schedule.stage_count(), 1);
    }

    #[test]
    fn test_parallel_schedule_actually_runs_systems() {
        COUNTER.store(0, Ordering::SeqCst);

        let mut schedule = ParallelSchedule::new(4);

        let mut a1 = SystemAccess::new();
        a1.read::<f32>();
        schedule.add_system(AccessSystem::new(
            ((|_: &mut World| {
                COUNTER.fetch_add(1, Ordering::SeqCst);
            }) as fn(&mut World))
                .system(),
            a1,
        ));

        let mut a2 = SystemAccess::new();
        a2.read::<i32>();
        schedule.add_system(AccessSystem::new(
            ((|_: &mut World| {
                COUNTER.fetch_add(10, Ordering::SeqCst);
            }) as fn(&mut World))
                .system(),
            a2,
        ));

        let mut world = World::new();
        schedule.run(&mut world);

        assert_eq!(COUNTER.load(Ordering::SeqCst), 11);
    }
}
