use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use engine_ecs::query::{Query, QueryPair};
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

criterion_group!(
    benches,
    bench_world_creation,
    bench_entity_spawn,
    bench_component_insertion,
    bench_component_query_iteration,
);
criterion_main!(benches);
