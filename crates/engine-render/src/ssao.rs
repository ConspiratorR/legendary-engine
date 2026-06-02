//! Screen Space Ambient Occlusion (SSAO) pass.
//!
//! Computes per-pixel ambient occlusion by sampling the G-Buffer
//! (position + normal) with a hemisphere kernel. Output is an R8Unorm
//! texture where 1.0 = fully lit, 0.0 = fully occluded.

use bytemuck::{Pod, Zeroable};
use rand::Rng;

/// SSAO configuration parameters.
#[derive(Debug, Clone)]
pub struct SsaoConfig {
    /// Number of kernel samples (max 64, default 32).
    pub kernel_size: u32,
    /// Sampling radius in world units (default 0.5).
    pub radius: f32,
    /// Depth bias to prevent self-occlusion (default 0.025).
    pub bias: f32,
    /// Occlusion intensity multiplier (default 1.0).
    pub intensity: f32,
    /// Depth threshold for blur edge detection (default 0.1).
    pub blur_depth_threshold: f32,
}

impl Default for SsaoConfig {
    fn default() -> Self {
        Self {
            kernel_size: 32,
            radius: 0.5,
            bias: 0.025,
            intensity: 1.0,
            blur_depth_threshold: 0.1,
        }
    }
}

/// GPU uniform for SSAO parameters (32 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SsaoUniform {
    pub kernel_size: u32,
    pub radius: f32,
    pub bias: f32,
    pub intensity: f32,
    pub noise_scale: [f32; 2],
    pub _pad0: f32,
    pub _pad1: f32,
}

/// GPU uniform for blur parameters (16 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BlurUniform {
    pub direction: [f32; 2],
    pub depth_threshold: f32,
    pub _pad: f32,
}

/// SSAO render target textures.
pub struct SsaoTarget {
    /// Raw SSAO output (before blur).
    pub raw_texture: wgpu::Texture,
    pub raw_view: wgpu::TextureView,
    /// Horizontally blurred intermediate.
    pub blur_h_texture: wgpu::Texture,
    pub blur_h_view: wgpu::TextureView,
    /// Final blurred output.
    pub output_texture: wgpu::Texture,
    pub output_view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

impl SsaoTarget {
    /// Create SSAO render targets with the given resolution.
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let (raw_texture, raw_view) = Self::create_r8(device, width, height, "ssao_raw");
        let (blur_h_texture, blur_h_view) = Self::create_r8(device, width, height, "ssao_blur_h");
        let (output_texture, output_view) = Self::create_r8(device, width, height, "ssao_output");

        Self {
            raw_texture,
            raw_view,
            blur_h_texture,
            blur_h_view,
            output_texture,
            output_view,
            width,
            height,
        }
    }

    /// Recreate all textures with a new resolution.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        let (raw_texture, raw_view) = Self::create_r8(device, width, height, "ssao_raw");
        let (blur_h_texture, blur_h_view) = Self::create_r8(device, width, height, "ssao_blur_h");
        let (output_texture, output_view) = Self::create_r8(device, width, height, "ssao_output");
        self.raw_texture = raw_texture;
        self.raw_view = raw_view;
        self.blur_h_texture = blur_h_texture;
        self.blur_h_view = blur_h_view;
        self.output_texture = output_texture;
        self.output_view = output_view;
    }

    fn create_r8(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        label: &str,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }
}

/// Noise texture for SSAO kernel rotation (4x4 tiled).
pub struct SsaoNoise {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl SsaoNoise {
    /// Create a 4x4 noise texture with random rotation vectors.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let mut rng = rand::rng();
        let mut data = Vec::with_capacity(16 * 4);
        for _ in 0..16 {
            let x: f32 = rng.random_range(-1.0..1.0);
            let y: f32 = rng.random_range(-1.0..1.0);
            // Pack as RG8 (normalized)
            data.push(((x * 0.5 + 0.5) * 255.0) as u8);
            data.push(((y * 0.5 + 0.5) * 255.0) as u8);
            data.push(0u8);
            data.push(0u8);
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ssao_noise"),
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * 4),
                rows_per_image: Some(4),
            },
            wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { texture, view }
    }
}

/// Generate a cosine-weighted hemisphere kernel.
pub fn generate_kernel(size: u32) -> Vec<[f32; 4]> {
    let mut rng = rand::rng();
    let mut kernel = Vec::with_capacity(size as usize);

    for i in 0..size {
        let t = i as f32 / size as f32;
        // Scale: more samples closer to the origin
        let scale = lerp(0.1, 1.0, t * t);

        let x: f32 = rng.random_range(-1.0..1.0);
        let y: f32 = rng.random_range(-1.0..1.0);
        let z: f32 = rng.random_range(0.0..1.0); // hemisphere: z >= 0

        let len = (x * x + y * y + z * z).sqrt();
        kernel.push([x / len * scale, y / len * scale, z / len * scale, 0.0]);
    }

    kernel
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// SSAO pass — computes screen-space ambient occlusion.
pub struct SsaoPass {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_buffer: wgpu::Buffer,
    pub kernel_buffer: wgpu::Buffer,
    pub sampler: wgpu::Sampler,
    pub noise: SsaoNoise,
    pub config: SsaoConfig,
}

impl SsaoPass {
    /// Create a new SSAO pass.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ssao_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "pipeline/ssao.wgsl"
            ))),
        });

        // Bind group layout for SSAO resources
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssao_bind_group_layout"),
            entries: &[
                // @binding(0): position texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // @binding(1): normal texture
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // @binding(2): noise texture
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // @binding(3): sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // @binding(4): params uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // @binding(5): kernel storage
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Camera bind group layout (group 1)
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssao_camera_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ssao_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout, &camera_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ssao_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_ssao"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let config = SsaoConfig::default();

        // Create uniform buffer
        let uniform = SsaoUniform {
            kernel_size: config.kernel_size,
            radius: config.radius,
            bias: config.bias,
            intensity: config.intensity,
            noise_scale: [1.0, 1.0], // Will be updated on resize
            _pad0: 0.0,
            _pad1: 0.0,
        };
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ssao_uniform"),
            size: std::mem::size_of::<SsaoUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        // Create kernel buffer
        let kernel = generate_kernel(64);
        let kernel_data: &[u8] = bytemuck::cast_slice(&kernel);
        let kernel_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ssao_kernel"),
            size: kernel_data.len() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&kernel_buffer, 0, kernel_data);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ssao_sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let noise = SsaoNoise::new(device, queue);

        Self {
            pipeline,
            bind_group_layout,
            uniform_buffer,
            kernel_buffer,
            sampler,
            noise,
            config,
        }
    }

    /// Create bind group for SSAO resources.
    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        position_view: &wgpu::TextureView,
        normal_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ssao_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(position_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.noise.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: self.kernel_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// Update SSAO configuration.
    pub fn set_config(&mut self, queue: &wgpu::Queue, config: SsaoConfig) {
        self.config = config;
        let uniform = SsaoUniform {
            kernel_size: self.config.kernel_size,
            radius: self.config.radius,
            bias: self.config.bias,
            intensity: self.config.intensity,
            noise_scale: [1.0, 1.0],
            _pad0: 0.0,
            _pad1: 0.0,
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    /// Execute the SSAO pass.
    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        position_view: &wgpu::TextureView,
        normal_view: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
        output_view: &wgpu::TextureView,
        device: &wgpu::Device,
    ) {
        let bind_group = self.create_bind_group(device, position_view, normal_view);

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ssao_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_bind_group(1, camera_bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// SSAO blur pass — smooths raw SSAO output with depth-aware bilateral blur.
pub struct SsaoBlurPass {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_buffer: wgpu::Buffer,
    pub sampler: wgpu::Sampler,
}

impl SsaoBlurPass {
    /// Create a new SSAO blur pass.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ssao_blur_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "pipeline/ssao_blur.wgsl"
            ))),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssao_blur_bind_group_layout"),
            entries: &[
                // @binding(0): SSAO texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // @binding(1): position texture (for depth-aware blur)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // @binding(2): sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // @binding(3): params uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ssao_blur_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ssao_blur_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_blur"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let blur_uniform = BlurUniform {
            direction: [1.0, 0.0],
            depth_threshold: 0.1,
            _pad: 0.0,
        };
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ssao_blur_uniform"),
            size: std::mem::size_of::<BlurUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&blur_uniform));

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ssao_blur_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            pipeline,
            bind_group_layout,
            uniform_buffer,
            sampler,
        }
    }

    /// Create bind group for a blur pass.
    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        ssao_view: &wgpu::TextureView,
        position_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ssao_blur_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(ssao_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(position_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// Execute a single blur pass with the given direction.
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        ssao_view: &wgpu::TextureView,
        position_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
        direction: [f32; 2],
        depth_threshold: f32,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
    ) {
        let uniform = BlurUniform {
            direction,
            depth_threshold,
            _pad: 0.0,
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        let bind_group = self.create_bind_group(device, ssao_view, position_view);

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ssao_blur_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// Complete SSAO effect: compute + blur.
pub struct SsaoEffect {
    pub pass: SsaoPass,
    pub blur: SsaoBlurPass,
    pub target: SsaoTarget,
}

impl SsaoEffect {
    /// Create a new SSAO effect with all resources.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) -> Self {
        let pass = SsaoPass::new(device, queue);
        let blur = SsaoBlurPass::new(device, queue);
        let target = SsaoTarget::new(device, width, height);

        Self { pass, blur, target }
    }

    /// Resize all internal targets.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.target.resize(device, width, height);
    }

    /// Execute the full SSAO pipeline.
    ///
    /// 1. Compute raw SSAO from G-Buffer
    /// 2. Horizontal blur
    /// 3. Vertical blur → final output
    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        position_view: &wgpu::TextureView,
        normal_view: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
    ) {
        // Step 1: Raw SSAO
        self.pass.execute(
            encoder,
            position_view,
            normal_view,
            camera_bind_group,
            &self.target.raw_view,
            device,
        );

        // Step 2: Horizontal blur (raw → blur_h)
        self.blur.execute(
            encoder,
            &self.target.raw_view,
            position_view,
            &self.target.blur_h_view,
            [1.0, 0.0],
            self.pass.config.blur_depth_threshold,
            queue,
            device,
        );

        // Step 3: Vertical blur (blur_h → output)
        self.blur.execute(
            encoder,
            &self.target.blur_h_view,
            position_view,
            &self.target.output_view,
            [0.0, 1.0],
            self.pass.config.blur_depth_threshold,
            queue,
            device,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssao_config_default() {
        let config = SsaoConfig::default();
        assert_eq!(config.kernel_size, 32);
        assert!((config.radius - 0.5).abs() < 1e-6);
        assert!((config.bias - 0.025).abs() < 1e-6);
        assert!((config.intensity - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_generate_kernel_size() {
        let kernel = generate_kernel(32);
        assert_eq!(kernel.len(), 32);
    }

    #[test]
    fn test_generate_kernel_hemisphere() {
        let kernel = generate_kernel(64);
        for sample in &kernel {
            // All z values should be >= 0 (hemisphere)
            assert!(
                sample[2] >= 0.0,
                "kernel z should be >= 0, got {}",
                sample[2]
            );
        }
    }

    #[test]
    fn test_generate_kernel_normalized() {
        let kernel = generate_kernel(64);
        for sample in &kernel {
            let len =
                (sample[0] * sample[0] + sample[1] * sample[1] + sample[2] * sample[2]).sqrt();
            // Kernel samples are hemisphere-distributed with scale factor,
            // so they won't be exactly unit length. Verify they're in a reasonable range.
            assert!(
                len > 0.01 && len <= 1.1,
                "kernel sample length should be in (0.01, 1.1], got {}",
                len
            );
        }
    }

    #[test]
    fn test_ssao_uniform_size() {
        assert_eq!(std::mem::size_of::<SsaoUniform>(), 32);
    }

    #[test]
    fn test_blur_uniform_size() {
        assert_eq!(std::mem::size_of::<BlurUniform>(), 16);
    }
}
