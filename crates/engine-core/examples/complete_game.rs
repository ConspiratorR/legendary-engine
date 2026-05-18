//! Complete integrated game example - uses all major engine features.
use engine_core::app::{App, AppBuilder};
use engine_core::plugin::Plugin;
use engine_core::time::Time;
use engine_ecs::world::World;
use engine_math::Vec3;
use engine_input::InputManager;
use engine_input::keyboard::KeyCode;
use engine_physics::{RigidBody, Collider, PhysicsWorld, PhysicsPlugin, BodyType};
use engine_network::{NetworkConfig, NetworkPlugin, NetworkMessage, Connection, ConnectionState};

/// Integrated game plugin - combines all features.
struct CompleteGamePlugin;

impl Plugin for CompleteGamePlugin {
    fn build(&self, app: &mut AppBuilder) {
        let world = app.world_mut();

        // Set up a player
        let player = world.spawn();
        world.add_component(player, RigidBody::new_dynamic());
        world.add_component(player, Collider::cuboid(0.5, 1.8, 0.5));

        // Set up the environment
        let ground = world.spawn();
        world.add_component(ground, RigidBody::new_static());
        world.add_component(ground, Collider::cuboid(10.0, 0.5, 10.0));

        // Add some enemies
        for i in 0..5 {
            let enemy = world.spawn();
            world.add_component(enemy, RigidBody::new_dynamic());
            world.add_component(enemy, Collider::sphere(0.5));
        }

        // Configure network for client mode
        if let Some(mut config) = world.get_mut::<NetworkConfig>() {
            config.is_server = false;
            config.port = 7777;
        }

        println!("✅ Complete game initialized!");
        println!("  - 1 player entity");
        println!("  - 1 ground platform");
        println!("  - 5 enemy entities");
        println!("  - Physics system active");
        println!("  - Network client mode configured");
    }
}

fn main() {
    println!("=== RustEngine Complete Game Demo ===\n");
    println!("Features included:");
    println!("  ✨ Entity Component System (ECS)");
    println!("  ⚡ Physics engine");
    println!("  🎮 Input system");
    println!("  🌐 Network support");
    println!("  📦 Resource system\n");

    let mut app = AppBuilder::new()
        .add_plugin(PhysicsPlugin)
        .add_plugin(NetworkPlugin)
        .add_plugin(CompleteGamePlugin)
        .build();

    println!("Starting simulation...\n");

    // Simulate 300 frames
    for frame in 0..300 {
        app.run();
        
        // Print status updates periodically
        if frame % 60 == 0 {
            let world = app.world();
            
            println!("Frame {:3}:", frame);
            
            if let Some(physics) = world.get::<PhysicsWorld>() {
                println!("  Physics: Active ({} bodies)", physics.body_count);
            }
            
            if let Some(config) = world.get::<NetworkConfig>() {
                println!("  Network: {} mode (port {})",
                    if config.is_server { "SERVER" } else { "CLIENT" },
                    config.port
                );
            }
            
            if let Some(input) = world.get::<InputManager>() {
                // Input manager is available
                let key_state = if input.is_key_pressed(KeyCode::Space) {
                    "SPACE pressed"
                } else {
                    "SPACE not pressed"
                };
                println!("  Input: {}", key_state);
            }
            
            println!();
        }
    }

    println!("=== Demo complete! ===");
    println!("🚀 Ready for real-time rendering and actual gameplay!");
}
