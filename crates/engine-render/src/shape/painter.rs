use super::batch::ShapeBatch;
use super::pipeline::ShapePipeline;
use super::types::{Color, FillMode, ShapeCommand, Stroke};

pub struct ShapePainter {
    batch: ShapeBatch,
}

impl ShapePainter {
    pub fn new() -> Self {
        Self {
            batch: ShapeBatch::new(),
        }
    }

    pub fn rect(&mut self, position: [f32; 2], size: [f32; 2], color: Color) {
        self.batch.push(ShapeCommand::Rect {
            position,
            size,
            fill: FillMode::Solid(color),
            stroke: None,
            corner_radius: 0.0,
        });
    }

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

    pub fn circle(&mut self, center: [f32; 2], radius: f32, color: Color) {
        self.batch.push(ShapeCommand::Circle {
            center,
            radius,
            fill: FillMode::Solid(color),
            stroke: None,
        });
    }

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

    pub fn ellipse(&mut self, center: [f32; 2], radii: [f32; 2], color: Color) {
        self.batch.push(ShapeCommand::Ellipse {
            center,
            radii,
            fill: FillMode::Solid(color),
            stroke: None,
        });
    }

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

    pub fn line(&mut self, start: [f32; 2], end: [f32; 2], color: Color, width: f32) {
        self.batch.push(ShapeCommand::Line {
            start,
            end,
            color,
            width,
        });
    }

    pub fn flush<'a>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pipeline: &'a ShapePipeline,
        render_pass: &mut wgpu::RenderPass<'a>,
    ) {
        if self.batch.is_empty() {
            return;
        }
        let prepared = self.batch.prepare(device, queue, &pipeline.uniform_layout);
        render_pass.set_pipeline(&pipeline.render_pipeline);
        for dc in &prepared.draw_calls {
            render_pass.set_bind_group(0, &dc.bind_group, &[]);
            render_pass.set_vertex_buffer(0, dc.vertex_buffer.slice(..));
            render_pass.draw(0..4, 0..1);
        }
        self.batch.clear();
    }

    pub fn clear(&mut self) {
        self.batch.clear();
    }

    pub fn pending_count(&self) -> usize {
        self.batch.len()
    }
}

impl Default for ShapePainter {
    fn default() -> Self {
        Self::new()
    }
}
