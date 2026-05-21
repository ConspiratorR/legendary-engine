use std::ops::Deref;
use std::sync::Arc;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use crate::graph::{RenderGraph, pass, execute};
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
    pub sprite_pipeline: Arc<SpritePipeline>,
    camera_uniform: wgpu::Buffer,
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

        Self {
            device: GpuDevice(Arc::new(device)),
            queue: GpuQueue(Arc::new(queue)),
            surface,
            config,
            graph: RenderGraph::new(),
            sprite_pipeline,
            camera_uniform,
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
        let swapchain = self.graph.import_texture_view("swapchain", view);

        let sprite_pipeline = self.sprite_pipeline.clone();
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
                ctx.pass.draw(0..6, 0..1);
            }),
        });

        let compiled = self.graph.compile(&self.device).unwrap();
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("main_encoder"),
        });

        // Upload camera uniform
        let matrix_data = camera_matrix.to_cols_array();
        self.queue.write_buffer(&self.camera_uniform, 0, bytemuck::cast_slice(&matrix_data));

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
