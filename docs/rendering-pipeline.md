# Rendering Pipeline Architecture

RustEngine uses `wgpu` for cross-platform GPU rendering. This document describes the rendering architecture.

## Overview

The rendering pipeline consists of:

1. **Renderer** — owns the wgpu device, queue, and surface
2. **Render Graph** — declarative pass dependency system with resource management
3. **Pipelines** — PBR and sprite shader pipelines
4. **Camera System** — projection, view matrices, frustum culling
5. **Lighting** — directional, point, and spot lights
6. **Shadow Mapping** — cascaded shadow maps (CSM)
7. **Sprite Rendering** — 2D sprite batching with texture grouping
8. **Deferred Rendering** — G-Buffer based deferred shading pipeline
9. **Post-Processing** — HDR, tone mapping, bloom, SSAO, TAA

## Renderer Initialization

```rust
use engine_render::renderer::Renderer;
use std::sync::Arc;

// Native initialization (blocking)
let renderer = Renderer::new(window)?;

// WASM initialization (async)
let renderer = Renderer::new_async(window).await?;
```

## Deferred Rendering Pipeline

The engine uses a deferred rendering pipeline for 3D scenes:

1. **Shadow Pass** — Renders depth from light's perspective for cascaded shadow maps
2. **G-Buffer Pass** — Renders geometry to multiple render targets:
   - Albedo (RGBA8)
   - Normal (RGBA16Float)
   - Position (RGBA16Float)
   - Material (RGBA8)
   - Depth (Depth32Float)
3. **Lighting Pass** — Applies lighting calculations using G-Buffer data
4. **Post-Processing** — HDR tone mapping, bloom, SSAO
5. **Sprite Pass** — Renders 2D sprites on top of the 3D scene

## Camera System

Cameras define how the scene is viewed:

```rust
use engine_render::camera::{Camera, Projection};

// Perspective camera for 3D
let camera = Camera::perspective(
    std::f32::consts::FRAC_PI_4, // FOV
    0.1,                         // near plane
    1000.0,                      // far plane
);

// Orthographic camera for 2D
let camera = Camera::orthographic(-10.0, 10.0, -10.0, 10.0, 0.0, 100.0);
```

The camera system supports:
- Multiple cameras with priority sorting
- Viewport configuration
- Frustum culling integration

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
    color: Vec3::new(1.0, 1.0, 1.0),
    intensity: 2.0,
    range: 10.0,
};
```

## Sprite Rendering

2D sprites are batched by texture for efficient rendering:

```rust
use engine_render::sprite::{Sprite, SpriteDraw};

let draw = SpriteDraw {
    position: Vec3::new(0.0, 0.0, 0.0),
    size: Vec2::new(64.0, 64.0),
    uv_rect: [0.0, 0.0, 1.0, 1.0],
    color: [1.0, 1.0, 1.0, 1.0],
    texture_id: 0,
    depth: 0.0,
};
```

## Render Graph

The render graph manages GPU resources and pass dependencies:

```rust
use engine_render::graph::{RenderGraph, TextureDesc};

let mut graph = RenderGraph::new();

// Create texture resources
let hdr_buffer = graph.create_texture(
    TextureDesc::new_2d(
        1920, 1080,
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    ).named("hdr_buffer")
);

// Add render passes with resource dependencies
graph.add_render_pass("shadow_pass", shadow_pass);
graph.add_render_pass("geometry_pass", geometry_pass);
graph.add_render_pass("lighting_pass", lighting_pass);

// Compile and execute
graph.compile(device);
graph.execute(&mut encoder, &view, &device);
```

## Particle Systems

GPU-accelerated particle effects:

```rust
use engine_render::particle::{ParticleEmitter, Curve};

let emitter = ParticleEmitter::new(10.0, texture_handle)
    .with_max_particles(1000)
    .with_lifetime(2.0..2.0)
    .with_speed(5.0..10.0)
    .with_size(4.0..8.0)
    .with_color(Curve::constant([1.0, 1.0, 1.0, 1.0]));
```

## Tilemap Rendering

2D tile-based map rendering:

```rust
use engine_render::tilemap::{Tileset, TileLayer, TilesetStore};

let mut store = TilesetStore::new();
store.add(Tileset::new(texture_handle, 256, 256, 32, 32));

let mut layer = TileLayer::new(0, 10, 10, Vec2::new(32.0, 32.0));
layer.set_tile(0, 0, 1);
layer.set_tile(1, 1, 2);
```

## Performance Features

- **Frustum Culling** — Skips rendering objects outside the camera view
- **LOD Selection** — Automatic level-of-detail based on distance
- **GPU Instancing** — Batches objects sharing the same mesh/material
- **Occlusion Culling** — Hardware occlusion queries
- **GPU Profiling** — Built-in performance metrics collection

## Platform Support

| Platform | Backend | Status |
|----------|---------|--------|
| Windows | Vulkan/DX12 | ✅ Full |
| Linux | Vulkan | ✅ Full |
| macOS | Metal | ✅ Full |
| Android | Vulkan | 🔨 Experimental |
| Web/WASM | WebGPU/WebGL2 | 🔨 Experimental |
