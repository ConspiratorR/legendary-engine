//! Complete integrated game example - uses ECS and input features.
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;
use engine_input::input_manager::InputManager;
use engine_input::keyboard::KeyCode;
use engine_math::Vec3;

#[derive(Debug, Clone)]
struct Position(Vec3);

#[derive(Debug, Clone)]
struct Velocity(Vec3);

#[derive(Debug, Clone)]
struct Player;

#[derive(Debug, Clone)]
struct Enemy;

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Health {
    current: f32,
    max: f32,
}

/// Integrated game plugin - combines ECS entities.
struct CompleteGamePlugin;

impl Plugin for CompleteGamePlugin {
    fn build(&self, app: &mut AppBuilder) {
        let world = app.world_mut();

        // Set up a player
        let player = world.spawn();
        world.add_component(player, Position(Vec3::new(0.0, 0.0, 0.0)));
        world.add_component(player, Velocity(Vec3::new(0.0, 0.0, 0.0)));
        world.add_component(player, Player);
        world.add_component(
            player,
            Health {
                current: 100.0,
                max: 100.0,
            },
        );

        // Set up the environment
        let ground = world.spawn();
        world.add_component(ground, Position(Vec3::new(0.0, -1.0, 0.0)));

        // Add some enemies
        for i in 0..5 {
            let angle = i as f32 * std::f32::consts::PI * 2.0 / 5.0;
            let enemy = world.spawn();
            world.add_component(
                enemy,
                Position(Vec3::new(angle.cos() * 8.0, angle.sin() * 8.0, 0.0)),
            );
            world.add_component(enemy, Velocity(Vec3::new(0.0, 0.0, 0.0)));
            world.add_component(enemy, Enemy);
            world.add_component(
                enemy,
                Health {
                    current: 50.0,
                    max: 50.0,
                },
            );
        }

        println!("Complete game initialized!");
        println!("  - 1 player entity");
        println!("  - 1 ground platform");
        println!("  - 5 enemy entities");
        println!("  - Input system active");
    }
}

fn main() {
    println!("=== RustEngine Complete Game Demo ===\n");
    println!("Features included:");
    println!("  Entity Component System (ECS)");
    println!("  Input system");
    println!("  Resource system\n");

    let mut app_builder = AppBuilder::new();
    app_builder.add_plugin(CompleteGamePlugin);
    let mut app = app_builder.build();

    println!("Starting simulation...\n");

    // Simulate 300 frames
    for frame in 0..300 {
        // Read input and update player velocity
        {
            let player_entities = app.world.component_entities::<Player>();
            if let Some(&player_idx) = player_entities.first() {
                let (pressed_w, pressed_s, pressed_a, pressed_d) = {
                    let input = app.world.get_resource::<InputManager>();
                    match input {
                        Some(i) => (
                            i.key_down(KeyCode::KeyW),
                            i.key_down(KeyCode::KeyS),
                            i.key_down(KeyCode::KeyA),
                            i.key_down(KeyCode::KeyD),
                        ),
                        None => (false, false, false, false),
                    }
                };

                let mut direction = Vec3::new(0.0, 0.0, 0.0);
                if pressed_w {
                    direction.y += 1.0;
                }
                if pressed_s {
                    direction.y -= 1.0;
                }
                if pressed_a {
                    direction.x -= 1.0;
                }
                if pressed_d {
                    direction.x += 1.0;
                }

                if let Some(vel) = app.world.get_by_index_mut::<Velocity>(player_idx) {
                    if direction.length_squared() > 0.0001 {
                        vel.0 = direction.normalize() * 5.0;
                    } else {
                        vel.0 = Vec3::new(0.0, 0.0, 0.0);
                    }
                }
            }
        }

        // Update positions
        {
            let indices = app.world.component_entities::<Velocity>();
            let moves: Vec<(u32, Vec3)> = indices
                .iter()
                .filter_map(|&idx| {
                    let vel = app.world.get_by_index::<Velocity>(idx)?;
                    Some((idx, vel.0 * 0.016))
                })
                .collect();
            for (idx, delta) in moves {
                if let Some(pos) = app.world.get_by_index_mut::<Position>(idx) {
                    pos.0 += delta;
                }
            }
        }

        // Print status updates periodically
        if frame % 60 == 0 {
            println!("Frame {:3}:", frame);

            // Count entities
            let player_count = app.world.component_entities::<Player>().len();
            let enemy_count = app.world.component_entities::<Enemy>().len();
            println!(
                "  Entities: {} players, {} enemies",
                player_count, enemy_count
            );

            // Show input state
            if let Some(input) = app.world.get_resource::<InputManager>() {
                let space = input.key_down(KeyCode::Space);
                println!("  Input: Space={}", space);
            }

            println!();
        }
    }

    println!("=== Demo complete! ===");
    println!("Ready for real-time rendering and actual gameplay!");
}
