# Render Graph + 2D Sprite Rendering

Date: 2026-05-19
Author: ConspiratorR
Status: Draft

## Summary

Build a full-featured Render Graph system for wgpu-based rendering, then implement 2D sprite rendering on top of it as the first render pass.

## Motivation

The engine currently has wgpu initialization and GPU resource scaffolding but issues no draw calls — the screen clears to black. A Render Graph provides a structured, extensible foundation for all future rendering (3D, post-processing, UI), while the sprite pass delivers the first visible output.

## Architecture

```
ECS World                    RenderGraph
├── Sprite component          ├── compile(device)
├── Transform component       ├── execute(encoder)
│                             │    ├── pass: sprite
SpriteCollectSystem           │    │   ├─ set_pipeline
└── collects Sprites          │    │   ├─ set_bind_group
    → writes SpriteBatch      │    │   └─ draw_indexed
                              │    └── pass: ...
                              └──
```

### Core Abstractions

**TextureNode** — A declared texture resource. Contains size, format, usage flags, and optional transient flag. Compilation resolves to a `wgpu::Texture` or aliases with another node's memory.

**BufferNode** — A declared buffer resource. Size, usage, transient flag.

**RenderPassNode** — A render pass with color/depth attachments and an execution callback. Attachments reference TextureNodes by handle.

**RenderGraph** — Container for all nodes. Supports builder-pattern construction, compilation (topological sort, resource allocation), and per-frame execution.

### Key Design Decisions

1. **Declaration vs allocation separation** — Users declare resources and passes, then call `compile()` which allocates GPU resources and determines execution order. Transient resources are freed after execution.

2. **Dependency derivation** — Pass dependencies are derived from resource read/write relationships: if pass A writes texture T and pass B reads T, B runs after A.

3. **Transient resources** — Marked with `transient: true`. Allocated at compile time, freed after execute. V1 allocates each transient resource independently; memory aliasing between non-overlapping resources is a future optimization.

4. **wgpu barrier handling** — wgpu handles synchronization internally. The graph focuses on resource lifecycle and pass scheduling.

## API

### Resource Declaration

```rust
let rt = graph.create_texture(TextureDesc {
    size: Extent3D::new(1920, 1080, 1),
    format: wgpu::TextureFormat::Bgra8UnormSrgb,
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    transient: true,
});

let depth = graph.create_texture(TextureDesc {
    size: Extent3D::new(1920, 1080, 1),
    format: wgpu::TextureFormat::Depth32Float,
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
    transient: true,
});

let swapchain = graph.import_texture("swapchain", &swapchain_texture);
```

### Pass Declaration

```rust
graph.add_render_pass("sprite", RenderPassDesc {
    color_attachments: vec![ColorAttachment {
        resource: swapchain_handle,
        load_op: LoadOp::Clear(Color::BLACK),
        store_op: StoreOp::Store,
    }],
    depth_stencil: None,
    execute: |ctx: &mut PassContext<'_, '_>| {
        // ctx.pass: &mut wgpu::RenderPass
        // ctx.resources: &RenderGraphResources (resolve handles to actual textures/buffers)
        // ctx.data: &FrameData (sprite batches, camera, etc.)
        ctx.pass.set_pipeline(&sprite_pipeline);
        for batch in &ctx.data.sprite_batches {
            ctx.pass.set_bind_group(0, &batch.bind_group, &[]);
            ctx.pass.set_vertex_buffer(0, batch.vertex_buffer.slice(..));
            ctx.pass.set_index_buffer(batch.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            ctx.pass.draw_indexed(0..batch.index_count, 0, 0..1);
        }
    },
});
```

### Compilation & Execution

```rust
graph.compile(&device);                       // allocate transient resources, sort passes
graph.execute(&mut encoder, &device, &frame_data);     // run all passes
```

## New Modules / Files

### `crates/engine-render/src/graph/`

| File | Responsibility |
|------|---------------|
| `mod.rs` | RenderGraph struct, builder methods |
| `texture.rs` | TextureDesc, TextureNode, ResourceHandle |
| `buffer.rs` | BufferDesc, BufferNode |
| `pass.rs` | RenderPassDesc, RenderPassNode, ColorAttachment, PassContext |
| `compile.rs` | Topological sort, resource allocation, memory aliasing |
| `execute.rs` | Per-frame execution loop |

### `crates/engine-render/src/sprite.rs`

| Item | Responsibility |
|------|---------------|
| `Sprite` component | Texture handle, color, size, flip flags |
| `SpriteBatch` | Per-texture batch of vertex/index data |
| `SpriteCollectSystem` | ECS system: iterates Sprite+Transform, populates batches |
| `SpritePipeline` | Updated existing pipeline with uniform buffer + texture sampling |

### Updated Files

| File | Change |
|------|--------|
| `pipeline/sprite.rs` | Add bind group layouts for uniform + texture sampler |
| `pipeline/sprite.wgsl` | Add CameraUniform, texture sampling, projection matrix |
| `renderer.rs` | Add `RenderGraph` field, call graph.execute in present() |
| `lib.rs` | Export new modules |

## Sprite Pipeline

### Vertex Shader (sprite.wgsl)

```wgsl
struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var sprite_texture: texture_2d<f32>;
@group(0) @binding(2) var sprite_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = camera.view_proj * vec4(input.position, 1.0);
    output.uv = input.uv;
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(sprite_texture, sprite_sampler, input.uv);
    return tex_color * input.color;
}
```

### Sprite Component

```rust
pub struct Sprite {
    pub texture: Handle<Texture>,  // engine-asset Handle
    pub color: Color,
    pub size: Vec2,
    pub flip_x: bool,
    pub flip_y: bool,
}
```

### Per-frame Data Flow

1. `SpriteCollectSystem` runs (ECS), queries `&Sprite + &GlobalTransform`, produces `Vec<SpriteDraw>`
2. `SpriteDraw` contains: texture handle, world matrix, color, size, flip flags
3. Sort by texture handle (batch optimization), convert to `SpriteBatch` list
4. Each `SpriteBatch` has one texture, one vertex/index buffer, one bind group
5. During graph execution, sprite pass iterates batches and issues draw calls

## Camera Integration

Uses existing `engine_render::view::Camera` (perspective/orthographic) as the uniform source. An orthographic camera projection is the default for 2D sprite rendering.

```rust
let camera = Camera::orthographic(0.0, screen_width, screen_height, 0.0, -1.0, 1.0);
// camera.view_proj_matrix() → upload to uniform buffer
```

## Error Handling

- `compile()` returns `Result<(), CompileError>` for resource allocation failures
- `execute()` returns `Result<(), wgpu::SurfaceError>` (surface errors are recoverable)
- Missing resources (e.g., sprite texture not loaded) → skip the batch, log warning

## Testing

- **Unit tests**: TextureDesc validation, resource handle tracking, topological sort
- **Integration**: Run 1 frame of sprite graph, verify no GPU errors
- **Example**: New `sprite_demo` example that draws multiple sprites

## Out of Scope (Future)

- Post-processing passes (bloom, tonemapping)
- Deferred rendering
- Compute passes
- Render graph serialization
- Dynamic resolution scaling
