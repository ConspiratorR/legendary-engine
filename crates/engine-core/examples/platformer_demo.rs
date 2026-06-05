//! 2D Platformer Demo — validates engine subsystems working together.
//!
//! Controls: Arrow keys / WASD to move, Space to jump, Escape to pause
//! Goal: Collect coins, avoid enemies, reach the flag

// AppBuilder: top-level entry point that owns the ECS World, plugin list, and system schedule.
use engine_core::app::AppBuilder;
// Transform: standard position/rotation/scale component, stored per-entity.
use engine_core::transform::Transform;
// World: the ECS container — stores entities, components, and resources.
use engine_ecs::world::World;
// FrameworkPlugin: registers core engine systems (tick, time, etc.).
use engine_framework::FrameworkPlugin;
// InputManager: polled keyboard/mouse state, registered as a resource by the input plugin.
use engine_input::input_manager::InputManager;
// KeyCode: enum of all keyboard keys (engine_input re-exports winit keys).
use engine_input::keyboard::KeyCode;
// Vec2: 2D vector type used throughout physics and transforms.
use engine_math::Vec2;
// Physics2DPlugin: registers PhysicsWorld2D resource + physics step system.
use engine_physics::Physics2DPlugin;
// Collider2D: axis-aligned collider (AABB or trigger). RigidBody2D: dynamic/static body.
// PhysicsWorld2D: resource holding gravity, broadphase, and contact list.
use engine_physics::physics_2d::{Collider2D, PhysicsWorld2D, RigidBody2D};

// Tuning constants — pixel-space values (not SI meters).
const PLAYER_SPEED: f32 = 200.0;
const JUMP_VELOCITY: f32 = 400.0;
const GRAVITY: f32 = -980.0;

fn main() {
    // Initialize logging — respects RUST_LOG env var, defaults to "info" level.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // AppBuilder constructs the ECS World and manages plugin/system registration.
    let mut builder = AppBuilder::new();
    // FrameworkPlugin adds core systems (time tracking, frame counting).
    builder.add_plugin(FrameworkPlugin);
    // Physics2DPlugin registers PhysicsWorld2D as a resource and adds the physics step system.
    builder.add_plugin(Physics2DPlugin);

    // Override gravity for platformer (pixels, not meters)
    // get_resource_mut::<T>() accesses a single-instance resource by type.
    // PhysicsWorld2D was registered by Physics2DPlugin.
    {
        let pw = builder
            .world_mut()
            .get_resource_mut::<PhysicsWorld2D>()
            .unwrap();
        pw.gravity = Vec2::new(0.0, GRAVITY);
    }

    // Spawn level — populates the World with entities and their component bundles.
    spawn_level(builder.world_mut());

    // Register systems — each system is a fn(&mut World) that runs every frame.
    // Systems are executed in registration order.
    builder.add_system(player_movement_system);
    builder.add_system(enemy_ai_system);
    builder.add_system(collectible_system);
    builder.add_system(goal_system);
    builder.add_system(death_zone_system);

    // build() finalizes plugins and returns the App with its run loop ready.
    let mut app = builder.build();

    // Simulation loop (terminal-based ASCII rendering)
    // In a real game, this would be the engine's main loop with fixed timestep.
    for frame in 0..300u32 {
        // Simulate input — InputManager is a resource, so we access it via get_resource_mut.
        // In production, the input plugin reads winit events automatically.
        {
            let input = app.world.get_resource_mut::<InputManager>().unwrap();
            // Simulate right arrow for first 100 frames, then left
            // press()/release() simulate key state changes between frames.
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

        // Run all systems — app.run() advances physics, calls each registered system,
        // and updates the InputManager frame state (just_pressed → pressed, etc.).
        app.run();

        // Render every 5th frame to keep terminal output readable.
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
    // Player — dynamic body that responds to forces and collisions.
    let player = world.spawn();
    world.add_component(player, Transform::from_xyz(100.0, 200.0, 0.0));
    // new_dynamic() creates a body affected by gravity and collisions.
    world.add_component(player, RigidBody2D::new_dynamic());
    // aabb() creates a solid axis-aligned bounding box collider.
    world.add_component(player, Collider2D::aabb(14.0, 14.0));
    // PlayerState is a game-specific component (lives, score) — not part of the engine.
    world.add_component(player, PlayerState::new());

    // Ground platform (wide)
    spawn_platform(world, 400.0, 50.0, 400.0, 20.0);

    // Floating platforms
    spawn_platform(world, 200.0, 200.0, 80.0, 10.0);
    spawn_platform(world, 400.0, 300.0, 80.0, 10.0);
    spawn_platform(world, 600.0, 250.0, 80.0, 10.0);

    // Coins — trigger colliders detect overlap without blocking movement.
    spawn_coin(world, 200.0, 230.0);
    spawn_coin(world, 400.0, 330.0);
    spawn_coin(world, 600.0, 280.0);

    // Enemy
    spawn_enemy(world, 350.0, 80.0);

    // Goal
    spawn_goal(world, 700.0, 80.0);
}

// Static bodies don't move but participate in collision resolution.
fn spawn_platform(world: &mut World, x: f32, y: f32, half_w: f32, half_h: f32) {
    let e = world.spawn();
    world.add_component(e, Transform::from_xyz(x, y, 0.0));
    // new_static() — infinite mass, never moves, but colliders push dynamic bodies away.
    world.add_component(e, RigidBody2D::new_static());
    world.add_component(e, Collider2D::aabb(half_w, half_h));
    // Platform is a marker component — no data, used by the renderer to identify platforms.
    world.add_component(e, Platform);
}

// Coins use trigger colliders: detect overlap events but don't physically block.
fn spawn_coin(world: &mut World, x: f32, y: f32) {
    let e = world.spawn();
    world.add_component(e, Transform::from_xyz(x, y, 0.0));
    // trigger() — generates contact events without collision response.
    world.add_component(e, Collider2D::trigger(10.0, 10.0));
    // Collectible tracks whether this coin has been picked up.
    world.add_component(e, Collectible { collected: false });
}

// Enemies are dynamic bodies with an AI patrol component.
fn spawn_enemy(world: &mut World, x: f32, y: f32) {
    let e = world.spawn();
    world.add_component(e, Transform::from_xyz(x, y, 0.0));
    world.add_component(e, RigidBody2D::new_dynamic());
    world.add_component(e, Collider2D::aabb(12.0, 12.0));
    // EnemyAI stores patrol state — the enemy_ai_system reads and updates this each frame.
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

// Goal is a trigger volume — when the player overlaps it, the level is complete.
fn spawn_goal(world: &mut World, x: f32, y: f32) {
    let e = world.spawn();
    world.add_component(e, Transform::from_xyz(x, y, 0.0));
    world.add_component(e, Collider2D::trigger(16.0, 32.0));
    // GoalMarker is a marker component for the goal_system to query.
    world.add_component(e, GoalMarker);
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------
// In ECS, components are plain data structs attached to entities.
// Systems query entities by component type to find relevant game objects.

// PlayerState: game-specific component tracking lives and score.
// Not part of the engine — each game defines its own components.
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

// Platform: zero-sized marker component. Marker components are an ECS pattern
// for tagging entities without adding data — queried by the renderer.
#[derive(Debug, Clone)]
struct Platform;

// Collectible: tracks pickup state. The `collected` flag prevents double-counting.
#[derive(Debug, Clone)]
struct Collectible {
    collected: bool,
}

// EnemyAI: stores patrol behavior state. Updated by enemy_ai_system each frame.
#[derive(Debug, Clone)]
struct EnemyAI {
    patrol_dir: f32,
    patrol_range: f32,
    spawn_x: f32,
    speed: f32,
}

// GoalMarker: marker component for the level exit trigger volume.
#[derive(Debug, Clone)]
struct GoalMarker;

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------
// Systems are plain functions that run each frame. They query the ECS World
// for entities with specific component types, then read/modify component data.
// This is the core ECS pattern: data (components) is separate from logic (systems).

// player_movement_system: reads input and updates the player's velocity.
// This demonstrates the standard system pattern:
//   1. Query entities by component type
//   2. Read input from a resource
//   3. Modify component data
fn player_movement_system(world: &mut World) {
    // component_entities::<T>() returns all entity IDs that have component T.
    // This is how systems find relevant entities to operate on.
    let entities: Vec<u32> = world.component_entities::<PlayerState>();

    for &eid in &entities {
        // Check grounded state — read-only access via get_by_index.
        // The physics system sets RigidBody2D.grounded based on collision contacts.
        let grounded = world
            .get_by_index::<RigidBody2D>(eid)
            .map(|b| b.grounded)
            .unwrap_or(false);

        // Read input — InputManager is a resource (single instance, not per-entity).
        // key_down() checks if a key is currently held; key_just_pressed() checks if
        // it was pressed this frame (edge-triggered vs level-triggered).
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

        // Apply horizontal velocity — get_by_index_mut gives mutable access to a component.
        // Setting velocity directly gives responsive, arcade-style movement.
        if let Some(body) = world.get_by_index_mut::<RigidBody2D>(eid) {
            body.velocity.x = if left {
                -PLAYER_SPEED
            } else if right {
                PLAYER_SPEED
            } else {
                0.0
            };

            // Jump — only allowed when grounded (prevents air-jumping).
            // Setting velocity.y directly overrides gravity for one frame.
            if jump && grounded {
                body.velocity.y = JUMP_VELOCITY;
            }
        }
    }
}

// enemy_ai_system: simple patrol behavior — reverses direction at patrol boundaries.
// Demonstrates reading multiple component types from the same entity.
fn enemy_ai_system(world: &mut World) {
    let entities: Vec<u32> = world.component_entities::<EnemyAI>();

    for &eid in &entities {
        // Copy AI state to avoid borrow conflicts (can't read and mut borrow simultaneously).
        let (patrol_dir, patrol_range, spawn_x, speed) = {
            let ai = world.get_by_index::<EnemyAI>(eid).unwrap();
            (ai.patrol_dir, ai.patrol_range, ai.spawn_x, ai.speed)
        };

        // Check if enemy has moved beyond patrol range from its spawn point.
        let mut new_dir = patrol_dir;
        if let Some(transform) = world.get_by_index::<Transform>(eid) {
            let dx = transform.position.x - spawn_x;
            if dx > patrol_range {
                new_dir = -1.0;
            } else if dx < -patrol_range {
                new_dir = 1.0;
            }
        }

        // Update AI direction.
        if let Some(ai) = world.get_by_index_mut::<EnemyAI>(eid) {
            ai.patrol_dir = new_dir;
        }

        // Apply patrol velocity — physics handles movement and collision.
        if let Some(body) = world.get_by_index_mut::<RigidBody2D>(eid) {
            body.velocity.x = new_dir * speed;
        }
    }
}

// collectible_system: checks physics contacts for player-coin overlaps.
// Demonstrates how game systems consume collision data from the physics engine.
fn collectible_system(world: &mut World) {
    // Read contacts from PhysicsWorld2D — these are generated by the physics step.
    // Contacts are pairs of entity IDs + metadata (is_trigger, normal, etc.).
    let contacts: Vec<(u32, u32)> = {
        let pw = match world.get_resource::<PhysicsWorld2D>() {
            Some(pw) => pw,
            None => return,
        };
        pw.contacts
            .iter()
            .filter(|c| c.is_trigger) // Only trigger contacts (coins don't block)
            .map(|c| (c.entity_a, c.entity_b))
            .collect()
    };

    // For each contact, determine which entity is the player and which is the collectible.
    // Contacts are unordered pairs, so we check both orderings.
    for (a, b) in contacts {
        let (player_eid, collectible_eid) = if world.get_by_index::<PlayerState>(a).is_some() {
            (a, b)
        } else if world.get_by_index::<PlayerState>(b).is_some() {
            (b, a)
        } else {
            continue; // Neither entity is a player — skip
        };

        // Check if already collected to prevent double-counting.
        let already_collected = world
            .get_by_index::<Collectible>(collectible_eid)
            .map(|c| c.collected)
            .unwrap_or(true);

        if !already_collected {
            // Mark collected and update score — two separate mutable borrows on different entities.
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

// goal_system: detects when the player reaches the goal trigger volume.
// Same contact-checking pattern as collectible_system, but simpler (no state mutation).
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

    // Check if any trigger contact involves both a player and a goal.
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

// death_zone_system: respawns the player if they fall below the level boundary.
// Demonstrates combining Transform queries with game state mutation.
fn death_zone_system(world: &mut World) {
    let entities: Vec<u32> = world.component_entities::<PlayerState>();

    for &eid in &entities {
        // Check if player has fallen below the death threshold.
        let fell = world
            .get_by_index::<Transform>(eid)
            .map(|t| t.position.y < -200.0)
            .unwrap_or(false);

        if fell {
            // Respawn: reset position, velocity, and deduct a life.
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
// This renderer demonstrates querying multiple component types to build a visual output.
// In production, you'd use the wgpu renderer instead of terminal ASCII.

fn render_ascii(world: &World) {
    let width = 80usize;
    let height = 25usize;
    let tile_size = 32.0f32; // World units per terminal cell
    let mut buffer = vec![vec![' '; width]; height];

    // Render platforms — query entities with Platform component.
    // Uses component_entities::<T>() to find all platforms, then reads their
    // Transform and Collider2D to compute screen-space bounds.
    let platforms: Vec<u32> = world.component_entities::<Platform>();
    for &eid in &platforms {
        // Rust 2024 let-chains allow combining multiple Option checks in one condition.
        // world_aabb() computes the collider's world-space bounding box from its local shape + position.
        if let Some(transform) = world.get_by_index::<Transform>(eid)
            && let Some(collider) = world.get_by_index::<Collider2D>(eid)
        {
            let aabb = collider.world_aabb(Vec2::new(transform.position.x, transform.position.y));
            // Convert world coordinates to terminal cell coordinates.
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

    // Render coins — skip collected ones (demonstrates filtering by component state).
    let coins: Vec<u32> = world.component_entities::<Collectible>();
    for &eid in &coins {
        // Skip collected coins — demonstrates reading component state for rendering decisions.
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

    // Render enemies — single-cell 'E' markers.
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

    // Render goal — 'F' for finish.
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

    // Render player last so it draws on top of other entities.
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

    // Print frame — flip Y axis (terminal Y=0 is top, world Y=0 is bottom).
    // ANSI escape codes clear screen and move cursor to home position.
    println!("\x1B[2J\x1B[H");
    for row in buffer.iter().rev() {
        println!("{}", row.iter().collect::<String>());
    }

    // HUD — query player state for score/lives display.
    // Demonstrates that render functions can read any component type.
    if let Some(&eid) = world.component_entities::<PlayerState>().first()
        && let Some(player) = world.get_by_index::<PlayerState>(eid)
    {
        println!("Score: {} | Lives: {}", player.score, player.lives);
    }
}
