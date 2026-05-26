use engine_core::app::{App, AppBuilder};
use engine_core::plugin::Plugin;
use engine_core::time::Time;
use engine_ecs::query::QueryPair;
use engine_ecs::system::IntoSystem;
use engine_ecs::world::World;
use engine_input::InputManager;
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
struct Projectile;

#[derive(Debug, Clone)]
struct Collider {
    radius: f32,
}

#[derive(Debug, Clone)]
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
        // Initialize game entities
        let world = app.world_mut();

        // Spawn player
        let player = world.spawn();
        world.add_component(player, Position(Vec3::new(0.0, 0.0, 0.0)));
        world.add_component(player, Velocity(Vec3::new(0.0, 0.0, 0.0)));
        world.add_component(player, Player);
        world.add_component(player, Collider { radius: 0.5 });
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
            world.add_component(enemy, Collider { radius: 0.4 });
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

        // Add systems
        app.add_system(player_control_system());
        app.add_system(movement_system());
        app.add_system(enemy_ai_system());
        app.add_system(collision_system());
        app.add_system(score_system());
    }
}

// Player control system
fn player_control_system() -> impl IntoSystem {
    |world: &mut World| {
        if let Some(input) = world.get_resource::<InputManager>() {
            let mut query = QueryPair::<&mut Velocity, &Player>::new();
            for (vel, _) in query.iter_mut(world) {
                let mut direction = Vec3::new(0.0, 0.0, 0.0);

                if input.is_key_pressed(KeyCode::W) || input.is_key_pressed(KeyCode::Up) {
                    direction.y += 1.0;
                }
                if input.is_key_pressed(KeyCode::S) || input.is_key_pressed(KeyCode::Down) {
                    direction.y -= 1.0;
                }
                if input.is_key_pressed(KeyCode::A) || input.is_key_pressed(KeyCode::Left) {
                    direction.x -= 1.0;
                }
                if input.is_key_pressed(KeyCode::D) || input.is_key_pressed(KeyCode::Right) {
                    direction.x += 1.0;
                }

                // Normalize and apply speed
                if direction.length_squared() > 0.0001 {
                    vel.0 = direction.normalize() * 5.0;
                } else {
                    vel.0 = Vec3::new(0.0, 0.0, 0.0);
                }
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

        let mut query = QueryPair::<&mut Position, &Velocity>::new();
        for (pos, vel) in query.iter_mut(world) {
            pos.0 += vel.0 * delta_time;

            // Simple world bounds (keep player in arena)
            let bounds = 10.0;
            pos.0.x = pos.0.x.clamp(-bounds, bounds);
            pos.0.y = pos.0.y.clamp(-bounds, bounds);
        }
    }
}

// Enemy AI system
fn enemy_ai_system() -> impl IntoSystem {
    |world: &mut World| {
        // Find player position
        let mut player_pos = Vec3::new(0.0, 0.0, 0.0);
        let player_query = QueryPair::<&Position, &Player>::new();
        for (pos, _) in player_query.iter(world) {
            player_pos = pos.0;
        }

        // Update enemy velocities to move towards player
        let mut enemy_query = QueryPair::<&mut Velocity, &Enemy>::new();
        for (vel, _) in enemy_query.iter_mut(world) {
            let current_pos = {
                let query = QueryPair::<&Position, &Enemy>::new();
                if let Some((pos, _)) = query.iter(world).next() {
                    pos.0
                } else {
                    Vec3::new(0.0, 0.0, 0.0)
                }
            };

            let direction = (player_pos - current_pos).normalize();
            vel.0 = direction * 2.0;
        }
    }
}

// Collision system
fn collision_system() -> impl IntoSystem {
    |world: &mut World| {
        // Simple collision damage system
        let mut player_query = QueryPair::<&Position, &mut Health>::new();

        // First, collect all enemy positions and colliders
        let mut enemies = Vec::new();
        let enemy_query = QueryPair::<&Position, &Collider>::new();
        for (pos, collider) in enemy_query.iter(world) {
            enemies.push((pos.0, collider.radius));
        }

        // Check player against enemies
        for (player_pos, mut player_health) in player_query.iter_mut(world) {
            let player_radius = 0.5;
            for (enemy_pos, enemy_radius) in &enemies {
                let distance = (player_pos.0 - *enemy_pos).length();
                if distance < player_radius + enemy_radius {
                    // Collision detected! Apply damage over time
                    player_health.current -= 10.0 * 0.016; // 10 damage per second
                    if player_health.current < 0.0 {
                        player_health.current = 0.0;
                        println!("Player died! Game Over!");
                    }
                }
            }
        }
    }
}

// Score and game status system
fn score_system() -> impl IntoSystem {
    |world: &mut World| {
        // Update score and print game status periodically
        let mut query = QueryPair::<&mut Score, ()>::new();
        for (score, _) in query.iter_mut(world) {
            // Increase score over time
            score.value += 1;

            // Only print every 60 frames
            if score.value % 60 == 0 {
                // Find player health
                let mut player_health = 100.0;
                let health_query = QueryPair::<&Health, &Player>::new();
                for (health, _) in health_query.iter(world) {
                    player_health = health.current;
                }

                println!(
                    "Score: {}, Player Health: {:.1}",
                    score.value, player_health
                );
            }
        }
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
    for frame in 1..=300 {
        app.run();
    }

    println!("\n=== Game Over! ===");
    println!("Thanks for playing!");
}
