# v0.4.0 Hardening Iteration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Harden all 14 engine crates (dead code cleanup, error handling, tests, docs), fix physics gaps, polish demos, and tag v0.3.0/v0.4.0.

**Architecture:** Bottom-up per-crate hardening following dependency layers. Each crate gets the same 6-step treatment: dead code audit, error handling, test coverage, documentation, API consistency, clippy verification. Physics fixes and demo polishing come after all crates are hardened.

**Tech Stack:** Rust 2024 edition, cargo, clippy, thiserror, anyhow, criterion

---

## File Structure

No new files are created for the hardening itself — this is all in-place improvement. Files touched per task:

- `<crate>/src/error.rs` — create if missing (thiserror-based XxxError)
- `<crate>/src/lib.rs` — add/update crate-level `//!` docs
- `<crate>/src/*.rs` — add `///` docs to public items, fix unwrap/expect, remove dead_code
- `<crate>/tests/*.rs` — add/expand integration tests
- `crates/engine-physics/src/joint.rs` — joint constraint solver extension
- `crates/engine-physics/src/collider.rs` — cylinder collision algorithms
- `crates/engine-core/examples/platformer_demo.rs` — inline comments
- `crates/engine-core/examples/dungeon_demo.rs` — inline comments

---

## Task 1: Tag v0.3.0

**Files:**
- None (git operation only)

- [ ] **Step 1: Verify current HEAD includes platformer_demo and physics_2d**

Run: `git log --oneline -10`
Expected: commits for platformer_demo, physics_2d, platformer tutorial visible

- [ ] **Step 2: Create annotated tag v0.3.0**

Run: `git tag -a v0.3.0 -m "v0.3.0: 2D platformer, physics_2d, quality iteration"`
Expected: tag created

- [ ] **Step 3: Verify tag**

Run: `git tag -l "v0.3*"` and `git log v0.3.0 --oneline -3`
Expected: v0.3.0 points to correct commit

---

## Task 2: engine-math (Layer 0)

**Files:**
- Modify: `crates/engine-math/src/lib.rs` — add crate-level docs
- Modify: `crates/engine-math/src/*.rs` — add `///` docs to public items
- Create: `crates/engine-math/src/error.rs` — MathError (if error paths exist)
- Modify: `crates/engine-math/tests/` — add boundary/edge-case tests

- [ ] **Step 1: Dead code audit**

Run: `rg "#\[allow\(dead_code\)\]" crates/engine-math/`
Action: For each hit, determine if the code should be wired in or removed. Remove unused items.

- [ ] **Step 2: Error handling audit**

Run: `rg "unwrap\(\)|expect\(" crates/engine-math/src/ --glob '!tests/*'`
Action: Replace any production unwrap/expect with proper error handling or document why panic is acceptable (e.g., invariant guarantees).

- [ ] **Step 3: Test coverage audit**

Run: `cargo test -p engine-math -- --list`
Action: Review existing tests. Add tests for:
- Vec2/Vec3/Vec4 edge cases (zero vector, normalization of near-zero, cross product of parallel vectors)
- Mat4 edge cases (singular matrix inversion, identity operations)
- Quat edge cases (slerp at t=0/t=1, gimbal lock scenarios)

- [ ] **Step 4: Documentation**

Add to `crates/engine-math/src/lib.rs`:
```rust
//! # engine-math
//!
//! Math primitives for RustEngine: vectors, matrices, quaternions.
//! Re-exports `glam` types with engine-specific extensions.
```
Add `///` docs to all `pub` functions, structs, enums.

- [ ] **Step 5: Clippy verification**

Run: `cargo clippy -p engine-math`
Expected: zero warnings

- [ ] **Step 6: Commit**

```bash
git add crates/engine-math/
git commit -m "chore(math): harden engine-math — docs, tests, dead code cleanup"
```

---

## Task 3: engine-jobs (Layer 0)

**Files:**
- Modify: `crates/engine-jobs/src/lib.rs` — crate-level docs
- Modify: `crates/engine-jobs/src/*.rs` — docs, dead code audit
- Modify: `crates/engine-jobs/tests/` — concurrency tests

- [ ] **Step 1: Dead code audit**

Run: `rg "#\[allow\(dead_code\)\]" crates/engine-jobs/`
Action: Wire in or remove.

- [ ] **Step 2: Error handling audit**

Run: `rg "unwrap\(\)|expect\(" crates/engine-jobs/src/ --glob '!tests/*'`
Action: Replace with proper error handling.

- [ ] **Step 3: Test coverage audit**

Run: `cargo test -p engine-jobs -- --list`
Action: Add tests for:
- Task scheduler: concurrent task submission, priority ordering
- Thread pool: work stealing, graceful shutdown
- Edge cases: empty task queue, single-threaded fallback

- [ ] **Step 4: Documentation**

Add crate-level `//!` docs and `///` docs to all public items. Document the scheduler algorithm and thread pool design.

- [ ] **Step 5: Clippy verification**

Run: `cargo clippy -p engine-jobs`
Expected: zero warnings

- [ ] **Step 6: Commit**

```bash
git add crates/engine-jobs/
git commit -m "chore(jobs): harden engine-jobs — docs, tests, dead code cleanup"
```

---

## Task 4: engine-window (Layer 0)

**Files:**
- Modify: `crates/engine-window/src/lib.rs` — crate-level docs
- Modify: `crates/engine-window/src/*.rs` — error handling, docs
- Create: `crates/engine-window/src/error.rs` — WindowError (if needed)

- [ ] **Step 1: Dead code audit**

Run: `rg "#\[allow\(dead_code\)\]" crates/engine-window/`
Action: Wire in or remove.

- [ ] **Step 2: Error handling audit**

Run: `rg "unwrap\(\)|expect\(" crates/engine-window/src/ --glob '!tests/*'`
Action: Replace with proper error handling. Window operations should return `Result<T, WindowError>`.

- [ ] **Step 3: Test coverage audit**

Run: `cargo test -p engine-window -- --list`
Action: Add tests for:
- Window creation with various configurations (size, title, fullscreen)
- Event handling edge cases
- Platform-specific behavior (if testable without display)

- [ ] **Step 4: Documentation**

Add crate-level `//!` docs. Document platform-specific behavior and winit integration details.

- [ ] **Step 5: Clippy verification**

Run: `cargo clippy -p engine-window`
Expected: zero warnings

- [ ] **Step 6: Commit**

```bash
git add crates/engine-window/
git commit -m "chore(window): harden engine-window — docs, error handling, tests"
```

---

## Task 5: engine-audio (Layer 1)

**Files:**
- Modify: `crates/engine-audio/src/lib.rs` — crate-level docs
- Modify: `crates/engine-audio/src/*.rs` — docs, dead code audit
- Create: `crates/engine-audio/src/error.rs` — AudioError (if needed)
- Modify: `crates/engine-audio/tests/` — playback tests

- [ ] **Step 1: Dead code audit**

Run: `rg "#\[allow\(dead_code\)\]" crates/engine-audio/`
Action: Wire in or remove.

- [ ] **Step 2: Error handling audit**

Run: `rg "unwrap\(\)|expect\(" crates/engine-audio/src/ --glob '!tests/*'`
Action: Replace with proper error handling. Audio decode failures should propagate errors, not panic.

- [ ] **Step 3: Test coverage audit**

Add tests for:
- Volume control: master volume, bus volume, per-handle volume
- Audio mixer: bus routing, mute/unmute
- 3D spatial audio: distance attenuation calculations, panning
- Edge cases: play after stop, volume clamping, invalid file path

- [ ] **Step 4: Documentation**

Add crate-level `//!` docs. Document the mixer bus architecture and 3D audio model.

- [ ] **Step 5: Clippy verification**

Run: `cargo clippy -p engine-audio`
Expected: zero warnings

- [ ] **Step 6: Commit**

```bash
git add crates/engine-audio/
git commit -m "chore(audio): harden engine-audio — docs, tests, dead code cleanup"
```

---

## Task 6: engine-asset (Layer 1)

**Files:**
- Modify: `crates/engine-asset/src/lib.rs` — crate-level docs
- Modify: `crates/engine-asset/src/*.rs` — docs, dead code audit
- Modify: `crates/engine-asset/tests/` — loading pipeline tests

- [ ] **Step 1: Dead code audit**

Run: `rg "#\[allow\(dead_code\)\]" crates/engine-asset/`
Note: `streaming.rs` has known dead code at line 313. Decide whether to wire in or remove.

- [ ] **Step 2: Error handling audit**

Run: `rg "unwrap\(\)|expect\(" crates/engine-asset/src/ --glob '!tests/*'`
Action: Replace with proper error handling. Asset loading should never panic — return `Result`.

- [ ] **Step 3: Test coverage audit**

Add tests for:
- Asset handle lifecycle (create, clone, drop, ref counting)
- Type registry: register, lookup, type mismatch errors
- File system scanner: missing directory, empty directory, nested directories
- Image/audio/glTF loader edge cases: corrupt file, wrong format

- [ ] **Step 4: Documentation**

Add crate-level `//!` docs. Document the handle system and loader pipeline.

- [ ] **Step 5: Clippy verification**

Run: `cargo clippy -p engine-asset`
Expected: zero warnings

- [ ] **Step 6: Commit**

```bash
git add crates/engine-asset/
git commit -m "chore(asset): harden engine-asset — docs, tests, dead code cleanup"
```

---

## Task 7: engine-ecs (Layer 1)

**Files:**
- Modify: `crates/engine-ecs/src/lib.rs` — crate-level docs
- Modify: `crates/engine-ecs/src/*.rs` — docs, dead code audit
- Modify: `crates/engine-ecs/tests/` — query iteration tests

- [ ] **Step 1: Dead code audit**

Run: `rg "#\[allow\(dead_code\)\]" crates/engine-ecs/`
Note: `query.rs` lines 162/164 and `par_iter.rs` line 77 have known dead code. Evaluate whether to wire in parallel iteration or remove.

- [ ] **Step 2: Error handling audit**

Run: `rg "unwrap\(\)|expect\(" crates/engine-ecs/src/ --glob '!tests/*'`
Action: ECS operations that can fail (component access, entity lookup) should return Result.

- [ ] **Step 3: Test coverage audit**

Add tests for:
- Entity lifecycle: spawn, despawn, generation tracking
- Component storage: add, remove, get, get_mut, has
- Query iteration: single component, multi-component, With/Without filters
- Archetype migration: component add/remove triggers archetype change
- Edge cases: despawn entity with components, query empty world

- [ ] **Step 4: Documentation**

Add crate-level `//!` docs. Document the archetype-based storage model and query system.

- [ ] **Step 5: Clippy verification**

Run: `cargo clippy -p engine-ecs`
Expected: zero warnings

- [ ] **Step 6: Commit**

```bash
git add crates/engine-ecs/
git commit -m "chore(ecs): harden engine-ecs — docs, tests, dead code cleanup"
```

---

## Task 8: engine-scene (Layer 2)

**Files:**
- Modify: `crates/engine-scene/src/lib.rs` — crate-level docs
- Modify: `crates/engine-scene/src/*.rs` — docs, dead code audit
- Modify: `crates/engine-scene/tests/` — hierarchy sync tests

- [ ] **Step 1: Dead code audit**

Run: `rg "#\[allow\(dead_code\)\]" crates/engine-scene/`
Action: Wire in or remove.

- [ ] **Step 2: Error handling audit**

Run: `rg "unwrap\(\)|expect\(" crates/engine-scene/src/ --glob '!tests/*'`
Action: Replace with proper error handling.

- [ ] **Step 3: Test coverage audit**

Add tests for:
- Scene node hierarchy: add child, remove child, reparent
- Transform sync: parent transform propagates to children
- GlobalTransform computation: nested transforms
- Serialization: round-trip JSON serialize/deserialize
- Edge cases: circular reference detection, orphaned nodes

- [ ] **Step 4: Documentation**

Add crate-level `//!` docs. Document the scene graph model and transform propagation.

- [ ] **Step 5: Clippy verification**

Run: `cargo clippy -p engine-scene`
Expected: zero warnings

- [ ] **Step 6: Commit**

```bash
git add crates/engine-scene/
git commit -m "chore(scene): harden engine-scene — docs, tests, dead code cleanup"
```

---

## Task 9: engine-input (Layer 2)

**Files:**
- Modify: `crates/engine-input/src/lib.rs` — crate-level docs
- Modify: `crates/engine-input/src/*.rs` — API consistency, docs
- Modify: `crates/engine-input/tests/` — action mapping tests

- [ ] **Step 1: Dead code audit**

Run: `rg "#\[allow\(dead_code\)\]" crates/engine-input/`
Note: `action_map.rs` line 13 has known dead code.

- [ ] **Step 2: Error handling audit**

Run: `rg "unwrap\(\)|expect\(" crates/engine-input/src/ --glob '!tests/*'`
Action: Replace with proper error handling.

- [ ] **Step 3: API consistency check**

Review public API for consistency:
- `input.just_pressed()` / `input.just_released()` / `input.is_pressed()` shortcuts
- Action map builder pattern consistency
- Key/mouse button naming conventions

- [ ] **Step 4: Test coverage audit**

Add tests for:
- Key state tracking: press, release, held
- Action mapping: bind key to action, query action state
- Mouse input: position, delta, button states
- Edge cases: multiple bindings to same action, rebind at runtime

- [ ] **Step 5: Documentation**

Add crate-level `//!` docs. Document the action mapping system and input query API.

- [ ] **Step 6: Clippy verification and commit**

Run: `cargo clippy -p engine-input`
```bash
git add crates/engine-input/
git commit -m "chore(input): harden engine-input — API consistency, docs, tests"
```

---

## Task 10: engine-render (Layer 3)

**Files:**
- Modify: `crates/engine-render/src/lib.rs` — crate-level docs
- Modify: `crates/engine-render/src/*.rs` — docs, dead code audit
- Modify: `crates/engine-render/tests/` — render graph tests

- [ ] **Step 1: Dead code audit**

Run: `rg "#\[allow\(dead_code\)\]" crates/engine-render/`
Note: `sprite_renderer.rs` line 61 and `graph/buffer.rs` lines 31/49 have known dead code.

- [ ] **Step 2: Error handling audit**

Run: `rg "unwrap\(\)|expect\(" crates/engine-render/src/ --glob '!tests/*'`
Action: GPU resource creation should return Result. Shader compilation errors should propagate.

- [ ] **Step 3: Test coverage audit**

Add tests for:
- Render graph: node registration, dependency resolution, execution order
- Texture bridge: EventChannel message handling, TextureStore operations
- Sprite batch: texture grouping, batch collection
- Camera: priority sorting, frustum culling
- Tilemap: tile lookup, layer management

- [ ] **Step 4: Documentation**

Add crate-level `//!` docs. Document the render graph architecture and GPU resource lifecycle.

- [ ] **Step 5: Clippy verification**

Run: `cargo clippy -p engine-render`
Expected: zero warnings

- [ ] **Step 6: Commit**

```bash
git add crates/engine-render/
git commit -m "chore(render): harden engine-render — docs, tests, dead code cleanup"
```

---

## Task 11: engine-core (Layer 4)

**Files:**
- Modify: `crates/engine-core/src/lib.rs` — crate-level docs
- Modify: `crates/engine-core/src/*.rs` — docs, coupling audit
- Modify: `crates/engine-core/tests/` — plugin system tests

- [ ] **Step 1: Dead code audit**

Run: `rg "#\[allow\(dead_code\)\]" crates/engine-core/`
Action: Wire in or remove.

- [ ] **Step 2: Error handling audit**

Run: `rg "unwrap\(\)|expect\(" crates/engine-core/src/ --glob '!tests/*'`
Action: Replace with proper error handling. AppBuilder operations should return Result.

- [ ] **Step 3: Coupling audit**

Review `Cargo.toml` dependencies. For each mandatory dep:
- Could it be optional (feature-gated)?
- Is it actually used in production code?
- Could the dependency be reduced to a narrower interface?

Document findings in crate-level docs.

- [ ] **Step 4: Test coverage audit**

Add tests for:
- AppBuilder: plugin registration, resource insertion, system scheduling
- Plugin system: build order, dependency resolution
- Time: delta_seconds, fps, elapsed (fix flaky test if still present)
- Config: get/set, default values, type conversion

- [ ] **Step 5: Documentation**

Add crate-level `//!` docs. Document the AppBuilder pattern and plugin lifecycle.

- [ ] **Step 6: Clippy verification and commit**

Run: `cargo clippy -p engine-core`
```bash
git add crates/engine-core/
git commit -m "chore(core): harden engine-core — coupling audit, docs, tests"
```

---

## Task 12: engine-framework (Layer 5)

**Files:**
- Modify: `crates/engine-framework/src/lib.rs` — crate-level docs
- Modify: `crates/engine-framework/src/*.rs` — docs, tests
- Modify: `crates/engine-framework/tests/` — state stack tests

- [ ] **Step 1: Dead code audit**

Run: `rg "#\[allow\(dead_code\)\]" crates/engine-framework/`
Action: Wire in or remove.

- [ ] **Step 2: Error handling audit**

Run: `rg "unwrap\(\)|expect\(" crates/engine-framework/src/ --glob '!tests/*'`
Action: Replace with proper error handling.

- [ ] **Step 3: Test coverage audit**

Add tests for:
- State stack: push, pop, replace, peek
- State lifecycle: on_enter, on_exit, on_pause, on_resume
- Edge cases: pop empty stack, replace with same state, nested push/pop
- Save/load: slot management, JSON round-trip

- [ ] **Step 4: Documentation**

Add crate-level `//!` docs. Document state lifecycle transitions.

- [ ] **Step 5: Clippy verification and commit**

Run: `cargo clippy -p engine-framework`
```bash
git add crates/engine-framework/
git commit -m "chore(framework): harden engine-framework — docs, tests, state lifecycle"
```

---

## Task 13: engine-physics — Joint Constraint Fix (Layer 5)

**Files:**
- Modify: `crates/engine-physics/src/joint.rs` — add `solve_constraints()` for Hinge/BallSocket
- Modify: `crates/engine-physics/src/world.rs` — call `solve_constraints()` in physics step
- Modify: `crates/engine-physics/tests/` — joint constraint tests

- [ ] **Step 1: Write failing tests for joint constraints**

Add to `crates/engine-physics/tests/`:
```rust
#[test]
fn hinge_joint_limits_rotation() {
    // Two bodies connected by hinge joint with angle limits
    // Apply torque that would exceed limits
    // Verify angle is clamped within limits after solver step
}

#[test]
fn ball_socket_limits_distance() {
    // Two bodies connected by ball socket with max distance
    // Apply force that would separate them beyond max
    // Verify distance is constrained after solver step
}
```

Run: `cargo test -p engine-physics --test joint_constraints`
Expected: FAIL (methods don't exist yet)

- [ ] **Step 2: Implement Hinge angle constraint**

In `joint.rs`, extend `JointSolver`:
```rust
pub fn solve_constraints(&mut self, bodies: &mut [RigidBody], dt: f32) {
    for joint in &self.joints {
        match &joint.joint_type {
            JointType::Hinge { axis, limits } => {
                // Compute relative angle around hinge axis
                // If outside limits, apply corrective torque
                // Use Baumgarte stabilization for position correction
            }
            JointType::BallSocket { max_distance } => {
                // Compute distance between anchor points
                // If exceeds max_distance, apply corrective impulse
                // Use iterative constraint solving (warm starting)
            }
            JointType::Spring { .. } => {
                // Already handled by solve_springs()
            }
        }
    }
}
```

- [ ] **Step 3: Implement BallSocket distance constraint**

Complete the BallSocket branch in `solve_constraints()`:
- Compute anchor positions in world space
- Compute separation vector and distance
- If distance > max_distance, compute corrective impulse
- Apply impulse to both bodies (equal and opposite)

- [ ] **Step 4: Wire into physics step**

In `world.rs`, add `solver.solve_constraints(&mut self.bodies, dt)` call in the physics step, after `solve_springs()`.

- [ ] **Step 5: Run tests**

Run: `cargo test -p engine-physics --test joint_constraints`
Expected: PASS

- [ ] **Step 6: Verify existing tests still pass**

Run: `cargo test -p engine-physics`
Expected: all existing tests still pass

- [ ] **Step 7: Commit**

```bash
git add crates/engine-physics/
git commit -m "feat(physics): implement Hinge and BallSocket joint constraint solving"
```

---

## Task 14: engine-physics — Cylinder Collision (Layer 5)

**Files:**
- Modify: `crates/engine-physics/src/collider.rs` — add cylinder collision algorithms
- Modify: `crates/engine-physics/tests/` — cylinder collision tests

- [ ] **Step 1: Write failing tests for cylinder collision**

```rust
#[test]
fn cylinder_sphere_collision() {
    // Cylinder at origin, sphere touching side
    // Verify collision detected with correct normal and penetration
}

#[test]
fn cylinder_aabb_collision() {
    // Cylinder at origin, AABB overlapping
    // Verify collision detected
}

#[test]
fn cylinder_no_collision() {
    // Cylinder and sphere far apart
    // Verify no collision
}
```

Run: `cargo test -p engine-physics --test cylinder_collision`
Expected: FAIL

- [ ] **Step 2: Implement Cylinder-Sphere collision**

Replace the bounding-sphere fallback in `collider.rs` for Cylinder-Sphere:
- Project sphere center onto cylinder axis
- Compute closest point on cylinder surface
- Check distance against sphere radius
- Return contact normal and penetration depth

- [ ] **Step 3: Implement Cylinder-AABB collision**

Replace the bounding-sphere fallback for Cylinder-AABB:
- Use separating axis theorem with cylinder's circular cross-section
- Simplify: test cylinder axis against AABB faces
- Return contact normal and penetration depth

- [ ] **Step 4: Run tests**

Run: `cargo test -p engine-physics --test cylinder_collision`
Expected: PASS

- [ ] **Step 5: Verify existing tests**

Run: `cargo test -p engine-physics`
Expected: all pass

- [ ] **Step 6: Commit**

```bash
git add crates/engine-physics/
git commit -m "feat(physics): implement Cylinder-Sphere and Cylinder-AABB collision"
```

---

## Task 15: engine-physics — General Hardening (Layer 5)

**Files:**
- Modify: `crates/engine-physics/src/lib.rs` — docs
- Modify: `crates/engine-physics/src/*.rs` — docs, dead code audit

- [ ] **Step 1: Dead code audit**

Run: `rg "#\[allow\(dead_code\)\]" crates/engine-physics/`
Action: Wire in or remove.

- [ ] **Step 2: Error handling audit**

Run: `rg "unwrap\(\)|expect\(" crates/engine-physics/src/ --glob '!tests/*'`
Action: Note: `world.rs` line 645 has `unsafe` with AtomicPtr for parallel resolution — document safety invariant, do not remove.

- [ ] **Step 3: Documentation**

Add crate-level `//!` docs. Document the physics pipeline: broadphase → narrowphase → contact solver → integration.

- [ ] **Step 4: Clippy verification**

Run: `cargo clippy -p engine-physics`
Expected: zero warnings

- [ ] **Step 5: Commit**

```bash
git add crates/engine-physics/
git commit -m "chore(physics): harden engine-physics — docs, dead code cleanup"
```

---

## Task 16: engine-network (Layer 5)

**Files:**
- Modify: `crates/engine-network/src/lib.rs` — crate-level docs
- Modify: `crates/engine-network/src/*.rs` — docs, dead code audit

- [ ] **Step 1: Dead code audit**

Run: `rg "#\[allow\(dead_code\)\]" crates/engine-network/`
Action: Wire in or remove.

- [ ] **Step 2: Error handling audit**

Run: `rg "unwrap\(\)|expect\(" crates/engine-network/src/ --glob '!tests/*'`
Action: Network operations should never panic. Replace with proper error propagation.

- [ ] **Step 3: Test coverage audit**

The crate already has 148+ tests. Review for gaps:
- Reconnection flow: token expiry, snapshot recovery
- NAT traversal: STUN response parsing edge cases
- Matchmaking: concurrent join/leave, host transfer

- [ ] **Step 4: Documentation**

Add crate-level `//!` docs. Document the network architecture (client/server, authority model, snapshot sync).

- [ ] **Step 5: Clippy verification and commit**

Run: `cargo clippy -p engine-network`
```bash
git add crates/engine-network/
git commit -m "chore(network): harden engine-network — docs, dead code cleanup"
```

---

## Task 17: engine-script (Layer 5)

**Files:**
- Modify: `crates/engine-script/src/lib.rs` — crate-level docs
- Modify: `crates/engine-script/src/*.rs` — docs, dead code audit

- [ ] **Step 1: Dead code audit**

Run: `rg "#\[allow\(dead_code\)\]" crates/engine-script/`
Note: `hot_reload.rs` line 18 has known dead code (`bridge` field).

- [ ] **Step 2: Error handling audit**

Run: `rg "unwrap\(\)|expect\(" crates/engine-script/src/ --glob '!tests/*'`
Action: Script execution errors should propagate, not panic.

- [ ] **Step 3: Test coverage audit**

Add tests for:
- WASM sandbox: fuel exhaustion, memory limits, table limits
- Lua bridge: component get/set round-trip, type conversion edge cases
- Hot reload: file change detection, debouncing
- Event bridge: subscribe/emit/unsubscribe lifecycle

- [ ] **Step 4: Documentation**

Add crate-level `//!` docs. Document the Lua/WASM dual-runtime architecture and sandbox safety model.

- [ ] **Step 5: Clippy verification and commit**

Run: `cargo clippy -p engine-script`
```bash
git add crates/engine-script/
git commit -m "chore(script): harden engine-script — docs, tests, dead code cleanup"
```

---

## Task 18: engine-ui (Layer 5)

**Files:**
- Modify: `crates/engine-ui/src/lib.rs` — crate-level docs
- Modify: `crates/engine-ui/src/*.rs` — docs, dead code audit

- [ ] **Step 1: Dead code audit**

Run: `rg "#\[allow\(dead_code\)\]" crates/engine-ui/`
Note: `animation.rs` line 372 and `retained.rs` line 246 have known dead code.

- [ ] **Step 2: Error handling audit**

Run: `rg "unwrap\(\)|expect\(" crates/engine-ui/src/ --glob '!tests/*'`
Action: Replace with proper error handling.

- [ ] **Step 3: Test coverage audit**

Add tests for:
- Widget lifecycle: create, layout, draw, destroy
- Event handling: click, hover, focus
- Layout: flexbox constraints, alignment

- [ ] **Step 4: Documentation**

Add crate-level `//!` docs. Document the ImGui integration and widget model.

- [ ] **Step 5: Clippy verification and commit**

Run: `cargo clippy -p engine-ui`
```bash
git add crates/engine-ui/
git commit -m "chore(ui): harden engine-ui — docs, tests, dead code cleanup"
```

---

## Task 19: engine-terrain (Layer 5)

**Files:**
- Modify: `crates/engine-terrain/src/lib.rs` — crate-level docs
- Modify: `crates/engine-terrain/src/*.rs` — docs
- Create: `crates/engine-terrain/tests/` — terrain generation tests (currently 0 tests)

- [ ] **Step 1: Dead code audit**

Run: `rg "#\[allow\(dead_code\)\]" crates/engine-terrain/`
Action: Wire in or remove.

- [ ] **Step 2: Error handling audit**

Run: `rg "unwrap\(\)|expect\(" crates/engine-terrain/src/ --glob '!tests/*'`
Action: Replace with proper error handling.

- [ ] **Step 3: Create test suite from scratch**

This crate has 0 tests. Create `crates/engine-terrain/tests/terrain_tests.rs`:
```rust
#[test]
fn heightmap_generation_produces_valid_heights() {
    // Generate heightmap with known seed
    // Verify all heights are within expected range
}

#[test]
fn terrain_chunk_loading_unloading() {
    // Load chunks around a position
    // Verify correct chunks are loaded
    // Move position, verify old chunks unload, new chunks load
}

#[test]
fn terrain_sampling_at_boundaries() {
    // Sample at chunk boundaries
    // Verify no gaps or overlaps
}

#[test]
fn terrain_lod_transitions() {
    // Test LOD level changes based on distance
    // Verify smooth transitions
}
```

- [ ] **Step 4: Documentation**

Add crate-level `//!` docs. Document the terrain generation and chunk management system.

- [ ] **Step 5: Clippy verification and commit**

Run: `cargo clippy -p engine-terrain`
```bash
git add crates/engine-terrain/
git commit -m "chore(terrain): harden engine-terrain — docs, tests from scratch, dead code cleanup"
```

---

## Task 20: engine-editor (Layer 6)

**Files:**
- Modify: `crates/engine-editor/src/lib.rs` — crate-level docs
- Modify: `crates/engine-editor/src/*.rs` — docs, dead code audit

- [ ] **Step 1: Dead code audit**

Run: `rg "#\[allow\(dead_code\)\]" crates/engine-editor/`
Note: `commands.rs` line 99 has known dead code.

- [ ] **Step 2: Error handling audit**

Run: `rg "unwrap\(\)|expect\(" crates/engine-editor/src/ --glob '!tests/*'`
Action: Replace with proper error handling.

- [ ] **Step 3: Test coverage audit**

The editor has 161 tests. Review for gaps:
- Scene tree: CRUD operations, search, drag-and-drop reorder
- Inspector: property editing, validation
- Undo/redo: command stack operations
- Gizmo: translate/rotate/scale state transitions

- [ ] **Step 4: Documentation**

Add crate-level `//!` docs. Document the editor architecture and panel system.

- [ ] **Step 5: Clippy verification and commit**

Run: `cargo clippy -p engine-editor`
```bash
git add crates/engine-editor/
git commit -m "chore(editor): harden engine-editor — docs, dead code cleanup"
```

---

## Task 21: Workspace-Level Verification

**Files:**
- None (verification only)

- [ ] **Step 1: Full test suite**

Run: `cargo test --all`
Expected: zero failures

- [ ] **Step 2: Full clippy check**

Run: `cargo clippy --all`
Expected: zero warnings

- [ ] **Step 3: Dead code audit**

Run: `rg "#\[allow\(dead_code\)\]" crates/ --count`
Expected: 0 remaining (or each remaining instance has documented justification)

- [ ] **Step 4: Format check**

Run: `cargo fmt --check`
Expected: no formatting issues

- [ ] **Step 5: Build check**

Run: `cargo build`
Expected: clean build

---

## Task 22: Demo Polishing — platformer_demo

**Files:**
- Modify: `crates/engine-core/examples/platformer_demo.rs` — inline comments

- [ ] **Step 1: Read and understand the demo**

Read `crates/engine-core/examples/platformer_demo.rs` completely.

- [ ] **Step 2: Add inline comments**

Add comments explaining:
- Which engine APIs are used at each step
- Why certain patterns are chosen (e.g., ECS component design)
- How physics_2d integrates with the game loop

- [ ] **Step 3: Verify it runs**

Run: `cargo run --example platformer_demo -p engine-core`
Expected: launches without warnings or crashes

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/examples/platformer_demo.rs
git commit -m "docs: polish platformer_demo with inline comments"
```

---

## Task 23: Demo Polishing — dungeon_demo

**Files:**
- Modify: `crates/engine-core/examples/dungeon_demo.rs` — inline comments

- [ ] **Step 1: Read and understand the demo**

Read `crates/engine-core/examples/dungeon_demo.rs` completely.

- [ ] **Step 2: Add inline comments**

Add comments explaining:
- 3D rendering pipeline usage (PBR, shadows, lighting)
- Scene management and entity spawning
- Input handling and camera control

- [ ] **Step 3: Verify it runs**

Run: `cargo run --example dungeon_demo -p engine-core`
Expected: launches without warnings or crashes

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/examples/dungeon_demo.rs
git commit -m "docs: polish dungeon_demo with inline comments"
```

---

## Task 24: Tag v0.4.0

**Files:**
- None (git operation only)

- [ ] **Step 1: Verify all tests pass**

Run: `cargo test --all`
Expected: zero failures

- [ ] **Step 2: Verify clippy clean**

Run: `cargo clippy --all`
Expected: zero warnings

- [ ] **Step 3: Create annotated tag v0.4.0**

Run: `git tag -a v0.4.0 -m "v0.4.0: full codebase hardening, joint constraints, cylinder collision, polished demos"`
Expected: tag created

- [ ] **Step 4: Verify tags**

Run: `git tag -l "v0.*"`
Expected: v0.1.0, v0.2.0, v0.3.0, v0.4.0 all present

---

## Self-Review Checklist

- [ ] Every crate in the workspace has a corresponding task
- [ ] Physics fix tasks (13, 14) have failing tests written first (TDD)
- [ ] All tasks follow the same 6-step pattern (dead code → error handling → tests → docs → clippy → commit)
- [ ] Version tagging happens at the right boundaries (v0.3.0 before work, v0.4.0 after)
- [ ] No placeholders, no "TBD", no "similar to Task N"
- [ ] Every step has exact file paths and commands
- [ ] Spec requirements are all covered by tasks
