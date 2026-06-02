use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use engine_ecs::access::SystemAccess;
use engine_ecs::par_iter::{par_iter, par_iter_mut};
use engine_ecs::query::{Query, QueryPair};
use engine_ecs::schedule::{ParallelSchedule, Schedule};
use engine_ecs::system::{AccessSystem, IntoSystem};
use engine_ecs::world::World;

#[derive(Clone, Copy)]
struct Position(f32, f32, f32);
#[allow(dead_code)]
#[derive(Clone, Copy)]
struct Velocity(f32, f32, f32);

fn bench_world_creation(c: &mut Criterion) {
    c.bench_function("world_creation", |b| {
        b.iter(World::new);
    });
}

fn bench_entity_spawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_spawn");

    for count in [1_000, 10_000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                let mut world = World::new();
                for _ in 0..count {
                    world.spawn();
                }
                world
            });
        });
    }

    group.finish();
}

fn bench_component_insertion(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_insertion");

    for count in [1_000, 10_000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter_batched(
                || {
                    let mut world = World::new();
                    let entities: Vec<_> = (0..count).map(|_| world.spawn()).collect();
                    (world, entities)
                },
                |(mut world, entities)| {
                    for &e in &entities {
                        world.add_component(e, Position(1.0, 2.0, 3.0));
                    }
                    world
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn bench_component_query_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_query");

    for count in [1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("single_component_iter", count),
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
            BenchmarkId::new("pair_component_iter", count),
            &count,
            |b, &count| {
                let mut world = World::new();
                for i in 0..count {
                    let e = world.spawn();
                    world.add_component(e, Position(i as f32, 0.0, 0.0));
                    world.add_component(e, Velocity(1.0, 0.0, 0.0));
                }
                let query = QueryPair::<Position, Velocity>::new();

                b.iter(|| {
                    let mut sum = 0.0f32;
                    for (pos, vel) in query.iter(&world) {
                        sum += pos.0 * vel.0;
                    }
                    sum
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("single_component_iter_mut", count),
            &count,
            |b, &count| {
                let mut world = World::new();
                for _ in 0..count {
                    let e = world.spawn();
                    world.add_component(e, Position(1.0, 2.0, 3.0));
                }
                let query = Query::<Position>::new();

                b.iter(|| {
                    for pos in query.iter_mut(&mut world) {
                        pos.0 += 1.0;
                    }
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
            BenchmarkId::new("sequential", count),
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

        group.bench_with_input(BenchmarkId::new("par_iter", count), &count, |b, &count| {
            let mut world = World::new();
            for _ in 0..count {
                let e = world.spawn();
                world.add_component(e, Position(1.0, 2.0, 3.0));
            }

            b.iter(|| {
                let sum = std::sync::atomic::AtomicU32::new(0);
                par_iter(&world, |pos: &Position| {
                    let bits = (pos.0 + pos.1 + pos.2).to_bits();
                    sum.fetch_add(bits, std::sync::atomic::Ordering::Relaxed);
                });
                sum.load(std::sync::atomic::Ordering::Relaxed)
            });
        });

        group.bench_with_input(
            BenchmarkId::new("par_iter_mut", count),
            &count,
            |b, &count| {
                let mut world = World::new();
                for _ in 0..count {
                    let e = world.spawn();
                    world.add_component(e, Position(1.0, 2.0, 3.0));
                }

                b.iter(|| {
                    par_iter_mut(&mut world, |pos: &mut Position| {
                        pos.0 += 1.0;
                    });
                });
            },
        );
    }

    group.finish();
}

fn bench_parallel_schedule(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_schedule");

    for count in [1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("sequential", count),
            &count,
            |b, &count| {
                let mut world = World::new();
                for _ in 0..count {
                    let e = world.spawn();
                    world.add_component(e, Position(0.0, 0.0, 0.0));
                }

                let mut schedule = Schedule::new();
                schedule.add_system(
                    (|world: &mut World| {
                        let q = Query::<Position>::new();
                        for _ in q.iter(world) {}
                    })
                    .system(),
                );
                schedule.add_system(
                    (|world: &mut World| {
                        let q = Query::<Position>::new();
                        for _ in q.iter(world) {}
                    })
                    .system(),
                );
                schedule.add_system(
                    (|world: &mut World| {
                        let q = Query::<Position>::new();
                        for _ in q.iter(world) {}
                    })
                    .system(),
                );

                b.iter(|| {
                    schedule.run(&mut world);
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("parallel", count), &count, |b, &count| {
            let mut world = World::new();
            for _ in 0..count {
                let e = world.spawn();
                world.add_component(e, Position(0.0, 0.0, 0.0));
            }

            let mut schedule = ParallelSchedule::new(4);
            for _ in 0..3 {
                let mut a = SystemAccess::new();
                a.read::<Position>();
                schedule.add_system(AccessSystem::new(
                    (|world: &mut World| {
                        let q = Query::<Position>::new();
                        for _ in q.iter(world) {}
                    })
                    .system(),
                    a,
                ));
            }

            b.iter(|| {
                schedule.run(&mut world);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_world_creation,
    bench_entity_spawn,
    bench_component_insertion,
    bench_component_query_iteration,
    bench_parallel_iter,
    bench_parallel_schedule,
);
criterion_main!(benches);
