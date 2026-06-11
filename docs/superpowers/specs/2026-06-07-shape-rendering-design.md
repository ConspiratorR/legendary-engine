# Shape 绘制系统设计

**日期**: 2026-06-07
**状态**: 已批准
**模块**: engine-render

## 概述

为 RustEngine 添加 2D Shape 绘制能力：矩形、圆形、椭圆、圆角矩形、线条。使用 SDF（有向距离场）fragment shader 实现高质量抗锯齿渲染，支持填充、描边、阴影。

## 设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| 渲染方式 | 独立 Shape pass + SDF shader | 质量最好，支持抗锯齿/描边/圆角 |
| 图元类型 | 矩形/圆形/椭圆/圆角矩形/线条 | 覆盖常用 2D 图形需求 |
| 模块归属 | engine-render | 需要 wgpu shader 和管线 |
| 抗锯齿 | SDF smoothstep | GPU 端 1px 平滑过渡 |
| 描边 | SDF 距离场偏移 | 支持任意粗细描边 |
| 阴影 | 可选 drop shadow | SDF 偏移 + 模糊 |

## 架构

```
用户代码
    │ ShapePainter::draw_rect(), draw_circle(), draw_line()
    ▼
ShapeBatch
    │ 收集 Shape 命令，生成顶点 + uniform
    ▼
ShapePipeline (WGSL SDF shader)
    │ 独立 render pass，每个 Shape 一次 draw call
    ▼
屏幕上的 Shape
```

## 新增文件

```
crates/engine-render/src/shape/
├── mod.rs          # 模块入口
├── error.rs        # ShapeError
├── types.rs        # ShapeCommand, FillMode, Stroke, Color
├── batch.rs        # ShapeBatch, PreparedBatch, DrawCall
├── pipeline.rs     # ShapePipeline (WGSL shader + render pipeline)
└── painter.rs      # ShapePainter (高级 API)

crates/engine-render/src/shaders/
└── shape.wgsl      # SDF fragment shader
```

## 模块详细设计

### 1. 类型定义（shape/types.rs）

```rust
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

pub enum FillMode {
    Solid(Color),
    None,
}

pub struct Stroke {
    pub color: Color,
    pub width: f32,
}

pub enum ShapeCommand {
    Rect {
        position: [f32; 2],
        size: [f32; 2],
        fill: FillMode,
        stroke: Option<Stroke>,
        corner_radius: f32,
    },
    Circle {
        center: [f32; 2],
        radius: f32,
        fill: FillMode,
        stroke: Option<Stroke>,
    },
    Ellipse {
        center: [f32; 2],
        radii: [f32; 2],
        fill: FillMode,
        stroke: Option<Stroke>,
    },
    RoundedRectangle {
        position: [f32; 2],
        size: [f32; 2],
        corner_radius: [f32; 4],
        fill: FillMode,
        stroke: Option<Stroke>,
    },
    Line {
        start: [f32; 2],
        end: [f32; 2],
        color: Color,
        width: f32,
    },
}
```

### 2. SDF Shader（shaders/shape.wgsl）

SDF 函数：
- `sdBox(p, b)` — 矩形
- `sdCircle(p, r)` — 圆形
- `sdEllipse(p, ab)` — 椭圆
- `sdRoundedBox(p, b, r)` — 圆角矩形
- `sdSegment(p, a, b)` — 线段

Uniform 结构：
```wgsl
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
```

Fragment shader 逻辑：
1. 根据 shape_type 调用对应 SDF 函数得到距离 d
2. 填充：`fill_alpha = 1.0 - smoothstep(-0.5, 0.5, d)`
3. 描边：`stroke_d = abs(d) - stroke_width`，`stroke_alpha = 1.0 - smoothstep(-0.5, 0.5, stroke_d)`
4. 混合填充和描边颜色

抗锯齿：smoothstep 在 SDF 边缘做 1px 平滑过渡。

### 3. ShapeBatch（shape/batch.rs）

```rust
pub struct ShapeBatch {
    commands: Vec<ShapeCommand>,
}

impl ShapeBatch {
    pub fn new() -> Self;
    pub fn push(&mut self, cmd: ShapeCommand);
    pub fn clear(&mut self);
    pub fn prepare(&self, device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout) -> PreparedBatch;
}

pub struct PreparedBatch {
    pub draw_calls: Vec<DrawCall>,
}

pub struct DrawCall {
    pub vertex_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub vertex_count: u32,
}
```

每个 Shape 生成一个 DrawCall（4 个顶点 + 1 个 uniform buffer）。

### 4. ShapePipeline（shape/pipeline.rs）

```rust
pub struct ShapePipeline {
    render_pipeline: wgpu::RenderPipeline,
    uniform_layout: wgpu::BindGroupLayout,
}

impl ShapePipeline {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self;
    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, prepared: &'a PreparedBatch);
}
```

管线配置：
- Vertex shader：全屏四边形（4 顶点）
- Fragment shader：SDF 计算
- Blend mode：Alpha blending
- Primitive topology：TriangleStrip

### 5. ShapePainter（shape/painter.rs）

```rust
pub struct ShapePainter {
    batch: ShapeBatch,
}

impl ShapePainter {
    pub fn new() -> Self;
    
    pub fn rect(&mut self, position: [f32; 2], size: [f32; 2], color: Color);
    pub fn rect_stroked(&mut self, position: [f32; 2], size: [f32; 2], fill: Color, stroke: Color, stroke_width: f32);
    pub fn circle(&mut self, center: [f32; 2], radius: f32, color: Color);
    pub fn circle_stroked(&mut self, center: [f32; 2], radius: f32, fill: Color, stroke: Color, stroke_width: f32);
    pub fn ellipse(&mut self, center: [f32; 2], radii: [f32; 2], color: Color);
    pub fn rounded_rect(&mut self, position: [f32; 2], size: [f32; 2], corner_radius: f32, color: Color);
    pub fn rounded_rect_stroked(&mut self, position: [f32; 2], size: [f32; 2], corner_radius: [f32; 4], fill: Color, stroke: Color, stroke_width: f32);
    pub fn line(&mut self, start: [f32; 2], end: [f32; 2], color: Color, width: f32);
    
    pub fn flush(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, pipeline: &ShapePipeline, render_pass: &mut wgpu::RenderPass);
    pub fn clear(&mut self);
}
```

### 6. 集成

在 `RenderPlugin2D::build()` 中：
```rust
let shape_pipeline = ShapePipeline::new(&renderer.device, surface_format);
let shape_painter = ShapePainter::new();
world.insert_resource(shape_pipeline);
world.insert_resource(shape_painter);
```

渲染顺序：Shape 在 Sprite 之前渲染（背景层），depth = 0.0。

## 依赖

无新依赖。使用 wgpu 原生 shader 能力。

## 测试策略

1. **类型测试**：ShapeCommand 构造、Color 转换
2. **SDF 函数测试**：在 shader 中验证 SDF 距离计算
3. **Batch 测试**：push 命令后 prepare 生成正确数量的 DrawCall
4. **渲染验证**：绘制基本形状，截图验证

## 性能考量

- 每个 Shape 一次 draw call，100 个 Shape = 100 次 draw call
- Uniform buffer 每帧重建（CPU → GPU）
- 后续可用 instancing 优化

## 后续优化（不在本次范围）

- Instancing 批量绘制
- 渐变填充
- 贝塞尔曲线
- 文本 SDF 渲染
