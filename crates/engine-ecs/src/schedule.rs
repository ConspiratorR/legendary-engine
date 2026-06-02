use crate::access::SystemAccess;
use crate::system::System;
use crate::world::World;
use rayon::prelude::*;
use std::sync::atomic::AtomicPtr;

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
/// Within each stage, systems execute in parallel using rayon. The
/// access analysis ([`SystemAccess`]) guarantees that parallel systems
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
    /// via rayon. The access analysis guarantees no data races on component storage.
    pub fn run(&mut self, world: &mut World) {
        if self.needs_rebuild {
            self.rebuild_stages();
        }

        // SAFETY: The access analysis guarantees that systems within a stage
        // do not access the same component types. Each system gets &mut World
        // but they operate on disjoint component sparse-sets. The AtomicPtr
        // allows us to hand out multiple &mut World references to parallel
        // systems without violating Rust's Send+Sync bounds.
        // Soundness depends entirely on the correctness of SystemAccess.
        let world_ptr = AtomicPtr::new(world as *mut World);

        for stage in &self.stages {
            if stage.len() == 1 {
                // Single system: no parallelism overhead
                let world_ref =
                    unsafe { &mut *world_ptr.load(std::sync::atomic::Ordering::Relaxed) };
                self.systems[stage[0]].run(world_ref);
            } else {
                // Multiple non-conflicting systems: run in parallel
                stage.par_iter().for_each(|&idx| {
                    // SAFETY: see above - access analysis guarantees non-overlapping types
                    let world_ref =
                        unsafe { &mut *world_ptr.load(std::sync::atomic::Ordering::Relaxed) };
                    self.systems[idx].run(world_ref);
                });
            }
        }
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
