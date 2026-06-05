# v0.3.0 Iteration: Example-Driven Development — 2D Platformer

**Date:** 2026-06-06
**Status:** Approved
**Scope:** 2D platformer game example to validate engine subsystems, fix exposed API issues, add tutorial docs
**Strategy:** Bottom-up incremental build

---

## Overview

RustEngine v0.2.0 completed all 9 development stages (17 crates, ~70k lines) and a quality iteration (error handling unification, per-crate docs/tests, integration tests). However, the subsystems have never been validated together in a real game scenario.

v0.3.0 uses a complete 2D platformer game as the quality gate. Building a real game exposes API pain points, integration gaps, and missing features that unit tests cannot catch.

**Deliverables:**
1. `engine-physics::physics_2d` — lightweight 2D physics module
2. `examples/platformer_demo` — complete 2D platformer game
3. API fixes discovered during development
4. Flaky test fix (`test_time_fps`)
5. `docs/platformer-tutorial.md` — step-by-step tutorial

---

## Incremental Build Steps

Each step produces a runnable program. Problems are fixed as they surface.

| Step | Content | Systems Validated |
|------|---------|-------------------|
| 1 | Window + colored square + game loop | engine-core, engine-window |
| 2 | Player movement + input | engine-input |
| 3 | Tilemap + collision | engine-render (tilemap), custom AABB |
| 4 | Gravity + platform physics | physics_2d (new) |
| 5 | Sprite animation | engine-render (animation) |
| 6 | Enemies + AI | ECS queries + simple state machine |
| 7 | SFX + background music | engine-audio |
| 8 | UI (score, lives, menus) | engine-ui |
| 9 | Game states (pause, game over, restart) | engine-framework |
| 10 | Polish + tutorial docs | All systems |

---

## 2D Physics Module Design

**Location:** `crates/engine-physics/src/physics_2d.rs`

**Core Structures:**

```rust
pub struct AABB2D {
    pub min: Vec2,
    pub max: Vec2,
}

pub struct RigidBody2D {
    pub velocity: Vec2,
    pub gravity_scale: f32,
    pub grounded: bool,
    pub body_type: BodyType2D, // Dynamic, Static, Kinematic
}

pub enum BodyType2D {
    Dynamic,
    Static,
    Kinematic,
}

pub struct Collider2D {
    pub aabb: AABB2D,
    pub friction: f32,
    pub restitution: f32,
    pub is_trigger: bool,
}

pub struct Contact2D {
    pub entity_a: Entity,
    pub entity_b: Entity,
    pub normal: Vec2,
    pub penetration: f32,
    pub is_trigger: bool,
}

pub struct PhysicsWorld2D {
    pub gravity: Vec2, // default (0.0, -9.81)
}
```

**Capabilities:**
- AABB vs AABB collision detection
- Simple gravity integration
- Ground detection (collision normal pointing up = grounded)
- Velocity damping
- Trigger support (for pickups, checkpoints, etc.)

**Not included:** rotation, circle collision, constraint solving, CCD — not needed for 2D platformers.

**ECS Integration:** New `physics_2d_step_system` in `plugin.rs`, parallel to existing 3D system.

---

## Platformer Game Design

### Gameplay

- Player controls a square character: left/right movement + jump
- 3 levels with increasing difficulty (more enemies, complex terrain)
- Enemies: patrol type (walk back and forth) and chase type (move toward player)
- Collectibles: coins (score) and hearts (extra lives)
- Goal: reach the end of each level (flag/door)

### Entity Design

| Entity | Components | Notes |
|--------|-----------|-------|
| Player | Sprite, RigidBody2D, Collider2D, Transform, PlayerState | Player character |
| Enemy | Sprite, RigidBody2D, Collider2D, Transform, EnemyAI | Enemy |
| Platform | Tilemap Tile or Sprite, Collider2D (static), Transform | Platform/ground |
| Coin | Sprite, Collider2D (trigger), Transform, Collectible | Pickup |
| Goal | Sprite, Collider2D (trigger), Transform, GoalMarker | Level exit |

### ASCII Art Resources

- Player: `@` or `P` character sprite
- Enemy: `E` or `X`
- Platform: `#` block
- Coin: `*` or `$`
- Goal: `F` (flag)
- Background: solid color gradient

---

## Expected API Fixes

Building the platformer will expose API issues. Types of problems expected:

| Category | Likely Issue | Fix Direction |
|----------|-------------|---------------|
| Sprite creation | Too many steps to create textured sprite | Simplify with builder pattern |
| Tilemap collision | Tilemap has rendering only, no collision query | Add tile collision query API |
| Input query | Multi-step to check key state | Add `input.just_pressed()` shortcuts |
| Audio trigger | SFX playback needs handle management | Simplify to `audio.play_sfx("jump")` |
| State switching | Boilerplate for game state transitions | Provide concise state transition methods |
| ECS queries | Multi-component queries verbose | Confirm Query API ergonomics |

**Principle:** Do NOT pre-improve APIs. Fix issues as discovered during example development. Each fix is a separate commit.

---

## Stability Fixes

### Flaky Test: `test_time_fps`

- **File:** `crates/engine-core/tests/core_tests.rs:260`
- **Problem:** Asserts `fps > 0.0` which fails on extremely short frame times
- **Fix:** Add minimum time delta or mock time source

### Runtime Stability

- Fix any crashes/panics discovered when running the full example
- Ensure `cargo run --example platformer_demo` runs stably on Windows

---

## Tutorial Documentation

**File:** `docs/platformer-tutorial.md`

**Structure:**
1. Overview and goals
2. Step 1: Window and game loop
3. Step 2: Player movement and input
4. Step 3: Tilemap and collision
5. Step 4: Gravity and platform physics
6. Step 5: Sprite animation
7. Step 6: Enemies and AI
8. Step 7: Sound effects
8. Step 8: UI
9. Step 9: Game state management
10. Step 10: Full code review

Each step includes: code snippets, expected behavior description, key API notes.

---

## Milestones

| Milestone | Content | Acceptance Criteria |
|-----------|---------|-------------------|
| **M1** | Window + player movement + input | Square moves on screen, key response |
| **M2** | Tilemap + AABB collision | Player stands on platform, no penetration |
| **M3** | Gravity + jump | Player can jump, lands with collision |
| **M4** | Enemies + collectibles | Enemies patrol, coins collectible |
| **M5** | Animation + audio | Character animated, jump/collect have SFX |
| **M6** | UI + state management | Score display, pause/game over menus |
| **M7** | 3 levels + tutorial doc | Complete playable game + tutorial |

---

## Success Criteria

| Criteria | Metric |
|----------|--------|
| Game runs | `cargo run --example platformer_demo` launches and is playable |
| All 3 levels | Player can progress through 3 levels |
| Zero crashes | No panics during normal gameplay |
| API improvements | At least 3 API pain points fixed |
| Flaky test fixed | `test_time_fps` passes consistently |
| Tutorial complete | `docs/platformer-tutorial.md` covers all 10 steps |
| Existing tests pass | `cargo test --all` — 0 new failures |
| Clippy clean | `cargo clippy --all` — 0 warnings |

---

## Out of Scope

- 3D rendering or physics integration
- Networking / multiplayer
- Editor integration
- External asset pipeline (all resources are procedural/ASCII)
- Performance optimization beyond identified bottlenecks
- New crate creation (physics_2d is a module inside engine-physics)
