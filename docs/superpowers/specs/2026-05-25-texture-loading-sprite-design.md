# Texture Loading → Sprite Integration

Date: 2026-05-25
Author: ConspiratorR
Status: Draft

## Summary

Bridge the gap between `engine-asset` image loading and `engine-render` sprite rendering. A new `TextureStore` ECS resource manages GPU textures with bind groups, providing a one-call preload API that takes a file path to a GPU-ready texture ID usable by `Sprite` components.

## Motivation

The engine currently has working pieces disconnected: `engine-asset::format::image` decodes images to `ImageData`, `engine-render::resource::Texture::from_bytes` uploads to GPU, and `SpritePipeline` defines a bind group layout with texture+sampler slots. But no code path connects these — `Sprite` uses a raw `u64` that nothing resolves. This design closes that gap to produce visible textured sprites.

## Architecture

```
User Code
│
├── texture_store.load("assets/player.png")  → u64 (texture ID)
├── Sprite { texture_id, color, size, ... }   (ECS component)
│
├── TextureStore (ECS Resource)
│   ├── textures: HashMap<u64, GpuTexture>
│   ├── bind_groups: HashMap<u64, BindGroup>   // one bind group per texture
│   ├── fallback_id: u64                       // 0 — magenta checkerboard
│   └── next_id: u64                           // starts at 1
│
├── SpriteCollectSystem
│   └── queries Sprite + GlobalTransform → Vec<SpriteDraw>
│
└── Sprite Render Pass
    └── texture_id → TextureStore.get_bind_group(id) → draw
```

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Texture cache location | Standalone `TextureStore` as ECS Resource | Decoupled from Renderer, sprite system accesses via ECS query |
| Loading API | One-call preload (`store.load(path)`) | Simple, explicit, no hidden GPU stalls |
| Sprite reference | `u64` texture ID | Lightweight, matches SpriteBatch indexing, no asset lifetime management in render path |
| Invalid ID handling | Magenta checkerboard fallback texture | Standard game engine pattern (Unity/Unreal), visually obvious, never crashes |

## API

### TextureStore

```rust
pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub width: u32,
    pub height: u32,
}

pub struct TextureStore {
    textures: HashMap<u64, GpuTexture>,
    bind_groups: HashMap<u64, wgpu::BindGroup>,
    sampler: wgpu::Sampler,
    fallback_texture: GpuTexture,
    fallback_bind_group: wgpu::BindGroup,
    fallback_id: u64,
    next_id: u64,
}

impl TextureStore {
    /// Create store with fallback texture and shared sampler.
    /// `texture_layout` is the bind group layout for (texture + sampler) — @group(1).
    pub fn new(device: &wgpu::Device, texture_layout: &wgpu::BindGroupLayout) -> Self;

    /// One-call load: decode image file → upload GPU → create bind group → return ID.
    pub fn load(
        &mut self, device: &wgpu::Device, queue: &wgpu::Queue,
        texture_layout: &wgpu::BindGroupLayout, path: &str,
    ) -> Result<u64, TextureLoadError>;

    /// Load from raw RGBA8 pixel data.
    pub fn load_from_bytes(
        &mut self, device: &wgpu::Device, queue: &wgpu::Queue,
        texture_layout: &wgpu::BindGroupLayout, pixels: &[u8],
        width: u32, height: u32,
    ) -> Result<u64, TextureLoadError>;

    /// Get bind group for draw calls. Returns fallback for invalid IDs.
    pub fn get_bind_group(&self, id: u64) -> &wgpu::BindGroup;

    /// Get texture dimensions. Returns fallback size for invalid IDs.
    pub fn get_size(&self, id: u64) -> (u32, u32);

    /// Check if texture ID is valid and loaded.
    pub fn contains(&self, id: u64) -> bool;

    /// Unload a texture and free GPU resources.
    pub fn unload(&mut self, id: u64);
}
```

### Sprite Component (updated)

```rust
pub struct Sprite {
    pub texture_id: u64,   // from TextureStore
    pub color: [f32; 4],
    pub size: Vec2,
    pub flip_x: bool,
    pub flip_y: bool,
}
```

### SpriteDraw (updated)

```rust
pub struct SpriteDraw {
    pub world_matrix: Mat4,
    pub color: [f32; 4],
    pub size: Vec2,
    pub texture_id: u64,
    pub flip_x: bool,
    pub flip_y: bool,
}
```

### SpriteBatch (updated)

```rust
pub struct SpriteBatch {
    pub texture_id: u64,
    pub vertices: Vec<SpriteVertex>,
    pub indices: Vec<u16>,
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
    pub index_count: u32,
}
```

### Bind Group Layout

The current `SpritePipeline` puts all 3 bindings (camera, texture, sampler) in `@group(0)`. This must be split:

- **`@group(0)`**: Camera uniform — set once per frame, shared across all batches
- **`@group(1)`**: Texture + Sampler — one bind group per texture, changes per batch

```wgsl
@group(0) @binding(0) var<uniform> camera: CameraUniform;

@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;
```

The `SpritePipeline` struct changes:

```rust
pub struct SpritePipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
}
```

Pipeline layout uses both: `[&camera_layout, &texture_layout]`.

`TextureStore::load_from_bytes` creates bind groups against `texture_bind_group_layout` (texture + sampler only).

### Renderer Integration

`present` signature gains `TextureStore` reference:

```rust
pub fn present(
    &mut self,
    camera_matrix: &Mat4,
    sprite_batches: &[SpriteBatch],
    texture_store: &TextureStore,
) -> Result<(), wgpu::SurfaceError>
```

The sprite render pass callback:
1. Creates camera bind group (uniform buffer) once per frame
2. Sets `@group(0)` with camera bind group
3. For each batch: sets `@group(1)` via `texture_store.get_bind_group(batch.texture_id)`

## Fallback Texture

- ID `0` is reserved for the fallback
- 2x2 magenta/black checkerboard pattern, RGBA8
- Created in `TextureStore::new()`
- Visible indicator of missing/invalid texture references

## Error Handling

- `TextureLoadError` — wraps IO errors, decode errors, invalid dimensions
- Invalid texture IDs in Sprite → silently use fallback (log warning once per unique invalid ID per frame)
- File not found → return `Err(TextureLoadError::Io)`

## Data Flow

1. **Load time**: `texture_store.load(device, queue, layout, "assets/player.png")`
   - `engine-asset::format::image::load_image(path)` → `DynamicImage`
   - Convert to RGBA8 pixel buffer
   - `wgpu::Device::create_texture` + `queue::write_texture`
   - Create `TextureView` and reuse shared `Sampler`
   - Create `BindGroup` with (uniform camera buffer, texture view, sampler)
   - Store in `HashMap<u64, GpuTexture>`, return ID

2. **Per frame**: `SpriteCollectSystem` queries `&Sprite + &GlobalTransform` → produces `Vec<SpriteDraw>`
   - `collect_batches(&draws)` groups by `texture_id` → `Vec<SpriteBatch>`
   - Each batch uploads vertex/index data to GPU

3. **Render**: `Renderer::present` builds render graph, sprite pass iterates batches
   - `texture_store.get_bind_group(batch.texture_id)` → set bind group
   - Draw indexed quads

## New Files

| File | Responsibility |
|------|---------------|
| `crates/engine-render/src/texture_store.rs` | TextureStore, GpuTexture, TextureLoadError |

## Modified Files

| File | Change |
|------|--------|
| `crates/engine-render/src/pipeline/sprite.rs` | Split bind group layout: `camera_bind_group_layout` + `texture_bind_group_layout` |
| `crates/engine-render/src/pipeline/sprite.wgsl` | Split bindings: `@group(0)` camera, `@group(1)` texture+sampler |
| `crates/engine-render/src/texture_store.rs` | New file — TextureStore, GpuTexture, TextureLoadError |
| `crates/engine-render/src/sprite.rs` | `texture_handle: u64` → `texture_id: u64` in Sprite, SpriteDraw |
| `crates/engine-render/src/renderer.rs` | `present` takes `&TextureStore`, creates camera bind group, sprite pass uses texture bind groups |
| `crates/engine-render/src/lib.rs` | Add `pub mod texture_store` |
| `crates/engine-core/examples/sprite_demo.rs` | Add texture loading demo |

## Testing

- **Unit tests**: TextureStore load/contains/unload, fallback behavior for invalid IDs
- **Integration**: Load a texture, verify `get_bind_group` returns valid bind group
- **Example**: `sprite_demo` loads a PNG, creates a Sprite, renders it on screen

## Out of Scope

- Async/streaming texture loading
- Texture atlases (future: sprite sheet support)
- Mipmap generation
- Texture compression (BC7, ASTC)
- Runtime hot-reload of textures
