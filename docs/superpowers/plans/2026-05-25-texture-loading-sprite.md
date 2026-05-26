# Texture Loading → Sprite Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bridge engine-asset image loading to engine-render sprite rendering via a TextureStore that manages GPU textures with pre-created bind groups.

**Architecture:** TextureStore as a standalone resource holds GPU textures indexed by u64 ID, with pre-created bind groups per texture. SpritePipeline is split into two bind group layouts (camera @group(0), texture+sampler @group(1)). A fallback magenta checkerboard texture is used for invalid IDs.

**Tech Stack:** wgpu 23, WGSL, image 0.25, bytemuck

---

## File Structure

### New Files
- `crates/engine-render/src/texture_store.rs` — TextureStore, GpuTexture, TextureLoadError

### Modified Files
- `crates/engine-render/src/pipeline/sprite.wgsl` — Split bindings: @group(0) camera, @group(1) texture+sampler
- `crates/engine-render/src/pipeline/sprite.rs` — Split bind group layout into two
- `crates/engine-render/src/sprite.rs` — `texture_handle`/`texture_index` → `texture_id`
- `crates/engine-render/src/renderer.rs` — `present` takes &TextureStore, creates camera bind group, iterates batches
- `crates/engine-render/src/lib.rs` — Add `pub mod texture_store`

### Modified Example
- `crates/engine-core/examples/sprite_demo.rs` — Add texture loading demo

---

### Task 1: Split sprite.wgsl bind groups

**Files:**
- Modify: `crates/engine-render/src/pipeline/sprite.wgsl`

- [ ] **Step 1: Rewrite sprite.wgsl with split bind groups**

Replace the entire file with:

```wgsl
struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = camera.view_proj * vec4(input.position, 1.0);
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

- [ ] **Step 2: Commit**

```bash
git add crates/engine-render/src/pipeline/sprite.wgsl
git commit -m "feat(render): split sprite shader bind groups (camera @group(0), texture @group(1))"
```

---

### Task 2: Split SpritePipeline bind group layouts

**Files:**
- Modify: `crates/engine-render/src/pipeline/sprite.rs`

- [ ] **Step 1: Update SpritePipeline struct and new()**

Replace the entire file with:

```rust
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SpriteVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl SpriteVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 12,
                    shader_location: 1,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 20,
                    shader_location: 2,
                },
            ],
        }
    }
}

pub struct SpritePipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl SpritePipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "sprite.wgsl"
            ))),
        });

        // @group(0): camera uniform
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sprite_camera_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // @group(1): texture + sampler
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sprite_texture_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite_pipeline_layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[SpriteVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            camera_bind_group_layout,
            texture_bind_group_layout,
        }
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p engine-render`
Expected: passes (renderer.rs may warn about unused fields, that's OK — fixed in later task)

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/src/pipeline/sprite.rs
git commit -m "feat(render): split SpritePipeline into camera + texture bind group layouts"
```

---

### Task 3: Create TextureStore

**Files:**
- Create: `crates/engine-render/src/texture_store.rs`
- Modify: `crates/engine-render/src/lib.rs`

- [ ] **Step 1: Create texture_store.rs**

```rust
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TextureLoadError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Image decode error: {0}")]
    Decode(String),
    #[error("Invalid dimensions: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },
}

pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

pub struct TextureStore {
    textures: HashMap<u64, GpuTexture>,
    bind_groups: HashMap<u64, wgpu::BindGroup>,
    sampler: wgpu::Sampler,
    fallback_id: u64,
    next_id: u64,
}

impl TextureStore {
    pub fn new(device: &wgpu::Device, texture_layout: &wgpu::BindGroupLayout) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("texture_store_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create fallback 2x2 magenta/black checkerboard
        let fallback_pixels: [u8; 16] = [
            255, 0, 255, 255, 0, 0, 0, 255,
            0, 0, 0, 255, 255, 0, 255, 255,
        ];
        let fallback_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fallback_texture"),
            size: wgpu::Extent3d {
                width: 2,
                height: 2,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let fallback_view = fallback_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let fallback_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fallback_bind_group"),
            layout: texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&fallback_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let mut textures = HashMap::new();
        let mut bind_groups = HashMap::new();
        textures.insert(
            0,
            GpuTexture {
                texture: fallback_texture,
                view: fallback_view,
                width: 2,
                height: 2,
            },
        );
        bind_groups.insert(0, fallback_bind_group);

        Self {
            textures,
            bind_groups,
            sampler,
            fallback_id: 0,
            next_id: 1,
        }
    }

    pub fn load(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_layout: &wgpu::BindGroupLayout,
        path: &str,
    ) -> Result<u64, TextureLoadError> {
        let bytes = std::fs::read(path)?;
        let img = image::load_from_memory(&bytes)
            .map_err(|e| TextureLoadError::Decode(e.to_string()))?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        self.load_from_bytes(device, queue, texture_layout, &rgba, width, height)
    }

    pub fn load_from_bytes(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_layout: &wgpu::BindGroupLayout,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> Result<u64, TextureLoadError> {
        if width == 0 || height == 0 {
            return Err(TextureLoadError::InvalidDimensions { width, height });
        }

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let id = self.next_id;
        self.next_id += 1;
        self.textures.insert(id, GpuTexture { texture, view, width, height });
        self.bind_groups.insert(id, bind_group);
        Ok(id)
    }

    pub fn get_bind_group(&self, id: u64) -> &wgpu::BindGroup {
        self.bind_groups
            .get(&id)
            .unwrap_or_else(|| &self.bind_groups[&self.fallback_id])
    }

    pub fn get_size(&self, id: u64) -> (u32, u32) {
        self.textures
            .get(&id)
            .map(|t| (t.width, t.height))
            .unwrap_or((2, 2))
    }

    pub fn contains(&self, id: u64) -> bool {
        self.textures.contains_key(&id)
    }

    pub fn unload(&mut self, id: u64) {
        if id == self.fallback_id {
            return;
        }
        self.textures.remove(&id);
        self.bind_groups.remove(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_device() -> (wgpu::Device, wgpu::Queue) {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .unwrap();
        pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .unwrap()
    }

    fn test_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("test_texture_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }

    #[test]
    fn test_fallback_exists() {
        let (device, _) = test_device();
        let layout = test_layout(&device);
        let store = TextureStore::new(&device, &layout);
        assert!(store.contains(0));
        assert_eq!(store.get_size(0), (2, 2));
    }

    #[test]
    fn test_invalid_id_returns_fallback() {
        let (device, _) = test_device();
        let layout = test_layout(&device);
        let store = TextureStore::new(&device, &layout);
        let bg = store.get_bind_group(999);
        let fallback_bg = store.get_bind_group(0);
        // Both return valid bind groups (fallback)
        assert!(std::ptr::eq(bg, fallback_bg));
    }

    #[test]
    fn test_load_from_bytes() {
        let (device, queue) = test_device();
        let layout = test_layout(&device);
        let mut store = TextureStore::new(&device, &layout);
        let pixels = vec![255u8, 0, 0, 255]; // 1x1 red
        let id = store.load_from_bytes(&device, &queue, &layout, &pixels, 1, 1).unwrap();
        assert_eq!(id, 1);
        assert!(store.contains(id));
        assert_eq!(store.get_size(id), (1, 1));
    }

    #[test]
    fn test_unload() {
        let (device, queue) = test_device();
        let layout = test_layout(&device);
        let mut store = TextureStore::new(&device, &layout);
        let pixels = vec![255u8, 0, 0, 255];
        let id = store.load_from_bytes(&device, &queue, &layout, &pixels, 1, 1).unwrap();
        store.unload(id);
        assert!(!store.contains(id));
    }

    #[test]
    fn test_cannot_unload_fallback() {
        let (device, _) = test_device();
        let layout = test_layout(&device);
        let mut store = TextureStore::new(&device, &layout);
        store.unload(0); // should be no-op
        assert!(store.contains(0));
    }
}
```

- [ ] **Step 2: Update lib.rs**

Replace contents with:

```rust
//! Rendering pipeline with wgpu.

pub mod graph;
pub mod pipeline;
pub mod renderer;
pub mod resource;
pub mod sprite;
pub mod texture_store;
pub mod view;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-render`
Expected: passes

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/texture_store.rs crates/engine-render/src/lib.rs
git commit -m "feat(render): add TextureStore with load, fallback, and bind group management"
```

---

### Task 4: Update sprite.rs field names

**Files:**
- Modify: `crates/engine-render/src/sprite.rs`

- [ ] **Step 1: Rename texture fields**

Replace the entire file with:

```rust
use wgpu::util::DeviceExt;
use engine_math::{Mat4, Vec2};
use crate::pipeline::sprite::SpriteVertex;

pub struct Sprite {
    pub texture_id: u64,
    pub color: [f32; 4],
    pub size: Vec2,
    pub flip_x: bool,
    pub flip_y: bool,
}

pub struct SpriteDraw {
    pub world_matrix: Mat4,
    pub color: [f32; 4],
    pub size: Vec2,
    pub texture_id: u64,
    pub flip_x: bool,
    pub flip_y: bool,
}

pub struct SpriteBatch {
    pub texture_id: u64,
    pub vertices: Vec<SpriteVertex>,
    pub indices: Vec<u16>,
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
    pub index_count: u32,
}

impl SpriteBatch {
    pub fn new(texture_id: u64) -> Self {
        Self {
            texture_id,
            vertices: Vec::new(),
            indices: Vec::new(),
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
        }
    }

    pub fn push(&mut self, draw: &SpriteDraw) {
        let base = self.vertices.len() as u16;
        let w = draw.size.x * 0.5;
        let h = draw.size.y * 0.5;
        let (u0, u1) = if draw.flip_x { (1.0, 0.0) } else { (0.0, 1.0) };
        let (v0, v1) = if draw.flip_y { (1.0, 0.0) } else { (0.0, 1.0) };

        self.vertices.extend_from_slice(&[
            SpriteVertex { position: [-w, -h, 0.0], uv: [u0, v1], color: draw.color },
            SpriteVertex { position: [ w, -h, 0.0], uv: [u1, v1], color: draw.color },
            SpriteVertex { position: [ w,  h, 0.0], uv: [u1, v0], color: draw.color },
            SpriteVertex { position: [-w,  h, 0.0], uv: [u0, v0], color: draw.color },
        ]);
        self.indices.extend_from_slice(&[
            base, base + 1, base + 2,
            base, base + 2, base + 3,
        ]);
    }

    pub fn upload(&mut self, device: &wgpu::Device) {
        let vertex_data = bytemuck::cast_slice(&self.vertices);
        self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sprite_vertex_buffer"),
            contents: vertex_data,
            usage: wgpu::BufferUsages::VERTEX,
        }));
        let index_data = bytemuck::cast_slice(&self.indices);
        self.index_count = self.indices.len() as u32;
        self.index_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sprite_index_buffer"),
            contents: index_data,
            usage: wgpu::BufferUsages::INDEX,
        }));
    }
}

pub fn collect_batches(sprites: &[SpriteDraw]) -> Vec<SpriteBatch> {
    let mut batch_map: std::collections::HashMap<u64, Vec<&SpriteDraw>> = std::collections::HashMap::new();
    for draw in sprites {
        batch_map.entry(draw.texture_id).or_default().push(draw);
    }

    let mut batches: Vec<SpriteBatch> = batch_map.into_iter().map(|(tex_id, draws)| {
        let mut batch = SpriteBatch::new(tex_id);
        for draw in draws {
            batch.push(draw);
        }
        batch
    }).collect();

    batches.sort_by_key(|b| b.texture_id);
    batches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprite_batch_push() {
        let mut batch = SpriteBatch::new(0);
        let draw = SpriteDraw {
            world_matrix: Mat4::IDENTITY,
            color: [1.0, 1.0, 1.0, 1.0],
            size: Vec2::new(100.0, 100.0),
            texture_id: 0,
            flip_x: false,
            flip_y: false,
        };
        batch.push(&draw);
        assert_eq!(batch.vertices.len(), 4);
        assert_eq!(batch.indices.len(), 6);
    }

    #[test]
    fn test_collect_batches_groups_by_texture() {
        let draws = vec![
            SpriteDraw { texture_id: 1, ..sprite_draw_default() },
            SpriteDraw { texture_id: 0, ..sprite_draw_default() },
            SpriteDraw { texture_id: 1, ..sprite_draw_default() },
        ];
        let batches = collect_batches(&draws);
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].texture_id, 0);
        assert_eq!(batches[1].texture_id, 1);
    }

    fn sprite_draw_default() -> SpriteDraw {
        SpriteDraw {
            world_matrix: Mat4::IDENTITY,
            color: [1.0, 1.0, 1.0, 1.0],
            size: Vec2::new(100.0, 100.0),
            texture_id: 0,
            flip_x: false,
            flip_y: false,
        }
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p engine-render`
Expected: passes

- [ ] **Step 3: Run existing sprite tests**

Run: `cargo test -p engine-render`
Expected: all pass

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/sprite.rs
git commit -m "refactor(render): rename texture_handle/texture_index to texture_id in sprite types"
```

---

### Task 5: Update renderer.rs to use TextureStore and sprite batches

**Files:**
- Modify: `crates/engine-render/src/renderer.rs`

- [ ] **Step 1: Rewrite renderer.rs**

Replace the entire file with:

```rust
use std::ops::Deref;
use std::sync::Arc;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use crate::graph::{RenderGraph, pass, execute};
use crate::pipeline::sprite::SpritePipeline;
use crate::sprite::SpriteBatch;
use crate::texture_store::TextureStore;
use engine_math::Mat4;

#[derive(Clone)]
pub struct GpuDevice(pub Arc<Device>);

impl Deref for GpuDevice {
    type Target = Device;
    fn deref(&self) -> &Device {
        &self.0
    }
}

#[derive(Clone)]
pub struct GpuQueue(pub Arc<Queue>);

impl Deref for GpuQueue {
    type Target = Queue;
    fn deref(&self) -> &Queue {
        &self.0
    }
}

pub struct Renderer {
    pub device: GpuDevice,
    pub queue: GpuQueue,
    pub surface: Surface<'static>,
    pub config: SurfaceConfiguration,
    pub graph: RenderGraph,
    pub sprite_pipeline: Arc<SpritePipeline>,
    camera_uniform: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
}

impl Renderer {
    pub fn new(window: std::sync::Arc<winit::window::Window>) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .unwrap();
        let size = window.inner_size();
        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();
        surface.configure(&device, &config);

        let sprite_pipeline = Arc::new(SpritePipeline::new(&device, config.format));

        let camera_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera_uniform"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bind_group"),
            layout: &sprite_pipeline.camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_uniform.as_entire_binding(),
            }],
        });

        Self {
            device: GpuDevice(Arc::new(device)),
            queue: GpuQueue(Arc::new(queue)),
            surface,
            config,
            graph: RenderGraph::new(),
            sprite_pipeline,
            camera_uniform,
            camera_bind_group,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn present(
        &mut self,
        camera_matrix: &Mat4,
        sprite_batches: &[SpriteBatch],
        texture_store: &TextureStore,
    ) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Build render graph for this frame
        self.graph.reset();
        let swapchain = self.graph.import_texture_view("swapchain", view);

        let sprite_pipeline = self.sprite_pipeline.clone();
        let camera_bind_group = self.camera_bind_group.clone();
        self.graph.add_render_pass(pass::RenderPassDesc {
            label: Some("sprite_pass".to_string()),
            color_attachments: vec![pass::ColorAttachment {
                resource: swapchain,
                load_op: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store_op: wgpu::StoreOp::Store,
            }],
            depth_stencil_attachment: None,
            execute: Box::new(move |ctx| {
                ctx.pass.set_pipeline(&sprite_pipeline.pipeline);
                ctx.pass.set_bind_group(0, &camera_bind_group, &[]);

                for batch in sprite_batches {
                    let bind_group = texture_store.get_bind_group(batch.texture_id);
                    ctx.pass.set_bind_group(1, bind_group, &[]);

                    if let (Some(vb), Some(ib)) = (&batch.vertex_buffer, &batch.index_buffer) {
                        ctx.pass.set_vertex_buffer(0, vb.slice(..));
                        ctx.pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint16);
                        ctx.pass.draw_indexed(0..batch.index_count, 0, 0..1);
                    }
                }
            }),
        });

        let compiled = self.graph.compile(&self.device).unwrap();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("main_encoder"),
            });

        // Upload camera uniform
        let matrix_data = camera_matrix.to_cols_array();
        self.queue.write_buffer(
            &self.camera_uniform,
            0,
            bytemuck::cast_slice(&matrix_data),
        );

        let mut exec_ctx = execute::ExecuteContext {
            device: &self.device,
            queue: &self.queue,
            encoder: &mut encoder,
        };
        self.graph.execute(&compiled, &mut exec_ctx).unwrap();

        self.queue.submit([encoder.finish()]);
        output.present();
        Ok(())
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p engine-render`
Expected: passes

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/src/renderer.rs
git commit -m "feat(render): wire TextureStore and sprite batches into Renderer.present"
```

---

### Task 6: Update sprite_demo example

**Files:**
- Modify: `crates/engine-core/examples/sprite_demo.rs`

- [ ] **Step 1: Check if sprite_demo.rs exists**

If it doesn't exist, create it. If it exists, replace its contents.

- [ ] **Step 2: Create/update sprite_demo.rs**

```rust
use engine_core::app::AppBuilder;
use engine_render::renderer::Renderer;
use engine_render::sprite::{SpriteBatch, SpriteDraw};
use engine_render::texture_store::TextureStore;
use engine_math::{Mat4, Vec2};
use log::info;
use std::sync::Arc;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("Starting Sprite Demo");

    let event_loop = EventLoop::new().unwrap();
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Sprite Demo")
            .build(&event_loop)
            .unwrap(),
    );

    let mut renderer = Renderer::new(window.clone());
    let mut texture_store = TextureStore::new(
        &renderer.device,
        &renderer.sprite_pipeline.texture_bind_group_layout,
    );

    // Try to load a test texture (will use fallback if not found)
    let texture_id = match texture_store.load(
        &renderer.device,
        &renderer.queue,
        &renderer.sprite_pipeline.texture_bind_group_layout,
        "assets/test.png",
    ) {
        Ok(id) => {
            info!("Loaded texture with id {}", id);
            id
        }
        Err(e) => {
            info!("Could not load texture: {} — using fallback", e);
            0
        }
    };

    // Create a sprite batch with one sprite
    let mut batch = SpriteBatch::new(texture_id);
    let draw = SpriteDraw {
        world_matrix: Mat4::IDENTITY,
        color: [1.0, 1.0, 1.0, 1.0],
        size: Vec2::new(128.0, 128.0),
        texture_id,
        flip_x: false,
        flip_y: false,
    };
    batch.push(&draw);
    batch.upload(&renderer.device);

    let batches = vec![batch];

    event_loop
        .run(move |event, elwt| {
            match event {
                winit::event::Event::WindowEvent { event, .. } => match event {
                    winit::event::WindowEvent::CloseRequested => elwt.exit(),
                    winit::event::WindowEvent::Resized(size) => {
                        renderer.resize(size.width, size.height);
                    }
                    winit::event::WindowEvent::RedrawRequested => {
                        let width = renderer.config.width as f32;
                        let height = renderer.config.height as f32;
                        let proj = Mat4::orthographic_rh(0.0, width, height, 0.0, -1.0, 1.0);
                        let view = Mat4::IDENTITY;
                        let camera_matrix = proj * view;

                        if let Err(e) = renderer.present(&camera_matrix, &batches, &texture_store) {
                            log::error!("Render error: {:?}", e);
                        }
                    }
                    _ => {}
                },
                winit::event::Event::AboutToWait => {
                    window.request_redraw();
                }
                _ => {}
            }
        })
        .unwrap();

    Ok(())
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check --example sprite_demo -p engine-core`
Expected: passes (or fix compile errors if engine-core doesn't depend on engine-render)

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/examples/sprite_demo.rs
git commit -m "feat: update sprite_demo with texture loading and batch rendering"
```

---

### Task 7: Run full test suite and lint

- [ ] **Step 1: Run clippy**

Run: `cargo clippy --all`
Expected: no errors (warnings acceptable)

- [ ] **Step 2: Run fmt check**

Run: `cargo fmt --check`
Expected: no diff

- [ ] **Step 3: Run tests**

Run: `cargo test -p engine-render`
Expected: all pass

- [ ] **Step 4: Final commit if fmt changed anything**

```bash
git add -A
git commit -m "style: apply cargo fmt"
```
