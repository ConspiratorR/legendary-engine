use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use engine_core::transform::Transform;
use engine_ecs::world::World;
use engine_physics::body::RigidBody;
use engine_physics::collider::{Collider, check_obb_obb, check_sphere_sphere};
use engine_physics::world::PhysicsWorld;
use engine_math::{EulerRot, Quat, Vec3};

fn bench_sphere_sphere_collision(c: &mut Criterion) {
    let mut group = c.benchmark_group("sphere_sphere_collision");

    for count in [100, 1_000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let pairs: Vec<(Vec3, Vec3)> = (0..count)
                .map(|i| {
                    let offset = (i as f32) * 0.01;
                    (
                        Vec3::new(offset, 0.0, 0.0),
                        Vec3::new(offset + 1.5, 0.0, 0.0),
                    )
                })
                .collect();

            b.iter(|| {
                let mut hits = 0u32;
                for (pos_a, pos_b) in &pairs {
                    if check_sphere_sphere(*pos_a, 1.0, *pos_b, 1.0).is_some() {
                        hits += 1;
                    }
                }
                hits
            });
        });
    }

    group.finish();
}

fn bench_obb_obb_collision(c: &mut Criterion) {
    let mut group = c.benchmark_group("obb_obb_collision");

    for count in [100, 1_000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let pairs: Vec<(Vec3, Quat, Vec3, Vec3, Quat, Vec3)> = (0..count)
                .map(|i| {
                    let offset = (i as f32) * 0.01;
                    let rot = Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, offset);
                    (
                        Vec3::new(offset, 0.0, 0.0),
                        Quat::IDENTITY,
                        Vec3::splat(1.0),
                        Vec3::new(offset + 0.5, 0.0, 0.0),
                        rot,
                        Vec3::splat(1.0),
                    )
                })
                .collect();

            b.iter(|| {
                let mut hits = 0u32;
                for (pos_a, rot_a, half_a, pos_b, rot_b, half_b) in &pairs {
                    if check_obb_obb(*pos_a, *rot_a, *half_a, *pos_b, *rot_b, *half_b).is_some() {
                        hits += 1;
                    }
                }
                hits
            });
        });
    }

    group.finish();
}

fn bench_physics_world_step(c: &mut Criterion) {
    let mut group = c.benchmark_group("physics_world_step");

    for count in [10, 100] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter_batched(
                || {
                    let mut world = World::new();

                    for i in 0..count {
                        let e = world.spawn();
                        let x = (i as f32) * 3.0;
                        world.add_component(e, Transform::from_xyz(x, 10.0, 0.0));
                        world.add_component(e, RigidBody::new_dynamic());
                        world.add_component(e, Collider::sphere(0.5));
                    }

                    let floor = world.spawn();
                    world.add_component(floor, Transform::from_xyz(0.0, -0.5, 0.0));
                    world.add_component(floor, RigidBody::new_static());
                    world.add_component(floor, Collider::cuboid(500.0, 0.5, 500.0));

                    let mut pw = PhysicsWorld::new();
                    pw.sub_steps = 4;
                    pw.delta_time = 1.0 / 60.0;
                    (world, pw)
                },
                |(mut world, mut pw)| {
                    pw.step(&mut world);
                    (world, pw)
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_sphere_sphere_collision,
    bench_obb_obb_collision,
    bench_physics_world_step,
);
criterion_main!(benches);
