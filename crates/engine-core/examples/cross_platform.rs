//! Cross-platform example demonstrating engine capabilities.
//!
//! This example runs on Windows, macOS, and Linux without platform-specific code.

use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;
use engine_core::plugins::CorePlugins;
use engine_ecs::query::QueryPair;
use engine_ecs::system::IntoSystem;
use engine_ecs::world::World;
use engine_math::Vec3;

// Components
struct Position(Vec3);
struct Velocity(Vec3);
struct Health(f32);

// Plugin that sets up the cross-platform demo
struct CrossPlatformPlugin;

impl Plugin for CrossPlatformPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let world = app.world_mut();

        // Create some entities
        let player = world.spawn();
        world.add_component(player, Position(Vec3::new(0.0, 0.0, 0.0)));
        world.add_component(player, Velocity(Vec3::new(1.0, 0.5, 0.0)));
        world.add_component(player, Health(100.0));

        let enemy = world.spawn();
        world.add_component(enemy, Position(Vec3::new(10.0, 5.0, 0.0)));
        world.add_component(enemy, Velocity(Vec3::new(-0.5, -0.25, 0.0)));
        world.add_component(enemy, Health(50.0));

        // Add systems
        app.add_system(movement_system());
        app.add_system(health_system());
        app.add_system(print_system());
    }
}

fn movement_system() -> impl IntoSystem {
    |world: &mut World| {
        let query = QueryPair::<Position, Velocity>::new();
        for (pos, vel) in query.iter_mut(world) {
            pos.0 += vel.0 * 0.016; // 60 FPS timestep
        }
    }
}

fn health_system() -> impl IntoSystem {
    |world: &mut World| {
        let query = QueryPair::<Position, Health>::new();
        for (pos, health) in query.iter_mut(world) {
            // Simple health regeneration based on position
            if pos.0.x > 5.0 {
                health.0 = (health.0 + 0.1).min(100.0);
            }
        }
    }
}

fn print_system() -> impl IntoSystem {
    |world: &mut World| {
        let query = QueryPair::<Position, Health>::new();
        for (pos, health) in query.iter(world) {
            println!(
                "Position: ({:.1}, {:.1}, {:.1}), Health: {:.1}",
                pos.0.x, pos.0.y, pos.0.z, health.0
            );
        }
    }
}

fn main() {
    println!("=== RustEngine Cross-Platform Example ===");
    println!("Platform: {}", std::env::consts::OS);
    println!("Architecture: {}", std::env::consts::ARCH);
    println!();

    let mut app_builder = AppBuilder::new();
    app_builder.add_plugin(CorePlugins);
    app_builder.add_plugin(CrossPlatformPlugin);
    let mut app = app_builder.build();

    println!("Running 3 frames of simulation...\n");

    for frame in 1..=3 {
        println!("--- Frame {} ---", frame);
        app.run();
        println!();
    }

    println!("=== Example Complete ===");
    println!("Successfully ran on {}!", std::env::consts::OS);
}
