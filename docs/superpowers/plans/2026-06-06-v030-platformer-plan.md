# v0.3.0 Platformer Iteration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a complete 2D platformer game as an engine validation exercise, adding lightweight 2D physics, fixing exposed API issues, and providing a step-by-step tutorial.

**Architecture:** Bottom-up incremental build. Start with physics_2d module, then build the platformer example step by step. Each step produces a runnable program. Problems are fixed as they surface.

**Tech Stack:** Rust 2024 edition, wgpu, winit, engine-physics, engine-render, engine-input, engine-audio, engine-ui, engine-framework

---

## File Map

### New Files
| File | Purpose |
|------|---------|
| `crates/engine-physics/src/physics_2d.rs` | 2D physics module (AABB, RigidBody2D, Collider2D, PhysicsWorld2D) |
| `crates/engine-core/examples/platformer_demo.rs` | Complete 2D platformer game example |
| `docs/platformer-tutorial.md` | Step-by-step tutorial document |

### Modified Files
| File | Change |
|------|--------|
| `crates/engine-physics/src/lib.rs` | Add `pub mod physics_2d;` and re-exports |
| `crates/engine-physics/src/plugin.rs` | Add `Physics2DPlugin` with `physics_2d_step_system` |
| `crates/engine-physics/tests/physics_tests.rs` | Add physics_2d unit tests |
| `crates/engine-core/tests/core_tests.rs` | Fix flaky `test_time_fps` |
| Various engine crates | API fixes discovered during platformer development |

---

## Task 1: Fix Flaky `test_time_fps`

**Problem:** `test_time_fps` asserts `fps > 0.0`. When `Instant::now() - last_frame_time` is 0 (extremely fast execution), `delta_seconds` is 0.0 and `fps()` returns 0.0.

**Files:**
- Modify: `crates/engine-core/tests/core_tests.rs:259-265`

- [ ] **Step 1: Read the test and Time::fps()**

The test at line 259:
```rust
#[test]
fn test_time_fps() {
    let mut time = Time::new();
    time.update();
    let fps = time.fps();
    assert!(fps > 0.0);
}
```

`fps()` returns `1.0 / delta_seconds` when `delta_seconds > 0.0`, else `0.0`.

- [ ] **Step 2: Fix the assertion**

Change `assert!(fps > 0.0)` to `assert!(fps >= 0.0)` �?fps of 0 is valid when frame time is instantaneous.

```rust
#[test]
fn test_time_fps() {
    let mut time = Time::new();
    time.update();
    let fps = time.fps();
    assert!(fps >= 0.0);
}
```

- [ ] **Step 3: Run test 5 times to verify stability**

Run: `for ($i=0; $i -lt 5; $i++) { cargo test -p engine-core test_time_fps 2>&1 | Select-String "test result" }`
Expected: PASS all 5 times

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/tests/core_tests.rs
git commit -m "fix(core): fix flaky test_time_fps assertion"
```

---

## Task 2: Create physics_2d Module �?Types

**Files:**
- Create: `crates/engine-physics/src/physics_2d.rs`

- [ ] **Step 1: Write the physics_2d types**

```rust
//! Lightweight 2D physics for platformer-style games.
//!
//! Provides AABB collision, simple gravity, ground detection, and trigger support.
//! Designed for tile-based 2D games �?no rotation, no circle collision, no constraints.

use engine_math::Vec2;

/// Axis-aligned bounding box in 2D.
#[derive(Debug, Clone, Copy)]
pub struct AABB2D {
    pub min: Vec2,
    pub max: Vec2,
}

impl AABB2D {
    /// Create a new AABB from min and max corners.
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    /// Create an AABB centered at a point with given half-extents.
    pub fn from_center(center: Vec2, half_extents: Vec2) -> Self {
        Self {
            min: center - half_extents,
            max: center + half_extents,
        }
    }

    /// Check overlap with another AABB.
    pub fn overlaps(&self, other: &AABB2D) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
    }

    /// Compute the overlap (penetration) between two AABBs.
    /// Returns None if no overlap.
    pub fn intersection(&self, other: &AABB2D) -> Option<(Vec2, f32)> {
        let overlap_x = (self.max.x - other.min.x).min(other.max.x - self.min.x);
        let overlap_y = (self.max.y - other.min.y).min(other.max.y - self.min.y);

        if overlap_x <= 0.0 || overlap_y <= 0.0 {
            return None;
        }

        // Minimum separation axis
        if overlap_x < overlap_y {
            let sign = if self.min.x + self.max.x < other.min.x + other.max.x {
                -1.0
            } else {
                1.0
            };
            Some((Vec2::new(sign, 0.0), overlap_x))
        } else {
            let sign = if self.min.y + self.max.y < other.min.y + other.max.y {
                -1.0
            } else {
                1.0
            };
            Some((Vec2::new(0.0, sign), overlap_y))
        }
    }

    /// Width of the AABB.
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    /// Height of the AABB.
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    /// Center point.
    pub fn center(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }
}

/// Body type for 2D physics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyType2D {
    /// Immovable �?ground, walls.
    Static,
    /// Moved by code only �?moving platforms, doors.
    Kinematic,
    /// Fully simulated �?player, enemies.
    Dynamic,
}

/// 2D rigid body component.
#[derive(Debug, Clone)]
pub struct RigidBody2D {
    pub body_type: BodyType2D,
    pub velocity: Vec2,
    pub gravity_scale: f32,
    pub grounded: bool,
    pub linear_damping: f32,
}

impl Default for RigidBody2D {
    fn default() -> Self {
        Self {
            body_type: BodyType2D::Dynamic,
            velocity: Vec2::ZERO,
            gravity_scale: 1.0,
            grounded: false,
            linear_damping: 0.0,
        }
    }
}

impl RigidBody2D {
    pub fn new_dynamic() -> Self {
        Self::default()
    }

    pub fn new_static() -> Self {
        Self {
            body_type: BodyType2D::Static,
            velocity: Vec2::ZERO,
            gravity_scale: 0.0,
            grounded: false,
            linear_damping: 0.0,
        }
    }

    pub fn new_kinematic() -> Self {
        Self {
            body_type: BodyType2D::Kinematic,
            velocity: Vec2::ZERO,
            gravity_scale: 0.0,
            grounded: false,
            linear_damping: 0.0,
        }
    }
}

/// 2D collider component.
#[derive(Debug, Clone)]
pub struct Collider2D {
    /// Local offset from entity position.
    pub offset: Vec2,
    /// Half-extents of the AABB (half-width, half-height).
    pub half_extents: Vec2,
    pub friction: f32,
    pub restitution: f32,
    pub is_trigger: bool,
    pub collision_layers: u32,
    pub collision_mask: u32,
}

impl Default for Collider2D {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            half_extents: Vec2::new(0.5, 0.5),
            friction: 0.5,
            restitution: 0.0,
            is_trigger: false,
            collision_layers: 0xFFFF_FFFF,
            collision_mask: 0xFFFF_FFFF,
        }
    }
}

impl Collider2D {
    /// Create a solid AABB collider with given half-extents.
    pub fn aabb(half_x: f32, half_y: f32) -> Self {
        Self {
            half_extents: Vec2::new(half_x, half_y),
            ..Default::default()
        }
    }

    /// Create a trigger (sensor) AABB collider.
    pub fn trigger(half_x: f32, half_y: f32) -> Self {
        Self {
            half_extents: Vec2::new(half_x, half_y),
            is_trigger: true,
            ..Default::default()
        }
    }

    /// Compute the world-space AABB given an entity position.
    pub fn world_aabb(&self, position: Vec2) -> AABB2D {
        let center = position + self.offset;
        AABB2D::from_center(center, self.half_extents)
    }
}

/// Contact result from 2D collision detection.
#[derive(Debug, Clone)]
pub struct Contact2D {
    pub entity_a: u32,
    pub entity_b: u32,
    pub normal: Vec2,
    pub penetration: f32,
    pub is_trigger: bool,
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p engine-physics`
Expected: Compiles with no errors

- [ ] **Step 3: Commit**

```bash
git add crates/engine-physics/src/physics_2d.rs
git commit -m "feat(physics): add physics_2d types (AABB2D, RigidBody2D, Collider2D)"
```

---

## Task 3: Create physics_2d Module �?World & Step

**Files:**
- Modify: `crates/engine-physics/src/physics_2d.rs` (append)

- [ ] **Step 1: Add PhysicsWorld2D and step logic**

Append to `physics_2d.rs`:

```rust
/// 2D physics world �?gravity, integration, collision detection & resolution.
#[derive(Debug, Clone)]
pub struct PhysicsWorld2D {
    pub gravity: Vec2,
    pub contacts: Vec<Contact2D>,
}

impl Default for PhysicsWorld2D {
    fn default() -> Self {
        Self {
            gravity: Vec2::new(0.0, -9.81),
            contacts: Vec::new(),
        }
    }
}

impl PhysicsWorld2D {
    pub fn new() -> Self {
        Self::default()
    }

    /// Step the 2D physics simulation.
    ///
    /// 1. Apply gravity to dynamic bodies
    /// 2. Integrate velocities into positions
    /// 3. Detect AABB collisions
    /// 4. Resolve solid collisions (push out + velocity correction)
    /// 5. Record trigger overlaps
    pub fn step(&mut self, world: &mut engine_ecs::world::World, dt: f32) {
        self.contacts.clear();

        // Phase 1: Apply gravity & integrate
        let entities: Vec<u32> = world.component_entities::<RigidBody2D>();
        for &eid in &entities {
            let body = world.get_by_index_mut::<RigidBody2D>(eid).unwrap();
            if body.body_type != BodyType2D::Dynamic {
                continue;
            }
            body.velocity += self.gravity * body.gravity_scale * dt;
            body.velocity *= 1.0 - body.linear_damping * dt;
        }

        // Phase 2: Move entities
        let entities: Vec<u32> = world.component_entities::<RigidBody2D>();
        for &eid in &entities {
            let body = world.get_by_index_mut::<RigidBody2D>(eid).unwrap();
            if body.body_type == BodyType2D::Static {
                continue;
            }
            let vel = body.velocity;
            // Read position from Transform
            if let Some(transform) = world.get_by_index_mut::<engine_core::transform::Transform>(eid) {
                transform.translation.x += vel.x * dt;
                transform.translation.y += vel.y * dt;
            }
        }

        // Phase 3: Detect & resolve collisions
        self.detect_and_resolve(world);
    }

    fn detect_and_resolve(&mut self, world: &mut engine_ecs::world::World) {
        // Reset grounded for all dynamic bodies
        let entities: Vec<u32> = world.component_entities::<RigidBody2D>();
        for &eid in &entities {
            if let Some(body) = world.get_by_index_mut::<RigidBody2D>(eid) {
                if body.body_type == BodyType2D::Dynamic {
                    body.grounded = false;
                }
            }
        }

        // Collect all colliders with positions
        let mut colliders: Vec<(u32, Vec2, Collider2D, BodyType2D)> = Vec::new();
        let entities: Vec<u32> = world.component_entities::<Collider2D>();
        for &eid in &entities {
            let collider = world.get_by_index::<Collider2D>(eid).unwrap();
            let body_type = world
                .get_by_index::<RigidBody2D>(eid)
                .map(|b| b.body_type)
                .unwrap_or(BodyType2D::Static);
            if let Some(transform) = world.get_by_index::<engine_core::transform::Transform>(eid) {
                let pos = Vec2::new(transform.translation.x, transform.translation.y);
                colliders.push((eid, pos, collider.clone(), body_type));
            }
        }

        // Broadphase: check all pairs
        for i in 0..colliders.len() {
            for j in (i + 1)..colliders.len() {
                let (eid_a, pos_a, col_a, type_a) = &colliders[i];
                let (eid_b, pos_b, col_b, type_b) = &colliders[j];

                // Skip if both static
                if *type_a == BodyType2D::Static && *type_b == BodyType2D::Static {
                    continue;
                }

                // Layer check
                if (col_a.collision_layers & col_b.collision_mask) == 0
                    || (col_b.collision_layers & col_a.collision_mask) == 0
                {
                    continue;
                }

                let aabb_a = col_a.world_aabb(*pos_a);
                let aabb_b = col_b.world_aabb(*pos_b);

                if let Some((normal, pen)) = aabb_a.intersection(&aabb_b) {
                    let is_trigger = col_a.is_trigger || col_b.is_trigger;

                    self.contacts.push(Contact2D {
                        entity_a: *eid_a,
                        entity_b: *eid_b,
                        normal,
                        penetration: pen,
                        is_trigger,
                    });

                    // Resolve solid collisions
                    if !is_trigger {
                        self.resolve_collision(world, *eid_a, *eid_b, type_a, type_b, normal, pen);
                    }
                }
            }
        }
    }

    fn resolve_collision(
        &self,
        world: &mut engine_ecs::world::World,
        eid_a: u32,
        eid_b: u32,
        type_a: &BodyType2D,
        type_b: &BodyType2D,
        normal: Vec2,
        pen: f32,
    ) {
        // Determine which entity to push (dynamic vs static/kinematic)
        let (push_eid, _other_eid, push_normal) = match (type_a, type_b) {
            (BodyType2D::Dynamic, BodyType2D::Static | BodyType2D::Kinematic) => {
                (eid_a, eid_b, normal)
            }
            (BodyType2D::Static | BodyType2D::Kinematic, BodyType2D::Dynamic) => {
                (eid_b, eid_a, -normal)
            }
            (BodyType2D::Dynamic, BodyType2D::Dynamic) => {
                // Both dynamic: push first entity by half
                (eid_a, eid_b, normal)
            }
            _ => return,
        };

        // Push out
        if let Some(transform) = world.get_by_index_mut::<engine_core::transform::Transform>(push_eid) {
            transform.translation.x += push_normal.x * pen;
            transform.translation.y += push_normal.y * pen;
        }

        // Velocity correction: cancel velocity into the collision surface
        if let Some(body) = world.get_by_index_mut::<RigidBody2D>(push_eid) {
            let vel_dot_normal = body.velocity.x * push_normal.x + body.velocity.y * push_normal.y;
            if vel_dot_normal < 0.0 {
                body.velocity.x -= push_normal.x * vel_dot_normal;
                body.velocity.y -= push_normal.y * vel_dot_normal;
            }

            // Ground detection: if normal points upward, entity is grounded
            if push_normal.y > 0.5 {
                body.grounded = true;
            }
        }
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p engine-physics`
Expected: Compiles with no errors

- [ ] **Step 3: Commit**

```bash
git add crates/engine-physics/src/physics_2d.rs
git commit -m "feat(physics): add PhysicsWorld2D with gravity, collision, ground detection"
```

---

## Task 4: Wire physics_2d into lib.rs and plugin.rs

**Files:**
- Modify: `crates/engine-physics/src/lib.rs`
- Modify: `crates/engine-physics/src/plugin.rs`

- [ ] **Step 1: Add module declaration to lib.rs**

Add after line 61 (`pub mod world;`):

```rust
pub mod physics_2d;
```

No re-exports needed from lib.rs �?users import from the module directly:
```rust
use engine_physics::physics_2d::{AABB2D, RigidBody2D, Collider2D, PhysicsWorld2D};
```

- [ ] **Step 2: Add Physics2DPlugin to plugin.rs**

Append to `plugin.rs`:

```rust
use crate::physics_2d::PhysicsWorld2D;

fn physics_2d_step_system(world: &mut engine_ecs::world::World) {
    let mut pw = match world.remove_resource::<PhysicsWorld2D>() {
        Some(pw) => pw,
        None => return,
    };
    let dt = world
        .get_resource::<engine_core::time::Time>()
        .map(|t| t.delta_seconds())
        .unwrap_or(1.0 / 60.0);
    pw.step(world, dt);
    world.insert_resource(pw);
}

/// Plugin that adds 2D physics simulation capabilities.
pub struct Physics2DPlugin;

impl Plugin for Physics2DPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(PhysicsWorld2D::default());
        app.add_system(physics_2d_step_system);
    }
}
```

- [ ] **Step 3: Add Physics2DPlugin to lib.rs re-exports**

Add to lib.rs re-exports:

```rust
pub use plugin::Physics2DPlugin;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p engine-physics`
Expected: Compiles with no errors

- [ ] **Step 5: Commit**

```bash
git add crates/engine-physics/src/lib.rs crates/engine-physics/src/plugin.rs
git commit -m "feat(physics): wire physics_2d module and Physics2DPlugin"
```

---

## Task 5: Add physics_2d Unit Tests

**Files:**
- Modify: `crates/engine-physics/tests/physics_tests.rs`

- [ ] **Step 1: Add AABB collision tests**

Append to physics_tests.rs:

```rust
#[cfg(test)]
mod physics_2d_tests {
    use super::*;
    use engine_physics::physics_2d::{AABB2D, BodyType2D, Collider2D, PhysicsWorld2D, RigidBody2D};
    use engine_math::Vec2;

    #[test]
    fn test_aabb_overlap() {
        let a = AABB2D::new(Vec2::new(0.0, 0.0), Vec2::new(2.0, 2.0));
        let b = AABB2D::new(Vec2::new(1.0, 1.0), Vec2::new(3.0, 3.0));
        assert!(a.overlaps(&b));
    }

    #[test]
    fn test_aabb_no_overlap() {
        let a = AABB2D::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0));
        let b = AABB2D::new(Vec2::new(2.0, 2.0), Vec2::new(3.0, 3.0));
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn test_aabb_intersection_x_axis() {
        let a = AABB2D::new(Vec2::new(0.0, 0.0), Vec2::new(2.0, 2.0));
        let b = AABB2D::new(Vec2::new(1.0, 0.0), Vec2::new(3.0, 2.0));
        let (normal, pen) = a.intersection(&b).unwrap();
        assert!(pen > 0.0);
        assert!(normal.x.abs() > 0.0); // Separation on X axis
    }

    #[test]
    fn test_aabb_intersection_y_axis() {
        let a = AABB2D::new(Vec2::new(0.0, 0.0), Vec2::new(2.0, 2.0));
        let b = AABB2D::new(Vec2::new(0.0, 1.0), Vec2::new(2.0, 3.0));
        let (normal, pen) = a.intersection(&b).unwrap();
        assert!(pen > 0.0);
        assert!(normal.y.abs() > 0.0); // Separation on Y axis
    }

    #[test]
    fn test_collider2d_world_aabb() {
        let col = Collider2D::aabb(0.5, 0.5);
        let aabb = col.world_aabb(Vec2::new(1.0, 2.0));
        assert!((aabb.min.x - 0.5).abs() < 0.001);
        assert!((aabb.min.y - 1.5).abs() < 0.001);
        assert!((aabb.max.x - 1.5).abs() < 0.001);
        assert!((aabb.max.y - 2.5).abs() < 0.001);
    }

    #[test]
    fn test_rigidbody2d_types() {
        let dynamic = RigidBody2D::new_dynamic();
        assert_eq!(dynamic.body_type, BodyType2D::Dynamic);
        assert_eq!(dynamic.gravity_scale, 1.0);

        let static_body = RigidBody2D::new_static();
        assert_eq!(static_body.body_type, BodyType2D::Static);
        assert_eq!(static_body.gravity_scale, 0.0);

        let kinematic = RigidBody2D::new_kinematic();
        assert_eq!(kinematic.body_type, BodyType2D::Kinematic);
    }

    #[test]
    fn test_physics_world_2d_gravity() {
        let mut world = engine_ecs::world::World::new();
        let entity = world.spawn();
        world.add_component(entity, engine_core::transform::Transform::from_xyz(0.0, 10.0, 0.0));
        world.add_component(entity, RigidBody2D::new_dynamic());
        world.add_component(entity, Collider2D::aabb(0.5, 0.5));

        let mut physics = PhysicsWorld2D::new();
        physics.step(&mut world, 1.0 / 60.0);

        let transform = world.get_by_index::<engine_core::transform::Transform>(entity).unwrap();
        // Entity should have fallen (Y decreased due to gravity)
        assert!(transform.translation.y < 10.0);
    }

    #[test]
    fn test_physics_world_2d_ground_detection() {
        let mut world = engine_ecs::world::World::new();

        // Dynamic body above static floor
        let player = world.spawn();
        world.add_component(player, engine_core::transform::Transform::from_xyz(0.0, 0.6, 0.0));
        world.add_component(player, RigidBody2D::new_dynamic());
        world.add_component(player, Collider2D::aabb(0.5, 0.5));

        let floor = world.spawn();
        world.add_component(floor, engine_core::transform::Transform::from_xyz(0.0, 0.0, 0.0));
        world.add_component(floor, RigidBody2D::new_static());
        world.add_component(floor, Collider2D::aabb(50.0, 0.5));

        let mut physics = PhysicsWorld2D::new();
        // Step multiple frames
        for _ in 0..60 {
            physics.step(&mut world, 1.0 / 60.0);
        }

        let body = world.get_by_index::<RigidBody2D>(player).unwrap();
        assert!(body.grounded, "Player should be grounded after falling onto floor");
    }
}
```

- [ ] **Step 2: Run physics tests**

Run: `cargo test -p engine-physics`
Expected: All tests pass (including new physics_2d tests)

- [ ] **Step 3: Commit**

```bash
git add crates/engine-physics/tests/physics_tests.rs
git commit -m "test(physics): add physics_2d unit tests"
```

---

## Task 6: Platformer �?Window + Game Loop (M1)

**Files:**
- Create: `crates/engine-core/examples/platformer_demo.rs`

- [ ] **Step 1: Create minimal platformer scaffold**

```rust
//! 2D Platformer Demo �?validates engine subsystems working together.
//!
//! Controls: Arrow keys / WASD to move, Space to jump, Escape to pause
//! Goal: Collect coins, avoid enemies, reach the flag

use engine_core::app::{App, AppBuilder};
use engine_core::plugin::Plugin;
use engine_core::time::Time;
use engine_core::transform::Transform;
use engine_ecs::world::World;
use engine_framework::{FrameworkPlugin, GameFlowPlugin, GameSession};
use engine_input::input_manager::InputManager;
use engine_input::keyboard::KeyCode;
use engine_math::Vec2;
use engine_physics::physics_2d::{
    BodyType2D, Collider2D, PhysicsWorld2D, RigidBody2D,
};
use engine_physics::Physics2DPlugin;

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;
const TILE_SIZE: f32 = 32.0;
const PLAYER_SPEED: f32 = 200.0;
const JUMP_VELOCITY: f32 = 400.0;
const GRAVITY: f32 = -980.0;

fn main() {
    // Build app with required plugins
    let mut app = AppBuilder::new();
    app.add_plugin(FrameworkPlugin);
    app.add_plugin(Physics2DPlugin);

    // Override gravity for platformer (pixels, not meters)
    {
        let mut pw = app.world_mut().get_resource_mut::<PhysicsWorld2D>().unwrap();
        pw.gravity = Vec2::new(0.0, GRAVITY);
    }

    // Spawn level
    spawn_level(app.world_mut());

    // Game loop placeholder �?in a real integration this would use winit
    println!("Platformer Demo �?entities spawned successfully");
    println!("Player, enemies, platforms, coins, and goal created.");
    println!("Run with a windowed backend to play.");
}

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
    spawn_enemy(world, 350.0, 80.0, EnemyType::Patrol);

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

fn spawn_enemy(world: &mut World, x: f32, y: f32, enemy_type: EnemyType) {
    let e = world.spawn();
    world.add_component(e, Transform::from_xyz(x, y, 0.0));
    world.add_component(e, RigidBody2D::new_dynamic());
    world.add_component(e, Collider2D::aabb(12.0, 12.0));
    world.add_component(e, EnemyAI {
        enemy_type,
        patrol_dir: 1.0,
        patrol_range: 100.0,
        spawn_x: x,
        speed: 80.0,
    });
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
    invincible_timer: f32,
}

impl PlayerState {
    fn new() -> Self {
        Self {
            lives: 3,
            score: 0,
            invincible_timer: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
struct Platform;

#[derive(Debug, Clone)]
struct Collectible {
    collected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnemyType {
    Patrol,
    Chase,
}

#[derive(Debug, Clone)]
struct EnemyAI {
    enemy_type: EnemyType,
    patrol_dir: f32,
    patrol_range: f32,
    spawn_x: f32,
    speed: f32,
}

#[derive(Debug, Clone)]
struct GoalMarker;
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check --example platformer_demo -p engine-core`
Expected: Compiles with no errors

- [ ] **Step 3: Commit**

```bash
git add crates/engine-core/examples/platformer_demo.rs
git commit -m "feat: add platformer_demo scaffold with level layout"
```

---

## Task 7: Platformer �?Player Movement System (M1+M2)

**Files:**
- Modify: `crates/engine-core/examples/platformer_demo.rs`

- [ ] **Step 1: Add player movement system**

Add after the component definitions:

```rust
// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn player_movement_system(world: &mut World) {
    let input = match world.get_resource::<InputManager>() {
        Some(input) => input.clone(),
        None => return,
    };

    let entities: Vec<u32> = world.component_entities::<PlayerState>();
    for &eid in &entities {
        let mut velocity = Vec2::ZERO;

        // Horizontal movement
        if input.key_down(KeyCode::ArrowLeft) || input.key_down(KeyCode::KeyA) {
            velocity.x = -PLAYER_SPEED;
        }
        if input.key_down(KeyCode::ArrowRight) || input.key_down(KeyCode::KeyD) {
            velocity.x = PLAYER_SPEED;
        }

        // Jump
        if (input.key_just_pressed(KeyCode::Space) || input.key_just_pressed(KeyCode::ArrowUp))
            && world.get_by_index::<RigidBody2D>(eid).map_or(false, |b| b.grounded)
        {
            if let Some(body) = world.get_by_index_mut::<RigidBody2D>(eid) {
                body.velocity.y = JUMP_VELOCITY;
            }
        }

        // Apply horizontal velocity
        if let Some(body) = world.get_by_index_mut::<RigidBody2D>(eid) {
            body.velocity.x = velocity.x;
        }
    }
}

fn enemy_ai_system(world: &mut World) {
    let dt = world
        .get_resource::<Time>()
        .map(|t| t.delta_seconds())
        .unwrap_or(1.0 / 60.0);

    let entities: Vec<u32> = world.component_entities::<EnemyAI>();
    for &eid in &entities {
        let (patrol_dir, patrol_range, spawn_x, speed) = {
            let ai = world.get_by_index::<EnemyAI>(eid).unwrap();
            (ai.patrol_dir, ai.patrol_range, ai.spawn_x, ai.speed)
        };

        if let Some(transform) = world.get_by_index::<Transform>(eid) {
            let dx = transform.translation.x - spawn_x;
            let mut dir = patrol_dir;
            if dx > patrol_range {
                dir = -1.0;
            } else if dx < -patrol_range {
                dir = 1.0;
            }

            // Update direction
            if let Some(ai) = world.get_by_index_mut::<EnemyAI>(eid) {
                ai.patrol_dir = dir;
            }

            // Apply velocity
            if let Some(body) = world.get_by_index_mut::<RigidBody2D>(eid) {
                body.velocity.x = dir * speed;
            }
        }
    }
}

fn collectible_system(world: &mut World) {
    // Check trigger overlaps from physics
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
        // Check if player + collectible
        let (player_eid, collectible_eid) = if world.get_by_index::<PlayerState>(a).is_some() {
            (a, b)
        } else if world.get_by_index::<PlayerState>(b).is_some() {
            (b, a)
        } else {
            continue;
        };

        if let Some(collectible) = world.get_by_index_mut::<Collectible>(collectible_eid) {
            if !collectible.collected {
                collectible.collected = true;
                if let Some(player) = world.get_by_index_mut::<PlayerState>(player_eid) {
                    player.score += 10;
                }
                // Despawn by removing transform (effectively hides it)
                world.remove_component::<Transform>(collectible_eid);
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
        if let Some(transform) = world.get_by_index::<Transform>(eid) {
            if transform.translation.y < -200.0 {
                // Respawn at start
                if let Some(t) = world.get_by_index_mut::<Transform>(eid) {
                    t.translation.x = 100.0;
                    t.translation.y = 200.0;
                }
                if let Some(body) = world.get_by_index_mut::<RigidBody2D>(eid) {
                    body.velocity = Vec2::ZERO;
                }
                if let Some(player) = world.get_by_index_mut::<PlayerState>(eid) {
                    player.lives -= 1;
                    if player.lives <= 0 {
                        println!("*** GAME OVER ***");
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 2: Register systems in main()**

Add after `app.add_plugin(Physics2DPlugin);`:

```rust
    app.add_system(player_movement_system);
    app.add_system(enemy_ai_system);
    app.add_system(collectible_system);
    app.add_system(goal_system);
    app.add_system(death_zone_system);
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check --example platformer_demo -p engine-core`
Expected: Compiles

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/examples/platformer_demo.rs
git commit -m "feat: add platformer player movement, enemy AI, collectibles, and goal systems"
```

---

## Task 8: Platformer �?ASCII Renderer (M2)

The platformer needs a way to visualize itself. Since engine-render requires wgpu setup, we'll create a simple terminal-based ASCII renderer for validation, with notes for wgpu integration.

**Files:**
- Modify: `crates/engine-core/examples/platformer_demo.rs`

- [ ] **Step 1: Add ASCII rendering for terminal validation**

Add before `main()`:

```rust
/// Simple ASCII renderer for terminal-based validation.
/// In a full integration, this would use engine-render's Sprite pipeline.
fn render_ascii(world: &World) {
    let width = 80usize;
    let height = 25usize;
    let mut buffer = vec![vec![' '; width]; height];

    // Render platforms
    let platforms: Vec<u32> = world.component_entities::<Platform>();
    for &eid in &platforms {
        if let Some(transform) = world.get_by_index::<Transform>(eid) {
            if let Some(collider) = world.get_by_index::<Collider2D>(eid) {
                let aabb = collider.world_aabb(Vec2::new(transform.translation.x, transform.translation.y));
                let x1 = ((aabb.min.x / TILE_SIZE) as i32).max(0) as usize;
                let x2 = ((aabb.max.x / TILE_SIZE) as i32).min(width as i32 - 1) as usize;
                let y1 = ((aabb.min.y / TILE_SIZE) as i32).max(0) as usize;
                let y2 = ((aabb.max.y / TILE_SIZE) as i32).min(height as i32 - 1) as usize;
                for y in y1..=y2 {
                    for x in x1..=x2 {
                        buffer[y][x] = '#';
                    }
                }
            }
        }
    }

    // Render coins
    let coins: Vec<u32> = world.component_entities::<Collectible>();
    for &eid in &coins {
        if let Some(col) = world.get_by_index::<Collectible>(eid) {
            if col.collected { continue; }
        }
        if let Some(transform) = world.get_by_index::<Transform>(eid) {
            let x = ((transform.translation.x / TILE_SIZE) as i32).max(0).min(width as i32 - 1) as usize;
            let y = ((transform.translation.y / TILE_SIZE) as i32).max(0).min(height as i32 - 1) as usize;
            buffer[y][x] = '*';
        }
    }

    // Render enemies
    let enemies: Vec<u32> = world.component_entities::<EnemyAI>();
    for &eid in &enemies {
        if let Some(transform) = world.get_by_index::<Transform>(eid) {
            let x = ((transform.translation.x / TILE_SIZE) as i32).max(0).min(width as i32 - 1) as usize;
            let y = ((transform.translation.y / TILE_SIZE) as i32).max(0).min(height as i32 - 1) as usize;
            buffer[y][x] = 'E';
        }
    }

    // Render goal
    let goals: Vec<u32> = world.component_entities::<GoalMarker>();
    for &eid in &goals {
        if let Some(transform) = world.get_by_index::<Transform>(eid) {
            let x = ((transform.translation.x / TILE_SIZE) as i32).max(0).min(width as i32 - 1) as usize;
            let y = ((transform.translation.y / TILE_SIZE) as i32).max(0).min(height as i32 - 1) as usize;
            buffer[y][x] = 'F';
        }
    }

    // Render player (last, so it's on top)
    let players: Vec<u32> = world.component_entities::<PlayerState>();
    for &eid in &players {
        if let Some(transform) = world.get_by_index::<Transform>(eid) {
            let x = ((transform.translation.x / TILE_SIZE) as i32).max(0).min(width as i32 - 1) as usize;
            let y = ((transform.translation.y / TILE_SIZE) as i32).max(0).min(height as i32 - 1) as usize;
            buffer[y][x] = '@';
        }
    }

    // Print (flip Y for terminal)
    println!("\x1B[2J\x1B[H"); // Clear screen
    for row in buffer.iter().rev() {
        println!("{}", row.iter().collect::<String>());
    }

    // HUD
    if let Some(eid) = world.component_entities::<PlayerState>().first().copied() {
        if let Some(player) = world.get_by_index::<PlayerState>(eid) {
            println!("Score: {} | Lives: {}", player.score, player.lives);
        }
    }
}
```

- [ ] **Step 2: Add simulation loop to main()**

Replace the `println!` statements at the end of main() with:

```rust
    // Simulation loop (terminal-based)
    for frame in 0..300 {
        // Update input (simulated �?no real window events in this mode)
        {
            let mut input = app.world_mut().get_resource_mut::<InputManager>().unwrap();
            // Simulate right arrow held for first 100 frames
            if frame < 100 {
                input.press(KeyCode::ArrowRight);
            } else if frame == 100 {
                input.release(KeyCode::ArrowRight);
            }
            input.update_frame();
        }

        // Step physics
        app.run();

        // Render every 3rd frame
        if frame % 3 == 0 {
            render_ascii(&app.world);
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
```

- [ ] **Step 3: Add missing import for InputManager**

Add to the imports at the top:

```rust
use engine_input::input_manager::InputManager;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check --example platformer_demo -p engine-core`
Expected: Compiles

- [ ] **Step 5: Commit**

```bash
git add crates/engine-core/examples/platformer_demo.rs
git commit -m "feat: add ASCII renderer and simulation loop to platformer demo"
```

---

## Task 9: Run & Debug Platformer (M3-M5)

**Files:**
- Modify: `crates/engine-core/examples/platformer_demo.rs`

- [ ] **Step 1: Run the platformer**

Run: `cargo run --example platformer_demo -p engine-core 2>&1 | Select-Object -First 100`
Expected: ASCII output showing player, platforms, coins, enemies

- [ ] **Step 2: Fix any compilation or runtime errors**

Address issues found in step 1. Common problems:
- Missing trait imports
- ECS API method name mismatches
- Type conversion issues

- [ ] **Step 3: Verify gameplay works**

- Player moves right when arrow key simulated
- Gravity pulls player down
- Player lands on platforms (ground detection works)
- Player doesn't fall through platforms

- [ ] **Step 4: Commit after fixes**

```bash
git add crates/engine-core/examples/platformer_demo.rs
git commit -m "fix: resolve runtime issues in platformer demo"
```

---

## Task 10: API Fixes (discovered during Tasks 6-9)

Based on issues found during platformer development, fix API pain points.

**Likely files to modify:**
- `crates/engine-input/src/input_manager.rs` �?if key_down/just_pressed ergonomics need improvement
- `crates/engine-ecs/src/world.rs` �?if component access patterns are verbose
- `crates/engine-physics/src/physics_2d.rs` �?if 2D physics API needs adjustment

- [ ] **Step 1: List all API friction points found**

Document each issue with: file, current API, proposed improvement, rationale.

- [ ] **Step 2: Implement fixes one at a time**

For each fix:
1. Make the change
2. Run `cargo check -p <crate>`
3. Run `cargo test -p <crate>`
4. Commit with descriptive message

- [ ] **Step 3: Verify platformer still works after all fixes**

Run: `cargo run --example platformer_demo -p engine-core`
Expected: Same behavior as before

---

## Task 11: Tutorial Documentation (M7)

**Files:**
- Create: `docs/platformer-tutorial.md`

- [ ] **Step 1: Write tutorial document**

```markdown
# Building a 2D Platformer with RustEngine

This tutorial walks through building a complete 2D platformer game using RustEngine.
By the end, you'll have a playable game with physics, enemies, collectibles, and game states.

## Prerequisites

- Rust 1.95.0+
- RustEngine cloned and built

## Overview

We'll build incrementally:
1. Window and game loop
2. Player movement with input
3. Tilemap and collision
4. Gravity and platform physics
5. Sprite animation
6. Enemies and AI
7. Sound effects
8. UI (score, lives)
9. Game states (pause, game over)

## Step 1: Window and Game Loop

[Content covering AppBuilder setup, plugins, basic entity spawning]

## Step 2: Player Movement

[Content covering InputManager, KeyCode, movement systems]

## Step 3: Physics and Collision

[Content covering physics_2d module, AABB, RigidBody2D, Collider2D]

## Step 4: Gravity and Ground Detection

[Content covering PhysicsWorld2D, gravity, grounded state]

## Step 5: Sprite Animation

[Content covering SpriteAnimation, frame sequences, ECS integration]

## Step 6: Enemies

[Content covering EnemyAI component, patrol behavior, collision with player]

## Step 7: Audio

[Content covering AudioManager, SFX triggers, background music]

## Step 8: UI

[Content covering engine-ui, score display, health bar]

## Step 9: Game States

[Content covering StateStack, pause, game over, restart]

## Complete Code

See `crates/engine-core/examples/platformer_demo.rs` for the full implementation.
```

- [ ] **Step 2: Fill in each step with actual code snippets from the platformer**

Reference the actual code in platformer_demo.rs, explaining key patterns.

- [ ] **Step 3: Commit**

```bash
git add docs/platformer-tutorial.md
git commit -m "docs: add 2D platformer tutorial"
```

---

## Task 12: Final Verification (M7)

- [ ] **Step 1: Run full test suite**

Run: `cargo test --all 2>&1 | Select-String "test result"`
Expected: All pass, 0 failures

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --all 2>&1 | Select-String "warning|error"`
Expected: 0 warnings

- [ ] **Step 3: Run platformer end-to-end**

Run: `cargo run --example platformer_demo -p engine-core`
Expected: Runs without crash, player interacts with level

- [ ] **Step 4: Verify all examples compile**

Run: `cargo build --examples 2>&1`
Expected: All examples compile

- [ ] **Step 5: Final commit if needed**

```bash
git add -A
git commit -m "chore: final cleanup for v0.3.0 platformer iteration"
```
