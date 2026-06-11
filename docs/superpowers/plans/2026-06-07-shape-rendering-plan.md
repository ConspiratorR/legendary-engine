# Shape Rendering System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 2D Shape rendering to RustEngine using SDF (signed distance field) fragment shader. Supports rect, circle, ellipse, rounded rect, line with fill, stroke, and anti-aliasing.

**Architecture:** ShapePainter collects commands → ShapeBatch generates GPU resources → ShapePipeline renders via SDF WGSL shader in a dedicated render pass.

**Tech Stack:** Rust, wgpu 23, WGSL

---

## File Structure

```
crates/engine-render/src/shape/
├── mod.rs              # Module entry, pub use
├── error.rs            # ShapeError type
├── types.rs            # Color, FillMode, Stroke, ShapeCommand
├── batch.rs            # ShapeBatch, PreparedBatch, DrawCall
├── pipeline.rs         # ShapePipeline (shader + render pipeline)
└── painter.rs          # ShapePainter (high-level API)

crates/engine-render/src/shaders/
└── shape.wgsl          # SDF fragment shader

Modified files:
├── crates/engine-render/src/lib.rs         # Add pub mod shape
└── crates/engine-render/src/plugin.rs      # Insert ShapePipeline + ShapePainter as ECS resources
```

---

### Task 1: Shape types and error

**Files:**
- Create: `crates/engine-render/src/shape/error.rs`
- Create: `crates/engine-render/src/shape/types.rs`
- Create: `crates/engine-render/src/shape/mod.rs` (placeholder)

- [ ] **Step 1: Create shape/error.rs**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShapeError {
    #[error("shader compilation failed: {0}")]
    ShaderCompilation(String),
    #[error("pipeline creation failed: {0}")]
    PipelineCreation(String),
}
```

- [ ] **Step 2: Create shape/types.rs**

```rust
/// RGBA color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    pub const RED: Self = Self::new(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: Self = Self::new(0.0, 1.0, 0.0, 1.0);
    pub const BLUE: Self = Self::new(0.0, 0.0, 1.0, 1.0);
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// Fill mode for shapes.
#[derive(Debug, Clone, PartialEq)]
pub enum FillMode {
    /// Solid color fill.
    Solid(Color),
    /// No fill (stroke only).
    None,
}

/// Stroke configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    pub color: Color,
    pub width: f32,
}

impl Stroke {
    pub fn new(color: Color, width: f32) -> Self {
        Self { color, width }
    }
}

/// Shape command for deferred rendering.
#[derive(Debug, Clone, PartialEq)]
pub enum ShapeCommand {
    /// Rectangle (axis-aligned).
    Rect {
        position: [f32; 2],
        size: [f32; 2],
        fill: FillMode,
        stroke: Option<Stroke>,
        corner_radius: f32,
    },
    /// Circle.
    Circle {
        center: [f32; 2],
        radius: f32,
        fill: FillMode,
        stroke: Option<Stroke>,
    },
    /// Ellipse.
    Ellipse {
        center: [f32; 2],
        radii: [f32; 2],
        fill: FillMode,
        stroke: Option<Stroke>,
    },
    /// Rounded rectangle with per-corner radii.
    RoundedRectangle {
        position: [f32; 2],
        size: [f32; 2],
        corner_radius: [f32; 4],
        fill: FillMode,
        stroke: Option<Stroke>,
    },
    /// Line segment.
    Line {
        start: [f32; 2],
        end: [f32; 2],
        color: Color,
        width: f32,
    },
}
```

- [ ] **Step 3: Create shape/mod.rs (placeholder)**

```rust
pub mod error;
pub mod types;
```

- [ ] **Step 4: Add `pub mod shape;` to lib.rs**

Add after `pub mod shadow;` in `crates/engine-render/src/lib.rs`.

- [ ] **Step 5: Verify compilation**

Run: `cargo check -p engine-render`

- [ ] **Step 6: Commit**

```bash
git add crates/engine-render/src/shape/ crates/engine-render/src/lib.rs
git commit -m "feat(render): add Shape types (Color, FillMode, Stroke, ShapeCommand)"
```

---

### Task 2: SDF WGSL shader

**Files:**
- Create: `crates/engine-render/src/shaders/shape.wgsl`

- [ ] **Step 1: Create shape.wgsl**

```wgsl
// SDF 2D Shape Rendering Shader
// Supports: rect, circle, ellipse, rounded rect, line

struct VertexInput {
    @location(0) position: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
};

struct ShapeUniform {
    transform: mat4x4<f32>,
    size: vec2<f32>,
    color: vec4<f32>,
    stroke_color: vec4<f32>,
    stroke_width: f32,
    corner_radius: f32,
    shape_type: u32,
    _padding: u32,
};

@group(0) @binding(0)
var<uniform> shape: ShapeUniform;

// Vertex shader: full-screen quad for each shape
@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = shape.transform * vec4<f32>(in.position, 0.0, 1.0);
    out.local_pos = in.position;
    return out;
}

// SDF functions

fn sdBox(p: vec2<f32>, b: vec2<f32>) -> f32 {
    let d = abs(p) - b;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0);
}

fn sdCircle(p: vec2<f32>, r: f32) -> f32 {
    return length(p) - r;
}

fn sdEllipse(p: vec2<f32>, ab: vec2<f32>) -> f32 {
    let pa = p / ab;
    return (length(pa) - 1.0) * min(ab.x, ab.y);
}

fn sdRoundedBox(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2<f32>(r);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn sdSegment(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h);
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.local_pos;
    var d: f32;

    // Compute SDF distance based on shape type
    switch shape.shape_type {
        case 0u: {
            // Rectangle
            d = sdBox(p, shape.size * 0.5);
        }
        case 1u: {
            // Circle
            d = sdCircle(p, shape.size.x * 0.5);
        }
        case 2u: {
            // Ellipse
            d = sdEllipse(p, shape.size * 0.5);
        }
        case 3u: {
            // Rounded rectangle
            d = sdRoundedBox(p, shape.size * 0.5, shape.corner_radius);
        }
        case 4u: {
            // Line: p is in [-0.5, 0.5] space, endpoints at (-0.5, 0) and (0.5, 0)
            d = sdSegment(p, vec2<f32>(-0.5, 0.0), vec2<f32>(0.5, 0.0)) - shape.size.y * 0.5;
        }
        default: {
            d = 1.0;
        }
    }

    // Anti-aliased fill
    let aa = 1.0; // 1 pixel anti-aliasing
    let fill_alpha = 1.0 - smoothstep(-aa, aa, d);

    var color = vec4<f32>(shape.color.rgb, shape.color.a * fill_alpha);

    // Stroke
    if shape.stroke_width > 0.0 {
        let stroke_d = abs(d) - shape.stroke_width;
        let stroke_alpha = 1.0 - smoothstep(-aa, aa, stroke_d);
        let stroke_mix = shape.stroke_color.a * stroke_alpha;
        color = mix(color, shape.stroke_color, stroke_mix);
    }

    return color;
}
```

- [ ] **Step 2: Verify shader loads**

Run: `cargo check -p engine-render` (shader is loaded at runtime via include_str!)

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/src/shaders/shape.wgsl
git commit -m "feat(render): add SDF shape WGSL shader"
```

---

### Task 3: ShapeBatch — command collection and GPU resource generation

**Files:**
- Create: `crates/engine-render/src/shape/batch.rs`
- Update: `crates/engine-render/src/shape/mod.rs`

- [ ] **Step 1: Create shape/batch.rs**

```rust
use super::types::{ShapeCommand, FillMode};
use bytemuck::{Pod, Zeroable};

/// GPU uniform for a single shape.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ShapeUniform {
    pub transform: [[f32; 4]; 4],
    pub size: [f32; 2],
    pub _pad0: [f32; 2],
    pub color: [f32; 4],
    pub stroke_color: [f32; 4],
    pub stroke_width: f32,
    pub corner_radius: f32,
    pub shape_type: u32,
    pub _padding: u32,
}

/// Vertex for shape quad (position only).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ShapeVertex {
    pub position: [f32; 2],
}

/// A prepared draw call for a single shape.
pub struct DrawCall {
    pub vertex_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

/// Prepared batch ready for rendering.
pub struct PreparedBatch {
    pub draw_calls: Vec<DrawCall>,
}

/// Collects shape commands and prepares GPU resources.
pub struct ShapeBatch {
    commands: Vec<ShapeCommand>,
}

impl ShapeBatch {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn push(&mut self, cmd: ShapeCommand) {
        self.commands.push(cmd);
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Prepare GPU resources for all commands.
    pub fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        uniform_layout: &wgpu::BindGroupLayout,
    ) -> PreparedBatch {
        let mut draw_calls = Vec::with_capacity(self.commands.len());

        for cmd in &self.commands {
            let (uniform, bbox) = self.cmd_to_uniform(cmd);

            // Upload uniform
            let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("shape_uniform"),
                contents: bytemuck::bytes_of(&uniform),
                usage: wgpu::BufferUsages::UNIFORM,
            });

            // Vertex buffer: quad covering bbox
            let (x0, y0, x1, y1) = bbox;
            let vertices = [
                ShapeVertex { position: [x0, y0] },
                ShapeVertex { position: [x1, y0] },
                ShapeVertex { position: [x0, y1] },
                ShapeVertex { position: [x1, y1] },
            ];
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("shape_vertex"),
                contents: bytemuck::bytes_of(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("shape_bind_group"),
                layout: uniform_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
            });

            draw_calls.push(DrawCall {
                vertex_buffer,
                uniform_buffer,
                bind_group,
            });
        }

        PreparedBatch { draw_calls }
    }

    /// Convert a ShapeCommand to (ShapeUniform, bounding_box).
    fn cmd_to_uniform(&self, cmd: &ShapeCommand) -> (ShapeUniform, (f32, f32, f32, f32)) {
        match cmd {
            ShapeCommand::Rect { position, size, fill, stroke, corner_radius } => {
                let color = match fill {
                    FillMode::Solid(c) => c.to_array(),
                    FillMode::None => [0.0; 4],
                };
                let stroke_color = stroke.as_ref().map(|s| s.color.to_array()).unwrap_or([0.0; 4]);
                let stroke_width = stroke.as_ref().map(|s| s.width).unwrap_or(0.0);

                let cx = position[0] + size[0] * 0.5;
                let cy = position[1] + size[1] * 0.5;

                let uniform = ShapeUniform {
                    transform: Self::ortho_transform(cx, cy, size[0], size[1]),
                    size: *size,
                    _pad0: [0.0; 2],
                    color,
                    stroke_color,
                    stroke_width,
                    corner_radius: *corner_radius,
                    shape_type: 0,
                    _padding: 0,
                };
                (uniform, (position[0], position[1], position[0] + size[0], position[1] + size[1]))
            }
            ShapeCommand::Circle { center, radius, fill, stroke } => {
                let color = match fill {
                    FillMode::Solid(c) => c.to_array(),
                    FillMode::None => [0.0; 4],
                };
                let stroke_color = stroke.as_ref().map(|s| s.color.to_array()).unwrap_or([0.0; 4]);
                let stroke_width = stroke.as_ref().map(|s| s.width).unwrap_or(0.0);
                let d = radius * 2.0;

                let uniform = ShapeUniform {
                    transform: Self::ortho_transform(center[0], center[1], d, d),
                    size: [d, d],
                    _pad0: [0.0; 2],
                    color,
                    stroke_color,
                    stroke_width,
                    corner_radius: 0.0,
                    shape_type: 1,
                    _padding: 0,
                };
                (uniform, (center[0] - radius, center[1] - radius, center[0] + radius, center[1] + radius))
            }
            ShapeCommand::Ellipse { center, radii, fill, stroke } => {
                let color = match fill {
                    FillMode::Solid(c) => c.to_array(),
                    FillMode::None => [0.0; 4],
                };
                let stroke_color = stroke.as_ref().map(|s| s.color.to_array()).unwrap_or([0.0; 4]);
                let stroke_width = stroke.as_ref().map(|s| s.width).unwrap_or(0.0);
                let w = radii[0] * 2.0;
                let h = radii[1] * 2.0;

                let uniform = ShapeUniform {
                    transform: Self::ortho_transform(center[0], center[1], w, h),
                    size: [w, h],
                    _pad0: [0.0; 2],
                    color,
                    stroke_color,
                    stroke_width,
                    corner_radius: 0.0,
                    shape_type: 2,
                    _padding: 0,
                };
                (uniform, (center[0] - radii[0], center[1] - radii[1], center[0] + radii[0], center[1] + radii[1]))
            }
            ShapeCommand::RoundedRectangle { position, size, corner_radius, fill, stroke } => {
                let color = match fill {
                    FillMode::Solid(c) => c.to_array(),
                    FillMode::None => [0.0; 4],
                };
                let stroke_color = stroke.as_ref().map(|s| s.color.to_array()).unwrap_or([0.0; 4]);
                let stroke_width = stroke.as_ref().map(|s| s.width).unwrap_or(0.0);
                let max_radius = corner_radius.iter().copied().fold(0.0f32, f32::max);

                let cx = position[0] + size[0] * 0.5;
                let cy = position[1] + size[1] * 0.5;

                let uniform = ShapeUniform {
                    transform: Self::ortho_transform(cx, cy, size[0], size[1]),
                    size: *size,
                    _pad0: [0.0; 2],
                    color,
                    stroke_color,
                    stroke_width,
                    corner_radius: max_radius,
                    shape_type: 3,
                    _padding: 0,
                };
                (uniform, (position[0], position[1], position[0] + size[0], position[1] + size[1]))
            }
            ShapeCommand::Line { start, end, color, width } => {
                let dx = end[0] - start[0];
                let dy = end[1] - start[1];
                let len = (dx * dx + dy * dy).sqrt();
                let cx = (start[0] + end[0]) * 0.5;
                let cy = (start[1] + end[1]) * 0.5;

                // Rotation angle
                let angle = dy.atan2(dx);
                let cos_a = angle.cos();
                let sin_a = angle.sin();

                // Build transform: translate to center, rotate, scale to length
                let scale_x = len;
                let scale_y = *width;
                let transform = [
                    [cos_a * scale_x, sin_a * scale_x, 0.0, 0.0],
                    [-sin_a * scale_y, cos_a * scale_y, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [cx, cy, 0.0, 1.0],
                ];

                let uniform = ShapeUniform {
                    transform,
                    size: [len, *width],
                    _pad0: [0.0; 2],
                    color: color.to_array(),
                    stroke_color: [0.0; 4],
                    stroke_width: 0.0,
                    corner_radius: 0.0,
                    shape_type: 4,
                    _padding: 0,
                };
                let hw = width * 0.5;
                let x0 = start[0].min(end[0]) - hw;
                let y0 = start[1].min(end[1]) - hw;
                let x1 = start[0].max(end[0]) + hw;
                let y1 = start[1].max(end[1]) + hw;
                (uniform, (x0, y0, x1, y1))
            }
        }
    }

    /// Build an orthographic transform for a shape centered at (cx, cy) with given size.
    fn ortho_transform(cx: f32, cy: f32, w: f32, h: f32) -> [[f32; 4]; 4] {
        // Scale by 1/(w,h) to normalize to [-0.5, 0.5], then translate to center
        [
            [1.0 / w, 0.0, 0.0, 0.0],
            [0.0, 1.0 / h, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [cx, cy, 0.0, 1.0],
        ]
    }
}

impl Default for ShapeBatch {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: Update mod.rs**

```rust
pub mod error;
pub mod types;
pub mod batch;

pub use error::ShapeError;
pub use types::{Color, FillMode, Stroke, ShapeCommand};
pub use batch::{ShapeBatch, PreparedBatch, DrawCall};
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-render`

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/shape/batch.rs crates/engine-render/src/shape/mod.rs
git commit -m "feat(render): add ShapeBatch with command collection and GPU resource generation"
```

---

### Task 4: ShapePipeline — SDF shader and render pipeline

**Files:**
- Create: `crates/engine-render/src/shape/pipeline.rs`
- Update: `crates/engine-render/src/shape/mod.rs`

- [ ] **Step 1: Create shape/pipeline.rs**

```rust
use super::batch::PreparedBatch;
use super::ShapeError;

/// SDF shape rendering pipeline.
pub struct ShapePipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub uniform_layout: wgpu::BindGroupLayout,
}

impl ShapePipeline {
    /// Create a new shape pipeline.
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        // Uniform bind group layout
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shape_uniform_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shape_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/shape.wgsl").into()),
        });

        // Pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shape_pipeline_layout"),
            bind_group_layouts: &[&uniform_layout],
            push_constant_ranges: &[],
        });

        // Render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shape_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 8, // 2 × f32
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    }],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            render_pipeline,
            uniform_layout,
        }
    }

    /// Render a prepared batch into the given render pass.
    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, prepared: &'a PreparedBatch) {
        pass.set_pipeline(&self.render_pipeline);
        for dc in &prepared.draw_calls {
            pass.set_bind_group(0, &dc.bind_group, &[]);
            pass.set_vertex_buffer(0, dc.vertex_buffer.slice(..));
            pass.draw(0..4, 0..1);
        }
    }
}
```

- [ ] **Step 2: Update mod.rs**

```rust
pub mod error;
pub mod types;
pub mod batch;
pub mod pipeline;

pub use error::ShapeError;
pub use types::{Color, FillMode, Stroke, ShapeCommand};
pub use batch::{ShapeBatch, PreparedBatch, DrawCall};
pub use pipeline::ShapePipeline;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-render`

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/shape/pipeline.rs crates/engine-render/src/shape/mod.rs
git commit -m "feat(render): add ShapePipeline with SDF shader and render pipeline"
```

---

### Task 5: ShapePainter — high-level API

**Files:**
- Create: `crates/engine-render/src/shape/painter.rs`
- Update: `crates/engine-render/src/shape/mod.rs`

- [ ] **Step 1: Create shape/painter.rs**

```rust
use super::batch::ShapeBatch;
use super::pipeline::ShapePipeline;
use super::types::{Color, FillMode, ShapeCommand, Stroke};

/// High-level API for drawing 2D shapes.
pub struct ShapePainter {
    batch: ShapeBatch,
}

impl ShapePainter {
    pub fn new() -> Self {
        Self {
            batch: ShapeBatch::new(),
        }
    }

    /// Draw a filled rectangle.
    pub fn rect(&mut self, position: [f32; 2], size: [f32; 2], color: Color) {
        self.batch.push(ShapeCommand::Rect {
            position,
            size,
            fill: FillMode::Solid(color),
            stroke: None,
            corner_radius: 0.0,
        });
    }

    /// Draw a rectangle with fill and stroke.
    pub fn rect_stroked(
        &mut self,
        position: [f32; 2],
        size: [f32; 2],
        fill: Color,
        stroke: Color,
        stroke_width: f32,
    ) {
        self.batch.push(ShapeCommand::Rect {
            position,
            size,
            fill: FillMode::Solid(fill),
            stroke: Some(Stroke::new(stroke, stroke_width)),
            corner_radius: 0.0,
        });
    }

    /// Draw a filled circle.
    pub fn circle(&mut self, center: [f32; 2], radius: f32, color: Color) {
        self.batch.push(ShapeCommand::Circle {
            center,
            radius,
            fill: FillMode::Solid(color),
            stroke: None,
        });
    }

    /// Draw a circle with fill and stroke.
    pub fn circle_stroked(
        &mut self,
        center: [f32; 2],
        radius: f32,
        fill: Color,
        stroke: Color,
        stroke_width: f32,
    ) {
        self.batch.push(ShapeCommand::Circle {
            center,
            radius,
            fill: FillMode::Solid(fill),
            stroke: Some(Stroke::new(stroke, stroke_width)),
        });
    }

    /// Draw a filled ellipse.
    pub fn ellipse(&mut self, center: [f32; 2], radii: [f32; 2], color: Color) {
        self.batch.push(ShapeCommand::Ellipse {
            center,
            radii,
            fill: FillMode::Solid(color),
            stroke: None,
        });
    }

    /// Draw a filled rounded rectangle.
    pub fn rounded_rect(
        &mut self,
        position: [f32; 2],
        size: [f32; 2],
        corner_radius: f32,
        color: Color,
    ) {
        self.batch.push(ShapeCommand::RoundedRectangle {
            position,
            size,
            corner_radius: [corner_radius; 4],
            fill: FillMode::Solid(color),
            stroke: None,
        });
    }

    /// Draw a rounded rectangle with per-corner radii.
    pub fn rounded_rect_ex(
        &mut self,
        position: [f32; 2],
        size: [f32; 2],
        corner_radius: [f32; 4],
        color: Color,
    ) {
        self.batch.push(ShapeCommand::RoundedRectangle {
            position,
            size,
            corner_radius,
            fill: FillMode::Solid(color),
            stroke: None,
        });
    }

    /// Draw a line segment.
    pub fn line(&mut self, start: [f32; 2], end: [f32; 2], color: Color, width: f32) {
        self.batch.push(ShapeCommand::Line {
            start,
            end,
            color,
            width,
        });
    }

    /// Flush all accumulated shapes to the GPU and render.
    pub fn flush(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pipeline: &ShapePipeline,
        render_pass: &mut wgpu::RenderPass,
    ) {
        if self.batch.is_empty() {
            return;
        }
        let prepared = self.batch.prepare(device, queue, &pipeline.uniform_layout);
        pipeline.render(render_pass, &prepared);
        self.batch.clear();
    }

    /// Clear all accumulated shapes without rendering.
    pub fn clear(&mut self) {
        self.batch.clear();
    }

    /// Number of shapes pending.
    pub fn pending_count(&self) -> usize {
        self.batch.len()
    }
}

impl Default for ShapePainter {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: Update mod.rs**

```rust
pub mod error;
pub mod types;
pub mod batch;
pub mod pipeline;
pub mod painter;

pub use error::ShapeError;
pub use types::{Color, FillMode, Stroke, ShapeCommand};
pub use batch::{ShapeBatch, PreparedBatch, DrawCall};
pub use pipeline::ShapePipeline;
pub use painter::ShapePainter;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-render`

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/shape/painter.rs crates/engine-render/src/shape/mod.rs
git commit -m "feat(render): add ShapePainter high-level API"
```

---

### Task 6: Integrate into RenderPlugin2D

**Files:**
- Modify: `crates/engine-render/src/plugin.rs`

- [ ] **Step 1: Add ShapePipeline and ShapePainter to plugin.rs**

Add imports:
```rust
use crate::shape::{ShapePipeline, ShapePainter};
```

In `build()`, after TextPainter creation, add:
```rust
let shape_pipeline = ShapePipeline::new(&renderer.device, wgpu::TextureFormat::Rgba16Float);
let shape_painter = ShapePainter::new();
world.insert_resource(shape_pipeline);
world.insert_resource(shape_painter);
```

Note: Use `Rgba16Float` as the target format (matches the HDR framebuffer used by the sprite pipeline). If the actual format is different, check the renderer's surface format.

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p engine-render`

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/src/plugin.rs
git commit -m "feat(render): integrate ShapePipeline and ShapePainter into RenderPlugin2D"
```

---

### Task 7: Tests and verification

**Files:**
- Add tests to existing files

- [ ] **Step 1: Add unit tests to shape/types.rs**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_to_array() {
        let c = Color::new(0.5, 0.6, 0.7, 0.8);
        assert_eq!(c.to_array(), [0.5, 0.6, 0.7, 0.8]);
    }

    #[test]
    fn test_color_constants() {
        assert_eq!(Color::WHITE.to_array(), [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(Color::BLACK.to_array(), [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_shape_command_clone() {
        let cmd = ShapeCommand::Rect {
            position: [10.0, 20.0],
            size: [100.0, 50.0],
            fill: FillMode::Solid(Color::RED),
            stroke: Some(Stroke::new(Color::WHITE, 2.0)),
            corner_radius: 0.0,
        };
        let cmd2 = cmd.clone();
        assert_eq!(cmd, cmd2);
    }
}
```

- [ ] **Step 2: Add tests to shape/batch.rs**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_push_and_len() {
        let mut batch = ShapeBatch::new();
        assert!(batch.is_empty());
        batch.push(ShapeCommand::Rect {
            position: [0.0, 0.0],
            size: [10.0, 10.0],
            fill: FillMode::Solid(Color::WHITE),
            stroke: None,
            corner_radius: 0.0,
        });
        assert_eq!(batch.len(), 1);
        batch.clear();
        assert!(batch.is_empty());
    }
}
```

- [ ] **Step 3: Run all tests**

Run: `cargo test -p engine-render --lib shape`
Expected: All tests pass

- [ ] **Step 4: Run clippy and fmt**

Run: `cargo clippy -p engine-render && cargo fmt -p engine-render`

- [ ] **Step 5: Full build verification**

Run: `cargo build`

- [ ] **Step 6: Commit**

```bash
git add crates/engine-render/src/shape/types.rs crates/engine-render/src/shape/batch.rs
git commit -m "test(render): add Shape type and batch unit tests"
```
