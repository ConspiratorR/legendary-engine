# Building a 2D Platformer with RustEngine

## Prerequisites

- Rust 1.95.0+
- RustEngine cloned and built (`cargo build`)
- Basic familiarity with ECS concepts (see [ecs-tutorial.md](ecs-tutorial.md))

## Overview

This tutorial walks through building a complete 2D platformer game using RustEngine.
The demo (`crates/engine-core/examples/platformer_demo.rs`) validates multiple engine
subsystems working together: ECS, physics, input, and the app framework.

**Features demonstrated:**
- Player movement with gravity and jumping
- AABB collision detection and resolution
- Trigger-based collectibles and goal detection
- Enemy patrol AI
- Death zones and respawn
- ASCII terminal rendering for validation

## Step 1: Setting Up

Start by importing the required modules and building the application:

```rust
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
```

The constants are tuned for pixel-space coordinates (not meters). Gravity at `-980`
provides a snappy arcade feel.

Create the app and register plugins:

```rust
let mut builder = AppBuilder::new();
builder.add_plugin(FrameworkPlugin);
builder.add_plugin(Physics2DPlugin);
```

`FrameworkPlugin` provides the core ECS framework. `Physics2DPlugin` registers the
`PhysicsWorld2D` resource and its step system.

Override gravity for the platformer:

```rust
{
    let pw = builder
        .world_mut()
        .get_resource_mut::<PhysicsWorld2D>()
        .unwrap();
    pw.gravity = Vec2::new(0.0, GRAVITY);
}
```

The default physics gravity is `-9.81` (meters). We override it to `-980` for
pixel-space feel.

## Step 2: Player Movement

### PlayerState Component

Define a component to track player stats:

```rust
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
```

### Spawning the Player

```rust
let player = world.spawn();
world.add_component(player, Transform::from_xyz(100.0, 200.0, 0.0));
world.add_component(player, RigidBody2D::new_dynamic());
world.add_component(player, Collider2D::aabb(14.0, 14.0));
world.add_component(player, PlayerState::new());
```

The player entity needs:
- **Transform** — position in world space
- **RigidBody2D** — dynamic body affected by gravity
- **Collider2D** — solid AABB for collision (14×14 pixel half-extents)
- **PlayerState** — custom game state

### Input Handling

Read input using the `InputManager` resource:

```rust
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
```

Key points:
- `key_down()` returns `true` while held (horizontal movement)
- `key_just_pressed()` returns `true` only on the frame it was pressed (jump)
- The `grounded` flag is set automatically by the physics engine when the body
  lands on a surface with an upward-facing collision normal
- Horizontal velocity is set directly (no acceleration) for responsive controls

## Step 3: Physics and Collision

### PhysicsWorld2D

`PhysicsWorld2D` manages the 2D simulation. Its `step()` method:

1. Applies gravity to dynamic bodies
2. Integrates velocities into positions
3. Detects AABB collisions between all collider pairs
4. Resolves solid collisions (pushes entities apart, corrects velocity)
5. Records trigger overlaps in `contacts`

The physics plugin handles stepping automatically each frame.

### AABB2D

All collision uses axis-aligned bounding boxes:

```rust
pub struct AABB2D {
    pub min: Vec2,
    pub max: Vec2,
}
```

`Collider2D::world_aabb(position)` computes the world-space AABB for an entity.

### RigidBody2D Types

| Type | Gravity | Collision Response | Use Case |
|------|---------|-------------------|----------|
| `Static` | No | Immovable | Ground, walls, platforms |
| `Kinematic` | No | Immovable (but moves by code) | Moving platforms |
| `Dynamic` | Yes | Pushed by collisions | Player, enemies |

Create bodies with constructors:

```rust
RigidBody2D::new_dynamic()   // Gravity, collision response
RigidBody2D::new_static()    // No gravity, immovable
RigidBody2D::new_kinematic() // No gravity, code-driven movement
```

### Collider2D: Solid vs Trigger

```rust
Collider2D::aabb(half_x, half_y)     // Solid — blocks movement
Collider2D::trigger(half_x, half_y)  // Trigger — overlaps only
```

Solid colliders cause physics resolution (push-out). Trigger colliders generate
contact events but don't block movement — ideal for coins, goal flags, and damage
zones.

## Step 4: Building the Level

### Platforms

Static bodies with solid colliders:

```rust
fn spawn_platform(world: &mut World, x: f32, y: f32, half_w: f32, half_h: f32) {
    let e = world.spawn();
    world.add_component(e, Transform::from_xyz(x, y, 0.0));
    world.add_component(e, RigidBody2D::new_static());
    world.add_component(e, Collider2D::aabb(half_w, half_h));
    world.add_component(e, Platform);
}
```

Level layout:

```rust
// Ground platform (wide)
spawn_platform(world, 400.0, 50.0, 400.0, 20.0);

// Floating platforms
spawn_platform(world, 200.0, 200.0, 80.0, 10.0);
spawn_platform(world, 400.0, 300.0, 80.0, 10.0);
spawn_platform(world, 600.0, 250.0, 80.0, 10.0);
```

### Coins

Trigger colliders for collectibles:

```rust
fn spawn_coin(world: &mut World, x: f32, y: f32) {
    let e = world.spawn();
    world.add_component(e, Transform::from_xyz(x, y, 0.0));
    world.add_component(e, Collider2D::trigger(10.0, 10.0));
    world.add_component(e, Collectible { collected: false });
}
```

### Enemies

Dynamic bodies with patrol AI:

```rust
fn spawn_enemy(world: &mut World, x: f32, y: f32) {
    let e = world.spawn();
    world.add_component(e, Transform::from_xyz(x, y, 0.0));
    world.add_component(e, RigidBody2D::new_dynamic());
    world.add_component(e, Collider2D::aabb(12.0, 12.0));
    world.add_component(e, EnemyAI {
        patrol_dir: 1.0,
        patrol_range: 100.0,
        spawn_x: x,
        speed: 80.0,
    });
}
```

### Goal

Trigger collider with a marker component:

```rust
fn spawn_goal(world: &mut World, x: f32, y: f32) {
    let e = world.spawn();
    world.add_component(e, Transform::from_xyz(x, y, 0.0));
    world.add_component(e, Collider2D::trigger(16.0, 32.0));
    world.add_component(e, GoalMarker);
}
```

## Step 5: Enemy AI

### EnemyAI Component

```rust
#[derive(Debug, Clone)]
struct EnemyAI {
    patrol_dir: f32,    // Current direction: 1.0 or -1.0
    patrol_range: f32,  // Distance from spawn to reverse
    spawn_x: f32,       // Starting X position
    speed: f32,         // Movement speed
}
```

### Patrol System

```rust
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
```

The enemy patrols back and forth within `patrol_range` pixels of its spawn point.
When it exceeds the range, it reverses direction. The physics engine handles gravity
and collision with platforms automatically.

## Step 6: Collectibles and Goals

### Collectible System

Reads trigger contacts from the physics world:

```rust
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
            }
        }
    }
}
```

Key pattern: filter `pw.contacts` for `is_trigger` entries, then check which entity
is the player and which is the collectible.

### Goal System

Same pattern, checking for `GoalMarker`:

```rust
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
```

## Step 7: Death and Respawn

When the player falls below a threshold, reset position and decrement lives:

```rust
fn death_zone_system(world: &mut World) {
    let entities: Vec<u32> = world.component_entities::<PlayerState>();

    for &eid in &entities {
        let fell = world
            .get_by_index::<Transform>(eid)
            .map(|t| t.position.y < -200.0)
            .unwrap_or(false);

        if fell {
            // Reset position
            if let Some(t) = world.get_by_index_mut::<Transform>(eid) {
                t.position.x = 100.0;
                t.position.y = 200.0;
            }
            // Clear velocity
            if let Some(body) = world.get_by_index_mut::<RigidBody2D>(eid) {
                body.velocity = Vec2::ZERO;
            }
            // Decrement lives
            if let Some(player) = world.get_by_index_mut::<PlayerState>(eid) {
                player.lives -= 1;
            }
        }
    }
}
```

## Step 8: ASCII Rendering

For terminal-based validation, the demo renders a top-down ASCII view:

```rust
fn render_ascii(world: &World) {
    let width = 80usize;
    let height = 25usize;
    let tile_size = 32.0f32;
    let mut buffer = vec![vec![' '; width]; height];
```

Entity-to-screen mapping: world position is divided by `tile_size` to get screen
coordinates. Each entity type gets a character:

| Entity | Character |
|--------|-----------|
| Platform | `#` |
| Coin | `$` |
| Enemy | `E` |
| Goal | `F` |
| Player | `@` |

The buffer is printed with Y flipped (terminal Y=0 is top, world Y=0 is bottom):

```rust
    // Print (flip Y for terminal — Y=0 at bottom)
    for row in buffer.iter().rev() {
        println!("{}", row.iter().collect::<String>());
    }
```

## Step 9: Registering Systems

Register all systems with the app builder:

```rust
builder.add_system(player_movement_system);
builder.add_system(enemy_ai_system);
builder.add_system(collectible_system);
builder.add_system(goal_system);
builder.add_system(death_zone_system);
```

Systems run in registration order each frame.

## Step 10: Running the Simulation

Build and run the app:

```rust
let mut app = builder.build();

for frame in 0..300u32 {
    // Simulate input
    {
        let input = app.world.get_resource_mut::<InputManager>().unwrap();
        if frame == 0 { input.press(KeyCode::ArrowRight); }
        if frame == 100 {
            input.release(KeyCode::ArrowRight);
            input.press(KeyCode::ArrowLeft);
        }
        if frame == 200 { input.release(KeyCode::ArrowLeft); }
    }

    app.run();

    if frame % 5 == 0 {
        render_ascii(&app.world);
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
```

`app.run()` calls `input.update_frame()` internally, which advances the input state
for `key_just_pressed()` tracking.

## Running the Demo

```bash
cargo run -p engine-core --example platformer_demo
```

## Complete Code

See [`crates/engine-core/examples/platformer_demo.rs`](../crates/engine-core/examples/platformer_demo.rs)
for the full implementation.

## Key APIs Reference

| API | Purpose |
|-----|---------|
| `PhysicsWorld2D` | 2D physics simulation, gravity, collision |
| `AABB2D` | Axis-aligned bounding box |
| `RigidBody2D` | Body type, velocity, grounded state |
| `Collider2D` | Solid or trigger collider |
| `InputManager::key_down()` | Is key currently held? |
| `InputManager::key_just_pressed()` | Was key pressed this frame? |
| `World::component_entities::<T>()` | Get all entities with component T |
| `World::get_by_index::<T>(eid)` | Read component on entity |
| `World::get_by_index_mut::<T>(eid)` | Write component on entity |
