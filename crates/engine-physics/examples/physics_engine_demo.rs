//! Physics demo example - shows ECS-based physics simulation with collision detection.
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;
use engine_core::transform::Transform;
use engine_math::Vec3;
use engine_physics::{Collider, PhysicsPlugin, PhysicsWorld, RigidBody};

/// Simple physics plugin that spawns demo entities.
struct PhysicsDemoPlugin;

impl Plugin for PhysicsDemoPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let world = app.world_mut();

        // Create a floor (static box)
        let floor = world.spawn();
        world.add_component(floor, Transform::from_xyz(0.0, -0.5, 0.0));
        world.add_component(floor, RigidBody::new_static());
        world.add_component(floor, Collider::cuboid(50.0, 0.5, 50.0));

        // Create bouncing spheres
        for i in 0..10 {
            let sphere = world.spawn();
            let x = (i as f32 - 4.5) * 2.0;
            world.add_component(sphere, Transform::from_xyz(x, 5.0 + i as f32 * 3.0, 0.0));

            let mut body = RigidBody::new_dynamic();
            body.linear_velocity = Vec3::new(
                (i as f32 * 0.5).sin() * 3.0,
                10.0 + i as f32 * 2.0,
                (i as f32 * 0.5).cos() * 3.0,
            );
            world.add_component(sphere, body);

            let mut collider = Collider::sphere(0.5);
            collider.restitution = 0.8;
            world.add_component(sphere, collider);
        }

        // Create falling cubes
        for i in 0..10 {
            let cube = world.spawn();
            let x = (i as f32 - 4.5) * 2.0;
            world.add_component(cube, Transform::from_xyz(x, 15.0 + i as f32 * 2.0, 3.0));

            let mut body = RigidBody::new_dynamic();
            body.linear_velocity = Vec3::new(0.0, 5.0 + i as f32, 0.0);
            world.add_component(cube, body);

            let mut collider = Collider::cuboid(0.5, 0.5, 0.5);
            collider.restitution = 0.3;
            world.add_component(cube, collider);
        }

        println!("Physics demo initialized: 10 spheres + 10 cubes on a floor!");
    }
}

fn main() {
    println!("=== RustEngine Physics Demo ===\n");

    let mut app_builder = AppBuilder::new();
    app_builder.add_plugin(PhysicsPlugin);
    app_builder.add_plugin(PhysicsDemoPlugin);
    let mut app = app_builder.build();

    println!("Running physics simulation (60 frames/second)...\n");

    // Simulate 300 frames (5 seconds)
    for frame in 0..300 {
        // Step physics manually since the system runs in schedule
        app.run();

        if frame % 60 == 0 {
            let pw = app.world.get_resource::<PhysicsWorld>().unwrap();
            let sec = frame as f32 / 60.0;
            println!(
                "t={:.1}s | Bodies: {} | Colliders: {} | Collisions: {}",
                sec,
                pw.body_count,
                pw.collider_count,
                pw.collisions.len()
            );

            // Show a few body positions
            let bodies = app.world.component_entities::<RigidBody>();
            for &idx in bodies.iter().take(3) {
                if let Some(transform) = app.world.get_by_index::<Transform>(idx) {
                    if let Some(body) = app.world.get_by_index::<RigidBody>(idx) {
                        let t = match body.body_type {
                            engine_physics::body::BodyType::Static => "static",
                            engine_physics::body::BodyType::Dynamic => "dynamic",
                            engine_physics::body::BodyType::Kinematic => "kinematic",
                        };
                        println!(
                            "  [{:?}] pos=({:.2}, {:.2}, {:.2}) vel=({:.2}, {:.2}, {:.2})",
                            t,
                            transform.Position().x,
                            transform.Position().y,
                            transform.Position().z,
                            body.linear_velocity.x,
                            body.linear_velocity.y,
                            body.linear_velocity.z
                        );
                    }
                }
            }
        }
    }

    println!("\n=== Simulation complete! ===");
}
