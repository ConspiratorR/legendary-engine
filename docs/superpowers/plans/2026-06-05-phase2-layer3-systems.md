# Phase 2: Layer 3 Systems Crates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Goal:** Polish the systems crates (engine-render) with tests, docs, benchmarks, and GPU resource lifecycle auditing.

**Architecture:** Focus on engine-render, the most complex crate in Layer 3. Audit GPU resource lifecycle, add deferred rendering tests, and document the render pipeline.

**Tech Stack:** Rust 2024 edition, wgpu, thiserror, anyhow, cargo test, cargo clippy, Criterion

---

### Task 1: Polish engine-render

**Files:**
- Modify: crates/engine-render/src/*.rs
- Create: crates/engine-render/tests/render_tests.rs

- [ ] **Step 1: Add module-level documentation**

Add to crates/engine-render/src/lib.rs:

`ust
//! # engine-render
//!
//! Rendering system for the RustEngine.
//!
//! A wgpu-based rendering pipeline featuring:
//! - Render graph for organizing render passes
//! - Sprite pipeline with batching
//! - 3D PBR rendering with deferred shading
//! - Shadow mapping (CSM)
//! - Environment mapping / IBL
//! - Camera system with ECS integration
//! - 2D/3D particle systems
//! - Tilemap support
//!
//! ## Architecture
//!
//! The rendering pipeline is organized as a render graph:
//!
//! `	ext
//! [Camera] -> [Shadow Pass] -> [G-Buffer Pass] -> [Lighting Pass] -> [Post-Processing] -> [Output]
//! `
//!
//! Each pass reads from and writes to GPU resources (textures, buffers)
//! managed by the resource manager.
//!
//! ## Quick Start
//!
//! `ust
//! use engine_render::RenderPipeline;
//!
//! let pipeline = RenderPipeline::new(&device, &surface_config)?;
//! pipeline.render(&mut encoder, &view, &world)?;
//! `
`

- [ ] **Step 2: Add documentation to all public functions**

Document all public types and functions with /// docs. Focus on:
- RenderGraph API
- SpriteRenderer API
- Camera API
- Material/Mesh/Texture APIs

- [ ] **Step 3: Add deferred rendering tests**

Create crates/engine-render/tests/render_tests.rs:

`ust
use engine_render::{RenderGraph, RenderPass, GBuffer, Camera};
use engine_math::Vec3;

#[test]
fn test_render_graph_creation() {
    let graph = RenderGraph::new();
    assert!(graph.passes().is_empty());
}

#[test]
fn test_render_graph_add_pass() {
    let mut graph = RenderGraph::new();
    let pass = RenderPass::new("gbuffer");
    graph.add_pass(pass);
    assert_eq!(graph.passes().len(), 1);
}

#[test]
fn test_render_graph_dependency() {
    let mut graph = RenderGraph::new();
    graph.add_pass(RenderPass::new("shadow"));
    graph.add_pass(RenderPass::new("gbuffer"));
    graph.add_dependency("gbuffer", "shadow");

    let order = graph.compile().unwrap();
    let shadow_idx = order.iter().position(|p| p.name() == "shadow").unwrap();
    let gbuffer_idx = order.iter().position(|p| p.name() == "gbuffer").unwrap();
    assert!(shadow_idx < gbuffer_idx);
}

#[test]
fn test_gbuffer_creation() {
    let gbuffer = GBuffer::new(1920, 1080);
    assert_eq!(gbuffer.width(), 1920);
    assert_eq!(gbuffer.height(), 1080);
    assert!(gbuffer.albedo_texture().is_some());
    assert!(gbuffer.normal_texture().is_some());
    assert!(gbuffer.depth_texture().is_some());
}

#[test]
fn test_camera_creation() {
    let camera = Camera::perspective(90.0, 16.0 / 9.0, 0.1, 1000.0);
    assert!(camera.is_perspective());

    let camera = Camera::orthographic(-10.0, 10.0, -10.0, 10.0, 0.0, 100.0);
    assert!(camera.is_orthographic());
}

#[test]
fn test_camera_view_projection() {
    let camera = Camera::perspective(90.0, 16.0 / 9.0, 0.1, 1000.0);
    let view = camera.view_matrix();
    let proj = camera.projection_matrix();
    let view_proj = camera.view_projection_matrix();

    // View-projection should be proj * view
    assert_eq!(view_proj, proj * view);
}
`

- [ ] **Step 4: Add GPU resource lifecycle tests**

Add to crates/engine-render/tests/render_tests.rs:

`ust
use engine_render::{TextureHandle, BufferHandle, ResourceManager};

#[test]
fn test_resource_manager_texture() {
    let mut manager = ResourceManager::new();
    let handle = manager.create_texture(256, 256);
    assert!(manager.has_texture(handle));

    manager.remove_texture(handle);
    assert!(!manager.has_texture(handle));
}

#[test]
fn test_resource_manager_buffer() {
    let mut manager = ResourceManager::new();
    let handle = manager.create_buffer(1024);
    assert!(manager.has_buffer(handle));

    manager.remove_buffer(handle);
    assert!(!manager.has_buffer(handle));
}

#[test]
fn test_resource_handle_reuse() {
    let mut manager = ResourceManager::new();
    let handle1 = manager.create_texture(256, 256);
    manager.remove_texture(handle1);
    let handle2 = manager.create_texture(256, 256);

    // Handles should be different (generational)
    assert_ne!(handle1, handle2);
}
`

- [ ] **Step 5: Add sprite batching tests**

Add to crates/engine-render/tests/render_tests.rs:

`ust
use engine_render::{Sprite, SpriteBatch};

#[test]
fn test_sprite_batch_creation() {
    let batch = SpriteBatch::new();
    assert!(batch.is_empty());
    assert_eq!(batch.len(), 0);
}

#[test]
fn test_sprite_batch_add() {
    let mut batch = SpriteBatch::new();
    batch.add(Sprite::new(Vec3::ZERO, Vec2::ONE));
    assert_eq!(batch.len(), 1);
}

#[test]
fn test_sprite_batch_grouping() {
    let mut batch = SpriteBatch::new();
    batch.add(Sprite::with_texture("a.png", Vec3::ZERO, Vec2::ONE));
    batch.add(Sprite::with_texture("b.png", Vec3::ONE, Vec2::ONE));
    batch.add(Sprite::with_texture("a.png", Vec3::new(2.0, 0.0, 0.0), Vec2::ONE));

    let groups = batch.group_by_texture();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups.get("a.png").unwrap().len(), 2);
    assert_eq!(groups.get("b.png").unwrap().len(), 1);
}
`

- [ ] **Step 6: Run tests**

Run: cargo test -p engine-render
Expected: All tests PASS

- [ ] **Step 7: Add benchmarks**

Create crates/engine-render/benches/render_benchmarks.rs:

`ust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use engine_render::{RenderGraph, SpriteBatch, Sprite};
use engine_math::{Vec3, Vec2};

fn bench_render_graph_compile(c: &mut Criterion) {
    let mut graph = RenderGraph::new();
    for i in 0..10 {
        graph.add_pass(RenderPass::new(&format!("pass_{}", i)));
    }
    for i in 1..10 {
        graph.add_dependency(&format!("pass_{}", i), &format!("pass_{}", i - 1));
    }

    c.bench_function("render_graph_compile_10_passes", |bencher| {
        bencher.iter(|| graph.compile())
    });
}

fn bench_sprite_batch_add(c: &mut Criterion) {
    c.bench_function("sprite_batch_add_1000", |bencher| {
        bencher.iter(|| {
            let mut batch = SpriteBatch::new();
            for i in 0..1000 {
                batch.add(Sprite::new(Vec3::new(i as f32, 0.0, 0.0), Vec2::ONE));
            }
            batch
        })
    });
}

fn bench_sprite_batch_group(c: &mut Criterion) {
    let mut batch = SpriteBatch::new();
    for i in 0..1000 {
        let texture = if i % 3 == 0 { "a.png" } else if i % 3 == 1 { "b.png" } else { "c.png" };
        batch.add(Sprite::with_texture(texture, Vec3::new(i as f32, 0.0, 0.0), Vec2::ONE));
    }

    c.bench_function("sprite_batch_group_1000", |bencher| {
        bencher.iter(|| batch.group_by_texture())
    });
}

criterion_group!(benches, bench_render_graph_compile, bench_sprite_batch_add, bench_sprite_batch_group);
criterion_main!(benches);
`

- [ ] **Step 8: Run benchmarks**

Run: cargo bench -p engine-render
Expected: Benchmarks run successfully

- [ ] **Step 9: Run clippy**

Run: cargo clippy -p engine-render
Expected: Zero warnings

- [ ] **Step 10: Commit**

`ash
git add crates/engine-render/
git commit -m "feat(render): add docs, tests, benchmarks for engine-render"
`

---

### Task 2: Final verification for Layer 3

**Files:**
- None (verification only)

- [ ] **Step 1: Run full test suite**

Run: cargo test -p engine-render
Expected: All tests PASS

- [ ] **Step 2: Run clippy**

Run: cargo clippy -p engine-render
Expected: Zero warnings

- [ ] **Step 3: Verify documentation**

Run: cargo doc -p engine-render --no-deps
Expected: No warnings about missing docs
