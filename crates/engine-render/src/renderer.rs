use crate::graph::RenderGraph;
use crate::graph::pass::{self, RenderPassDesc};
use crate::pipeline::sprite::SpritePipeline;
use crate::sprite::SpriteBatch;
use crate::texture_store::TextureStore;
use engine_math::Mat4;
use std::ops::Deref;
use std::sync::Arc;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};

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
    pub texture_store: TextureStore,
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

        let texture_store =
            TextureStore::new(&device, &queue, &sprite_pipeline.texture_bind_group_layout);

        Self {
            device: GpuDevice(Arc::new(device)),
            queue: GpuQueue(Arc::new(queue)),
            surface,
            config,
            graph: RenderGraph::new(),
            sprite_pipeline,
            texture_store,
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
    ) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Upload camera uniform
        let matrix_data = camera_matrix.to_cols_array();
        self.queue
            .write_buffer(&self.camera_uniform, 0, bytemuck::cast_slice(&matrix_data));

        // Reset graph and import swapchain view
        self.graph.reset();
        let swapchain = self.graph.import_texture_view("swapchain", view);

        // Borrow data for the closure — no cloning needed since the closure
        // lives only as long as this function scope.
        let camera_bg = &self.camera_bind_group;
        let pipeline = &self.sprite_pipeline.pipeline;
        let batch_refs: Vec<(
            &wgpu::BindGroup,
            &Option<wgpu::Buffer>,
            &Option<wgpu::Buffer>,
            u32,
        )> = sprite_batches
            .iter()
            .map(|b| {
                let bg = self.texture_store.get_bind_group(b.texture_id);
                (bg, &b.vertex_buffer, &b.index_buffer, b.index_count)
            })
            .collect();

        let sprite_closure: pass::ExecuteFn<'_> = self.graph.add_render_pass(RenderPassDesc {
            label: Some("sprite_pass".to_string()),
            color_attachments: vec![pass::ColorAttachment {
                resource: swapchain,
                load_op: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store_op: wgpu::StoreOp::Store,
            }],
            depth_stencil_attachment: None,
            execute: Box::new(move |ctx| {
                ctx.pass.set_pipeline(pipeline);
                ctx.pass.set_bind_group(0, camera_bg, &[]);
                for (bind_group, vb, ib, index_count) in &batch_refs {
                    ctx.pass.set_bind_group(1, *bind_group, &[]);
                    if let (Some(vb), Some(ib)) = (vb, ib) {
                        ctx.pass.set_vertex_buffer(0, vb.slice(..));
                        ctx.pass
                            .set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint16);
                        ctx.pass.draw_indexed(0..*index_count, 0, 0..1);
                    }
                }
            }),
        });

        // Compile and execute graph
        let compiled = self.graph.compile(&self.device).unwrap();
        let mut closures: Vec<pass::ExecuteFn<'_>> = vec![sprite_closure];
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("main_encoder"),
            });
        {
            let mut ctx = crate::graph::execute::ExecuteContext {
                device: &self.device,
                queue: &self.queue,
                encoder: &mut encoder,
            };
            self.graph.execute(&compiled, &mut closures, &mut ctx)?;
        }

        self.queue.submit([encoder.finish()]);
        output.present();
        Ok(())
    }
}
