# Render Graph + 2D Sprite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a declarative Render Graph system and implement 2D sprite rendering as the first render pass.

**Architecture:** Declarative render graph with resource declaration, topological pass scheduling, and managed transient resource lifetimes. Sprite rendering uses ECS-driven collection, batch optimization by texture, and the updated wgpu sprite pipeline with uniform buffers and texture sampling.

**Tech Stack:** wgpu 23, WGSL, ECS (engine-ecs), bytemuck

---

## File Structure

### New Files
- `crates/engine-render/src/graph/mod.rs` — RenderGraph struct + builder
- `crates/engine-render/src/graph/texture.rs` — TextureDesc, TextureNode
- `crates/engine-render/src/graph/buffer.rs` — BufferDesc, BufferNode
- `crates/engine-render/src/graph/pass.rs` — RenderPassDesc, PassContext
- `crates/engine-render/src/graph/compile.rs` — topological sort, resource allocation
- `crates/engine-render/src/graph/execute.rs` — per-frame execution
- `crates/engine-render/src/sprite.rs` — Sprite component, SpriteBatch, SpriteCollectSystem

### Modified Files
- `crates/engine-render/src/lib.rs` — add graph, sprite modules
- `crates/engine-render/src/renderer.rs` — add RenderGraph field, wire execution
- `crates/engine-render/src/pipeline/sprite.rs` — add bind group layouts, create_uniform_buffer
- `crates/engine-render/src/pipeline/sprite.wgsl` — camera uniform, texture sampling

### New Example
- `crates/engine-core/examples/sprite_demo.rs` — minimal sprite rendering demo

---

### Task 1: Graph module skeleton + resource types

**Files:**
- Create: `crates/engine-render/src/graph/mod.rs`
- Create: `crates/engine-render/src/graph/texture.rs`
- Create: `crates/engine-render/src/graph/buffer.rs`
- Modify: `crates/engine-render/src/lib.rs`

- [ ] **Step 1: Create graph/texture.rs**

```rust
use std::num::NonZeroU32;

#[derive(Debug, Clone)]
pub struct TextureDesc {
    pub label: Option<String>,
    pub size: wgpu::Extent3d,
    pub mip_levels: u32,
    pub sample_count: u32,
    pub dimension: wgpu::TextureDimension,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
    pub transient: bool,
}

impl TextureDesc {
    pub fn new_2d(width: u32, height: u32, format: wgpu::TextureFormat, usage: wgpu::TextureUsages) -> Self {
        Self {
            label: None,
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_levels: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            transient: false,
        }
    }

    pub fn named(mut self, name: &str) -> Self {
        self.label = Some(name.to_string());
        self
    }

    pub fn transient(mut self) -> Self {
        self.transient = true;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferHandle(u32);

pub(crate) struct TextureNode {
    pub desc: TextureDesc,
    pub texture: Option<wgpu::Texture>,
    pub view: Option<wgpu::TextureView>,
    pub import: bool,
}

impl TextureNode {
    pub fn new(desc: TextureDesc) -> Self {
        Self { desc, texture: None, view: None, import: false }
    }

    pub fn imported(texture: wgpu::Texture, view: wgpu::TextureView) -> Self {
        Self {
            desc: TextureDesc::new_2d(texture.width(), texture.height(), texture.format(), texture.usage()),
            texture: Some(texture),
            view: Some(view),
            import: true,
        }
    }
}
```

- [ ] **Step 2: Create graph/buffer.rs**

```rust
#[derive(Debug, Clone)]
pub struct BufferDesc {
    pub label: Option<String>,
    pub size: u64,
    pub usage: wgpu::BufferUsages,
    pub transient: bool,
}

impl BufferDesc {
    pub fn new(size: u64, usage: wgpu::BufferUsages) -> Self {
        Self { label: None, size, usage, transient: false }
    }

    pub fn named(mut self, name: &str) -> Self {
        self.label = Some(name.to_string());
        self
    }

    pub fn transient(mut self) -> Self {
        self.transient = true;
        self
    }
}

pub(crate) struct BufferNode {
    pub desc: BufferDesc,
    pub buffer: Option<wgpu::Buffer>,
    pub import: bool,
}

impl BufferNode {
    pub fn new(desc: BufferDesc) -> Self {
        Self { desc, buffer: None, import: false }
    }

    pub fn imported(buffer: wgpu::Buffer) -> Self {
        Self {
            desc: BufferDesc::new(buffer.size(), buffer.usage()),
            buffer: Some(buffer),
            import: true,
        }
    }
}
```

- [ ] **Step 3: Create graph/mod.rs**

```rust
mod texture;
mod buffer;
mod pass;
mod compile;
mod execute;

pub use texture::{TextureDesc, TextureHandle, BufferHandle as _};
pub use buffer::BufferDesc;

use std::collections::HashMap;

pub struct RenderGraph {
    textures: Vec<Option<texture::TextureNode>>,
    buffers: Vec<Option<buffer::BufferNode>>,
    passes: Vec<pass::RenderPassNode>,
    texture_map: HashMap<String, TextureHandle>,
    buffer_map: HashMap<String, BufferHandle>,
    compiled: bool,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self {
            textures: Vec::new(),
            buffers: Vec::new(),
            passes: Vec::new(),
            texture_map: HashMap::new(),
            buffer_map: HashMap::new(),
            compiled: false,
        }
    }

    pub fn create_texture(&mut self, desc: TextureDesc) -> TextureHandle {
        let id = TextureHandle(self.textures.len() as u32);
        if let Some(ref name) = desc.label {
            self.texture_map.insert(name.clone(), id);
        }
        self.textures.push(Some(texture::TextureNode::new(desc)));
        id
    }

    pub fn create_buffer(&mut self, desc: BufferDesc) -> BufferHandle {
        let id = BufferHandle(self.buffers.len() as u32);
        if let Some(ref name) = desc.label {
            self.buffer_map.insert(name.clone(), id);
        }
        self.buffers.push(Some(buffer::BufferNode::new(desc)));
        id
    }

    pub fn import_texture(&mut self, name: &str, texture: wgpu::Texture, view: wgpu::TextureView) -> TextureHandle {
        let id = TextureHandle(self.textures.len() as u32);
        self.texture_map.insert(name.to_string(), id);
        self.textures.push(Some(texture::TextureNode::imported(texture, view)));
        id
    }

    pub fn add_render_pass(&mut self, desc: pass::RenderPassDesc) {
        self.passes.push(pass::RenderPassNode::new(desc));
    }

    pub fn is_compiled(&self) -> bool {
        self.compiled
    }

    pub fn reset(&mut self) {
        self.textures.retain(|t| t.as_ref().map_or(false, |n| n.import));
        self.buffers.retain(|b| b.as_ref().map_or(false, |n| n.import));
        self.passes.clear();
    }
}

impl Default for RenderGraph {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 4: Create graph/pass.rs**

```rust
use crate::graph::texture::TextureHandle;

pub struct ColorAttachment {
    pub resource: TextureHandle,
    pub load_op: wgpu::LoadOp<wgpu::Color>,
    pub store_op: wgpu::StoreOp,
}

pub struct DepthStencilAttachment {
    pub resource: TextureHandle,
    pub depth_load_op: wgpu::LoadOp<f32>,
    pub depth_store_op: wgpu::StoreOp,
}

pub struct RenderPassDesc {
    pub label: Option<String>,
    pub color_attachments: Vec<ColorAttachment>,
    pub depth_stencil_attachment: Option<DepthStencilAttachment>,
    pub execute: Box<dyn FnOnce(&mut PassContext<'_>) + Send>,
}

pub struct RenderPassNode {
    pub desc: RenderPassDesc,
}

impl RenderPassNode {
    pub fn new(desc: RenderPassDesc) -> Self {
        Self { desc }
    }
}

pub struct PassContext<'a> {
    pub pass: wgpu::RenderPass<'a>,
    pub resources: &'a RenderGraphResources,
}

pub struct RenderGraphResources {
    pub textures: Vec<Option<wgpu::TextureView>>,
    pub buffers: Vec<Option<wgpu::Buffer>>,
}
```

- [ ] **Step 5: Update lib.rs**

```rust
//! Rendering pipeline with wgpu.

pub mod graph;
pub mod pipeline;
pub mod renderer;
pub mod resource;
pub mod sprite;
pub mod view;
```

- [ ] **Step 6: Verify compilation**

Run: `cargo check -p engine-render`
Expected: passes

- [ ] **Step 7: Commit**

```bash
git add crates/engine-render/src/graph/ crates/engine-render/src/lib.rs
git commit -m "feat(render): add RenderGraph module skeleton + resource types"
```

---

### Task 2: Compilation (topological sort + resource allocation)

**Files:**
- Create: `crates/engine-render/src/graph/compile.rs`

- [ ] **Step 1: Create graph/compile.rs**

```rust
use std::collections::{HashMap, HashSet};
use crate::graph::{RenderGraph, TextureHandle, BufferHandle};
use crate::graph::pass::{ColorAttachment, DepthStencilAttachment};

#[derive(Debug)]
pub enum CompileError {
    NoSwapchainAttachment,
    TextureNotFound(TextureHandle),
    BufferNotFound(BufferHandle),
}

pub struct CompiledGraph {
    pub passes: Vec<CompiledPass>,
    pub transient_textures: Vec<wgpu::Texture>,
}

pub struct CompiledPass {
    pub label: Option<String>,
    pub color_attachments: Vec<wgpu::RenderPassColorAttachment>,
    pub depth_stencil_attachment: Option<wgpu::RenderPassDepthStencilAttachment>,
}

impl RenderGraph {
    pub fn compile(&mut self, device: &wgpu::Device) -> Result<CompiledGraph, CompileError> {
        // 1. Build dependency graph (pass -> passes it depends on)
        let mut pass_outputs: Vec<HashSet<TextureHandle>> = Vec::new();
        let mut pass_inputs: Vec<HashSet<TextureHandle>> = Vec::new();
        for pass in &self.passes {
            let mut outputs = HashSet::new();
            let mut inputs = HashSet::new();
            for att in &pass.desc.color_attachments {
                outputs.insert(att.resource);
            }
            if let Some(ref ds) = pass.desc.depth_stencil_attachment {
                outputs.insert(ds.resource);
            }
            pass_outputs.push(outputs);
            // Inputs: for now we don't track read-only resources — wgpu handles barriers.
            // Future: track shader-read textures.
            pass_inputs.push(inputs);
        }

        // 2. Topological sort (simple: current order since no explicit deps yet)
        // Future: sort using pass_inputs/pass_outputs

        // 3. Allocate transient textures
        let mut transient_textures = Vec::new();
        let texture_count = self.textures.len();
        for i in 0..texture_count {
            if let Some(ref mut node) = self.textures[i] {
                if node.import {
                    continue;
                }
                let device_texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: node.desc.label.as_deref(),
                    size: node.desc.size,
                    mip_level_count: node.desc.mip_levels,
                    sample_count: node.desc.sample_count,
                    dimension: node.desc.dimension,
                    format: node.desc.format,
                    usage: node.desc.usage,
                    view_formats: &[],
                });
                let view = device_texture.create_view(&wgpu::TextureViewDescriptor::default());
                if node.desc.transient {
                    transient_textures.push(device_texture);
                } else {
                    node.texture = Some(device_texture);
                }
                node.view = Some(view);
            }
        }

        // 4. Compile passes
        let mut compiled_passes = Vec::new();
        for pass in &self.passes {
            let color_attachments: Vec<wgpu::RenderPassColorAttachment> = pass.desc.color_attachments.iter().map(|ca| {
                let view = self.resolve_texture_view(ca.resource)
                    .expect("Color attachment texture not allocated");
                wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: ca.load_op.clone(),
                        store: ca.store_op,
                    },
                }
            }).collect();

            let depth_stencil = pass.desc.depth_stencil_attachment.as_ref().map(|ds| {
                let view = self.resolve_texture_view(ds.resource)
                    .expect("Depth attachment texture not allocated");
                wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: ds.depth_load_op.clone(),
                        store: ds.depth_store_op,
                    }),
                    stencil_ops: None,
                }
            });

            compiled_passes.push(CompiledPass {
                label: pass.desc.label.clone(),
                color_attachments,
                depth_stencil_attachment: depth_stencil,
            });
        }

        self.compiled = true;
        Ok(CompiledGraph {
            passes: compiled_passes,
            transient_textures,
        })
    }

    fn resolve_texture_view(&self, handle: TextureHandle) -> Option<&wgpu::TextureView> {
        self.textures.get(handle.0 as usize)?.as_ref()?.view.as_ref()
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p engine-render`
Expected: passes

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/src/graph/compile.rs
git commit -m "feat(render): add RenderGraph compile with transient resource allocation"
```

---

### Task 3: Execution

**Files:**
- Create: `crates/engine-render/src/graph/execute.rs`
- Modify: `crates/engine-render/src/graph/mod.rs`

- [ ] **Step 1: Create graph/execute.rs**

```rust
use crate::graph::compile::CompiledGraph;
use crate::graph::pass::PassContext;
use crate::graph::RenderGraph;

pub struct ExecuteContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub encoder: &'a mut wgpu::CommandEncoder,
}

impl RenderGraph {
    pub fn execute(
        &mut self,
        compiled: &CompiledGraph,
        ctx: &mut ExecuteContext<'_>,
    ) -> Result<(), wgpu::SurfaceError> {
        for (pass_idx, compiled_pass) in compiled.passes.iter().enumerate() {
            let pass_node = &self.passes[pass_idx];
            let mut rpass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: compiled_pass.label.as_deref(),
                color_attachments: &compiled_pass.color_attachments,
                depth_stencil_attachment: compiled_pass.depth_stencil_attachment.as_ref(),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            let resources = &crate::graph::pass::RenderGraphResources {
                textures: self.textures.iter().map(|t| {
                    t.as_ref().and_then(|n| n.view.clone())
                }).collect(),
                buffers: self.buffers.iter().map(|b| {
                    b.as_ref().and_then(|n| n.buffer.clone())
                }).collect(),
            };

            let mut pass_ctx = PassContext {
                pass: rpass,
                resources,
            };

            (pass_node.desc.execute)(&mut pass_ctx);
        }

        Ok(())
    }
}
```

- [ ] **Step 2: Update graph/mod.rs — add `mod execute;`**

Add `mod execute;` to the module declarations in `mod.rs`.

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-render`
Expected: passes

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/graph/
git commit -m "feat(render): add RenderGraph execution loop"
```

---

### Task 4: Update Sprite Pipeline with uniform + texture

**Files:**
- Modify: `crates/engine-render/src/pipeline/sprite.rs`
- Modify: `crates/engine-render/src/pipeline/sprite.wgsl`

- [ ] **Step 1: Update sprite.wgsl**

```wgsl
struct CameraUniform {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var sprite_texture: texture_2d<f32>;
@group(0) @binding(2) var sprite_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

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

- [ ] **Step 2: Update sprite.rs**

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
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl SpritePipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("sprite.wgsl"))),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sprite_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
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

        Self { pipeline, bind_group_layout }
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-render`
Expected: passes

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/pipeline/sprite.rs crates/engine-render/src/pipeline/sprite.wgsl
git commit -m "feat(render): update sprite pipeline with uniform buffer + texture sampling"
```

---

### Task 5: Sprite component + batch system

**Files:**
- Create: `crates/engine-render/src/sprite.rs`

- [ ] **Step 1: Create sprite.rs**

```rust
use engine_math::{Mat4, Vec2};
use crate::pipeline::sprite::SpriteVertex;

/// ECS component for a renderable sprite.
pub struct Sprite {
    pub texture_handle: u64,     // index into runtime texture array
    pub color: [f32; 4],
    pub size: Vec2,
    pub flip_x: bool,
    pub flip_y: bool,
}

/// A single sprite draw instance, collected per frame.
pub struct SpriteDraw {
    pub world_matrix: Mat4,
    pub color: [f32; 4],
    pub size: Vec2,
    pub texture_index: u64,
    pub flip_x: bool,
    pub flip_y: bool,
}

/// Batch of sprites sharing the same texture.
pub struct SpriteBatch {
    pub texture_index: u64,
    pub vertices: Vec<SpriteVertex>,
    pub indices: Vec<u16>,
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
    pub index_count: u32,
}

impl SpriteBatch {
    pub fn new(texture_index: u64) -> Self {
        Self {
            texture_index,
            vertices: Vec::new(),
            indices: Vec::new(),
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
        }
    }

    /// Add a sprite quad to this batch.
    pub fn push(&mut self, draw: &SpriteDraw) {
        let base = self.vertices.len() as u16;
        let w = draw.size.x * 0.5;
        let h = draw.size.y * 0.5;
        let (u0, u1) = if draw.flip_x { (1.0, 0.0) } else { (0.0, 1.0) };
        let (v0, v1) = if draw.flip_y { (1.0, 0.0) } else { (0.0, 1.0) };

        // Quad vertices in local space (transform applied in shader via world matrix)
        self.vertices.extend_from_slice(&[
            SpriteVertex { position: [-w, -h, 0.0], uv: [u0, v1], color: draw.color },
            SpriteVertex { position: [ w, -h, 0.0], uv: [u1, v1], color: draw.color },
            SpriteVertex { position: [ w,  h, 0.0], uv: [u1, v0], color: draw.color },
            SpriteVertex { position: [-w,  h, 0.0], uv: [u0, v0], color: draw.color },
        ]);
        // Two triangles
        self.indices.extend_from_slice(&[
            base, base + 1, base + 2,
            base, base + 2, base + 3,
        ]);
    }

    /// Upload vertex/index data to GPU buffers.
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

/// Collect all sprite draws into batches sorted by texture.
pub fn collect_batches(sprites: &[SpriteDraw]) -> Vec<SpriteBatch> {
    let mut batch_map: std::collections::HashMap<u64, Vec<&SpriteDraw>> = std::collections::HashMap::new();
    for draw in sprites {
        batch_map.entry(draw.texture_index).or_default().push(draw);
    }

    let mut batches: Vec<SpriteBatch> = batch_map.into_iter().map(|(tex_idx, draws)| {
        let mut batch = SpriteBatch::new(tex_idx);
        for draw in draws {
            batch.push(draw);
        }
        batch
    }).collect();

    batches.sort_by_key(|b| b.texture_index);
    batches
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p engine-render`
Expected: passes

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/src/sprite.rs
git commit -m "feat(render): add Sprite component, SpriteBatch, collect_batches"
```

---

### Task 6: Wire RenderGraph into Renderer

**Files:**
- Modify: `crates/engine-render/src/renderer.rs`
- Modify: `crates/engine-render/src/pipeline/mod.rs`

- [ ] **Step 1: Check pipeline/mod.rs exists**

Read and confirm it has `pub mod sprite;`.

- [ ] **Step 2: Update renderer.rs**

```rust
use std::ops::Deref;
use std::sync::Arc;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use crate::graph::{RenderGraph, TextureDesc, BufferDesc};
use crate::graph::compile::CompiledGraph;
use crate::graph::execute::ExecuteContext;
use crate::pipeline::sprite::SpritePipeline;
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
    pub sprite_pipeline: std::sync::Arc<SpritePipeline>,
    camera_uniform: wgpu::Buffer,
    compiled: Option<CompiledGraph>,
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
            size: 64,  // 4x4 f32 matrix
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            device: GpuDevice(Arc::new(device)),
            queue: GpuQueue(Arc::new(queue)),
            surface,
            config,
            graph: RenderGraph::new(),
            sprite_pipeline,
            camera_uniform,
            compiled: None,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn present(&mut self, camera_matrix: &Mat4, sprite_data: &[crate::sprite::SpriteDraw]) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Build render graph for this frame
        self.graph.reset();
        let swapchain = self.graph.import_texture("swapchain", output.texture, view);

        let sprite_pipeline = self.sprite_pipeline.clone();
        self.graph.add_render_pass(crate::graph::pass::RenderPassDesc {
            label: Some("sprite_pass".to_string()),
            color_attachments: vec![crate::graph::pass::ColorAttachment {
                resource: swapchain,
                load_op: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store_op: wgpu::StoreOp::Store,
            }],
            depth_stencil_attachment: None,
            execute: Box::new(move |ctx| {
                ctx.pass.set_pipeline(&sprite_pipeline.pipeline);
                // Draw a test triangle (6 vertices for full-screen quad)
                // In production, iterate sprite batches here
                ctx.pass.draw(0..6, 0..1);
            }),
        });

        let compiled = self.graph.compile(&self.device).unwrap();
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("main_encoder"),
        });

        // Upload camera uniform
        self.queue.write_buffer(&self.camera_uniform, 0, bytemuck::bytes_of(camera_matrix));

        let mut exec_ctx = ExecuteContext {
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

Note: The `present` method above is a simplified first pass. It will be refined when we integrate sprite batches.

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-render`
Expected: passes

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/renderer.rs
git commit -m "feat(render): wire RenderGraph into Renderer with camera uniform"
```

---

### Task 7: Create sprite_demo example

**Files:**
- Create: `crates/engine-core/examples/sprite_demo.rs`

- [ ] **Step 1: Create sprite_demo.rs**

```rust
use engine_core::app::AppBuilder;
use engine_core::engine;
use engine_math::Mat4;
use log::info;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("Starting Sprite Demo");

    let app_builder = AppBuilder::new();

    engine::run_default(app_builder, |renderer, _app, _elapsed| {
        // Simple orthographic camera
        let width = renderer.config.width as f32;
        let height = renderer.config.height as f32;
        let proj = Mat4::orthographic_rh(0.0, width, height, 0.0, -1.0, 1.0);
        let view = Mat4::IDENTITY;
        let camera_matrix = proj * view;

        // Empty sprite data for now
        let sprites = Vec::new();

        renderer.present(&camera_matrix, &sprites)?;
        Ok(())
    })?;

    Ok(())
}
```

- [ ] **Step 2: Check engine::run_default signature**

If `run_default` doesn't accept a frame callback, create a minimal winit loop directly similar to `crates/engine-editor/src/main.rs`.

- [ ] **Step 3: Verify compilation**

Run: `cargo check --example sprite_demo -p engine-core`
Expected: passes (or fix compile errors)

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/examples/sprite_demo.rs
git commit -m "feat: add sprite_demo example validating render graph"
```

---

### Task 8: Tests

**Files:**
- Modify: `crates/engine-render/src/graph/texture.rs`

- [ ] **Step 1: Add unit tests for graph types**

Add at end of `texture.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_desc_2d() {
        let desc = TextureDesc::new_2d(640, 480, wgpu::TextureFormat::Bgra8UnormSrgb, wgpu::TextureUsages::RENDER_ATTACHMENT);
        assert_eq!(desc.size.width, 640);
        assert_eq!(desc.size.height, 480);
        assert_eq!(desc.format, wgpu::TextureFormat::Bgra8UnormSrgb);
    }

    #[test]
    fn test_texture_handle_distinct() {
        let h1 = TextureHandle(0);
        let h2 = TextureHandle(1);
        assert_ne!(h1, h2);
    }
}
```

- [ ] **Step 2: Test SpriteBatch creation**

Add at end of `sprite.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use engine_math::{Mat4, Vec2};

    #[test]
    fn test_sprite_batch_push() {
        let mut batch = SpriteBatch::new(0);
        let draw = SpriteDraw {
            world_matrix: Mat4::IDENTITY,
            color: [1.0, 1.0, 1.0, 1.0],
            size: Vec2::new(100.0, 100.0),
            texture_index: 0,
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
            SpriteDraw { texture_index: 1, ..sprite_draw_default() },
            SpriteDraw { texture_index: 0, ..sprite_draw_default() },
            SpriteDraw { texture_index: 1, ..sprite_draw_default() },
        ];
        let batches = collect_batches(&draws);
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].texture_index, 0);
        assert_eq!(batches[1].texture_index, 1);
    }

    fn sprite_draw_default() -> SpriteDraw {
        SpriteDraw {
            world_matrix: Mat4::IDENTITY,
            color: [1.0, 1.0, 1.0, 1.0],
            size: Vec2::new(100.0, 100.0),
            texture_index: 0,
            flip_x: false,
            flip_y: false,
        }
    }
}
```

- [ ] **Step 3: Run all render tests**

Run: `cargo test -p engine-render`
Expected: all pass

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/graph/texture.rs crates/engine-render/src/sprite.rs
git commit -m "test: add graph and sprite unit tests"
```
