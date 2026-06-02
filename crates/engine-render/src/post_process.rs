//! Post-processing pipeline: HDR framebuffer, tone mapping, and composable effect chain.

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
/// and post-processing passes. Format supports values beyond [0,1].
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

/// Post-processing chain manager.
///
/// Manages the HDR framebuffer and all post-processing passes.
/// Currently supports tone mapping; will be extended with SSAO, Bloom, etc.
pub struct PostProcessChain {
    pub hdr_framebuffer: HdrFramebuffer,
    pub tonemapping: TonemappingPass,
}

impl PostProcessChain {
    /// Create a new post-processing chain.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        output_format: wgpu::TextureFormat,
    ) -> Self {
        let hdr_framebuffer = HdrFramebuffer::new(device, width, height);
        let tonemapping = TonemappingPass::new(device, queue, output_format);

        Self {
            hdr_framebuffer,
            tonemapping,
        }
    }

    /// Resize all internal buffers.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.hdr_framebuffer.resize(device, width, height);
    }

    /// Get the HDR framebuffer view for use as a render target.
    pub fn hdr_target(&self) -> &wgpu::TextureView {
        &self.hdr_framebuffer.view
    }

    /// Execute the full post-processing chain.
    ///
    /// 1. Reads from the HDR framebuffer
    /// 2. Applies tone mapping
    /// 3. Writes to `output_view` (swapchain)
    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        device: &wgpu::Device,
    ) {
        self.tonemapping
            .execute(encoder, &self.hdr_framebuffer.view, output_view, device);
    }

    /// Update tone mapping settings.
    pub fn set_tonemapping(&mut self, queue: &wgpu::Queue, config: TonemappingConfig) {
        self.tonemapping.set_config(queue, config);
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
        let chain = PostProcessChain::new(
            &device,
            &queue,
            1280,
            720,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        );

        assert_eq!(chain.hdr_framebuffer.width, 1280);
        assert_eq!(chain.hdr_framebuffer.height, 720);
    }

    #[test]
    fn test_post_process_chain_resize() {
        let (device, queue) = create_test_device();
        let mut chain = PostProcessChain::new(
            &device,
            &queue,
            1280,
            720,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        );

        chain.resize(&device, 1920, 1080);

        assert_eq!(chain.hdr_framebuffer.width, 1920);
        assert_eq!(chain.hdr_framebuffer.height, 1080);
    }
}
