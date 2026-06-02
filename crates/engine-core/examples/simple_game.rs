use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;
use engine_core::time::Time;
use engine_ecs::query::QueryPair;
use engine_ecs::system::IntoSystem;
use engine_ecs::world::World;
use engine_input::input_manager::InputManager;
use engine_input::keyboard::KeyCode;
use engine_math::Vec3;

// Game components
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

#[derive(Debug, Clone)]
struct Score {
    value: i32,
}

// Game plugin
struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut AppBuilder) {
        let world = app.world_mut();

        // Spawn player
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

        // Spawn some enemies
        for i in 0..5 {
            let enemy = world.spawn();
            let angle = i as f32 * std::f32::consts::PI * 2.0 / 5.0;
            let distance = 8.0;
            world.add_component(
                enemy,
                Position(Vec3::new(
                    angle.cos() * distance,
                    angle.sin() * distance,
                    0.0,
                )),
            );
            world.add_component(
                enemy,
                Velocity(Vec3::new(-angle.sin() * 0.5, angle.cos() * 0.5, 0.0)),
            );
            world.add_component(enemy, Enemy);
            world.add_component(
                enemy,
                Health {
                    current: 50.0,
                    max: 50.0,
                },
            );
        }

        // Spawn score tracker
        let score_entity = world.spawn();
        world.add_component(score_entity, Score { value: 0 });

        // Insert Time resource
        world.insert_resource(Time::new());

        // Add systems
        app.add_system(player_control_system());
        app.add_system(movement_system());
        app.add_system(enemy_ai_system());
        app.add_system(score_system());
    }
}

// Player control system
fn player_control_system() -> impl IntoSystem {
    |world: &mut World| {
        let (pressed_w, pressed_s, pressed_a, pressed_d) = {
            if let Some(input) = world.get_resource::<InputManager>() {
                (
                    input.key_down(KeyCode::KeyW) || input.key_down(KeyCode::ArrowUp),
                    input.key_down(KeyCode::KeyS) || input.key_down(KeyCode::ArrowDown),
                    input.key_down(KeyCode::KeyA) || input.key_down(KeyCode::ArrowLeft),
                    input.key_down(KeyCode::KeyD) || input.key_down(KeyCode::ArrowRight),
                )
            } else {
                (false, false, false, false)
            }
        };

        let query = QueryPair::<&mut Velocity, &Player>::new();
        for (vel, _) in query.iter_mut(world) {
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

            // Apply speed
            if direction.length_squared() > 0.0001 {
                vel.0 = direction.normalize() * 5.0;
            } else {
                vel.0 = Vec3::new(0.0, 0.0, 0.0);
            }
        }
    }
}

// Movement system
fn movement_system() -> impl IntoSystem {
    |world: &mut World| {
        let delta_time = if let Some(time) = world.get_resource::<Time>() {
            time.delta_seconds()
        } else {
            0.016
        };

        let query = QueryPair::<&mut Position, &Velocity>::new();
        for (pos, vel) in query.iter_mut(world) {
            pos.0 += vel.0 * delta_time;

            // Simple world bounds (keep player in arena)
            let bounds = 10.0;
            pos.0.x = pos.0.x.clamp(-bounds, bounds);
            pos.0.y = pos.0.y.clamp(-bounds, bounds);
        }
    }
}

// Enemy AI system - collect positions first, then update velocities
fn enemy_ai_system() -> impl IntoSystem {
    |world: &mut World| {
        // Find player position
        let mut player_pos = Vec3::new(0.0, 0.0, 0.0);
        let player_query = QueryPair::<&Position, &Player>::new();
        for (pos, _) in player_query.iter(world) {
            player_pos = pos.0;
        }

        // Collect enemy positions (separate from velocity update to avoid borrow conflict)
        let enemy_positions: Vec<Vec3> = {
            let pos_query = QueryPair::<&Position, &Enemy>::new();
            pos_query.iter(world).map(|(pos, _)| pos.0).collect()
        };

        // Update enemy velocities
        let vel_query = QueryPair::<&mut Velocity, &Enemy>::new();
        for (i, (vel, _)) in vel_query.iter_mut(world).enumerate() {
            if let Some(&current_pos) = enemy_positions.get(i) {
                let to_player = player_pos - current_pos;
                if to_player.length_squared() > 0.0001 {
                    vel.0 = to_player.normalize() * 2.0;
                }
            }
        }
    }
}

// Score system - track score and print status
fn score_system() -> impl IntoSystem {
    |world: &mut World| {
        // Collect player health first (immutable borrows)
        let player_health: f32 = {
            let health_query = QueryPair::<&Health, &Player>::new();
            health_query
                .iter(world)
                .map(|(h, _)| h.current)
                .next()
                .unwrap_or(100.0)
        };

        // Update score (mutable borrow, released before next immutable borrows)
        let should_print = {
            let query = QueryPair::<&mut Score, ()>::new();
            let mut printed = false;
            for (score, _) in query.iter_mut(world) {
                score.value += 1;
                if score.value % 60 == 0 {
                    println!(
                        "Score: {}, Player Health: {:.1}",
                        score.value, player_health
                    );
                    printed = true;
                }
            }
            printed
        };
        let _ = should_print;
    }
}

pub fn main() {
    println!("=== RustEngine Simple Game Example ===\n");
    println!("Controls:");
    println!("  W/Up    - Move Up");
    println!("  S/Down  - Move Down");
    println!("  A/Left  - Move Left");
    println!("  D/Right - Move Right");
    println!("\nObjective: Avoid enemies and survive as long as possible!\n");

    let mut app_builder = AppBuilder::new();
    app_builder.add_plugin(GamePlugin);
    let mut app = app_builder.build();

    println!("Game Starting...\n");

    // Run 300 frames (5 seconds at 60fps)
    for _frame in 1..=300 {
        app.run();
    }

    println!("\n=== Game Over! ===");
    println!("Thanks for playing!");
}
