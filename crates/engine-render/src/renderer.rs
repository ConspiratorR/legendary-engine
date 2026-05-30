use crate::pipeline::sprite::SpritePipeline;
use crate::sprite_renderer::SpriteRenderer;
use engine_math::Vec3;
use std::ops::Deref;
use std::sync::Arc;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};

const CAMERA_UNIFORM_SIZE: u64 = 64;
const DEFAULT_SPRITE_CAPACITY: usize = 10000;

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
    pub graph: crate::graph::RenderGraph,
    pub sprite_pipeline: Arc<SpritePipeline>,
    camera_uniform: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    pub sprite_renderer: SpriteRenderer,
}

impl Renderer {
    pub fn new(
        window: std::sync::Arc<winit::window::Window>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or("Failed to find suitable adapter")?;
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .map_err(|e| format!("Failed to create device: {}", e))?;
        let size = window.inner_size();
        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .ok_or("Failed to get surface config")?;
        surface.configure(&device, &config);

        let sprite_pipeline = Arc::new(SpritePipeline::new(&device, config.format));

        let camera_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera_uniform"),
            size: CAMERA_UNIFORM_SIZE,
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

        let sprite_renderer =
            SpriteRenderer::new(&device, sprite_pipeline.clone(), DEFAULT_SPRITE_CAPACITY);

        Ok(Self {
            device: GpuDevice(Arc::new(device)),
            queue: GpuQueue(Arc::new(queue)),
            surface,
            config,
            graph: crate::graph::RenderGraph::new(),
            sprite_pipeline,
            camera_uniform,
            camera_bind_group,
            sprite_renderer,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn render_frame(
        &mut self,
        cameras: &[&crate::camera::Camera],
        all_sprites: &[crate::sprite::Sprite],
        bridge: &mut crate::texture_bridge::TextureBridge,
        registry: &engine_asset::registry::Registry,
    ) -> Result<(), wgpu::SurfaceError> {
        use crate::camera::{Camera, RenderTarget};
        use crate::frustum::Frustum;
        use crate::sprite::SpriteDraw;

        let mut sorted: Vec<&Camera> = cameras.to_vec();
        sorted.sort_by_key(|c| c.priority);

        bridge.auto_sync(registry);
        bridge.flush(&self.device, &self.queue);

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

            for batch in &mut batches {
                batch.update_indirect_cmd();
            }

            let mut vertex_offset: usize = 0;
            let mut instance_offset: usize = 0;
            let mut indirect_offset: usize = 0;

            for batch in &batches {
                self.sprite_renderer.upload_batch(
                    &self.queue,
                    batch,
                    vertex_offset,
                    instance_offset,
                    indirect_offset,
                );
                vertex_offset += batch.vertices.len()
                    * std::mem::size_of::<crate::pipeline::sprite::SpriteVertex>();
                instance_offset +=
                    batch.instance_data.len() * std::mem::size_of::<engine_math::Mat4>();
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
                    .ok_or(wgpu::SurfaceError::Lost)?,
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

                pass.set_vertex_buffer(0, buffers.vertex_buffer.slice(..));
                pass.set_vertex_buffer(1, buffers.instance_buffer.slice(..));
                pass.set_index_buffer(buffers.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

                let mut indirect_offset: u64 = 0;
                for batch in &batches {
                    let bind_group = bridge.texture_store().get_bind_group(batch.texture_id);
                    pass.set_bind_group(1, bind_group, &[]);
                    pass.draw_indexed_indirect(buffers.indirect_buffer, indirect_offset);
                    indirect_offset +=
                        std::mem::size_of::<crate::indirect::DrawIndexedIndirectArgs>() as u64;
                }
            }
        }

        self.queue.submit([encoder.finish()]);
        output.present();
        Ok(())
    }
}
