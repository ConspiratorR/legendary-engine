//! Bloom post-processing effect.
//!
//! Pipeline: HDR → Brightness Extract → Gaussian Blur (H) → Gaussian Blur (V) → Combine with HDR.

use bytemuck::{Pod, Zeroable};

/// Bloom configuration.
#[derive(Debug, Clone)]
pub struct BloomConfig {
    /// Luminance threshold for bloom extraction (default 1.0).
    pub threshold: f32,
    /// Soft knee for smooth threshold transition (default 0.5).
    pub soft_knee: f32,
    /// Blur radius multiplier (default 1.0).
    pub blur_radius: f32,
    /// Bloom intensity when combining (default 0.3).
    pub intensity: f32,
}

impl Default for BloomConfig {
    fn default() -> Self {
        Self {
            threshold: 1.0,
            soft_knee: 0.5,
            blur_radius: 1.0,
            intensity: 0.3,
        }
    }
}

/// GPU uniform for bloom extraction (16 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BloomExtractUniform {
    pub threshold: f32,
    pub soft_knee: f32,
    pub _pad: [f32; 2],
}

/// GPU uniform for bloom blur (16 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BloomBlurUniform {
    pub direction: [f32; 2],
    pub radius: f32,
    pub _pad: f32,
}

/// GPU uniform for bloom combine (16 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BloomCombineUniform {
    pub intensity: f32,
    pub _pad: [f32; 3],
}

/// Bloom render targets (half-resolution for performance).
pub struct BloomTarget {
    /// Brightness-extracted image.
    pub extract_texture: wgpu::Texture,
    pub extract_view: wgpu::TextureView,
    /// Horizontal blur intermediate.
    pub blur_h_texture: wgpu::Texture,
    pub blur_h_view: wgpu::TextureView,
    /// Vertical blur output (final bloom).
    pub blur_v_texture: wgpu::Texture,
    pub blur_v_view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

impl BloomTarget {
    /// Create bloom targets at half resolution.
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        // Half resolution for bloom (performance optimization)
        let w = (width / 2).max(1);
        let h = (height / 2).max(1);
        let (extract_texture, extract_view) = Self::create_rgba16f(device, w, h, "bloom_extract");
        let (blur_h_texture, blur_h_view) = Self::create_rgba16f(device, w, h, "bloom_blur_h");
        let (blur_v_texture, blur_v_view) = Self::create_rgba16f(device, w, h, "bloom_blur_v");

        Self {
            extract_texture,
            extract_view,
            blur_h_texture,
            blur_h_view,
            blur_v_texture,
            blur_v_view,
            width: w,
            height: h,
        }
    }

    /// Resize targets (half resolution).
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let w = (width / 2).max(1);
        let h = (height / 2).max(1);
        let (extract_texture, extract_view) = Self::create_rgba16f(device, w, h, "bloom_extract");
        let (blur_h_texture, blur_h_view) = Self::create_rgba16f(device, w, h, "bloom_blur_h");
        let (blur_v_texture, blur_v_view) = Self::create_rgba16f(device, w, h, "bloom_blur_v");
        self.extract_texture = extract_texture;
        self.extract_view = extract_view;
        self.blur_h_texture = blur_h_texture;
        self.blur_h_view = blur_h_view;
        self.blur_v_texture = blur_v_texture;
        self.blur_v_view = blur_v_view;
        self.width = w;
        self.height = h;
    }

    fn create_rgba16f(
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
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }
}

/// Bloom brightness extraction pass.
pub struct BloomExtractPass {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_buffer: wgpu::Buffer,
    pub sampler: wgpu::Sampler,
}

impl BloomExtractPass {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloom_extract_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "pipeline/bloom_extract.wgsl"
            ))),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom_extract_bind_group_layout"),
            entries: &[
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
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
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
            label: Some("bloom_extract_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bloom_extract_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_extract"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
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

        let uniform = BloomExtractUniform {
            threshold: 1.0,
            soft_knee: 0.5,
            _pad: [0.0; 2],
        };
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bloom_extract_uniform"),
            size: std::mem::size_of::<BloomExtractUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bloom_sampler"),
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

    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        hdr_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
        device: &wgpu::Device,
    ) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom_extract_bind_group"),
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
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("bloom_extract_pass"),
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

/// Bloom Gaussian blur pass (reusable for H and V).
pub struct BloomBlurPass {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_buffer: wgpu::Buffer,
    pub sampler: wgpu::Sampler,
}

impl BloomBlurPass {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloom_blur_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "pipeline/bloom_blur.wgsl"
            ))),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom_blur_bind_group_layout"),
            entries: &[
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
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
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
            label: Some("bloom_blur_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bloom_blur_pipeline"),
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
                    format: wgpu::TextureFormat::Rgba16Float,
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

        let uniform = BloomBlurUniform {
            direction: [1.0, 0.0],
            radius: 1.0,
            _pad: 0.0,
        };
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bloom_blur_uniform"),
            size: std::mem::size_of::<BloomBlurUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bloom_blur_sampler"),
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

    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        input_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
        direction: [f32; 2],
        radius: f32,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
    ) {
        let uniform = BloomBlurUniform {
            direction,
            radius,
            _pad: 0.0,
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom_blur_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(input_view),
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
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("bloom_blur_pass"),
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

/// Bloom combine pass — additive blend of bloom onto HDR.
pub struct BloomCombinePass {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_buffer: wgpu::Buffer,
    pub sampler: wgpu::Sampler,
}

impl BloomCombinePass {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloom_combine_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "pipeline/bloom_combine.wgsl"
            ))),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom_combine_bind_group_layout"),
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
                // @binding(1): bloom texture
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
                // @binding(3): uniform
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
            label: Some("bloom_combine_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bloom_combine_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_combine"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
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

        let uniform = BloomCombineUniform {
            intensity: 0.3,
            _pad: [0.0; 3],
        };
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bloom_combine_uniform"),
            size: std::mem::size_of::<BloomCombineUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bloom_combine_sampler"),
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

    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        hdr_view: &wgpu::TextureView,
        bloom_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
        intensity: f32,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
    ) {
        let uniform = BloomCombineUniform {
            intensity,
            _pad: [0.0; 3],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom_combine_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(hdr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(bloom_view),
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
            label: Some("bloom_combine_pass"),
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

/// Complete bloom effect: extract → blur → combine.
pub struct BloomEffect {
    pub extract: BloomExtractPass,
    pub blur: BloomBlurPass,
    pub combine: BloomCombinePass,
    pub target: BloomTarget,
    pub config: BloomConfig,
}

impl BloomEffect {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) -> Self {
        let extract = BloomExtractPass::new(device, queue);
        let blur = BloomBlurPass::new(device, queue);
        let combine = BloomCombinePass::new(device, queue);
        let target = BloomTarget::new(device, width, height);
        let config = BloomConfig::default();

        Self {
            extract,
            blur,
            combine,
            target,
            config,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.target.resize(device, width, height);
    }

    /// Execute the full bloom pipeline.
    ///
    /// Reads from `hdr_view`, writes combined result to `output_view`.
    /// Note: This modifies the HDR buffer content by blending bloom into it.
    /// For the current architecture, we write to a separate output to avoid
    /// feedback. The caller should use the output as the new HDR source for tone mapping.
    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        hdr_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
    ) {
        // Step 1: Extract bright areas
        self.extract
            .execute(encoder, hdr_view, &self.target.extract_view, device);

        // Step 2: Horizontal blur
        self.blur.execute(
            encoder,
            &self.target.extract_view,
            &self.target.blur_h_view,
            [1.0, 0.0],
            self.config.blur_radius,
            queue,
            device,
        );

        // Step 3: Vertical blur
        self.blur.execute(
            encoder,
            &self.target.blur_h_view,
            &self.target.blur_v_view,
            [0.0, 1.0],
            self.config.blur_radius,
            queue,
            device,
        );

        // Step 4: Combine bloom with HDR
        self.combine.execute(
            encoder,
            hdr_view,
            &self.target.blur_v_view,
            output_view,
            self.config.intensity,
            queue,
            device,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloom_config_default() {
        let config = BloomConfig::default();
        assert!((config.threshold - 1.0).abs() < 1e-6);
        assert!((config.soft_knee - 0.5).abs() < 1e-6);
        assert!((config.blur_radius - 1.0).abs() < 1e-6);
        assert!((config.intensity - 0.3).abs() < 1e-6);
    }

    #[test]
    fn test_bloom_extract_uniform_size() {
        assert_eq!(std::mem::size_of::<BloomExtractUniform>(), 16);
    }

    #[test]
    fn test_bloom_blur_uniform_size() {
        assert_eq!(std::mem::size_of::<BloomBlurUniform>(), 16);
    }

    #[test]
    fn test_bloom_combine_uniform_size() {
        assert_eq!(std::mem::size_of::<BloomCombineUniform>(), 16);
    }
}
