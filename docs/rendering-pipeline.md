# Rendering Pipeline Architecture

RustEngine uses `wgpu` for cross-platform GPU rendering. This document describes the rendering architecture.

## Overview

The rendering pipeline consists of:

1. **Renderer** — owns the wgpu device, queue, and surface
2. **Render Graph** — declarative pass dependency system
3. **Pipelines** — PBR and sprite shader pipelines
4. **Camera System** — projection, view matrices, frustum culling
5. **Lighting** — directional, point, and spot lights
6. **Shadow Mapping** — cascaded shadow maps
7. **Sprite Rendering** — 2D sprite batching with indirect draw

## Renderer Initialization

```rust
use engine_render::renderer::Renderer;

// Created during app setup via AppBuilder
let renderer = Renderer::new(window).await;
app.set_renderer(renderer);
```

## Camera System

Cameras define how the scene is viewed:

```rust
use engine_render::camera::{Camera, Projection};

// Perspective camera for 3D
let camera = Camera::perspective(
    std::f32::consts::FRAC_PI_4, // FOV
    16.0 / 9.0,                  // aspect ratio
    0.1,                         // near plane
    1000.0,                      // far plane
);

// Orthographic camera for 2D
let camera = Camera::orthographic(-10.0, 10.0, -10.0, 10.0, 0.0, 100.0);
```

The `CameraStack` manages multiple cameras sorted by priority. Use `sort_cameras_system` to update.

## Lighting

Three light types are supported:

```rust
use engine_render::light::{DirectionalLight, PointLight, SpotLight};

let sun = DirectionalLight {
    direction: Vec3::new(0.0, -1.0, -1.0).normalize(),
    color: Vec3::new(1.0, 0.95, 0.9),
    intensity: 1.0,
};

let point = PointLight {
    position: Vec3::new(0.0, 5.0, 0.0),
    color: Vec3::new(1.0, 1.0, 1.0),
    intensity: 2.0,
    range: 10.0,
};
```

## Sprite Rendering

2D sprites are batched for efficient rendering:

```rust
use engine_render::sprite::{Sprite, SpriteBatch};

let sprite = Sprite {
    position: Vec3::new(0.0, 0.0, 0.0),
    size: Vec2::new(64.0, 64.0),
    uv_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
    color: Color::WHITE,
    texture_id: 0,
};
```

## Render Graph

The render graph defines pass dependencies:

```rust
use engine_render::graph::RenderGraph;

let mut graph = RenderGraph::new();
graph.add_pass("shadow_pass", shadow_pass);
graph.add_pass("main_pass", main_pass);
graph.add_dependency("main_pass", "shadow_pass");
```

## Particle Systems

GPU-accelerated particle effects:

```rust
use engine_render::particle::{ParticleEmitter, ParticleSystem};

let emitter = ParticleEmitter::new()
    .with_max_particles(1000)
    .with_lifetime(2.0)
    .with_speed(5.0)
    .with_color(Curve::constant(Color::WHITE));
```
