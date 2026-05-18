//! Physics demo example - shows physics engine usage.
use engine_core::app::{App, AppBuilder};
use engine_core::plugin::Plugin;
use engine_core::time::Time;
use engine_ecs::world::World;
use engine_math::Vec3;
use engine_physics::{RigidBody, Collider, PhysicsWorld, PhysicsPlugin, BodyType};

/// Simple physics plugin that adds some basic systems.
struct PhysicsDemoPlugin;

impl Plugin for PhysicsDemoPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let world = app.world_mut();

        // Create a floor
        let floor = world.spawn();
        world.add_component(floor, RigidBody::new_static());
        world.add_component(floor, Collider::cuboid(50.0, 0.5, 50.0));

        // Create some cubes
        for i in 0..20 {
            let cube = world.spawn();
            let mut body = RigidBody::new_dynamic();
            body.set_linear_velocity(Vec3::new(
                (i as f32 * 0.5).sin() * 5.0,
                10.0 + i as f32,
                (i as f32 * 0.5).cos() * 5.0,
            ));
            world.add_component(cube, body);
            world.add_component(cube, Collider::cuboid(0.5, 0.5, 0.5));
        }

        println!("Physics demo initialized with 20 cubes!");
    }
}

fn main() {
    println!("=== RustEngine Physics Demo ===\n");

    let mut app = AppBuilder::new()
        .add_plugin(PhysicsPlugin)
        .add_plugin(PhysicsDemoPlugin)
        .build();

    println!("Running physics simulation...\n");

    // Simulate 500 frames
    for frame in 0..500 {
        app.run();
        
        if frame % 60 == 0 {
            // Check physics world
            let world = app.world();
            if let Some(physics_world) = world.get::<PhysicsWorld>() {
                println!(
                    "Frame {} - Gravity: ({:.1}, {:.1}, {:.1})",
                    frame,
                    physics_world.gravity.x,
                    physics_world.gravity.y,
                    physics_world.gravity.z
                );
            }
        }
    }

    println!("\n=== Simulation complete! ===");
}
