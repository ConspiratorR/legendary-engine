use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use engine_ecs::access::SystemAccess;
use engine_ecs::query::Query;
use engine_ecs::schedule::ParallelSchedule;
use engine_ecs::system::{AccessSystem, IntoSystem};
use engine_ecs::world::World;
use std::sync::Arc;

#[derive(Clone, Copy)]
struct CompA(f32);
#[derive(Clone, Copy)]
struct CompB(f32);
#[derive(Clone, Copy)]
struct CompC(f32);
#[derive(Clone, Copy)]
struct CompD(f32);
#[derive(Clone, Copy)]
struct CompE(f32);
#[derive(Clone, Copy)]
struct CompF(f32);

const ENTITY_COUNT: usize = 10_000;

fn spawn_world(components: &[&str]) -> World {
    let mut world = World::new();
    for i in 0..ENTITY_COUNT {
        let e = world.spawn();
        for comp in components {
            match *comp {
                "A" => world.add_component(e, CompA(i as f32)),
                "B" => world.add_component(e, CompB(i as f32)),
                "C" => world.add_component(e, CompC(i as f32)),
                "D" => world.add_component(e, CompD(i as f32)),
                "E" => world.add_component(e, CompE(i as f32)),
                "F" => world.add_component(e, CompF(i as f32)),
                _ => {}
            }
        }
    }
    world
}

macro_rules! read_system {
    ($comp:ty) => {{
        let mut access = SystemAccess::new();
        access.read::<$comp>();
        AccessSystem::new(
            (|world: &mut World| {
                let q = Query::<$comp>::new();
                let mut sum = 0.0f32;
                for val in q.iter(world) {
                    sum += val.0;
                }
                std::hint::black_box(sum);
            })
            .system(),
            access,
        )
    }};
}

macro_rules! write_system {
    ($comp:ty) => {{
        let mut access = SystemAccess::new();
        access.write::<$comp>();
        AccessSystem::new(
            (|world: &mut World| {
                let q = Query::<$comp>::new();
                for val in q.iter_mut(world) {
                    val.0 += 1.0;
                }
            })
            .system(),
            access,
        )
    }};
}

// Scenario 1: Many independent systems (all read-only, different types).
// All fit in one stage and execute in parallel.
fn bench_independent_systems(c: &mut Criterion) {
    let mut group = c.benchmark_group("independent_systems_6_read");
    let pool = Arc::new(engine_jobs::ThreadPool::new(4));

    group.bench_function("rayon", |b| {
        b.iter_batched(
            || {
                let mut schedule = ParallelSchedule::new(4);
                schedule.add_system(read_system!(CompA));
                schedule.add_system(read_system!(CompB));
                schedule.add_system(read_system!(CompC));
                schedule.add_system(read_system!(CompD));
                schedule.add_system(read_system!(CompE));
                schedule.add_system(read_system!(CompF));
                (schedule, spawn_world(&["A", "B", "C", "D", "E", "F"]))
            },
            |(mut schedule, mut world)| {
                schedule.run(&mut world);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("jobs", |b| {
        let pool = Arc::clone(&pool);
        b.iter_batched(
            || {
                let mut schedule = ParallelSchedule::new(4);
                schedule.add_system(read_system!(CompA));
                schedule.add_system(read_system!(CompB));
                schedule.add_system(read_system!(CompC));
                schedule.add_system(read_system!(CompD));
                schedule.add_system(read_system!(CompE));
                schedule.add_system(read_system!(CompF));
                (schedule, spawn_world(&["A", "B", "C", "D", "E", "F"]))
            },
            |(mut schedule, mut world)| {
                schedule.run_with_jobs(&mut world, &pool);
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

// Scenario 2: Multi-stage dependency chain (strictly sequential).
// Each system writes the same type, forcing 3 sequential stages.
fn bench_dependency_chain(c: &mut Criterion) {
    let mut group = c.benchmark_group("dependency_chain_3_stages");
    let pool = Arc::new(engine_jobs::ThreadPool::new(4));

    let make_chain_schedule = || {
        let mut schedule = ParallelSchedule::new(4);
        // Stage 1: write A
        let mut a1 = SystemAccess::new();
        a1.write::<CompA>();
        schedule.add_system(AccessSystem::new(
            (|world: &mut World| {
                let q = Query::<CompA>::new();
                for val in q.iter_mut(world) {
                    val.0 += 1.0;
                }
            })
            .system(),
            a1,
        ));
        // Stage 2: write A (conflicts with stage 1)
        let mut a2 = SystemAccess::new();
        a2.write::<CompA>();
        schedule.add_system(AccessSystem::new(
            (|world: &mut World| {
                let q = Query::<CompA>::new();
                for val in q.iter_mut(world) {
                    val.0 *= 2.0;
                }
            })
            .system(),
            a2,
        ));
        // Stage 3: write A (conflicts with stages 1 and 2)
        let mut a3 = SystemAccess::new();
        a3.write::<CompA>();
        schedule.add_system(AccessSystem::new(
            (|world: &mut World| {
                let q = Query::<CompA>::new();
                for val in q.iter_mut(world) {
                    val.0 -= 0.5;
                }
            })
            .system(),
            a3,
        ));
        schedule
    };

    group.bench_function("rayon", |b| {
        b.iter_batched(
            || (make_chain_schedule(), spawn_world(&["A"])),
            |(mut schedule, mut world)| {
                schedule.run(&mut world);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("jobs", |b| {
        let pool = Arc::clone(&pool);
        b.iter_batched(
            || (make_chain_schedule(), spawn_world(&["A"])),
            |(mut schedule, mut world)| {
                schedule.run_with_jobs(&mut world, &pool);
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

// Scenario 3: Mixed read/write (real ECS workload).
// Stage 1: parallel reads (A, B, C).
// Stage 2: parallel writes (D, E, F) — different types, no conflicts.
// Stage 3: read A + write B — B conflicts with stage 1 read on B.
fn bench_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_read_write_workload");
    let pool = Arc::new(engine_jobs::ThreadPool::new(4));

    let make_mixed_schedule = || {
        let mut schedule = ParallelSchedule::new(4);

        // Stage 1: parallel reads
        schedule.add_system(read_system!(CompA));
        schedule.add_system(read_system!(CompB));
        schedule.add_system(read_system!(CompC));

        // Stage 2: parallel writes (different types)
        schedule.add_system(write_system!(CompD));
        schedule.add_system(write_system!(CompE));
        schedule.add_system(write_system!(CompF));

        // Stage 3: read A + write B
        let mut a_read = SystemAccess::new();
        a_read.read::<CompA>();
        schedule.add_system(AccessSystem::new(
            (|world: &mut World| {
                let q = Query::<CompA>::new();
                let mut sum = 0.0f32;
                for val in q.iter(world) {
                    sum += val.0;
                }
                std::hint::black_box(sum);
            })
            .system(),
            a_read,
        ));

        let mut b_write = SystemAccess::new();
        b_write.write::<CompB>();
        schedule.add_system(AccessSystem::new(
            (|world: &mut World| {
                let q = Query::<CompB>::new();
                for val in q.iter_mut(world) {
                    val.0 += 10.0;
                }
            })
            .system(),
            b_write,
        ));

        schedule
    };

    group.bench_function("rayon", |b| {
        b.iter_batched(
            || {
                (
                    make_mixed_schedule(),
                    spawn_world(&["A", "B", "C", "D", "E", "F"]),
                )
            },
            |(mut schedule, mut world)| {
                schedule.run(&mut world);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("jobs", |b| {
        let pool = Arc::clone(&pool);
        b.iter_batched(
            || {
                (
                    make_mixed_schedule(),
                    spawn_world(&["A", "B", "C", "D", "E", "F"]),
                )
            },
            |(mut schedule, mut world)| {
                schedule.run_with_jobs(&mut world, &pool);
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_independent_systems,
    bench_dependency_chain,
    bench_mixed_workload,
);
criterion_main!(benches);
