use engine_ecs::access::SystemAccess;
use engine_ecs::par_iter::WorldParExt;
use engine_ecs::query::Query;
use engine_ecs::schedule::{ParallelSchedule, RayonExecutor};
use engine_ecs::system::{AccessSystem, IntoSystem};
use engine_ecs::world::World;

#[allow(dead_code)]
struct Position(f32, f32, f32);
#[allow(dead_code)]
struct Velocity(f32, f32, f32);
#[allow(dead_code)]
struct Health(i32);

#[test]
fn test_parallel_schedule_with_mixed_systems() {
    let mut world = World::new();

    for i in 0..100 {
        let e = world.spawn();
        world.add_component(e, Position(i as f32, 0.0, 0.0));
        world.add_component(e, Velocity(1.0, 0.0, 0.0));
        world.add_component(e, Health(100));
    }

    let mut schedule = ParallelSchedule::new(4);

    // System 1: reads Position, writes Velocity
    let mut access_physics = SystemAccess::new();
    access_physics.read::<Position>();
    access_physics.write::<Velocity>();
    schedule.add_system(AccessSystem::new(
        (|world: &mut World| {
            let query = Query::<Velocity>::new();
            for vel in query.iter_mut(world) {
                vel.0 *= 0.99;
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
    let seq_count = query.iter(&world).count();

    // Parallel count
    let count = std::sync::atomic::AtomicUsize::new(0);
    world.par_for_each::<Position, _>(|_| {
        count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    });

    assert_eq!(seq_count, 1000);
    assert_eq!(count.load(std::sync::atomic::Ordering::Relaxed), 1000);
}

#[test]
fn test_stage_count_with_no_conflicts() {
    let mut schedule = ParallelSchedule::new(2);

    let mut a1 = SystemAccess::new();
    a1.read::<Position>();
    schedule.add_system(AccessSystem::new((|_: &mut World| {}).system(), a1));

    let mut a2 = SystemAccess::new();
    a2.read::<Velocity>();
    schedule.add_system(AccessSystem::new((|_: &mut World| {}).system(), a2));

    let mut a3 = SystemAccess::new();
    a3.read::<Health>();
    schedule.add_system(AccessSystem::new((|_: &mut World| {}).system(), a3));

    // All read-only, non-overlapping — should be 1 stage
    assert_eq!(schedule.stage_count(), 1);
}

#[test]
fn test_stage_count_with_write_conflicts() {
    let mut schedule = ParallelSchedule::new(2);

    let mut a1 = SystemAccess::new();
    a1.write::<Position>();
    schedule.add_system(AccessSystem::new((|_: &mut World| {}).system(), a1));

    let mut a2 = SystemAccess::new();
    a2.write::<Position>();
    schedule.add_system(AccessSystem::new((|_: &mut World| {}).system(), a2));

    // Both write Position — must be 2 stages
    assert_eq!(schedule.stage_count(), 2);
}

#[test]
fn test_stage_count_mixed_read_write() {
    let mut schedule = ParallelSchedule::new(2);

    // System 1: reads Position
    let mut a1 = SystemAccess::new();
    a1.read::<Position>();
    schedule.add_system(AccessSystem::new((|_: &mut World| {}).system(), a1));

    // System 2: writes Velocity (no conflict with Position read)
    let mut a2 = SystemAccess::new();
    a2.write::<Velocity>();
    schedule.add_system(AccessSystem::new((|_: &mut World| {}).system(), a2));

    // System 3: reads Position + writes Health
    let mut a3 = SystemAccess::new();
    a3.read::<Position>();
    a3.write::<Health>();
    schedule.add_system(AccessSystem::new((|_: &mut World| {}).system(), a3));

    // System 1 and 2 can run in parallel (no conflict)
    // System 3 reads Position (conflicts with nothing) and writes Health (conflicts with nothing)
    // But System 1 reads Position and System 3 reads Position — both reads, no conflict
    // System 2 writes Velocity and System 3 writes Health — different types, no conflict
    // All 3 can be in 1 stage
    assert_eq!(schedule.stage_count(), 1);
}

#[test]
fn test_run_with_executor_rayon() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    COUNTER.store(0, Ordering::SeqCst);

    let mut schedule = ParallelSchedule::new(4);

    let mut a1 = SystemAccess::new();
    a1.read::<Position>();
    schedule.add_system(AccessSystem::new(
        ((|_: &mut World| {
            COUNTER.fetch_add(1, Ordering::SeqCst);
        }) as fn(&mut World))
            .system(),
        a1,
    ));

    let mut a2 = SystemAccess::new();
    a2.read::<Velocity>();
    schedule.add_system(AccessSystem::new(
        ((|_: &mut World| {
            COUNTER.fetch_add(10, Ordering::SeqCst);
        }) as fn(&mut World))
            .system(),
        a2,
    ));

    let mut world = World::new();
    schedule.run_with_executor(&mut world, &RayonExecutor);

    assert_eq!(COUNTER.load(Ordering::SeqCst), 11);
}

#[cfg(feature = "jobs-backend")]
mod job_graph_tests {
    use super::*;
    use engine_ecs::schedule::JobGraphExecutor;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_job_graph_executor_runs_systems() {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        COUNTER.store(0, Ordering::SeqCst);

        let mut schedule = ParallelSchedule::new(4);

        let mut a1 = SystemAccess::new();
        a1.read::<Position>();
        schedule.add_system(AccessSystem::new(
            ((|_: &mut World| {
                COUNTER.fetch_add(1, Ordering::SeqCst);
            }) as fn(&mut World))
                .system(),
            a1,
        ));

        let mut a2 = SystemAccess::new();
        a2.read::<Velocity>();
        schedule.add_system(AccessSystem::new(
            ((|_: &mut World| {
                COUNTER.fetch_add(10, Ordering::SeqCst);
            }) as fn(&mut World))
                .system(),
            a2,
        ));

        let mut world = World::new();
        let pool = Arc::new(engine_jobs::ThreadPool::new(4));
        let executor = JobGraphExecutor::new(pool);
        schedule.run_with_executor(&mut world, &executor);

        assert_eq!(COUNTER.load(Ordering::SeqCst), 11);
    }

    #[test]
    fn test_run_with_jobs() {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        COUNTER.store(0, Ordering::SeqCst);

        let mut schedule = ParallelSchedule::new(4);

        let mut a1 = SystemAccess::new();
        a1.read::<Position>();
        schedule.add_system(AccessSystem::new(
            ((|_: &mut World| {
                COUNTER.fetch_add(1, Ordering::SeqCst);
            }) as fn(&mut World))
                .system(),
            a1,
        ));

        let mut a2 = SystemAccess::new();
        a2.read::<Velocity>();
        schedule.add_system(AccessSystem::new(
            ((|_: &mut World| {
                COUNTER.fetch_add(10, Ordering::SeqCst);
            }) as fn(&mut World))
                .system(),
            a2,
        ));

        let mut world = World::new();
        let pool = Arc::new(engine_jobs::ThreadPool::new(4));
        schedule.run_with_jobs(&mut world, &pool);

        assert_eq!(COUNTER.load(Ordering::SeqCst), 11);
    }

    #[test]
    fn test_job_graph_executor_mixed_systems() {
        let mut world = World::new();

        for i in 0..100 {
            let e = world.spawn();
            world.add_component(e, Position(i as f32, 0.0, 0.0));
            world.add_component(e, Velocity(1.0, 0.0, 0.0));
            world.add_component(e, Health(100));
        }

        let mut schedule = ParallelSchedule::new(4);

        // System 1: reads Position, writes Velocity
        let mut access_physics = SystemAccess::new();
        access_physics.read::<Position>();
        access_physics.write::<Velocity>();
        schedule.add_system(AccessSystem::new(
            (|world: &mut World| {
                let query = Query::<Velocity>::new();
                for vel in query.iter_mut(world) {
                    vel.0 *= 0.99;
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

        // Run multiple frames via JobGraph
        let pool = Arc::new(engine_jobs::ThreadPool::new(4));
        for _ in 0..10 {
            schedule.run_with_jobs(&mut world, &pool);
        }

        // Verify velocities were damped
        let vel = world.get_by_index::<Velocity>(0).unwrap();
        assert!(vel.0 < 1.0, "Velocity should be damped");
        assert!(vel.0 > 0.0, "Velocity should still be positive");
    }
}
