//! Post-processing pipeline: HDR framebuffer, tone mapping, and composable effect chain.
//!
//! Execution order: SSAO → Bloom → TAA → Height Fog → Volumetric → Tonemapping

use crate::atmosphere::{
    HeightFogConfig, HeightFogPass, SsrConfig, SsrEffect, VolumetricConfig, VolumetricEffect,
};
use crate::bloom::{BloomConfig, BloomEffect};
use crate::ssao::{SsaoConfig, SsaoEffect};
use crate::taa::{TaaConfig, TaaEffect};
use bytemuck::{Pod, Zeroable};

/// Tone mapping operator selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TonemappingOperator {
    /// Reinhard simple tone mapping.
    Reinhard = 0,
    /// ACES filmic tone mapping (Narkowicz 2015) — default.
    #[default]
    Aces = 1,
    /// Exponential tone mapping.
    Exponential = 2,
    /// Linear passthrough (no mapping, just exposure).
    Linear = 3,
}

/// Configuration for the tone mapping pass.
#[derive(Debug, Clone)]
pub struct TonemappingConfig {
    /// Exposure multiplier (default 1.0).
    pub exposure: f32,
    /// Tone mapping operator.
    pub operator: TonemappingOperator,
    /// Gamma for output correction (default 2.2).
    pub gamma: f32,
}

impl Default for TonemappingConfig {
    fn default() -> Self {
        Self {
            exposure: 1.0,
            operator: TonemappingOperator::Aces,
            gamma: 2.2,
        }
    }
}

/// GPU uniform for tone mapping parameters (16 bytes, 4-byte aligned).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct TonemappingUniform {
    pub exposure: f32,
    pub operator: u32,
    pub gamma: f32,
    pub _pad: f32,
}

impl From<&TonemappingConfig> for TonemappingUniform {
    fn from(config: &TonemappingConfig) -> Self {
        Self {
            exposure: config.exposure,
            operator: config.operator as u32,
            gamma: config.gamma,
            _pad: 0.0,
        }
    }
}

/// HDR floating-point framebuffer (Rgba16Float).
///
/// Serves as the intermediate render target between the lighting pass
/// and post-processing passes. Format supports values beyond \[0,1\].
pub struct HdrFramebuffer {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

impl HdrFramebuffer {
    /// Create a new HDR framebuffer with the given resolution.
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let texture = Self::create_texture(device, width, height);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            texture,
            view,
            width,
            height,
        }
    }

    /// Recreate the framebuffer with a new resolution.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.texture = Self::create_texture(device, width, height);
        self.view = self
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
    }

    fn create_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hdr_framebuffer"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }
}

/// Tone mapping pass — converts HDR framebuffer to LDR swapchain output.
///
/// Renders a full-screen triangle that samples the HDR texture and applies
/// the selected tone mapping operator with exposure and gamma correction.
pub struct TonemappingPass {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_buffer: wgpu::Buffer,
    pub sampler: wgpu::Sampler,
    config: TonemappingConfig,
}

impl TonemappingPass {
    /// Create a new tone mapping pass.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("tonemapping_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "pipeline/tonemapping.wgsl"
            ))),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tonemapping_bind_group_layout"),
            entries: &[
                // @binding(0): HDR texture
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
                // @binding(1): sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // @binding(2): uniform params
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
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
            label: Some("tonemapping_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("tonemapping_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_tonemapping"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: output_format,
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

        let config = TonemappingConfig::default();
        let uniform = TonemappingUniform::from(&config);
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tonemapping_uniform"),
            size: std::mem::size_of::<TonemappingUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("tonemapping_sampler"),
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
            config,
        }
    }

    /// Create a bind group for the given HDR framebuffer.
    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        hdr_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tonemapping_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(hdr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// Update tone mapping configuration and upload to GPU.
    pub fn set_config(&mut self, queue: &wgpu::Queue, config: TonemappingConfig) {
        self.config = config;
        let uniform = TonemappingUniform::from(&self.config);
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    /// Get the current configuration.
    pub fn config(&self) -> &TonemappingConfig {
        &self.config
    }

    /// Record the tone mapping pass onto the given encoder.
    ///
    /// Reads from `hdr_view` and writes to `output_view` (typically the swapchain).
    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        hdr_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
        device: &wgpu::Device,
    ) {
        let bind_group = self.create_bind_group(device, hdr_view);

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("tonemapping_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
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

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// Compositing blend mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositeMode {
    /// Multiply destination by source red channel (SSAO application).
    Multiply = 0,
    /// Replace destination with source color (TAA/fog resolve).
    Copy = 1,
    /// Add source * intensity to destination (SSR/volumetric blend).
    Additive = 2,
}

/// GPU uniform for compositing parameters (16 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CompositeUniform {
    pub mode: u32,
    pub intensity: f32,
    pub _pad: [f32; 2],
}

/// General-purpose compositing pass for applying post-processing results
/// back to the active HDR buffer.
///
/// Supports multiply (SSAO), copy (TAA/fog), and additive (SSR/volumetric) modes.
pub struct CompositePass {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_buffer: wgpu::Buffer,
    pub sampler: wgpu::Sampler,
}

impl CompositePass {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("composite_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "pipeline/composite.wgsl"
            ))),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("composite_bind_group_layout"),
            entries: &[
                // @binding(0): source texture
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
                // @binding(1): destination texture
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
                // @binding(3): uniform params
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
            label: Some("composite_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("composite_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_composite"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: output_format,
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

        let uniform = CompositeUniform {
            mode: 0,
            intensity: 1.0,
            _pad: [0.0; 2],
        };
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("composite_uniform"),
            size: std::mem::size_of::<CompositeUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("composite_sampler"),
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

    /// Execute a compositing pass.
    ///
    /// Reads from `src_view` and `dst_view`, writes composited result to `output_view`.
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        src_view: &wgpu::TextureView,
        dst_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
        mode: CompositeMode,
        intensity: f32,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
    ) {
        let uniform = CompositeUniform {
            mode: mode as u32,
            intensity,
            _pad: [0.0; 2],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("composite_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(src_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(dst_view),
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
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("composite_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
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

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// G-Buffer inputs for post-processing passes that need scene geometry data.
///
/// Optional — passes requiring G-Buffer data (SSAO, SSR, height fog, volumetric)
/// will be skipped if these are not provided.
pub struct GBufferInputs<'a> {
    pub position_view: &'a wgpu::TextureView,
    pub normal_view: &'a wgpu::TextureView,
    pub depth_view: &'a wgpu::TextureView,
    pub camera_bind_group: &'a wgpu::BindGroup,
}

/// Height fog effect wrapper with its own render target.
pub struct HeightFogEffect {
    pub pass: HeightFogPass,
    pub target: HdrFramebuffer,
}

impl HeightFogEffect {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        output_format: wgpu::TextureFormat,
    ) -> Self {
        let pass = HeightFogPass::new(device, queue, output_format);
        let target = HdrFramebuffer::new(device, width, height);
        Self { pass, target }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.target.resize(device, width, height);
    }

    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        scene_view: &wgpu::TextureView,
        position_view: &wgpu::TextureView,
        device: &wgpu::Device,
    ) {
        self.pass.execute(
            encoder,
            scene_view,
            position_view,
            &self.target.view,
            device,
        );
    }
}

/// Post-processing chain manager.
///
/// Manages the HDR framebuffer and all post-processing passes:
/// SSAO → Bloom → TAA → Height Fog → Volumetric → Tonemapping
pub struct PostProcessChain {
    pub hdr_framebuffer: HdrFramebuffer,
    pub tonemapping: TonemappingPass,
    pub composite: CompositePass,
    pub ssao: Option<SsaoEffect>,
    pub bloom: Option<BloomEffect>,
    pub taa: Option<TaaEffect>,
    pub height_fog: Option<HeightFogEffect>,
    pub ssr: Option<SsrEffect>,
    pub volumetric: Option<VolumetricEffect>,
    /// Secondary HDR buffer for ping-pong between passes.
    pub hdr_aux: HdrFramebuffer,
    pub width: u32,
    pub height: u32,
    pub output_format: wgpu::TextureFormat,
}

impl PostProcessChain {
    /// Create a new post-processing chain with all passes enabled.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        output_format: wgpu::TextureFormat,
    ) -> Self {
        let hdr_framebuffer = HdrFramebuffer::new(device, width, height);
        let hdr_aux = HdrFramebuffer::new(device, width, height);
        let tonemapping = TonemappingPass::new(device, queue, output_format);
        let composite = CompositePass::new(device, queue, wgpu::TextureFormat::Rgba16Float);

        let ssao = SsaoEffect::new(device, queue, width, height);
        let bloom = BloomEffect::new(device, queue, width, height);
        let taa = TaaEffect::new(device, queue, width, height);
        let height_fog = HeightFogEffect::new(device, queue, width, height, output_format);
        let ssr = SsrEffect::new(device, queue, width, height);
        let volumetric = VolumetricEffect::new(device, queue, width, height);

        Self {
            hdr_framebuffer,
            tonemapping,
            composite,
            ssao: Some(ssao),
            bloom: Some(bloom),
            taa: Some(taa),
            height_fog: Some(height_fog),
            ssr: Some(ssr),
            volumetric: Some(volumetric),
            hdr_aux,
            width,
            height,
            output_format,
        }
    }

    /// Create a minimal chain with only tonemapping (no other passes).
    pub fn new_minimal(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        output_format: wgpu::TextureFormat,
    ) -> Self {
        let hdr_framebuffer = HdrFramebuffer::new(device, width, height);
        let hdr_aux = HdrFramebuffer::new(device, width, height);
        let tonemapping = TonemappingPass::new(device, queue, output_format);
        let composite = CompositePass::new(device, queue, wgpu::TextureFormat::Rgba16Float);

        Self {
            hdr_framebuffer,
            tonemapping,
            composite,
            ssao: None,
            bloom: None,
            taa: None,
            height_fog: None,
            ssr: None,
            volumetric: None,
            hdr_aux,
            width,
            height,
            output_format,
        }
    }

    /// Resize all internal buffers.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.hdr_framebuffer.resize(device, width, height);
        self.hdr_aux.resize(device, width, height);

        if let Some(ref mut ssao) = self.ssao {
            ssao.resize(device, width, height);
        }
        if let Some(ref mut bloom) = self.bloom {
            bloom.resize(device, width, height);
        }
        if let Some(ref mut taa) = self.taa {
            taa.resize(device, width, height);
        }
        if let Some(ref mut fog) = self.height_fog {
            fog.resize(device, width, height);
        }
        if let Some(ref mut ssr) = self.ssr {
            ssr.resize(device, width, height);
        }
        if let Some(ref mut vol) = self.volumetric {
            vol.resize(device, width, height);
        }
    }

    /// Get the HDR framebuffer view for use as a render target.
    pub fn hdr_target(&self) -> &wgpu::TextureView {
        &self.hdr_framebuffer.view
    }

    /// Execute the full post-processing chain.
    ///
    /// Order: SSAO → Bloom → TAA → Height Fog → Volumetric → Tonemapping
    ///
    /// `gbuffer` is optional — passes requiring G-Buffer data will be skipped
    /// if not provided.
    pub fn execute(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        gbuffer: Option<&GBufferInputs<'_>>,
    ) {
        // Track which buffer has the current scene data.
        // Start: hdr_framebuffer has the scene render.
        let mut current_is_primary = true;

        // ── SSAO ──────────────────────────────────────────────
        // Compute occlusion, then composite (multiply) onto the active HDR buffer.
        if let (Some(ssao), Some(gb)) = (&self.ssao, gbuffer) {
            ssao.execute(
                encoder,
                gb.position_view,
                gb.normal_view,
                gb.camera_bind_group,
                queue,
                device,
            );
            // Apply SSAO: multiply scene by occlusion factor.
            let (dst, tmp) = if current_is_primary {
                (&self.hdr_framebuffer.view, &self.hdr_aux.view)
            } else {
                (&self.hdr_aux.view, &self.hdr_framebuffer.view)
            };
            self.composite.execute(
                encoder,
                &ssao.target.output_view,
                dst,
                tmp,
                CompositeMode::Multiply,
                1.0,
                queue,
                device,
            );
            current_is_primary = !current_is_primary;
        }

        // ── Bloom ─────────────────────────────────────────────
        if let Some(ref bloom) = self.bloom {
            let (src, dst) = if current_is_primary {
                (&self.hdr_framebuffer.view, &self.hdr_aux.view)
            } else {
                (&self.hdr_aux.view, &self.hdr_framebuffer.view)
            };
            bloom.execute(encoder, src, dst, queue, device);
            current_is_primary = !current_is_primary;
        }

        // ── TAA ───────────────────────────────────────────────
        // Resolve temporal blend, then copy result back to the active HDR buffer.
        let mut taa_resolved = false;
        if let (Some(taa), Some(gb)) = (&self.taa, gbuffer) {
            let src = if current_is_primary {
                &self.hdr_framebuffer.view
            } else {
                &self.hdr_aux.view
            };
            taa.execute(encoder, src, gb.depth_view, device);
            // Copy TAA resolved output to the active buffer.
            let (dst, tmp) = if current_is_primary {
                (&self.hdr_framebuffer.view, &self.hdr_aux.view)
            } else {
                (&self.hdr_aux.view, &self.hdr_framebuffer.view)
            };
            self.composite.execute(
                encoder,
                &taa.target.resolved_view,
                dst,
                tmp,
                CompositeMode::Copy,
                1.0,
                queue,
                device,
            );
            current_is_primary = !current_is_primary;
            taa_resolved = true;
        }
        if taa_resolved {
            self.taa.as_mut().unwrap().target.swap();
        }

        // ── SSR ───────────────────────────────────────────────
        // Compute reflections, then composite (additive) onto the active HDR buffer.
        if let (Some(ssr), Some(gb)) = (&self.ssr, gbuffer) {
            let scene_view = if current_is_primary {
                &self.hdr_framebuffer.view
            } else {
                &self.hdr_aux.view
            };
            ssr.execute(
                encoder,
                scene_view,
                gb.depth_view,
                gb.normal_view,
                gb.position_view,
                device,
            );
            // Add reflections to the scene.
            let (dst, tmp) = if current_is_primary {
                (&self.hdr_framebuffer.view, &self.hdr_aux.view)
            } else {
                (&self.hdr_aux.view, &self.hdr_framebuffer.view)
            };
            self.composite.execute(
                encoder,
                &ssr.target.reflection_view,
                dst,
                tmp,
                CompositeMode::Additive,
                1.0,
                queue,
                device,
            );
            current_is_primary = !current_is_primary;
        }

        // ── Height Fog ────────────────────────────────────────
        // Apply fog, then copy result back to the active HDR buffer.
        if let (Some(fog), Some(gb)) = (&self.height_fog, gbuffer) {
            let src = if current_is_primary {
                &self.hdr_framebuffer.view
            } else {
                &self.hdr_aux.view
            };
            fog.execute(encoder, src, gb.position_view, device);
            // Copy fogged scene to the active buffer.
            let (dst, tmp) = if current_is_primary {
                (&self.hdr_framebuffer.view, &self.hdr_aux.view)
            } else {
                (&self.hdr_aux.view, &self.hdr_framebuffer.view)
            };
            self.composite.execute(
                encoder,
                &fog.target.view,
                dst,
                tmp,
                CompositeMode::Copy,
                1.0,
                queue,
                device,
            );
            current_is_primary = !current_is_primary;
        }

        // ── Volumetric ────────────────────────────────────────
        // Compute volumetric lighting, then composite (additive) onto the active HDR buffer.
        if let (Some(vol), Some(gb)) = (&self.volumetric, gbuffer) {
            vol.execute(encoder, gb.depth_view, gb.position_view, device);
            // Add volumetric lighting to the scene.
            let (dst, tmp) = if current_is_primary {
                (&self.hdr_framebuffer.view, &self.hdr_aux.view)
            } else {
                (&self.hdr_aux.view, &self.hdr_framebuffer.view)
            };
            self.composite.execute(
                encoder,
                &vol.target.view,
                dst,
                tmp,
                CompositeMode::Additive,
                1.0,
                queue,
                device,
            );
            current_is_primary = !current_is_primary;
        }

        // ── Tonemapping ───────────────────────────────────────
        // Read from whichever buffer has the final scene data.
        let final_src = if current_is_primary {
            &self.hdr_framebuffer.view
        } else {
            &self.hdr_aux.view
        };
        self.tonemapping
            .execute(encoder, final_src, output_view, device);
    }

    /// Legacy execute path (no G-Buffer, no queue) — tonemapping only.
    pub fn execute_simple(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        device: &wgpu::Device,
    ) {
        self.tonemapping
            .execute(encoder, &self.hdr_framebuffer.view, output_view, device);
    }

    // ── Configuration API ──────────────────────────────────────

    /// Update tone mapping settings.
    pub fn set_tonemapping(&mut self, queue: &wgpu::Queue, config: TonemappingConfig) {
        self.tonemapping.set_config(queue, config);
    }

    /// Update SSAO settings. No-op if SSAO is disabled.
    pub fn set_ssao(&mut self, queue: &wgpu::Queue, config: SsaoConfig) {
        if let Some(ref mut ssao) = self.ssao {
            ssao.pass.set_config(queue, config);
        }
    }

    /// Update bloom settings. No-op if bloom is disabled.
    pub fn set_bloom(&mut self, _queue: &wgpu::Queue, config: BloomConfig) {
        if let Some(ref mut bloom) = self.bloom {
            bloom.config = config;
        }
    }

    /// Update TAA settings. No-op if TAA is disabled.
    pub fn set_taa(&mut self, queue: &wgpu::Queue, config: TaaConfig) {
        if let Some(ref mut taa) = self.taa {
            taa.pass.set_config(queue, config);
        }
    }

    /// Update height fog settings. No-op if fog is disabled.
    pub fn set_height_fog(&mut self, queue: &wgpu::Queue, config: HeightFogConfig) {
        if let Some(ref mut fog) = self.height_fog {
            fog.pass.set_config(queue, config);
        }
    }

    /// Update SSR settings. No-op if SSR is disabled.
    pub fn set_ssr(&mut self, queue: &wgpu::Queue, config: SsrConfig) {
        if let Some(ref mut ssr) = self.ssr {
            ssr.pass.set_config(queue, config);
        }
    }

    /// Update volumetric light settings. No-op if volumetric is disabled.
    pub fn set_volumetric(&mut self, queue: &wgpu::Queue, config: VolumetricConfig) {
        if let Some(ref mut vol) = self.volumetric {
            vol.pass.set_config(queue, config);
        }
    }

    /// Enable or disable SSAO.
    pub fn enable_ssao(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, enable: bool) {
        if enable && self.ssao.is_none() {
            self.ssao = Some(SsaoEffect::new(device, queue, self.width, self.height));
        } else if !enable {
            self.ssao = None;
        }
    }

    /// Enable or disable bloom.
    pub fn enable_bloom(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, enable: bool) {
        if enable && self.bloom.is_none() {
            self.bloom = Some(BloomEffect::new(device, queue, self.width, self.height));
        } else if !enable {
            self.bloom = None;
        }
    }

    /// Enable or disable TAA.
    pub fn enable_taa(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, enable: bool) {
        if enable && self.taa.is_none() {
            self.taa = Some(TaaEffect::new(device, queue, self.width, self.height));
        } else if !enable {
            self.taa = None;
        }
    }

    /// Enable or disable height fog.
    pub fn enable_height_fog(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, enable: bool) {
        if enable && self.height_fog.is_none() {
            self.height_fog = Some(HeightFogEffect::new(
                device,
                queue,
                self.width,
                self.height,
                self.output_format,
            ));
        } else if !enable {
            self.height_fog = None;
        }
    }

    /// Enable or disable SSR.
    pub fn enable_ssr(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, enable: bool) {
        if enable && self.ssr.is_none() {
            self.ssr = Some(SsrEffect::new(device, queue, self.width, self.height));
        } else if !enable {
            self.ssr = None;
        }
    }

    /// Enable or disable volumetric lighting.
    pub fn enable_volumetric(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, enable: bool) {
        if enable && self.volumetric.is_none() {
            self.volumetric = Some(VolumetricEffect::new(
                device,
                queue,
                self.width,
                self.height,
            ));
        } else if !enable {
            self.volumetric = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_device() -> (wgpu::Device, wgpu::Queue) {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
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
        (device, queue)
    }

    #[test]
    fn test_hdr_framebuffer_creation() {
        let (device, _) = create_test_device();
        let fb = HdrFramebuffer::new(&device, 1280, 720);

        assert_eq!(fb.width, 1280);
        assert_eq!(fb.height, 720);
        assert_eq!(fb.texture.format(), wgpu::TextureFormat::Rgba16Float);
        assert_eq!(fb.texture.width(), 1280);
        assert_eq!(fb.texture.height(), 720);
    }

    #[test]
    fn test_hdr_framebuffer_resize() {
        let (device, _) = create_test_device();
        let mut fb = HdrFramebuffer::new(&device, 1280, 720);

        fb.resize(&device, 1920, 1080);

        assert_eq!(fb.width, 1920);
        assert_eq!(fb.height, 1080);
        assert_eq!(fb.texture.width(), 1920);
        assert_eq!(fb.texture.height(), 1080);
    }

    #[test]
    fn test_tonemapping_uniform_size() {
        assert_eq!(std::mem::size_of::<TonemappingUniform>(), 16);
    }

    #[test]
    fn test_tonemapping_config_default() {
        let config = TonemappingConfig::default();
        assert!((config.exposure - 1.0).abs() < 1e-6);
        assert_eq!(config.operator, TonemappingOperator::Aces);
        assert!((config.gamma - 2.2).abs() < 1e-6);
    }

    #[test]
    fn test_tonemapping_uniform_conversion() {
        let config = TonemappingConfig {
            exposure: 2.0,
            operator: TonemappingOperator::Reinhard,
            gamma: 2.4,
        };
        let uniform = TonemappingUniform::from(&config);
        assert!((uniform.exposure - 2.0).abs() < 1e-6);
        assert_eq!(uniform.operator, 0);
        assert!((uniform.gamma - 2.4).abs() < 1e-6);
    }

    #[test]
    fn test_tonemapping_pass_creation() {
        let (device, queue) = create_test_device();
        let pass = TonemappingPass::new(&device, &queue, wgpu::TextureFormat::Bgra8UnormSrgb);
        // Verify pipeline was created successfully
        let _ = pass.pipeline;
    }

    #[test]
    fn test_post_process_chain_creation() {
        let (device, queue) = create_test_device();
        let chain = PostProcessChain::new_minimal(
            &device,
            &queue,
            1280,
            720,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        );

        assert_eq!(chain.hdr_framebuffer.width, 1280);
        assert_eq!(chain.hdr_framebuffer.height, 720);
        assert_eq!(chain.hdr_aux.width, 1280);
        assert_eq!(chain.width, 1280);
        assert_eq!(chain.height, 720);
    }

    #[test]
    fn test_post_process_chain_minimal() {
        let (device, queue) = create_test_device();
        let chain = PostProcessChain::new_minimal(
            &device,
            &queue,
            1280,
            720,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        );

        assert_eq!(chain.hdr_framebuffer.width, 1280);
        assert!(chain.ssao.is_none());
        assert!(chain.bloom.is_none());
        assert!(chain.taa.is_none());
        assert!(chain.height_fog.is_none());
        assert!(chain.ssr.is_none());
        assert!(chain.volumetric.is_none());
    }

    #[test]
    fn test_post_process_chain_resize() {
        let (device, queue) = create_test_device();
        let mut chain = PostProcessChain::new_minimal(
            &device,
            &queue,
            1280,
            720,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        );

        chain.resize(&device, 1920, 1080);

        assert_eq!(chain.hdr_framebuffer.width, 1920);
        assert_eq!(chain.hdr_framebuffer.height, 1080);
        assert_eq!(chain.hdr_aux.width, 1920);
        assert_eq!(chain.width, 1920);
        assert_eq!(chain.height, 1080);
    }

    #[test]
    fn test_composite_uniform_size() {
        assert_eq!(std::mem::size_of::<CompositeUniform>(), 16);
    }

    #[test]
    fn test_composite_pass_creation() {
        let (device, queue) = create_test_device();
        let pass = CompositePass::new(&device, &queue, wgpu::TextureFormat::Rgba16Float);
        let _ = pass.pipeline;
    }

    #[test]
    fn test_composite_mode_values() {
        assert_eq!(CompositeMode::Multiply as u32, 0);
        assert_eq!(CompositeMode::Copy as u32, 1);
        assert_eq!(CompositeMode::Additive as u32, 2);
    }
}
