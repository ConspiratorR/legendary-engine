# Phase 2: Layer 0-1 Foundation Crates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Goal:** Polish the foundation crates (engine-math, engine-jobs, engine-window, engine-audio, engine-asset, engine-ecs) with tests, docs, benchmarks, and API consistency.

**Architecture:** Process each crate sequentially, completing all quality items before moving to the next. Each crate follows the universal checklist: error migration (done in Phase 1), test coverage, module docs, function docs, API consistency, clippy clean, benchmarks.

**Tech Stack:** Rust 2024 edition, thiserror, anyhow, cargo test, cargo clippy, Criterion

---

### Task 1: Polish engine-math

**Files:**
- Modify: crates/engine-math/src/*.rs
- Create: crates/engine-math/benches/math_benchmarks.rs

- [ ] **Step 1: Add module-level documentation**

Add to top of crates/engine-math/src/lib.rs:

`ust
//! # engine-math
//!
//! Core math types and operations for the RustEngine.
//!
//! Provides vector, matrix, and quaternion types backed by glam.
//! Includes extension traits for additional functionality like
//! interpolation, angle conversions, and geometric operations.
//!
//! ## Quick Start
//!
//! `ust
//! use engine_math::{Vec3, Mat4, Quat};
//!
//! let position = Vec3::new(1.0, 2.0, 3.0);
//! let rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
//! let transform = Mat4::from_rotation_translation(rotation, position);
//! `
`

- [ ] **Step 2: Add documentation to all public functions**

Go through each public function in the math crate and add /// documentation with:
- One-line description
- Parameters explanation
- Return value
- Example usage

Example:
`ust
/// Linearly interpolate between two vectors.
///
/// # Arguments
/// *  - Start vector
/// *  - End vector
/// * 	 - Interpolation factor (0.0 = a, 1.0 = b)
///
/// # Example
/// `ust
/// use engine_math::Vec3;
/// let a = Vec3::ZERO;
/// let b = Vec3::ONE;
/// let mid = engine_math::lerp(a, b, 0.5);
/// assert_eq!(mid, Vec3::new(0.5, 0.5, 0.5));
/// `
pub fn lerp(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a.lerp(b, t)
}
`

- [ ] **Step 3: Add edge-case tests**

Create or update crates/engine-math/tests/math_tests.rs:

`ust
use engine_math::{Vec3, Mat4, Quat, MathError};

#[test]
fn test_normalize_zero_vector() {
    let v = Vec3::ZERO;
    assert!(v.normalize().is_err());
}

#[test]
fn test_normalize_unit_vector() {
    let v = Vec3::X;
    let n = v.normalize().unwrap();
    assert_eq!(n, Vec3::X);
}

#[test]
fn test_matrix_inverse_identity() {
    let m = Mat4::IDENTITY;
    let inv = m.inverse();
    assert_eq!(inv, Mat4::IDENTITY);
}

#[test]
fn test_quat_slerp_same_rotation() {
    let q = Quat::IDENTITY;
    let result = q.slerp(Quat::IDENTITY, 0.5);
    assert_eq!(result, Quat::IDENTITY);
}

#[test]
fn test_vec3_cross_product_parallel() {
    let a = Vec3::X;
    let b = Vec3::X;
    let result = a.cross(b);
    assert_eq!(result, Vec3::ZERO);
}
`

- [ ] **Step 4: Run tests**

Run: cargo test -p engine-math
Expected: All tests PASS

- [ ] **Step 5: Add benchmarks**

Create crates/engine-math/benches/math_benchmarks.rs:

`ust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use engine_math::{Vec3, Mat4, Quat};

fn bench_vec3_operations(c: &mut Criterion) {
    let a = black_box(Vec3::new(1.0, 2.0, 3.0));
    let b = black_box(Vec3::new(4.0, 5.0, 6.0));

    c.bench_function("vec3_add", |bencher| {
        bencher.iter(|| a + b)
    });

    c.bench_function("vec3_dot", |bencher| {
        bencher.iter(|| a.dot(b))
    });

    c.bench_function("vec3_cross", |bencher| {
        bencher.iter(|| a.cross(b))
    });

    c.bench_function("vec3_normalize", |bencher| {
        bencher.iter(|| a.normalize())
    });
}

fn bench_mat4_operations(c: &mut Criterion) {
    let m = black_box(Mat4::from_rotation_y(1.0));
    let v = black_box(Vec3::new(1.0, 2.0, 3.0));

    c.bench_function("mat4_mul_vec3", |bencher| {
        bencher.iter(|| m.transform_point3(v))
    });

    c.bench_function("mat4_inverse", |bencher| {
        bencher.iter(|| m.inverse())
    });

    c.bench_function("mat4_mul_mat4", |bencher| {
        bencher.iter(|| m * m)
    });
}

fn bench_quat_operations(c: &mut Criterion) {
    let q = black_box(Quat::from_rotation_y(1.0));

    c.bench_function("quat_mul_quat", |bencher| {
        bencher.iter(|| q * q)
    });

    c.bench_function("quat_slerp", |bencher| {
        bencher.iter(|| q.slerp(Quat::IDENTITY, 0.5))
    });
}

criterion_group!(benches, bench_vec3_operations, bench_mat4_operations, bench_quat_operations);
criterion_main!(benches);
`

- [ ] **Step 6: Run benchmarks**

Run: cargo bench -p engine-math
Expected: Benchmarks run successfully

- [ ] **Step 7: Run clippy**

Run: cargo clippy -p engine-math
Expected: Zero warnings

- [ ] **Step 8: Commit**

`ash
git add crates/engine-math/
git commit -m "feat(math): add docs, tests, benchmarks for engine-math"
`

---

### Task 2: Polish engine-jobs

**Files:**
- Modify: crates/engine-jobs/src/*.rs
- Create: crates/engine-jobs/tests/jobs_tests.rs

- [ ] **Step 1: Add module-level documentation**

Add to crates/engine-jobs/src/lib.rs:

`ust
//! # engine-jobs
//!
//! Task scheduling and parallel execution for the RustEngine.
//!
//! Provides a thread pool and task scheduler for parallelizing
//! work across multiple threads. Used by engine-ecs for
//! parallel system execution.
//!
//! ## Quick Start
//!
//! `ust
//! use engine_jobs::TaskPool;
//!
//! let pool = TaskPool::new(4);
//! let results: Vec<i32> = pool.scope(|scope| {
//!     scope.spawn(|| 1);
//!     scope.spawn(|| 2);
//!     scope.spawn(|| 3);
//! });
//! `
`

- [ ] **Step 2: Add documentation to all public functions**

Document all public types and functions with /// docs.

- [ ] **Step 3: Add concurrency tests**

Create crates/engine-jobs/tests/jobs_tests.rs:

`ust
use engine_jobs::TaskPool;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

#[test]
fn test_task_pool_basic() {
    let pool = TaskPool::new(4);
    let result = pool.scope(|scope| {
        scope.spawn(|| 42);
    });
    assert_eq!(result, vec![42]);
}

#[test]
fn test_task_pool_parallel_execution() {
    let pool = TaskPool::new(4);
    let counter = Arc::new(AtomicI32::new(0));

    pool.scope(|scope| {
        for _ in 0..100 {
            let counter = counter.clone();
            scope.spawn(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            });
        }
    });

    assert_eq!(counter.load(Ordering::SeqCst), 100);
}

#[test]
fn test_task_pool_single_thread() {
    let pool = TaskPool::new(1);
    let result = pool.scope(|scope| {
        scope.spawn(|| 1);
        scope.spawn(|| 2);
    });
    assert_eq!(result.len(), 2);
}
`

- [ ] **Step 4: Run tests**

Run: cargo test -p engine-jobs
Expected: All tests PASS

- [ ] **Step 5: Run clippy**

Run: cargo clippy -p engine-jobs
Expected: Zero warnings

- [ ] **Step 6: Commit**

`ash
git add crates/engine-jobs/
git commit -m "feat(jobs): add docs and concurrency tests for engine-jobs"
`

---

### Task 3: Polish engine-window

**Files:**
- Modify: crates/engine-window/src/*.rs

- [ ] **Step 1: Add module-level documentation**

Add to crates/engine-window/src/lib.rs:

`ust
//! # engine-window
//!
//! Window management for the RustEngine.
//!
//! Wraps winit to provide cross-platform window creation
//! and event handling. Supports Windows, macOS, and Linux
//! (Wayland/X11).
//!
//! ## Quick Start
//!
//! `ust
//! use engine_window::WindowConfig;
//!
//! let config = WindowConfig::new()
//!     .with_title("My Game")
//!     .with_size(1280, 720);
//! `
`

- [ ] **Step 2: Add documentation to all public functions**

Document all public types and functions with /// docs.

- [ ] **Step 3: Add platform compatibility tests**

Create crates/engine-window/tests/window_tests.rs:

`ust
use engine_window::WindowConfig;

#[test]
fn test_window_config_default() {
    let config = WindowConfig::default();
    assert_eq!(config.width, 1280);
    assert_eq!(config.height, 720);
}

#[test]
fn test_window_config_builder() {
    let config = WindowConfig::new()
        .with_title("Test")
        .with_size(800, 600);

    assert_eq!(config.title, "Test");
    assert_eq!(config.width, 800);
    assert_eq!(config.height, 600);
}

#[test]
fn test_window_config_invalid_size() {
    let config = WindowConfig::new().with_size(0, 0);
    // Should handle gracefully, not panic
    assert!(config.width > 0 || config.height > 0);
}
`

- [ ] **Step 4: Run tests**

Run: cargo test -p engine-window
Expected: All tests PASS

- [ ] **Step 5: Run clippy**

Run: cargo clippy -p engine-window
Expected: Zero warnings

- [ ] **Step 6: Commit**

`ash
git add crates/engine-window/
git commit -m "feat(window): add docs and platform tests for engine-window"
`

---

### Task 4: Polish engine-audio

**Files:**
- Modify: crates/engine-audio/src/*.rs

- [ ] **Step 1: Add module-level documentation**

Add to crates/engine-audio/src/lib.rs:

`ust
//! # engine-audio
//!
//! Audio system for the RustEngine.
//!
//! Provides playback, volume control, 3D spatial audio, mixing,
//! and streaming capabilities. Built on odio.
//!
//! ## Quick Start
//!
//! `ust
//! use engine_audio::AudioManager;
//!
//! let mut audio = AudioManager::new()?;
//! audio.play_sound("click.ogg")?;
//! audio.set_master_volume(0.8);
//! `
`

- [ ] **Step 2: Add documentation to all public functions**

Document all public types and functions with /// docs.

- [ ] **Step 3: Add playback tests**

Create crates/engine-audio/tests/audio_tests.rs:

`ust
use engine_audio::AudioManager;

#[test]
fn test_audio_manager_creation() {
    let audio = AudioManager::new();
    assert!(audio.is_ok());
}

#[test]
fn test_volume_control() {
    let mut audio = AudioManager::new().unwrap();
    audio.set_master_volume(0.5);
    assert_eq!(audio.master_volume(), 0.5);
}

#[test]
fn test_bus_volume() {
    let mut audio = AudioManager::new().unwrap();
    audio.set_bus_volume("sfx", 0.7);
    assert_eq!(audio.bus_volume("sfx"), 0.7);
}

#[test]
fn test_invalid_bus() {
    let audio = AudioManager::new().unwrap();
    assert!(audio.bus_volume("nonexistent").is_none());
}
`

- [ ] **Step 4: Run tests**

Run: cargo test -p engine-audio
Expected: All tests PASS

- [ ] **Step 5: Run clippy**

Run: cargo clippy -p engine-audio
Expected: Zero warnings

- [ ] **Step 6: Commit**

`ash
git add crates/engine-audio/
git commit -m "feat(audio): add docs and playback tests for engine-audio"
`

---

### Task 5: Polish engine-asset

**Files:**
- Modify: crates/engine-asset/src/*.rs

- [ ] **Step 1: Add module-level documentation**

Add to crates/engine-asset/src/lib.rs:

`ust
//! # engine-asset
//!
//! Asset loading and management for the RustEngine.
//!
//! Provides handle-based asset management with reference counting,
//! type registration, file system scanning, and loaders for
//! images, glTF models, and audio files.
//!
//! ## Quick Start
//!
//! `ust
//! use engine_asset::AssetManager;
//!
//! let mut assets = AssetManager::new();
//! let texture: Handle<Texture> = assets.load("textures/player.png")?;
//! `
`

- [ ] **Step 2: Add documentation to all public functions**

Document all public types and functions with /// docs.

- [ ] **Step 3: Add loading pipeline tests**

Create crates/engine-asset/tests/asset_tests.rs:

`ust
use engine_asset::{AssetManager, Handle};

#[test]
fn test_asset_manager_creation() {
    let manager = AssetManager::new();
    assert!(manager.is_ok());
}

#[test]
fn test_handle_generation() {
    let manager = AssetManager::new().unwrap();
    let handle: Handle<String> = manager.create_handle();
    assert!(handle.is_valid());
}

#[test]
fn test_asset_loading_nonexistent() {
    let mut manager = AssetManager::new().unwrap();
    let result = manager.load::<String>("nonexistent.txt");
    assert!(result.is_err());
}
`

- [ ] **Step 4: Run tests**

Run: cargo test -p engine-asset
Expected: All tests PASS (including previously failing tests)

- [ ] **Step 5: Run clippy**

Run: cargo clippy -p engine-asset
Expected: Zero warnings

- [ ] **Step 6: Commit**

`ash
git add crates/engine-asset/
git commit -m "feat(asset): add docs and loading tests for engine-asset"
`

---

### Task 6: Polish engine-ecs

**Files:**
- Modify: crates/engine-ecs/src/*.rs
- Create: crates/engine-ecs/benches/ecs_benchmarks.rs

- [ ] **Step 1: Add module-level documentation**

Add to crates/engine-ecs/src/lib.rs:

`ust
//! # engine-ecs
//!
//! Entity Component System for the RustEngine.
//!
//! A high-performance, sparse-set ECS implementation featuring:
//! - Generational entity IDs
//! - Type-erased component storage
//! - Efficient query iteration
//! - Parallel system execution (optional, via engine-jobs)
//!
//! ## Quick Start
//!
//! `ust
//! use engine_ecs::World;
//!
//! struct Position { x: f32, y: f32 }
//! struct Velocity { dx: f32, dy: f32 }
//!
//! let mut world = World::new();
//! let entity = world.spawn();
//! world.add_component(entity, Position { x: 0.0, y: 0.0 });
//! world.add_component(entity, Velocity { dx: 1.0, dy: 0.5 });
//! `
`

- [ ] **Step 2: Add documentation to all public functions**

Document all public types and functions with /// docs.

- [ ] **Step 3: Add query performance tests**

Create crates/engine-ecs/tests/ecs_tests.rs:

`ust
use engine_ecs::World;

#[derive(Debug, Clone)]
struct Position { x: f32, y: f32 }

#[derive(Debug, Clone)]
struct Velocity { dx: f32, dy: f32 }

#[test]
fn test_entity_spawn() {
    let mut world = World::new();
    let entity = world.spawn();
    assert!(world.is_alive(entity));
}

#[test]
fn test_component_add_remove() {
    let mut world = World::new();
    let entity = world.spawn();

    world.add_component(entity, Position { x: 0.0, y: 0.0 });
    assert!(world.has_component::<Position>(entity));

    world.remove_component::<Position>(entity);
    assert!(!world.has_component::<Position>(entity));
}

#[test]
fn test_query_iteration() {
    let mut world = World::new();

    for i in 0..1000 {
        let entity = world.spawn();
        world.add_component(entity, Position { x: i as f32, y: 0.0 });
        if i % 2 == 0 {
            world.add_component(entity, Velocity { dx: 1.0, dy: 0.0 });
        }
    }

    let mut count = 0;
    // Query entities with both Position and Velocity
    for entity in world.query::<Position>().iter() {
        if let Some(vel) = world.get_component::<Velocity>(entity) {
            count += 1;
        }
    }
    assert_eq!(count, 500);
}

#[test]
fn test_entity_deletion() {
    let mut world = World::new();
    let entity = world.spawn();
    world.add_component(entity, Position { x: 0.0, y: 0.0 });

    world.despawn(entity);
    assert!(!world.is_alive(entity));
}
`

- [ ] **Step 4: Add benchmarks**

Create crates/engine-ecs/benches/ecs_benchmarks.rs:

`ust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use engine_ecs::World;

#[derive(Debug, Clone)]
struct Position { x: f32, y: f32 }

#[derive(Debug, Clone)]
struct Velocity { dx: f32, dy: f32 }

fn bench_spawn_entities(c: &mut Criterion) {
    c.bench_function("spawn_1000", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..1000 {
                let entity = world.spawn();
                world.add_component(entity, Position { x: 0.0, y: 0.0 });
            }
            world
        })
    });
}

fn bench_query_iteration(c: &mut Criterion) {
    let mut world = World::new();
    for i in 0..10000 {
        let entity = world.spawn();
        world.add_component(entity, Position { x: i as f32, y: 0.0 });
        world.add_component(entity, Velocity { dx: 1.0, dy: 0.0 });
    }

    c.bench_function("query_10000", |bencher| {
        bencher.iter(|| {
            for entity in world.query::<Position>().iter() {
                black_box(world.get_component::<Velocity>(entity));
            }
        })
    });
}

fn bench_component_access(c: &mut Criterion) {
    let mut world = World::new();
    let entity = world.spawn();
    world.add_component(entity, Position { x: 0.0, y: 0.0 });

    c.bench_function("get_component", |bencher| {
        bencher.iter(|| {
            black_box(world.get_component::<Position>(entity));
        })
    });
}

criterion_group!(benches, bench_spawn_entities, bench_query_iteration, bench_component_access);
criterion_main!(benches);
`

- [ ] **Step 5: Run tests**

Run: cargo test -p engine-ecs
Expected: All tests PASS

- [ ] **Step 6: Run benchmarks**

Run: cargo bench -p engine-ecs
Expected: Benchmarks run successfully

- [ ] **Step 7: Run clippy**

Run: cargo clippy -p engine-ecs
Expected: Zero warnings

- [ ] **Step 8: Commit**

`ash
git add crates/engine-ecs/
git commit -m "feat(ecs): add docs, tests, benchmarks for engine-ecs"
`

---

### Task 7: Final verification for Layer 0-1

**Files:**
- None (verification only)

- [ ] **Step 1: Run full test suite**

Run: cargo test -p engine-math -p engine-jobs -p engine-window -p engine-audio -p engine-asset -p engine-ecs
Expected: All tests PASS

- [ ] **Step 2: Run clippy for all Layer 0-1 crates**

Run: cargo clippy -p engine-math -p engine-jobs -p engine-window -p engine-audio -p engine-asset -p engine-ecs
Expected: Zero warnings

- [ ] **Step 3: Verify documentation**

Check that all public items have /// documentation by running:
Run: cargo doc -p engine-math -p engine-jobs -p engine-window -p engine-audio -p engine-asset -p engine-ecs --no-deps
Expected: No warnings about missing docs
