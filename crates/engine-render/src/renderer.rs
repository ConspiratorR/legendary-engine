//! Main renderer implementation.
//!
//! The [`Renderer`] is the central component that owns the wgpu device, queue,
//! surface, and all render pipelines. It coordinates the rendering of both 2D
//! sprites and 3D deferred shading passes.

use crate::command_batch::{CommandBatcher, QueueSubmitBatchExt};
use crate::deferred::{DeferredPass, GBuffer, GeometryPassUniform};
use crate::instancing::InstanceBatch;
use crate::light::LightingUniform;
use crate::pipeline::pbr::CameraUniform;
use crate::pipeline::sprite::SpritePipeline;
use crate::post_process::{GBufferInputs, PostProcessChain, TonemappingConfig};
use crate::resource::material::MaterialStore;
use crate::resource::mesh::MeshStore;
use crate::shadow::{ShadowMapConfig, ShadowPass, ShadowUniform};
use crate::sprite_renderer::SpriteRenderer;
use engine_math::{Mat4, Vec3};
use rayon::prelude::*;
use std::ops::Deref;
use std::sync::Arc;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};

const CAMERA_UNIFORM_SIZE: u64 = std::mem::size_of::<CameraUniform>() as u64;
const DEFAULT_SPRITE_CAPACITY: usize = 10000;

/// Thread-safe wrapper around [`wgpu::Device`].
#[derive(Clone)]
pub struct GpuDevice(pub Arc<Device>);

impl Deref for GpuDevice {
    type Target = Device;
    fn deref(&self) -> &Device {
        &self.0
    }
}

/// Thread-safe wrapper around [`wgpu::Queue`].
#[derive(Clone)]
pub struct GpuQueue(pub Arc<Queue>);

impl Deref for GpuQueue {
    type Target = Queue;
    fn deref(&self) -> &Queue {
        &self.0
    }
}

/// Input data for the 3D deferred rendering path.
///
/// Contains all resources needed to render a 3D scene through the deferred
/// pipeline: meshes, materials, lighting, camera, and visible geometry batches.
pub struct Scene3d<'a> {
    pub mesh_store: &'a MeshStore,
    pub material_store: &'a MaterialStore,
    pub lighting_uniform: &'a LightingUniform,
    pub camera_vp: &'a Mat4,
    pub camera_pos: &'a [f32; 3],
    pub light_direction: &'a [f32; 3],
    pub batches: &'a [InstanceBatch],
    pub scene_aabb_min: Vec3,
    pub scene_aabb_max: Vec3,
    pub shadow_config: ShadowMapConfig,
}

/// Main renderer owning the wgpu device, queue, surface, and render pipelines.
///
/// Created via [`Renderer::new`] with a window handle. Manages the render graph,
/// sprite pipeline, camera uniforms, and post-processing chain.
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
    pub post_process: PostProcessChain,
    // 3D deferred rendering resources (lazy-initialized on first use)
    deferred_pass: Option<DeferredPass>,
    gbuffer: Option<GBuffer>,
    shadow_pass: Option<ShadowPass>,
    deferred_camera_uniform: Option<wgpu::Buffer>,
    deferred_camera_bind_group: Option<wgpu::BindGroup>,
    light_uniform_buffer: Option<wgpu::Buffer>,
    light_bind_group: Option<wgpu::BindGroup>,
    shadow_uniform_buffer: Option<wgpu::Buffer>,
    shadow_lighting_bind_group: Option<wgpu::BindGroup>,
    viewport_post_process: Option<PostProcessChain>,
}

impl Renderer {
    /// Create a new renderer for the given window (native only — uses blocking calls).
    ///
    /// Initializes the wgpu adapter, device, queue, and surface. Returns an
    /// error if no suitable GPU adapter is found.
    #[cfg(not(target_arch = "wasm32"))]
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
                required_features: wgpu::Features::PUSH_CONSTANTS,
                required_limits: wgpu::Limits {
                    max_push_constant_size: 128,
                    ..wgpu::Limits::default()
                },
                label: None,
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .map_err(|e| format!("Failed to create device: {}", e))?;
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width, size.height)
            .ok_or("Failed to get surface config")?;
        config.present_mode = wgpu::PresentMode::Fifo;
        surface.configure(&device, &config);

        // Sprite pipeline renders to HDR framebuffer (Rgba16Float), not swapchain
        let sprite_pipeline = Arc::new(SpritePipeline::new(
            &device,
            wgpu::TextureFormat::Rgba16Float,
        ));

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

        let post_process = PostProcessChain::new_minimal(
            &device,
            &queue,
            config.width,
            config.height,
            config.format,
        );

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
            post_process,
            deferred_pass: None,
            gbuffer: None,
            shadow_pass: None,
            deferred_camera_uniform: None,
            deferred_camera_bind_group: None,
            light_uniform_buffer: None,
            light_bind_group: None,
            shadow_uniform_buffer: None,
            shadow_lighting_bind_group: None,
            viewport_post_process: None,
        })
    }

    /// Create a new renderer asynchronously (for WASM targets).
    ///
    /// WASM cannot use `pollster::block_on`, so this version uses `await`.
    #[cfg(target_arch = "wasm32")]
    pub async fn new_async(
        window: std::sync::Arc<winit::window::Window>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("Failed to find suitable adapter")?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::PUSH_CONSTANTS,
                    required_limits: wgpu::Limits {
                        max_push_constant_size: 128,
                        ..wgpu::Limits::default()
                    },
                    label: None,
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(|e| format!("Failed to create device: {}", e))?;
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width, size.height)
            .ok_or("Failed to get surface config")?;
        config.present_mode = wgpu::PresentMode::Fifo;
        surface.configure(&device, &config);

        let sprite_pipeline = Arc::new(crate::pipeline::sprite::SpritePipeline::new(
            &device,
            wgpu::TextureFormat::Rgba16Float,
        ));

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

        let sprite_renderer = crate::sprite_renderer::SpriteRenderer::new(
            &device,
            sprite_pipeline.clone(),
            DEFAULT_SPRITE_CAPACITY,
        );

        let post_process = crate::post_process::PostProcessChain::new_minimal(
            &device,
            &queue,
            config.width,
            config.height,
            config.format,
        );

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
            post_process,
            deferred_pass: None,
            gbuffer: None,
            shadow_pass: None,
            deferred_camera_uniform: None,
            deferred_camera_bind_group: None,
            light_uniform_buffer: None,
            light_bind_group: None,
            shadow_uniform_buffer: None,
            shadow_lighting_bind_group: None,
            viewport_post_process: None,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.post_process.resize(&self.device, width, height);
        if let Some(ref mut gb) = self.gbuffer {
            gb.resize(&self.device, width, height);
        }
    }

    /// Update tone mapping settings.
    pub fn set_tonemapping(&mut self, queue: &wgpu::Queue, config: TonemappingConfig) {
        self.post_process.set_tonemapping(queue, config);
    }

    pub fn render_frame(
        &mut self,
        cameras: &[&crate::camera::Camera],
        all_sprites: &[crate::sprite::Sprite],
        bridge: &mut crate::texture_bridge::TextureBridge,
        registry: &engine_asset::registry::Registry,
    ) -> Result<(), wgpu::SurfaceError> {
        use crate::sprite::SpriteDraw;

        let mut sorted: Vec<&crate::camera::Camera> = cameras.to_vec();
        sorted.sort_by_key(|c| c.priority);

        bridge.auto_sync(&self.device, &self.queue, registry);
        bridge.flush(&self.device, &self.queue);

        let sprite_draws: Vec<SpriteDraw> = all_sprites
            .iter()
            .map(|s| {
                let pos = s.transform.transform_point3(engine_math::Vec3::ZERO);
                SpriteDraw {
                    world_matrix: s.transform,
                    color: s.color,
                    size: s.size,
                    texture_id: bridge.resolve(&s.texture),
                    flip_x: s.flip_x,
                    flip_y: s.flip_y,
                    depth: pos.z,
                    uv_region: s.uv_region,
                }
            })
            .collect();

        let output = self.surface.get_current_texture()?;
        let swapchain_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let surface_width = self.config.width;
        let surface_height = self.config.height;

        self.sprite_renderer.begin_frame();

        // 将精灵缓冲区注册到 Render Graph
        self.graph.reset();
        let buffers = self.sprite_renderer.get_buffers();
        let _vertex_handle = self
            .graph
            .import_buffer_ref("sprite_vertex", buffers.vertex_buffer);
        let _index_handle = self
            .graph
            .import_buffer_ref("sprite_index", buffers.index_buffer);
        let _instance_handle = self
            .graph
            .import_buffer_ref("sprite_instance", buffers.instance_buffer);
        let _indirect_handle = self
            .graph
            .import_buffer_ref("sprite_indirect", buffers.indirect_buffer);

        let active_cameras: Vec<&crate::camera::Camera> =
            sorted.iter().filter(|c| c.is_active).copied().collect();

        // Extract buffer references before parallel section
        // SAFETY: These are just pointers to GPU buffers that outlive this function.
        // We extract them to avoid borrowing self.graph during parallel recording.
        let graph_buffers = self.graph.get_buffers();
        let vb_ptr = graph_buffers
            .first()
            .copied()
            .flatten()
            .map(|b| b as *const wgpu::Buffer);
        let ib_ptr = graph_buffers
            .get(1)
            .copied()
            .flatten()
            .map(|b| b as *const wgpu::Buffer);
        let instb_ptr = graph_buffers
            .get(2)
            .copied()
            .flatten()
            .map(|b| b as *const wgpu::Buffer);
        let indb_ptr = graph_buffers
            .get(3)
            .copied()
            .flatten()
            .map(|b| b as *const wgpu::Buffer);
        drop(graph_buffers);

        // SAFETY: All pointers reference buffers owned by self.graph, which outlives
        // this function. The graph_buffers guard was dropped, but the underlying
        // Buffer objects are still alive in self.graph.
        let vb = vb_ptr.map(|p| unsafe { &*p });
        let ib = ib_ptr.map(|p| unsafe { &*p });
        let instb = instb_ptr.map(|p| unsafe { &*p });
        let indb = indb_ptr.map(|p| unsafe { &*p });

        // Extract HDR framebuffer view pointer to avoid borrow conflicts in parallel section
        // SAFETY: The HDR framebuffer outlives this function call.
        let hdr_view_ptr = std::sync::atomic::AtomicPtr::new(
            &self.post_process.hdr_framebuffer.view as *const wgpu::TextureView
                as *mut wgpu::TextureView,
        );

        if active_cameras.len() <= 1 {
            // Single camera: use sequential encoding (lower overhead)
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("main_encoder"),
                });

            for camera in &active_cameras {
                self.record_camera_pass(
                    &mut encoder,
                    camera,
                    &sprite_draws,
                    unsafe { &*hdr_view_ptr.load(std::sync::atomic::Ordering::Relaxed) },
                    bridge,
                    surface_width,
                    surface_height,
                    vb,
                    ib,
                    instb,
                    indb,
                );
            }

            // Tone mapping: HDR framebuffer → swapchain
            self.post_process.execute(
                &mut encoder,
                &swapchain_view,
                &self.device,
                &self.queue,
                None,
            );

            self.queue.submit([encoder.finish()]);
        } else {
            // Multiple cameras: use parallel command recording
            let batcher = CommandBatcher::new(&self.device, active_cameras.len());

            // Record passes in parallel
            // SAFETY: The Renderer contains RenderGraph with raw pointers that are not Sync.
            // However, record_camera_pass only accesses queue, sprite_renderer, camera_uniform,
            // camera_bind_group, and sprite_pipeline - all of which are Sync-safe. We use
            // AtomicPtr to safely share the renderer across threads.
            let renderer_ptr = std::sync::atomic::AtomicPtr::new(self as *mut Renderer);

            active_cameras
                .par_iter()
                .enumerate()
                .for_each(|(i, camera)| {
                    let mut encoder = batcher.get(i);
                    // SAFETY: Each encoder is independent. The renderer's Sync-unsafe fields
                    // (RenderGraph) are not accessed during parallel recording - only the
                    // camera_uniform buffer is written, and each camera writes to the same
                    // uniform which is safe because cameras are recorded sequentially on
                    // the GPU.
                    let renderer =
                        unsafe { &*renderer_ptr.load(std::sync::atomic::Ordering::Relaxed) };
                    renderer.record_camera_pass(
                        &mut encoder,
                        camera,
                        &sprite_draws,
                        unsafe { &*hdr_view_ptr.load(std::sync::atomic::Ordering::Relaxed) },
                        bridge,
                        surface_width,
                        surface_height,
                        vb,
                        ib,
                        instb,
                        indb,
                    );
                });

            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("post_process_encoder"),
                });
            self.post_process.execute(
                &mut encoder,
                &swapchain_view,
                &self.device,
                &self.queue,
                None,
            );

            let mut submits = batcher.finish();
            submits.push(encoder.finish());
            self.queue.submit_batch(submits);
        }

        output.present();
        Ok(())
    }

    /// Record a single camera's render pass onto the given encoder.
    ///
    /// Screen-target cameras render to the HDR framebuffer for post-processing.
    /// Texture-target cameras render directly to their assigned render target.
    #[allow(clippy::too_many_arguments)]
    fn record_camera_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        camera: &crate::camera::Camera,
        sprite_draws: &[crate::sprite::SpriteDraw],
        hdr_view: &wgpu::TextureView,
        bridge: &crate::texture_bridge::TextureBridge,
        surface_width: u32,
        surface_height: u32,
        vb: Option<&wgpu::Buffer>,
        ib: Option<&wgpu::Buffer>,
        instb: Option<&wgpu::Buffer>,
        indb: Option<&wgpu::Buffer>,
    ) {
        use crate::camera::RenderTarget;
        use crate::frustum::Frustum;

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

        // 合并上传：4 次 write_buffer 替代 4N 次
        let batch_draw_infos = self.sprite_renderer.upload_batches(&self.queue, &batches);

        let matrix_data = vp_matrix.to_cols_array();
        self.queue
            .write_buffer(&self.camera_uniform, 0, bytemuck::cast_slice(&matrix_data));

        // Screen targets render to HDR framebuffer; texture targets render directly
        let target_view = match camera.render_target {
            RenderTarget::Screen => hdr_view,
            RenderTarget::Texture(key) => bridge
                .texture_store()
                .get_render_target_view(key)
                .expect("Render target texture must be loaded before rendering"),
        };

        let camera_bg = &self.camera_bind_group;
        let pipeline = &self.sprite_pipeline.pipeline;

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

            // Use pre-extracted buffer references
            if let (Some(vb), Some(ib), Some(instb), Some(indb)) = (vb, ib, instb, indb) {
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_vertex_buffer(1, instb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint16);

                for (batch, info) in batches.iter().zip(batch_draw_infos.iter()) {
                    let bind_group = bridge.texture_store().get_bind_group(batch.texture_id);
                    pass.set_bind_group(1, bind_group, &[]);
                    pass.draw_indexed_indirect(indb, info.indirect_offset);
                }
            }
        }
    }
}

impl Renderer {
    /// Initialize deferred rendering resources (G-Buffer, deferred pass, shadow pass).
    ///
    /// Called lazily on first use of `render_frame_3d`. Safe to call multiple times;
    /// subsequent calls are no-ops.
    fn init_deferred_resources(&mut self) {
        if self.deferred_pass.is_some() {
            return;
        }

        let device = &*self.device;
        let _queue = &*self.queue;
        let w = self.config.width.max(1);
        let h = self.config.height.max(1);

        // Shadow pass (created first so its layout can be passed to deferred)
        let shadow_bind_group_layout = ShadowPass::create_bind_group_layout(device);
        let shadow_pass =
            ShadowPass::new(device, ShadowMapConfig::default(), shadow_bind_group_layout);

        // Deferred pass (geometry + lighting pipelines)
        let deferred_pass =
            DeferredPass::new(device, wgpu::TextureFormat::Rgba16Float, &shadow_pass.bind_group_layout);

        // Camera uniform for deferred path (80 bytes)
        let deferred_camera_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("deferred_camera_uniform"),
            size: CAMERA_UNIFORM_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let deferred_camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("deferred_camera_bind_group"),
            layout: &deferred_pass.camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: deferred_camera_uniform.as_entire_binding(),
            }],
        });

        // Lighting uniform buffer
        let light_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("light_uniform_buffer"),
            size: std::mem::size_of::<LightingUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("light_bind_group"),
            layout: &deferred_pass.light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_uniform_buffer.as_entire_binding(),
            }],
        });

        // G-Buffer
        let gbuffer = GBuffer::new(device, w, h);

        // Shadow uniform buffer
        let shadow_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow_uniform_buffer"),
            size: std::mem::size_of::<ShadowUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Shadow lighting bind group
        let shadow_lighting_bind_group =
            shadow_pass.create_lighting_bind_group(device, &shadow_uniform_buffer);

        self.deferred_pass = Some(deferred_pass);
        self.gbuffer = Some(gbuffer);
        self.shadow_pass = Some(shadow_pass);
        self.deferred_camera_uniform = Some(deferred_camera_uniform);
        self.deferred_camera_bind_group = Some(deferred_camera_bind_group);
        self.light_uniform_buffer = Some(light_uniform_buffer);
        self.light_bind_group = Some(light_bind_group);
        self.shadow_uniform_buffer = Some(shadow_uniform_buffer);
        self.shadow_lighting_bind_group = Some(shadow_lighting_bind_group);
    }

    /// Render a 3D scene through the deferred rendering pipeline.
    ///
    /// Pipeline order:
    /// 1. Shadow pass — render scene depth from light's perspective
    /// 2. Deferred geometry pass — render scene to G-Buffer (4 MRT + depth)
    /// 3. Deferred lighting pass — full-screen triangle, sample G-Buffer + shadows + lights
    /// 4. Post-process — SSAO/Bloom/TAA/etc. using G-Buffer data
    /// 5. Tonemapping — HDR → swapchain
    ///
    /// The existing 2D sprite path in [`render_frame`] remains unaffected.
    pub fn render_frame_3d(&mut self, scene: &Scene3d<'_>) -> Result<(), wgpu::SurfaceError> {
        self.init_deferred_resources();

        let device = &*self.device;
        let queue = &*self.queue;

        let _deferred_pass = self
            .deferred_pass
            .as_ref()
            .expect("deferred pass must be initialized");
        let gbuffer = self.gbuffer.as_ref().expect("gbuffer must be initialized");
        let _shadow_pass = self
            .shadow_pass
            .as_ref()
            .expect("shadow pass must be initialized");
        let cam_buf = self
            .deferred_camera_uniform
            .as_ref()
            .expect("camera uniform must be initialized");
        let cam_bg = self
            .deferred_camera_bind_group
            .as_ref()
            .expect("camera bind group must be initialized");
        let light_buf = self
            .light_uniform_buffer
            .as_ref()
            .expect("light uniform must be initialized");

        // Update camera uniform
        let camera_uniform = CameraUniform {
            view_proj: scene.camera_vp.to_cols_array_2d(),
            camera_pos: *scene.camera_pos,
            _pad: 0.0,
        };
        queue.write_buffer(cam_buf, 0, bytemuck::bytes_of(&camera_uniform));

        // Update lighting uniform
        queue.write_buffer(light_buf, 0, bytemuck::bytes_of(scene.lighting_uniform));

        // Compute light VP for shadow mapping
        let light_vp = ShadowPass::compute_light_matrices(
            Vec3::from_array(*scene.light_direction),
            crate::shadow::AABB::new(scene.scene_aabb_min, scene.scene_aabb_max),
        );

        let shadow_uniform = ShadowUniform {
            light_vp: light_vp.to_cols_array_2d(),
            shadow_bias: scene.shadow_config.shadow_bias,
            normal_bias: scene.shadow_config.normal_bias,
            cascade_count: scene.shadow_config.cascade_count,
            _pad: 0.0,
        };

        // Upload shadow uniform to GPU
        let shadow_buf = self
            .shadow_uniform_buffer
            .as_ref()
            .expect("shadow uniform buffer must be initialized");
        queue.write_buffer(shadow_buf, 0, bytemuck::bytes_of(&shadow_uniform));

        let output = self.surface.get_current_texture()?;
        let swapchain_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("deferred_encoder"),
        });

        // ── Shadow Pass ─────────────────────────────────────
        self.record_shadow_pass(&mut encoder, scene.batches, &light_vp, scene.mesh_store);

        // ── Deferred Geometry Pass ──────────────────────────
        self.record_deferred_geometry_pass(
            &mut encoder,
            scene.batches,
            scene.material_store,
            scene.mesh_store,
        );

        // ── Deferred Lighting Pass ──────────────────────────
        self.record_deferred_lighting_pass(&mut encoder);

        // ── Post-process (with G-Buffer inputs) ─────────────
        let gbuffer_inputs = GBufferInputs {
            position_view: &gbuffer.textures.position_view,
            normal_view: &gbuffer.textures.normal_view,
            depth_view: &gbuffer.textures.depth_view,
            camera_bind_group: cam_bg,
        };
        self.post_process.execute(
            &mut encoder,
            &swapchain_view,
            device,
            queue,
            Some(&gbuffer_inputs),
        );

        queue.submit([encoder.finish()]);
        output.present();
        Ok(())
    }

    /// Record the shadow pass: render scene depth from the light's perspective.
    fn record_shadow_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        batches: &[InstanceBatch],
        light_vp: &Mat4,
        mesh_store: &MeshStore,
    ) {
        let shadow = self
            .shadow_pass
            .as_ref()
            .expect("shadow pass must be initialized");

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadow_pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &shadow.depth_texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        let res = shadow.config.resolution;
        pass.set_viewport(0.0, 0.0, res as f32, res as f32, 0.0, 1.0);
        pass.set_scissor_rect(0, 0, res, res);
        pass.set_pipeline(&shadow.pipeline);

        for batch in batches {
            let Some(mesh) = mesh_store.get(batch.key.mesh_id) else {
                continue;
            };

            for transform in &batch.transforms {
                let mvp = *light_vp * *transform;
                let pc_data = mvp.to_cols_array();
                pass.set_push_constants(
                    wgpu::ShaderStages::VERTEX,
                    0,
                    bytemuck::cast_slice(&pc_data),
                );

                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                if let Some(ref ib) = mesh.index_buffer {
                    pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
                } else {
                    pass.draw(0..mesh.num_vertices, 0..1);
                }
            }
        }
    }

    /// Record the deferred geometry pass: render scene to G-Buffer (4 MRT + depth).
    fn record_deferred_geometry_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        batches: &[InstanceBatch],
        material_store: &MaterialStore,
        mesh_store: &MeshStore,
    ) {
        let deferred = self
            .deferred_pass
            .as_ref()
            .expect("deferred pass must be initialized");
        let gb = self.gbuffer.as_ref().expect("gbuffer must be initialized");
        let cam_bg = self
            .deferred_camera_bind_group
            .as_ref()
            .expect("camera bind group must be initialized");
        let light_bg = self
            .light_bind_group
            .as_ref()
            .expect("light bind group must be initialized");

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("deferred_geometry_pass"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &gb.textures.albedo_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &gb.textures.normal_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &gb.textures.position_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &gb.textures.material_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &gb.textures.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_viewport(0.0, 0.0, gb.width as f32, gb.height as f32, 0.0, 1.0);
        pass.set_scissor_rect(0, 0, gb.width, gb.height);
        pass.set_pipeline(&deferred.geometry_pipeline);

        // Bind groups: camera (0), light (1)
        pass.set_bind_group(0, cam_bg, &[]);
        pass.set_bind_group(1, light_bg, &[]);

        for batch in batches {
            // Material bind group (group 2)
            if let Some(mat_bg) = material_store.get_bind_group(batch.key.material_id) {
                pass.set_bind_group(2, mat_bg, &[]);
            }

            let Some(mesh) = mesh_store.get(batch.key.mesh_id) else {
                continue;
            };

            for transform in &batch.transforms {
                let normal_matrix = Self::compute_normal_matrix(transform);
                let pc = GeometryPassUniform {
                    model_matrix: transform.to_cols_array_2d(),
                    normal_matrix: normal_matrix.to_cols_array_2d(),
                };
                pass.set_push_constants(wgpu::ShaderStages::VERTEX, 0, bytemuck::bytes_of(&pc));

                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                if let Some(ref ib) = mesh.index_buffer {
                    pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
                } else {
                    pass.draw(0..mesh.num_vertices, 0..1);
                }
            }
        }
    }

    /// Record the deferred lighting pass: full-screen triangle lighting computation.
    fn record_deferred_lighting_pass(&self, encoder: &mut wgpu::CommandEncoder) {
        let deferred = self
            .deferred_pass
            .as_ref()
            .expect("deferred pass must be initialized");
        let gb = self.gbuffer.as_ref().expect("gbuffer must be initialized");
        let cam_bg = self
            .deferred_camera_bind_group
            .as_ref()
            .expect("camera bind group must be initialized");
        let light_bg = self
            .light_bind_group
            .as_ref()
            .expect("light bind group must be initialized");
        let device = &*self.device;

        // Create G-Buffer bind group for the lighting pass
        let gbuffer_bg_layout = &deferred.gbuffer_bind_group_layout;
        let gbuffer_bg = gb.create_bind_group(device, gbuffer_bg_layout);

        let hdr_view = &self.post_process.hdr_framebuffer.view;

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("deferred_lighting_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: hdr_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&deferred.lighting_pipeline);
        pass.set_bind_group(0, cam_bg, &[]);
        pass.set_bind_group(1, light_bg, &[]);
        pass.set_bind_group(2, &gbuffer_bg, &[]);

        // Shadow bind group
        let shadow_bg = self
            .shadow_lighting_bind_group
            .as_ref()
            .expect("shadow bind group must be initialized");
        pass.set_bind_group(3, shadow_bg, &[]);

        pass.draw(0..3, 0..1);
    }

    /// Render a 3D scene to an arbitrary target texture view (not the swapchain).
    ///
    /// This is used by the editor to render the scene to an offscreen viewport texture.
    /// Uses a separate PostProcessChain initialized with `Rgba8UnormSrgb` format.
    pub fn render_frame_3d_to_target(
        &mut self,
        target_view: &wgpu::TextureView,
        target_width: u32,
        target_height: u32,
        scene: &Scene3d<'_>,
        clear_color: Option<wgpu::Color>,
    ) {
        let _span = tracing::info_span!("render_frame_3d_to_target").entered();
        self.init_deferred_resources();

        let device: &wgpu::Device = &self.device;
        let queue: &wgpu::Queue = &self.queue;

        // Lazy-init viewport post-process chain
        if self.viewport_post_process.is_none() {
            self.viewport_post_process = Some(PostProcessChain::new_minimal(
                device,
                queue,
                target_width,
                target_height,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            ));
        }

        let viewport_pp = self.viewport_post_process.as_mut().unwrap();
        if viewport_pp.width != target_width || viewport_pp.height != target_height {
            viewport_pp.resize(device, target_width, target_height);
        }

        // Extract all needed references upfront to avoid borrow conflicts
        let cam_buf = self.deferred_camera_uniform.as_ref().unwrap();
        let cam_bg = self.deferred_camera_bind_group.as_ref().unwrap();
        let light_buf = self.light_uniform_buffer.as_ref().unwrap();
        let shadow_buf = self.shadow_uniform_buffer.as_ref().unwrap();
        let light_bg = self.light_bind_group.as_ref().unwrap();
        let shadow_lighting_bg = self.shadow_lighting_bind_group.as_ref().unwrap();
        let gbuffer = self.gbuffer.as_ref().unwrap();
        let deferred = self.deferred_pass.as_ref().unwrap();
        let shadow = self.shadow_pass.as_ref().unwrap();

        // Update camera uniform
        let camera_uniform = CameraUniform {
            view_proj: scene.camera_vp.to_cols_array_2d(),
            camera_pos: *scene.camera_pos,
            _pad: 0.0,
        };
        queue.write_buffer(cam_buf, 0, bytemuck::bytes_of(&camera_uniform));

        // Update lighting uniform
        queue.write_buffer(light_buf, 0, bytemuck::bytes_of(scene.lighting_uniform));

        // Compute light VP for shadow mapping
        let light_vp = ShadowPass::compute_light_matrices(
            Vec3::from_array(*scene.light_direction),
            crate::shadow::AABB::new(scene.scene_aabb_min, scene.scene_aabb_max),
        );

        let shadow_uniform = ShadowUniform {
            light_vp: light_vp.to_cols_array_2d(),
            shadow_bias: scene.shadow_config.shadow_bias,
            normal_bias: scene.shadow_config.normal_bias,
            cascade_count: scene.shadow_config.cascade_count,
            _pad: 0.0,
        };
        queue.write_buffer(shadow_buf, 0, bytemuck::bytes_of(&shadow_uniform));

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("viewport_deferred_encoder"),
        });

        // ── Shadow Pass ─────────────────────────────────────
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport_shadow_pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &shadow.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            let res = shadow.config.resolution;
            pass.set_viewport(0.0, 0.0, res as f32, res as f32, 0.0, 1.0);
            pass.set_scissor_rect(0, 0, res, res);
            pass.set_pipeline(&shadow.pipeline);

            for batch in scene.batches {
                let Some(mesh) = scene.mesh_store.get(batch.key.mesh_id) else {
                    continue;
                };
                for transform in &batch.transforms {
                    let mvp = light_vp * *transform;
                    let pc_data = mvp.to_cols_array();
                    pass.set_push_constants(
                        wgpu::ShaderStages::VERTEX,
                        0,
                        bytemuck::cast_slice(&pc_data),
                    );
                    pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    if let Some(ref ib) = mesh.index_buffer {
                        pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                        pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
                    } else {
                        pass.draw(0..mesh.num_vertices, 0..1);
                    }
                }
            }
        }

        // ── Deferred Geometry Pass ──────────────────────────
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport_deferred_geometry_pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &gbuffer.textures.albedo_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &gbuffer.textures.normal_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &gbuffer.textures.position_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &gbuffer.textures.material_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &gbuffer.textures.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_viewport(
                0.0,
                0.0,
                gbuffer.width as f32,
                gbuffer.height as f32,
                0.0,
                1.0,
            );
            pass.set_scissor_rect(0, 0, gbuffer.width, gbuffer.height);
            pass.set_pipeline(&deferred.geometry_pipeline);
            pass.set_bind_group(0, cam_bg, &[]);
            pass.set_bind_group(1, light_bg, &[]);

            for batch in scene.batches {
                if let Some(mat_bg) = scene.material_store.get_bind_group(batch.key.material_id) {
                    pass.set_bind_group(2, mat_bg, &[]);
                }
                let Some(mesh) = scene.mesh_store.get(batch.key.mesh_id) else {
                    continue;
                };
                for transform in &batch.transforms {
                    let normal_matrix = Self::compute_normal_matrix(transform);
                    let pc = GeometryPassUniform {
                        model_matrix: transform.to_cols_array_2d(),
                        normal_matrix: normal_matrix.to_cols_array_2d(),
                    };
                    pass.set_push_constants(
                        wgpu::ShaderStages::VERTEX,
                        0,
                        bytemuck::bytes_of(&pc),
                    );
                    pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    if let Some(ref ib) = mesh.index_buffer {
                        pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                        pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
                    } else {
                        pass.draw(0..mesh.num_vertices, 0..1);
                    }
                }
            }
        }

        // ── Deferred Lighting Pass → viewport HDR framebuffer ──
        {
            let gbuffer_bg =
                gbuffer.create_bind_group(device, &deferred.gbuffer_bind_group_layout);
            let hdr_view = &viewport_pp.hdr_framebuffer.view;

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport_lighting_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: hdr_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color.unwrap_or(wgpu::Color {
                            r: 0.15,
                            g: 0.20,
                            b: 0.30,
                            a: 1.0,
                        })),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&deferred.lighting_pipeline);
            pass.set_bind_group(0, cam_bg, &[]);
            pass.set_bind_group(1, light_bg, &[]);
            pass.set_bind_group(2, &gbuffer_bg, &[]);
            pass.set_bind_group(3, shadow_lighting_bg, &[]);

            pass.draw(0..3, 0..1);
        }

        // ── Post-process → viewport target ──────────────────
        let viewport_pp = self.viewport_post_process.as_mut().unwrap();
        let gbuffer_inputs = GBufferInputs {
            position_view: &gbuffer.textures.position_view,
            normal_view: &gbuffer.textures.normal_view,
            depth_view: &gbuffer.textures.depth_view,
            camera_bind_group: cam_bg,
        };
        viewport_pp.execute(&mut encoder, target_view, device, queue, Some(&gbuffer_inputs));

        queue.submit([encoder.finish()]);
    }

    /// Render 3D line overlays into the given target texture view.
    ///
    /// Renders on top of the existing content (LoadOp::Load). Uses the deferred
    /// camera uniform set during the last `render_frame_3d_to_target` call.
    pub fn render_overlay_to_target(
        &mut self,
        target_view: &wgpu::TextureView,
        batch: &crate::line3d::Line3dBatch,
        line_pipeline: &crate::line3d::Line3dPipeline,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        let _span = tracing::info_span!("render_overlay_to_target").entered();
        if batch.is_empty() {
            return;
        }

        let device: &wgpu::Device = &self.device;
        let queue: &wgpu::Queue = &self.queue;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("overlay_encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("overlay_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            batch.render(device, queue, line_pipeline, camera_bind_group, &mut pass);
        }

        queue.submit([encoder.finish()]);
    }
    fn compute_normal_matrix(model: &Mat4) -> Mat4 {
        // For orthonormal transforms (rotation only), the normal matrix equals the model matrix.
        // For non-uniform scales, we need the inverse-transpose. Use a general approach.
        let m = model.to_cols_array();
        // Extract upper-left 3x3
        let a00 = m[0];
        let a01 = m[1];
        let a02 = m[2];
        let a10 = m[4];
        let a11 = m[5];
        let a12 = m[6];
        let a20 = m[8];
        let a21 = m[9];
        let a22 = m[10];
        // Determinant of 3x3
        let det = a00 * (a11 * a22 - a12 * a21) - a01 * (a10 * a22 - a12 * a20)
            + a02 * (a10 * a21 - a11 * a20);
        if det.abs() < 1e-10 {
            return Mat4::IDENTITY;
        }
        let inv_det = 1.0 / det;
        // Inverse-transpose of 3x3 (transpose of cofactor matrix / det)
        let n00 = (a11 * a22 - a12 * a21) * inv_det;
        let n01 = (a10 * a22 - a12 * a20) * inv_det;
        let n02 = (a10 * a21 - a11 * a20) * inv_det;
        let n10 = (a01 * a22 - a02 * a21) * inv_det;
        let n11 = (a00 * a22 - a02 * a20) * inv_det;
        let n12 = (a00 * a21 - a01 * a20) * inv_det;
        let n20 = (a01 * a12 - a02 * a11) * inv_det;
        let n21 = (a00 * a12 - a02 * a10) * inv_det;
        let n22 = (a00 * a11 - a01 * a10) * inv_det;
        // Note: these are the cofactors / det, which is the inverse. We need the
        // transpose of the inverse, so we swap indices.
        Mat4::from_cols_array(&[
            n00, n10, n20, 0.0, n01, n11, n21, 0.0, n02, n12, n22, 0.0, 0.0, 0.0, 0.0, 1.0,
        ])
    }
}
