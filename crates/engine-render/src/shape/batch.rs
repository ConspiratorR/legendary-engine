use super::types::{FillMode, ShapeCommand};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

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

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ShapeVertex {
    pub position: [f32; 2],
}

pub struct DrawCall {
    pub vertex_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

pub struct PreparedBatch {
    pub draw_calls: Vec<DrawCall>,
}

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

    pub fn prepare(
        &self,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        uniform_layout: &wgpu::BindGroupLayout,
    ) -> PreparedBatch {
        let mut draw_calls = Vec::with_capacity(self.commands.len());
        for cmd in &self.commands {
            let (uniform, bbox) = self.cmd_to_uniform(cmd);
            let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("shape_uniform"),
                contents: bytemuck::bytes_of(&uniform),
                usage: wgpu::BufferUsages::UNIFORM,
            });
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

    fn cmd_to_uniform(&self, cmd: &ShapeCommand) -> (ShapeUniform, (f32, f32, f32, f32)) {
        match cmd {
            ShapeCommand::Rect {
                position,
                size,
                fill,
                stroke,
                corner_radius,
            } => {
                let color = match fill {
                    FillMode::Solid(c) => c.to_array(),
                    FillMode::None => [0.0; 4],
                };
                let stroke_color = stroke
                    .as_ref()
                    .map(|s| s.color.to_array())
                    .unwrap_or([0.0; 4]);
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
                (
                    uniform,
                    (
                        position[0],
                        position[1],
                        position[0] + size[0],
                        position[1] + size[1],
                    ),
                )
            }
            ShapeCommand::Circle {
                center,
                radius,
                fill,
                stroke,
            } => {
                let color = match fill {
                    FillMode::Solid(c) => c.to_array(),
                    FillMode::None => [0.0; 4],
                };
                let stroke_color = stroke
                    .as_ref()
                    .map(|s| s.color.to_array())
                    .unwrap_or([0.0; 4]);
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
                (
                    uniform,
                    (
                        center[0] - radius,
                        center[1] - radius,
                        center[0] + radius,
                        center[1] + radius,
                    ),
                )
            }
            ShapeCommand::Ellipse {
                center,
                radii,
                fill,
                stroke,
            } => {
                let color = match fill {
                    FillMode::Solid(c) => c.to_array(),
                    FillMode::None => [0.0; 4],
                };
                let stroke_color = stroke
                    .as_ref()
                    .map(|s| s.color.to_array())
                    .unwrap_or([0.0; 4]);
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
                (
                    uniform,
                    (
                        center[0] - radii[0],
                        center[1] - radii[1],
                        center[0] + radii[0],
                        center[1] + radii[1],
                    ),
                )
            }
            ShapeCommand::RoundedRectangle {
                position,
                size,
                corner_radius,
                fill,
                stroke,
            } => {
                let color = match fill {
                    FillMode::Solid(c) => c.to_array(),
                    FillMode::None => [0.0; 4],
                };
                let stroke_color = stroke
                    .as_ref()
                    .map(|s| s.color.to_array())
                    .unwrap_or([0.0; 4]);
                let stroke_width = stroke.as_ref().map(|s| s.width).unwrap_or(0.0);
                let max_r = corner_radius.iter().copied().fold(0.0f32, f32::max);
                let cx = position[0] + size[0] * 0.5;
                let cy = position[1] + size[1] * 0.5;
                let uniform = ShapeUniform {
                    transform: Self::ortho_transform(cx, cy, size[0], size[1]),
                    size: *size,
                    _pad0: [0.0; 2],
                    color,
                    stroke_color,
                    stroke_width,
                    corner_radius: max_r,
                    shape_type: 3,
                    _padding: 0,
                };
                (
                    uniform,
                    (
                        position[0],
                        position[1],
                        position[0] + size[0],
                        position[1] + size[1],
                    ),
                )
            }
            ShapeCommand::Line {
                start,
                end,
                color,
                width,
            } => {
                let dx = end[0] - start[0];
                let dy = end[1] - start[1];
                let len = (dx * dx + dy * dy).sqrt();
                let cx = (start[0] + end[0]) * 0.5;
                let cy = (start[1] + end[1]) * 0.5;
                let angle = dy.atan2(dx);
                let cos_a = angle.cos();
                let sin_a = angle.sin();
                let transform = [
                    [cos_a * len, sin_a * len, 0.0, 0.0],
                    [-sin_a * width, cos_a * width, 0.0, 0.0],
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
                (
                    uniform,
                    (
                        start[0].min(end[0]) - hw,
                        start[1].min(end[1]) - hw,
                        start[0].max(end[0]) + hw,
                        start[1].max(end[1]) + hw,
                    ),
                )
            }
        }
    }

    fn ortho_transform(cx: f32, cy: f32, w: f32, h: f32) -> [[f32; 4]; 4] {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shape::types::Color;

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
