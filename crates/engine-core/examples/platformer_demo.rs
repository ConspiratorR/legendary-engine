//! 2D Platformer Demo — validates engine subsystems working together.
//!
//! Controls: Arrow keys / WASD to move, Space to jump, Escape to pause
//! Goal: Collect coins, avoid enemies, reach the flag

use engine_core::app::AppBuilder;
use engine_core::transform::Transform;
use engine_ecs::world::World;
use engine_framework::FrameworkPlugin;
use engine_input::input_manager::InputManager;
use engine_input::keyboard::KeyCode;
use engine_math::Vec2;
use engine_physics::Physics2DPlugin;
use engine_physics::physics_2d::{Collider2D, PhysicsWorld2D, RigidBody2D};

const PLAYER_SPEED: f32 = 200.0;
const JUMP_VELOCITY: f32 = 400.0;
const GRAVITY: f32 = -980.0;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let mut builder = AppBuilder::new();
    builder.add_plugin(FrameworkPlugin);
    builder.add_plugin(Physics2DPlugin);

    // Override gravity for platformer (pixels, not meters)
    {
        let pw = builder
            .world_mut()
            .get_resource_mut::<PhysicsWorld2D>()
            .unwrap();
        pw.gravity = Vec2::new(0.0, GRAVITY);
    }

    // Spawn level
    spawn_level(builder.world_mut());

    // Register systems
    builder.add_system(player_movement_system);
    builder.add_system(enemy_ai_system);
    builder.add_system(collectible_system);
    builder.add_system(goal_system);
    builder.add_system(death_zone_system);

    let mut app = builder.build();

    // Simulation loop (terminal-based ASCII rendering)
    for frame in 0..300u32 {
        // Simulate input
        {
            let input = app.world.get_resource_mut::<InputManager>().unwrap();
            // Simulate right arrow for first 100 frames, then left
            if frame == 0 {
                input.press(KeyCode::ArrowRight);
            }
            if frame == 100 {
                input.release(KeyCode::ArrowRight);
                input.press(KeyCode::ArrowLeft);
            }
            if frame == 200 {
                input.release(KeyCode::ArrowLeft);
            }
        }

        // Run all systems (app.run() calls input.update_frame() internally)
        app.run();

        // Render every 5th frame
        if frame % 5 == 0 {
            render_ascii(&app.world);
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    println!("Simulation complete.");
}

// ---------------------------------------------------------------------------
// Level setup
// ---------------------------------------------------------------------------

fn spawn_level(world: &mut World) {
    // Player
    let player = world.spawn();
    world.add_component(player, Transform::from_xyz(100.0, 200.0, 0.0));
    world.add_component(player, RigidBody2D::new_dynamic());
    world.add_component(player, Collider2D::aabb(14.0, 14.0));
    world.add_component(player, PlayerState::new());

    // Ground platform (wide)
    spawn_platform(world, 400.0, 50.0, 400.0, 20.0);

    // Floating platforms
    spawn_platform(world, 200.0, 200.0, 80.0, 10.0);
    spawn_platform(world, 400.0, 300.0, 80.0, 10.0);
    spawn_platform(world, 600.0, 250.0, 80.0, 10.0);

    // Coins
    spawn_coin(world, 200.0, 230.0);
    spawn_coin(world, 400.0, 330.0);
    spawn_coin(world, 600.0, 280.0);

    // Enemy
    spawn_enemy(world, 350.0, 80.0);

    // Goal
    spawn_goal(world, 700.0, 80.0);
}

fn spawn_platform(world: &mut World, x: f32, y: f32, half_w: f32, half_h: f32) {
    let e = world.spawn();
    world.add_component(e, Transform::from_xyz(x, y, 0.0));
    world.add_component(e, RigidBody2D::new_static());
    world.add_component(e, Collider2D::aabb(half_w, half_h));
    world.add_component(e, Platform);
}

fn spawn_coin(world: &mut World, x: f32, y: f32) {
    let e = world.spawn();
    world.add_component(e, Transform::from_xyz(x, y, 0.0));
    world.add_component(e, Collider2D::trigger(10.0, 10.0));
    world.add_component(e, Collectible { collected: false });
}

fn spawn_enemy(world: &mut World, x: f32, y: f32) {
    let e = world.spawn();
    world.add_component(e, Transform::from_xyz(x, y, 0.0));
    world.add_component(e, RigidBody2D::new_dynamic());
    world.add_component(e, Collider2D::aabb(12.0, 12.0));
    world.add_component(
        e,
        EnemyAI {
            patrol_dir: 1.0,
            patrol_range: 100.0,
            spawn_x: x,
            speed: 80.0,
        },
    );
}

fn spawn_goal(world: &mut World, x: f32, y: f32) {
    let e = world.spawn();
    world.add_component(e, Transform::from_xyz(x, y, 0.0));
    world.add_component(e, Collider2D::trigger(16.0, 32.0));
    world.add_component(e, GoalMarker);
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct PlayerState {
    lives: i32,
    score: i32,
}

impl PlayerState {
    fn new() -> Self {
        Self { lives: 3, score: 0 }
    }
}

#[derive(Debug, Clone)]
struct Platform;

#[derive(Debug, Clone)]
struct Collectible {
    collected: bool,
}

#[derive(Debug, Clone)]
struct EnemyAI {
    patrol_dir: f32,
    patrol_range: f32,
    spawn_x: f32,
    speed: f32,
}

#[derive(Debug, Clone)]
struct GoalMarker;

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn player_movement_system(world: &mut World) {
    let entities: Vec<u32> = world.component_entities::<PlayerState>();

    for &eid in &entities {
        // Check grounded state
        let grounded = world
            .get_by_index::<RigidBody2D>(eid)
            .map(|b| b.grounded)
            .unwrap_or(false);

        // Read input
        let (left, right, jump) = {
            let input = match world.get_resource::<InputManager>() {
                Some(i) => i,
                None => return,
            };
            (
                input.key_down(KeyCode::ArrowLeft) || input.key_down(KeyCode::KeyA),
                input.key_down(KeyCode::ArrowRight) || input.key_down(KeyCode::KeyD),
                input.key_just_pressed(KeyCode::Space) || input.key_just_pressed(KeyCode::ArrowUp),
            )
        };

        // Apply horizontal velocity
        if let Some(body) = world.get_by_index_mut::<RigidBody2D>(eid) {
            body.velocity.x = if left {
                -PLAYER_SPEED
            } else if right {
                PLAYER_SPEED
            } else {
                0.0
            };

            // Jump
            if jump && grounded {
                body.velocity.y = JUMP_VELOCITY;
            }
        }
    }
}

fn enemy_ai_system(world: &mut World) {
    let entities: Vec<u32> = world.component_entities::<EnemyAI>();

    for &eid in &entities {
        let (patrol_dir, patrol_range, spawn_x, speed) = {
            let ai = world.get_by_index::<EnemyAI>(eid).unwrap();
            (ai.patrol_dir, ai.patrol_range, ai.spawn_x, ai.speed)
        };

        let mut new_dir = patrol_dir;
        if let Some(transform) = world.get_by_index::<Transform>(eid) {
            let dx = transform.position.x - spawn_x;
            if dx > patrol_range {
                new_dir = -1.0;
            } else if dx < -patrol_range {
                new_dir = 1.0;
            }
        }

        if let Some(ai) = world.get_by_index_mut::<EnemyAI>(eid) {
            ai.patrol_dir = new_dir;
        }

        if let Some(body) = world.get_by_index_mut::<RigidBody2D>(eid) {
            body.velocity.x = new_dir * speed;
        }
    }
}

fn collectible_system(world: &mut World) {
    let contacts: Vec<(u32, u32)> = {
        let pw = match world.get_resource::<PhysicsWorld2D>() {
            Some(pw) => pw,
            None => return,
        };
        pw.contacts
            .iter()
            .filter(|c| c.is_trigger)
            .map(|c| (c.entity_a, c.entity_b))
            .collect()
    };

    for (a, b) in contacts {
        let (player_eid, collectible_eid) = if world.get_by_index::<PlayerState>(a).is_some() {
            (a, b)
        } else if world.get_by_index::<PlayerState>(b).is_some() {
            (b, a)
        } else {
            continue;
        };

        let already_collected = world
            .get_by_index::<Collectible>(collectible_eid)
            .map(|c| c.collected)
            .unwrap_or(true);

        if !already_collected {
            if let Some(col) = world.get_by_index_mut::<Collectible>(collectible_eid) {
                col.collected = true;
            }
            if let Some(player) = world.get_by_index_mut::<PlayerState>(player_eid) {
                player.score += 10;
                println!("Coin collected! Score: {}", player.score);
            }
        }
    }
}

fn goal_system(world: &mut World) {
    let contacts: Vec<(u32, u32)> = {
        let pw = match world.get_resource::<PhysicsWorld2D>() {
            Some(pw) => pw,
            None => return,
        };
        pw.contacts
            .iter()
            .filter(|c| c.is_trigger)
            .map(|c| (c.entity_a, c.entity_b))
            .collect()
    };

    for (a, b) in contacts {
        let has_player = world.get_by_index::<PlayerState>(a).is_some()
            || world.get_by_index::<PlayerState>(b).is_some();
        let has_goal = world.get_by_index::<GoalMarker>(a).is_some()
            || world.get_by_index::<GoalMarker>(b).is_some();

        if has_player && has_goal {
            println!("*** LEVEL COMPLETE! ***");
        }
    }
}

fn death_zone_system(world: &mut World) {
    let entities: Vec<u32> = world.component_entities::<PlayerState>();

    for &eid in &entities {
        let fell = world
            .get_by_index::<Transform>(eid)
            .map(|t| t.position.y < -200.0)
            .unwrap_or(false);

        if fell {
            if let Some(t) = world.get_by_index_mut::<Transform>(eid) {
                t.position.x = 100.0;
                t.position.y = 200.0;
            }
            if let Some(body) = world.get_by_index_mut::<RigidBody2D>(eid) {
                body.velocity = Vec2::ZERO;
            }
            if let Some(player) = world.get_by_index_mut::<PlayerState>(eid) {
                player.lives -= 1;
                println!("Fell! Lives: {}", player.lives);
                if player.lives <= 0 {
                    println!("*** GAME OVER ***");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ASCII Renderer (terminal validation)
// ---------------------------------------------------------------------------

fn render_ascii(world: &World) {
    let width = 80usize;
    let height = 25usize;
    let tile_size = 32.0f32;
    let mut buffer = vec![vec![' '; width]; height];

    // Render platforms
    let platforms: Vec<u32> = world.component_entities::<Platform>();
    for &eid in &platforms {
        if let Some(transform) = world.get_by_index::<Transform>(eid)
            && let Some(collider) = world.get_by_index::<Collider2D>(eid)
        {
            let aabb = collider.world_aabb(Vec2::new(transform.position.x, transform.position.y));
            let x1 = ((aabb.min.x / tile_size) as i32).max(0) as usize;
            let x2 = ((aabb.max.x / tile_size) as i32).min(width as i32 - 1) as usize;
            let y1 = ((aabb.min.y / tile_size) as i32).max(0) as usize;
            let y2 = ((aabb.max.y / tile_size) as i32).min(height as i32 - 1) as usize;
            for row in buffer.iter_mut().take(y2 + 1).skip(y1) {
                for cell in row.iter_mut().take(x2 + 1).skip(x1) {
                    *cell = '#';
                }
            }
        }
    }

    // Render coins
    let coins: Vec<u32> = world.component_entities::<Collectible>();
    for &eid in &coins {
        if let Some(col) = world.get_by_index::<Collectible>(eid)
            && col.collected
        {
            continue;
        }
        if let Some(transform) = world.get_by_index::<Transform>(eid) {
            let x = ((transform.position.x / tile_size) as i32)
                .max(0)
                .min(width as i32 - 1) as usize;
            let y = ((transform.position.y / tile_size) as i32)
                .max(0)
                .min(height as i32 - 1) as usize;
            buffer[y][x] = '$';
        }
    }

    // Render enemies
    let enemies: Vec<u32> = world.component_entities::<EnemyAI>();
    for &eid in &enemies {
        if let Some(transform) = world.get_by_index::<Transform>(eid) {
            let x = ((transform.position.x / tile_size) as i32)
                .max(0)
                .min(width as i32 - 1) as usize;
            let y = ((transform.position.y / tile_size) as i32)
                .max(0)
                .min(height as i32 - 1) as usize;
            buffer[y][x] = 'E';
        }
    }

    // Render goal
    let goals: Vec<u32> = world.component_entities::<GoalMarker>();
    for &eid in &goals {
        if let Some(transform) = world.get_by_index::<Transform>(eid) {
            let x = ((transform.position.x / tile_size) as i32)
                .max(0)
                .min(width as i32 - 1) as usize;
            let y = ((transform.position.y / tile_size) as i32)
                .max(0)
                .min(height as i32 - 1) as usize;
            buffer[y][x] = 'F';
        }
    }

    // Render player (last, on top)
    let players: Vec<u32> = world.component_entities::<PlayerState>();
    for &eid in &players {
        if let Some(transform) = world.get_by_index::<Transform>(eid) {
            let x = ((transform.position.x / tile_size) as i32)
                .max(0)
                .min(width as i32 - 1) as usize;
            let y = ((transform.position.y / tile_size) as i32)
                .max(0)
                .min(height as i32 - 1) as usize;
            buffer[y][x] = '@';
        }
    }

    // Print (flip Y for terminal — Y=0 at bottom)
    println!("\x1B[2J\x1B[H");
    for row in buffer.iter().rev() {
        println!("{}", row.iter().collect::<String>());
    }

    // HUD
    if let Some(&eid) = world.component_entities::<PlayerState>().first()
        && let Some(player) = world.get_by_index::<PlayerState>(eid)
    {
        println!("Score: {} | Lives: {}", player.score, player.lives);
    }
}
