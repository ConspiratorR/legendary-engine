# Sprite 批量绘制性能优化实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现持久映射缓冲 + 间接绘制，优化大规模精灵渲染性能

**架构:** 使用持久映射缓冲实现零拷贝上传，配合间接绘制减少 draw call 开销，双缓冲避免 CPU/GPU 竞争

**Tech Stack:** wgpu 23, bytemuck 1, Rust 2024 edition

---

## 文件结构

### 新增文件
- `crates/engine-render/src/indirect.rs` — DrawIndexedIndirect 相关类型定义
- `crates/engine-render/src/sprite_renderer.rs` — SpriteRenderer + PersistentBuffer 实现

### 修改文件
- `crates/engine-render/src/sprite.rs` — SpriteBatch 增加 instance_data 字段
- `crates/engine-render/src/pipeline/sprite.rs` — 修改顶点布局，支持实例化
- `crates/engine-render/src/pipeline/sprite.wgsl` — 修改着色器，读取实例矩阵
- `crates/engine-render/src/renderer.rs` — 集成 SpriteRenderer
- `crates/engine-render/src/lib.rs` — 导出新模块

---

### Task 1: 间接绘制类型定义

**Files:**
- Create: `crates/engine-render/src/indirect.rs`
- Modify: `crates/engine-render/src/lib.rs`

- [ ] **Step 1: 创建 indirect.rs 文件**

```rust
// crates/engine-render/src/indirect.rs

use bytemuck::{Pod, Zeroable};

/// DrawIndexedIndirect 命令参数
/// 对应 wgpu 的 DrawIndexedIndirectArgs
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct DrawIndexedIndirectArgs {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub base_vertex: i32,
    pub first_instance: u32,
}

impl DrawIndexedIndirectArgs {
    pub fn new(index_count: u32, instance_count: u32) -> Self {
        Self {
            index_count,
            instance_count,
            first_index: 0,
            base_vertex: 0,
            first_instance: 0,
        }
    }
}
```

- [ ] **Step 2: 修改 lib.rs 导出新模块**

在 `crates/engine-render/src/lib.rs` 中添加：

```rust
pub mod indirect;
```

- [ ] **Step 3: 运行测试验证编译**

Run: `cargo build -p engine-render`
Expected: 编译成功

- [ ] **Step 4: 提交**

```bash
git add crates/engine-render/src/indirect.rs crates/engine-render/src/lib.rs
git commit -m "feat(render): add DrawIndexedIndirectArgs type"
```

---

### Task 2: SpriteBatch 增加实例数据支持

**Files:**
- Modify: `crates/engine-render/src/sprite.rs`

- [ ] **Step 1: 修改 SpriteBatch 结构体**

在 `crates/engine-render/src/sprite.rs` 中修改 SpriteBatch：

```rust
use crate::indirect::DrawIndexedIndirectArgs;
use engine_math::Mat4;

pub struct SpriteBatch {
    pub texture_id: u64,
    pub vertices: Vec<SpriteVertex>,
    pub indices: Vec<u16>,
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
    pub index_count: u32,
    // 新增字段
    pub instance_data: Vec<Mat4>,
    pub indirect_cmd: DrawIndexedIndirectArgs,
}
```

- [ ] **Step 2: 修改 SpriteBatch::new()**

```rust
impl SpriteBatch {
    pub fn new(texture_id: u64) -> Self {
        Self {
            texture_id,
            vertices: Vec::new(),
            indices: Vec::new(),
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
            instance_data: Vec::new(),
            indirect_cmd: DrawIndexedIndirectArgs::new(0, 0),
        }
    }
```

- [ ] **Step 3: 修改 push() 方法收集实例数据**

```rust
    pub fn push(&mut self, draw: &SpriteDraw) {
        let base = self.vertices.len() as u16;
        let w = draw.size.x * 0.5;
        let h = draw.size.y * 0.5;
        let (u0, u1) = if draw.flip_x { (1.0, 0.0) } else { (0.0, 1.0) };
        let (v0, v1) = if draw.flip_y { (1.0, 0.0) } else { (0.0, 1.0) };

        self.vertices.extend_from_slice(&[
            SpriteVertex {
                position: [-w, -h, 0.0],
                uv: [u0, v1],
                color: draw.color,
            },
            SpriteVertex {
                position: [w, -h, 0.0],
                uv: [u1, v1],
                color: draw.color,
            },
            SpriteVertex {
                position: [w, h, 0.0],
                uv: [u1, v0],
                color: draw.color,
            },
            SpriteVertex {
                position: [-w, h, 0.0],
                uv: [u0, v0],
                color: draw.color,
            },
        ]);
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        
        // 收集实例变换矩阵
        self.instance_data.push(draw.world_matrix);
    }
```

- [ ] **Step 4: 添加 update_indirect_cmd() 方法**

```rust
    pub fn update_indirect_cmd(&mut self) {
        self.indirect_cmd = DrawIndexedIndirectArgs::new(
            self.indices.len() as u32,
            self.instance_data.len() as u32,
        );
    }
```

- [ ] **Step 5: 运行测试**

Run: `cargo test -p engine-render`
Expected: 现有测试通过

- [ ] **Step 6: 提交**

```bash
git add crates/engine-render/src/sprite.rs
git commit -m "feat(render): add instance_data to SpriteBatch"
```

---

### Task 3: 持久映射缓冲实现

**Files:**
- Create: `crates/engine-render/src/sprite_renderer.rs`

- [ ] **Step 1: 创建 PersistentBuffer 结构体**

```rust
// crates/engine-render/src/sprite_renderer.rs

use wgpu::util::DeviceExt;
use std::ptr;

/// 持久映射的 GPU 缓冲
/// 实现零拷贝上传，CPU 直接写入 GPU 内存
pub struct PersistentBuffer {
    buffer: wgpu::Buffer,
    size: usize,
    mapped_ptr: *mut u8,
}

unsafe impl Send for PersistentBuffer {}
unsafe impl Sync for PersistentBuffer {}

impl PersistentBuffer {
    /// 创建持久映射缓冲
    /// 
    /// # Safety
    /// 返回的缓冲区的映射指针必须在缓冲区有效期内有效
    pub fn new(device: &wgpu::Device, size: usize, label: Option<&str>) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label,
            size: size as u64,
            usage: wgpu::BufferUsages::VERTEX 
                | wgpu::BufferUsages::INDEX 
                | wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });

        let mapped_ptr = buffer.slice(..).get_mapped_range_mut().as_mut_ptr();

        Self {
            buffer,
            size,
            mapped_ptr,
        }
    }

    /// 获取缓冲区引用
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// 获取缓冲区大小
    pub fn size(&self) -> usize {
        self.size
    }

    /// 写入数据到缓冲区指定偏移
    /// 
    /// # Safety
    /// 调用者必须确保 offset + data.len() <= self.size
    pub unsafe fn write(&self, offset: usize, data: &[u8]) {
        debug_assert!(
            offset + data.len() <= self.size,
            "Write out of bounds: offset={}, len={}, size={}",
            offset,
            data.len(),
            self.size
        );

        let dst = self.mapped_ptr.add(offset);
        ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
    }

    /// 取消映射（在 GPU 使用前调用）
    pub fn unmap(&self) {
        self.buffer.unmap();
    }
}
```

- [ ] **Step 2: 运行测试验证编译**

Run: `cargo build -p engine-render`
Expected: 编译成功

- [ ] **Step 3: 提交**

```bash
git add crates/engine-render/src/sprite_renderer.rs
git commit -m "feat(render): add PersistentBuffer for zero-copy upload"
```

---

### Task 4: SpriteRenderer 实现

**Files:**
- Modify: `crates/engine-render/src/sprite_renderer.rs`

- [ ] **Step 1: 添加 SpriteRenderer 结构体**

在 `sprite_renderer.rs` 中添加：

```rust
use crate::pipeline::sprite::SpritePipeline;
use crate::sprite::SpriteBatch;
use crate::indirect::DrawIndexedIndirectArgs;
use std::sync::Arc;

/// 高性能精灵渲染器
/// 使用持久映射缓冲 + 间接绘制
pub struct SpriteRenderer {
    // 双缓冲实例缓冲
    instance_buffers: [PersistentBuffer; 2],
    // 间接绘制命令缓冲
    indirect_buffer: PersistentBuffer,
    // 当前帧索引 (0 or 1)
    current_frame: usize,
    // 精灵容量上限
    sprite_capacity: usize,
    // 复用现有管线
    pipeline: Arc<SpritePipeline>,
    // 顶点缓冲（复用）
    vertex_buffer: PersistentBuffer,
    // 索引缓冲（复用）
    index_buffer: PersistentBuffer,
}

impl SpriteRenderer {
    pub fn new(
        device: &wgpu::Device,
        pipeline: Arc<SpritePipeline>,
        sprite_capacity: usize,
    ) -> Self {
        // 计算缓冲区大小
        // 每个精灵: 4 顶点 * 36 字节 + 6 索引 * 2 字节 + 1 实例 * 64 字节
        let vertex_size = sprite_capacity * 4 * 36;
        let index_size = sprite_capacity * 6 * 2;
        let instance_size = sprite_capacity * 64;
        let indirect_size = sprite_capacity * std::mem::size_of::<DrawIndexedIndirectArgs>();

        let vertex_buffer = PersistentBuffer::new(
            device,
            vertex_size,
            Some("sprite_vertex_buffer"),
        );

        let index_buffer = PersistentBuffer::new(
            device,
            index_size,
            Some("sprite_index_buffer"),
        );

        let instance_buffers = [
            PersistentBuffer::new(device, instance_size, Some("sprite_instance_buffer_0")),
            PersistentBuffer::new(device, instance_size, Some("sprite_instance_buffer_1")),
        ];

        let indirect_buffer = PersistentBuffer::new(
            device,
            indirect_size,
            Some("sprite_indirect_buffer"),
        );

        // 取消映射，准备 GPU 使用
        vertex_buffer.unmap();
        index_buffer.unmap();
        instance_buffers[0].unmap();
        instance_buffers[1].unmap();
        indirect_buffer.unmap();

        Self {
            instance_buffers,
            indirect_buffer,
            current_frame: 0,
            sprite_capacity,
            pipeline,
            vertex_buffer,
            index_buffer,
        }
    }

    /// 开始新帧，切换双缓冲
    pub fn begin_frame(&mut self) {
        self.current_frame = 1 - self.current_frame;
    }

    /// 获取当前帧的实例缓冲
    pub fn current_instance_buffer(&self) -> &PersistentBuffer {
        &self.instance_buffers[self.current_frame]
    }

    /// 上传批次数据到持久缓冲
    pub fn upload_batch(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        batch: &SpriteBatch,
        vertex_offset: usize,
        instance_offset: usize,
        indirect_offset: usize,
    ) {
        // 写入顶点数据
        let vertex_data = bytemuck::cast_slice(&batch.vertices);
        unsafe {
            self.vertex_buffer.write(vertex_offset, vertex_data);
        }

        // 写入索引数据
        let index_data = bytemuck::cast_slice(&batch.indices);
        unsafe {
            self.index_buffer.write(
                vertex_offset / 36 * 6 * 2, // 索引偏移 = 顶点偏移 / 顶点大小 * 索引大小
                index_data,
            );
        }

        // 写入实例数据
        let instance_data = bytemuck::cast_slice(&batch.instance_data);
        unsafe {
            self.current_instance_buffer().write(instance_offset, instance_data);
        }

        // 写入间接绘制命令
        let indirect_data = bytemuck::bytes_of(&batch.indirect_cmd);
        unsafe {
            self.indirect_buffer.write(indirect_offset, indirect_data);
        }
    }

    /// 获取绘制所需的缓冲区引用
    pub fn get_buffers(&self) -> SpriteRendererBuffers {
        SpriteRendererBuffers {
            vertex_buffer: self.vertex_buffer.buffer(),
            index_buffer: self.index_buffer.buffer(),
            instance_buffer: self.current_instance_buffer().buffer(),
            indirect_buffer: self.indirect_buffer.buffer(),
        }
    }
}

/// 绘制所需的缓冲区引用集合
pub struct SpriteRendererBuffers<'a> {
    pub vertex_buffer: &'a wgpu::Buffer,
    pub index_buffer: &'a wgpu::Buffer,
    pub instance_buffer: &'a wgpu::Buffer,
    pub indirect_buffer: &'a wgpu::Buffer,
}
```

- [ ] **Step 2: 运行测试验证编译**

Run: `cargo build -p engine-render`
Expected: 编译成功

- [ ] **Step 3: 提交**

```bash
git add crates/engine-render/src/sprite_renderer.rs
git commit -m "feat(render): add SpriteRenderer with persistent buffers"
```

---

### Task 5: 修改着色器支持实例化渲染

**Files:**
- Modify: `crates/engine-render/src/pipeline/sprite.wgsl`

- [ ] **Step 1: 修改顶点着色器输入**

```wgsl
// pipeline/sprite.wgsl

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct InstanceInput {
    @location(3) model_matrix_0: vec4<f32>,
    @location(4) model_matrix_1: vec4<f32>,
    @location(5) model_matrix_2: vec4<f32>,
    @location(6) model_matrix_3: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    
    var out: VertexOutput;
    out.clip_position = camera.view_proj * model * vec4<f32>(vertex.position, 1.0);
    out.uv = vertex.uv;
    out.color = vertex.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texture_color = textureSample(t_diffuse, s_diffuse, in.uv);
    return texture_color * in.color;
}
```

- [ ] **Step 2: 运行测试验证编译**

Run: `cargo build -p engine-render`
Expected: 编译成功

- [ ] **Step 3: 提交**

```bash
git add crates/engine-render/src/pipeline/sprite.wgsl
git commit -m "feat(render): update sprite shader for instanced rendering"
```

---

### Task 6: 修改管线支持实例化顶点布局

**Files:**
- Modify: `crates/engine-render/src/pipeline/sprite.rs`

- [ ] **Step 1: 修改顶点缓冲布局**

在 `pipeline/sprite.rs` 中修改 `SpritePipeline::new()`：

```rust
impl SpritePipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        // ... 现有代码 ...

        // 修改顶点布局，支持实例化
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // uv
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // color
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        };

        // 实例缓冲布局 (mat4x4 = 4 x vec4)
        let instance_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<engine_math::Mat4>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // model_matrix_0
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // model_matrix_1
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // model_matrix_2
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // model_matrix_3
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        };

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
                buffers: &[vertex_layout, instance_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            // ... 其余配置保持不变 ...
        });

        // ... 返回 Self ...
    }
}
```

- [ ] **Step 2: 运行测试验证编译**

Run: `cargo build -p engine-render`
Expected: 编译成功

- [ ] **Step 3: 提交**

```bash
git add crates/engine-render/src/pipeline/sprite.rs
git commit -m "feat(render): update pipeline for instanced rendering"
```

---

### Task 7: 集成 SpriteRenderer 到 Renderer

**Files:**
- Modify: `crates/engine-render/src/renderer.rs`

- [ ] **Step 1: 添加 SpriteRenderer 字段**

在 `renderer.rs` 中修改 Renderer 结构体：

```rust
use crate::sprite_renderer::SpriteRenderer;

pub struct Renderer {
    pub device: GpuDevice,
    pub queue: GpuQueue,
    pub surface: Surface<'static>,
    pub config: SurfaceConfiguration,
    pub graph: crate::graph::RenderGraph,
    pub sprite_pipeline: Arc<SpritePipeline>,
    camera_uniform: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    // 新增
    pub sprite_renderer: SpriteRenderer,
}
```

- [ ] **Step 2: 修改 Renderer::new() 初始化 SpriteRenderer**

```rust
impl Renderer {
    pub fn new(window: std::sync::Arc<winit::window::Window>) -> Self {
        // ... 现有初始化代码 ...

        let sprite_renderer = SpriteRenderer::new(
            &device,
            sprite_pipeline.clone(),
            10000, // 默认容量
        );

        Self {
            device: GpuDevice(Arc::new(device)),
            queue: GpuQueue(Arc::new(queue)),
            surface,
            config,
            graph: crate::graph::RenderGraph::new(),
            sprite_pipeline,
            camera_uniform,
            camera_bind_group,
            sprite_renderer,
        }
    }
```

- [ ] **Step 3: 修改 render_frame() 使用 SpriteRenderer**

```rust
    pub fn render_frame(
        &mut self,
        cameras: &[&crate::camera::Camera],
        all_sprites: &[crate::sprite::Sprite],
        bridge: &mut crate::texture_bridge::TextureBridge,
    ) -> Result<(), wgpu::SurfaceError> {
        use crate::camera::{Camera, RenderTarget};
        use crate::frustum::Frustum;
        use crate::sprite::SpriteDraw;

        let mut sorted: Vec<&Camera> = cameras.to_vec();
        sorted.sort_by_key(|c| c.priority);

        // Flush pending texture uploads
        bridge.flush(&self.device, &self.queue);

        // Convert Sprite → SpriteDraw
        let sprite_draws: Vec<SpriteDraw> = all_sprites
            .iter()
            .map(|s| SpriteDraw {
                world_matrix: s.transform,
                color: s.color,
                size: s.size,
                texture_id: bridge.resolve(&s.texture),
                flip_x: s.flip_x,
                flip_y: s.flip_y,
            })
            .collect();

        let output = self.surface.get_current_texture()?;
        let swapchain_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("main_encoder"),
            });

        let surface_width = self.config.width;
        let surface_height = self.config.height;

        // 开始新帧
        self.sprite_renderer.begin_frame();

        for camera in &sorted {
            if !camera.is_active {
                continue;
            }

            let (vx, vy, vw, vh) = camera.viewport.to_absolute(surface_width, surface_height);
            let aspect = vw as f32 / vh.max(1) as f32;
            let vp_matrix = camera.view_projection(aspect);

            let frustum = Frustum::from_view_projection(&vp_matrix);
            let visible: Vec<crate::sprite::SpriteDraw> = sprite_draws
                .iter()
                .filter(|s| {
                    let pos = s.world_matrix.transform_point3(Vec3::ZERO);
                    let half = Vec3::new(s.size.x * 0.5, s.size.y * 0.5, 0.1);
                    frustum.test_aabb(pos - half, pos + half)
                })
                .cloned()
                .collect();

            let mut batches = crate::sprite::collect_batches(&visible);
            
            // 更新间接绘制命令
            for batch in &mut batches {
                batch.update_indirect_cmd();
            }

            // 上传批次数据到持久缓冲
            let mut vertex_offset = 0;
            let mut instance_offset = 0;
            let mut indirect_offset = 0;
            
            for batch in &batches {
                self.sprite_renderer.upload_batch(
                    &self.device,
                    &self.queue,
                    batch,
                    vertex_offset,
                    instance_offset,
                    indirect_offset,
                );
                vertex_offset += batch.vertices.len() * std::mem::size_of::<crate::pipeline::sprite::SpriteVertex>();
                instance_offset += batch.instance_data.len() * std::mem::size_of::<engine_math::Mat4>();
                indirect_offset += std::mem::size_of::<crate::indirect::DrawIndexedIndirectArgs>();
            }

            let matrix_data = vp_matrix.to_cols_array();
            self.queue
                .write_buffer(&self.camera_uniform, 0, bytemuck::cast_slice(&matrix_data));

            let target_view = match camera.render_target {
                RenderTarget::Screen => &swapchain_view,
                RenderTarget::Texture(key) => bridge
                    .texture_store()
                    .get_render_target_view(key)
                    .expect("render target texture not found"),
            };

            let camera_bg = &self.camera_bind_group;
            let pipeline = &self.sprite_pipeline.pipeline;
            let buffers = self.sprite_renderer.get_buffers();

            let clear = camera.clear_color.unwrap_or(crate::camera::Color::BLACK);
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("camera_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(clear.to_wgpu()),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                pass.set_viewport(vx as f32, vy as f32, vw as f32, vh as f32, 0.0, 1.0);
                pass.set_scissor_rect(vx, vy, vw, vh);

                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, camera_bg, &[]);
                
                // 设置顶点和实例缓冲
                pass.set_vertex_buffer(0, buffers.vertex_buffer.slice(..));
                pass.set_vertex_buffer(1, buffers.instance_buffer.slice(..));
                pass.set_index_buffer(buffers.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                
                // 使用间接绘制
                let mut indirect_offset = 0;
                for batch in &batches {
                    let bind_group = bridge.texture_store().get_bind_group(batch.texture_id);
                    pass.set_bind_group(1, bind_group, &[]);
                    pass.draw_indexed_indirect(
                        buffers.indirect_buffer,
                        indirect_offset as u64,
                    );
                    indirect_offset += std::mem::size_of::<crate::indirect::DrawIndexedIndirectArgs>();
                }
            }
        }

        self.queue.submit([encoder.finish()]);
        output.present();
        Ok(())
    }
```

- [ ] **Step 4: 运行测试验证编译**

Run: `cargo build -p engine-render`
Expected: 编译成功

- [ ] **Step 5: 提交**

```bash
git add crates/engine-render/src/renderer.rs
git commit -m "feat(render): integrate SpriteRenderer for instanced rendering"
```

---

### Task 8: 集成测试

**Files:**
- Test: `crates/engine-render/tests/sprite_batch_performance.rs`

- [ ] **Step 1: 创建性能测试文件**

```rust
// crates/engine-render/tests/sprite_batch_performance.rs

use engine_render::sprite::{SpriteBatch, SpriteDraw};
use engine_math::{Mat4, Vec2};

#[test]
fn test_sprite_batch_instance_data() {
    let mut batch = SpriteBatch::new(0);
    
    let draw = SpriteDraw {
        world_matrix: Mat4::from_translation(glam::Vec3::new(100.0, 200.0, 0.0)),
        color: [1.0, 1.0, 1.0, 1.0],
        size: Vec2::new(50.0, 50.0),
        texture_id: 0,
        flip_x: false,
        flip_y: false,
    };
    
    batch.push(&draw);
    batch.push(&draw);
    batch.push(&draw);
    
    assert_eq!(batch.instance_data.len(), 3);
    assert_eq!(batch.vertices.len(), 12); // 3 sprites * 4 vertices
    assert_eq!(batch.indices.len(), 18);  // 3 sprites * 6 indices
    
    batch.update_indirect_cmd();
    assert_eq!(batch.indirect_cmd.instance_count, 3);
    assert_eq!(batch.indirect_cmd.index_count, 18);
}

#[test]
fn test_collect_batches_with_instances() {
    let draws = vec![
        SpriteDraw {
            texture_id: 1,
            world_matrix: Mat4::IDENTITY,
            color: [1.0, 1.0, 1.0, 1.0],
            size: Vec2::new(100.0, 100.0),
            flip_x: false,
            flip_y: false,
        },
        SpriteDraw {
            texture_id: 0,
            world_matrix: Mat4::IDENTITY,
            color: [1.0, 1.0, 1.0, 1.0],
            size: Vec2::new(100.0, 100.0),
            flip_x: false,
            flip_y: false,
        },
        SpriteDraw {
            texture_id: 1,
            world_matrix: Mat4::IDENTITY,
            color: [1.0, 1.0, 1.0, 1.0],
            size: Vec2::new(100.0, 100.0),
            flip_x: false,
            flip_y: false,
        },
    ];
    
    let batches = engine_render::sprite::collect_batches(&draws);
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0].texture_id, 0);
    assert_eq!(batches[0].instance_data.len(), 1);
    assert_eq!(batches[1].texture_id, 1);
    assert_eq!(batches[1].instance_data.len(), 2);
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test -p engine-render`
Expected: 所有测试通过

- [ ] **Step 3: 提交**

```bash
git add crates/engine-render/tests/sprite_batch_performance.rs
git commit -m "test(render): add sprite batch performance tests"
```

---

### Task 9: 更新示例使用新渲染器

**Files:**
- Modify: `examples/sprite_demo/src/main.rs`

- [ ] **Step 1: 检查现有示例**

Run: `cargo build --example sprite_demo -p engine-core`
Expected: 编译成功（确认现有示例正常工作）

- [ ] **Step 2: 运行示例验证渲染效果**

Run: `cargo run --example sprite_demo -p engine-core`
Expected: 精灵正常渲染，无视觉错误

- [ ] **Step 3: 提交最终版本**

```bash
git add .
git commit -m "feat(render): complete sprite batch performance optimization"
```

---

## 验证清单

- [ ] 所有测试通过：`cargo test -p engine-render`
- [ ] 代码格式正确：`cargo fmt`
- [ ] 无 clippy 警告：`cargo clippy`
- [ ] 示例正常运行：`cargo run --example sprite_demo -p engine-core`
- [ ] 性能提升：10000 精灵场景下帧率提升

---

## 风险与回退方案

1. **持久映射缓冲不支持** — 回退到常规 `write_buffer` 上传
2. **内存对齐问题** — 调整缓冲区大小计算
3. **同步问题** — 添加显式 fence 同步

---

## 成功标准

- 同屏 10000 精灵稳定 60fps
- Draw call 数量减少（按纹理批次分组）
- CPU 端内存拷贝减少 90%+
- 所有现有测试通过
- 示例正常运行
