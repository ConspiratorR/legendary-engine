//! 3D line rendering for editor overlays (grid, gizmos, selection highlights).
//!
//! Accumulates line segments with per-vertex color and renders them as `LineList`
//! topology using a simple camera-transformed shader.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LineVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

impl LineVertex {
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
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 12,
                    shader_location: 1,
                },
            ],
        }
    }
}

/// Camera uniform for the 3D line shader.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LineCameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

/// Render pipeline for 3D lines.
pub struct Line3dPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
}

impl Line3dPipeline {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("line3d_camera_bgl"),
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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("line3d_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/line3d.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("line3d_pipeline_layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("line3d_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[LineVertex::desc()],
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
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            camera_bind_group_layout,
        }
    }
}

/// Accumulates 3D line segments for batched rendering.
pub struct Line3dBatch {
    vertices: Vec<LineVertex>,
}

impl Line3dBatch {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
        }
    }

    /// Add a single line segment.
    pub fn line(&mut self, a: [f32; 3], b: [f32; 3], color: [f32; 4]) {
        self.vertices.push(LineVertex {
            position: a,
            color,
        });
        self.vertices.push(LineVertex {
            position: b,
            color,
        });
    }

    /// Add an axis-aligned box wireframe.
    pub fn aabb(&mut self, min: [f32; 3], max: [f32; 3], color: [f32; 4]) {
        let [x0, y0, z0] = min;
        let [x1, y1, z1] = max;
        // Bottom face
        self.line([x0, y0, z0], [x1, y0, z0], color);
        self.line([x1, y0, z0], [x1, y0, z1], color);
        self.line([x1, y0, z1], [x0, y0, z1], color);
        self.line([x0, y0, z1], [x0, y0, z0], color);
        // Top face
        self.line([x0, y1, z0], [x1, y1, z0], color);
        self.line([x1, y1, z0], [x1, y1, z1], color);
        self.line([x1, y1, z1], [x0, y1, z1], color);
        self.line([x0, y1, z1], [x0, y1, z0], color);
        // Vertical edges
        self.line([x0, y0, z0], [x0, y1, z0], color);
        self.line([x1, y0, z0], [x1, y1, z0], color);
        self.line([x1, y0, z1], [x1, y1, z1], color);
        self.line([x0, y0, z1], [x0, y1, z1], color);
    }

    /// Add a 3D grid on the XZ plane.
    pub fn grid_xz(&mut self, center: [f32; 3], size: f32, divisions: i32, color: [f32; 4]) {
        let half = size * 0.5;
        let step = size / divisions as f32;
        let faded = [color[0], color[1], color[2], color[3] * 0.3];

        for i in -divisions..=divisions {
            let offset = i as f32 * step;
            let c = if i == 0 { color } else { faded };
            // Lines along Z
            self.line(
                [center[0] + offset, center[1], center[2] - half],
                [center[0] + offset, center[1], center[2] + half],
                c,
            );
            // Lines along X
            self.line(
                [center[0] - half, center[1], center[2] + offset],
                [center[0] + half, center[1], center[2] + offset],
                c,
            );
        }
    }

    /// Add a translation gizmo (three axis arrows) at a position.
    pub fn translate_gizmo(&mut self, pos: [f32; 3], scale: f32) {
        let len = scale;
        let head = scale * 0.15;
        // X axis (red)
        self.line(pos, [pos[0] + len, pos[1], pos[2]], [1.0, 0.2, 0.2, 1.0]);
        self.line(
            [pos[0] + len, pos[1], pos[2]],
            [pos[0] + len - head, pos[1] + head * 0.4, pos[2]],
            [1.0, 0.2, 0.2, 1.0],
        );
        self.line(
            [pos[0] + len, pos[1], pos[2]],
            [pos[0] + len - head, pos[1] - head * 0.4, pos[2]],
            [1.0, 0.2, 0.2, 1.0],
        );
        // Y axis (green)
        self.line(pos, [pos[0], pos[1] + len, pos[2]], [0.2, 1.0, 0.2, 1.0]);
        self.line(
            [pos[0], pos[1] + len, pos[2]],
            [pos[0] + head * 0.4, pos[1] + len - head, pos[2]],
            [0.2, 1.0, 0.2, 1.0],
        );
        self.line(
            [pos[0], pos[1] + len, pos[2]],
            [pos[0] - head * 0.4, pos[1] + len - head, pos[2]],
            [0.2, 1.0, 0.2, 1.0],
        );
        // Z axis (blue)
        self.line(pos, [pos[0], pos[1], pos[2] + len], [0.3, 0.5, 1.0, 1.0]);
        self.line(
            [pos[0], pos[1], pos[2] + len],
            [pos[0], pos[1] + head * 0.4, pos[2] + len - head],
            [0.3, 0.5, 1.0, 1.0],
        );
        self.line(
            [pos[0], pos[1], pos[2] + len],
            [pos[0], pos[1] - head * 0.4, pos[2] + len - head],
            [0.3, 0.5, 1.0, 1.0],
        );
    }

    /// Add a rotation gizmo (three axis circles) at a position.
    pub fn rotate_gizmo(&mut self, pos: [f32; 3], radius: f32) {
        let segments = 32;
        for a in 0..segments {
            let t0 = a as f32 / segments as f32 * std::f32::consts::TAU;
            let t1 = (a + 1) as f32 / segments as f32 * std::f32::consts::TAU;
            // X circle (YZ plane)
            self.line(
                [
                    pos[0],
                    pos[1] + t0.cos() * radius,
                    pos[2] + t0.sin() * radius,
                ],
                [
                    pos[0],
                    pos[1] + t1.cos() * radius,
                    pos[2] + t1.sin() * radius,
                ],
                [1.0, 0.2, 0.2, 1.0],
            );
            // Y circle (XZ plane)
            self.line(
                [
                    pos[0] + t0.cos() * radius,
                    pos[1],
                    pos[2] + t0.sin() * radius,
                ],
                [
                    pos[0] + t1.cos() * radius,
                    pos[1],
                    pos[2] + t1.sin() * radius,
                ],
                [0.2, 1.0, 0.2, 1.0],
            );
            // Z circle (XY plane)
            self.line(
                [
                    pos[0] + t0.cos() * radius,
                    pos[1] + t0.sin() * radius,
                    pos[2],
                ],
                [
                    pos[0] + t1.cos() * radius,
                    pos[1] + t1.sin() * radius,
                    pos[2],
                ],
                [0.3, 0.5, 1.0, 1.0],
            );
        }
    }

    /// Add a scale gizmo (three axis lines with end cubes) at a position.
    pub fn scale_gizmo(&mut self, pos: [f32; 3], scale: f32) {
        let len = scale;
        let cube = scale * 0.08;
        // X axis
        self.line(pos, [pos[0] + len, pos[1], pos[2]], [1.0, 0.2, 0.2, 1.0]);
        self.aabb(
            [pos[0] + len - cube, pos[1] - cube, pos[2] - cube],
            [pos[0] + len + cube, pos[1] + cube, pos[2] + cube],
            [1.0, 0.2, 0.2, 1.0],
        );
        // Y axis
        self.line(pos, [pos[0], pos[1] + len, pos[2]], [0.2, 1.0, 0.2, 1.0]);
        self.aabb(
            [pos[0] - cube, pos[1] + len - cube, pos[2] - cube],
            [pos[0] + cube, pos[1] + len + cube, pos[2] + cube],
            [0.2, 1.0, 0.2, 1.0],
        );
        // Z axis
        self.line(pos, [pos[0], pos[1], pos[2] + len], [0.3, 0.5, 1.0, 1.0]);
        self.aabb(
            [pos[0] - cube, pos[1] - cube, pos[2] + len - cube],
            [pos[0] + cube, pos[1] + cube, pos[2] + len + cube],
            [0.3, 0.5, 1.0, 1.0],
        );
        // Center cube (white)
        self.aabb(
            [pos[0] - cube, pos[1] - cube, pos[2] - cube],
            [pos[0] + cube, pos[1] + cube, pos[2] + cube],
            [1.0, 1.0, 1.0, 1.0],
        );
    }

    /// Add a selection highlight wireframe sphere approximation (icosphere-like).
    pub fn selection_sphere(&mut self, center: [f32; 3], radius: f32, color: [f32; 4]) {
        let segments = 16;
        // XY circle
        for i in 0..segments {
            let t0 = i as f32 / segments as f32 * std::f32::consts::TAU;
            let t1 = (i + 1) as f32 / segments as f32 * std::f32::consts::TAU;
            self.line(
                [
                    center[0] + t0.cos() * radius,
                    center[1] + t0.sin() * radius,
                    center[2],
                ],
                [
                    center[0] + t1.cos() * radius,
                    center[1] + t1.sin() * radius,
                    center[2],
                ],
                color,
            );
        }
        // XZ circle
        for i in 0..segments {
            let t0 = i as f32 / segments as f32 * std::f32::consts::TAU;
            let t1 = (i + 1) as f32 / segments as f32 * std::f32::consts::TAU;
            self.line(
                [
                    center[0] + t0.cos() * radius,
                    center[1],
                    center[2] + t0.sin() * radius,
                ],
                [
                    center[0] + t1.cos() * radius,
                    center[1],
                    center[2] + t1.sin() * radius,
                ],
                color,
            );
        }
        // YZ circle
        for i in 0..segments {
            let t0 = i as f32 / segments as f32 * std::f32::consts::TAU;
            let t1 = (i + 1) as f32 / segments as f32 * std::f32::consts::TAU;
            self.line(
                [
                    center[0],
                    center[1] + t0.cos() * radius,
                    center[2] + t0.sin() * radius,
                ],
                [
                    center[0],
                    center[1] + t1.cos() * radius,
                    center[2] + t1.sin() * radius,
                ],
                color,
            );
        }
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    pub fn vertex_count(&self) -> u32 {
        self.vertices.len() as u32
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
    }

    /// Upload vertices to GPU and render all lines into the given render pass.
    pub fn render<'a>(
        &self,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        pipeline: &'a Line3dPipeline,
        camera_bind_group: &wgpu::BindGroup,
        pass: &mut wgpu::RenderPass<'a>,
    ) {
        if self.vertices.is_empty() {
            return;
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("line3d_vertex_buffer"),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        pass.set_pipeline(&pipeline.pipeline);
        pass.set_bind_group(0, camera_bind_group, &[]);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.draw(0..self.vertex_count(), 0..1);
    }
}

impl Default for Line3dBatch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_batch_push() {
        let mut batch = Line3dBatch::new();
        assert!(batch.is_empty());
        batch.line([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(batch.vertex_count(), 2);
    }

    #[test]
    fn test_aabb_vertex_count() {
        let mut batch = Line3dBatch::new();
        batch.aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], [1.0; 4]);
        // 12 edges * 2 vertices = 24
        assert_eq!(batch.vertex_count(), 24);
    }

    #[test]
    fn test_grid_vertex_count() {
        let mut batch = Line3dBatch::new();
        batch.grid_xz([0.0, 0.0, 0.0], 10.0, 2, [1.0; 4]);
        // (2*2+1) lines * 2 directions * 2 vertices = 5*2*2 = 20
        assert_eq!(batch.vertex_count(), 20);
    }

    #[test]
    fn test_translate_gizmo_vertex_count() {
        let mut batch = Line3dBatch::new();
        batch.translate_gizmo([0.0, 0.0, 0.0], 1.0);
        // 3 axes * (1 main + 2 arrow) = 9 lines * 2 = 18 vertices
        assert_eq!(batch.vertex_count(), 18);
    }

    #[test]
    fn test_clear() {
        let mut batch = Line3dBatch::new();
        batch.line([0.0; 3], [1.0; 3], [1.0; 4]);
        batch.clear();
        assert!(batch.is_empty());
    }
}
