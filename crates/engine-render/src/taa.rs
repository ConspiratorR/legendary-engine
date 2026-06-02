//! Temporal Anti-Aliasing (TAA) pass.
//!
//! Blends the current frame with a history buffer using motion vectors
//! to produce temporally stable output. Uses neighborhood clamping
//! to reduce ghosting artifacts.

use bytemuck::{Pod, Zeroable};

/// TAA configuration.
#[derive(Debug, Clone)]
pub struct TaaConfig {
    /// Blend factor: how much of the current frame to use (default 0.05 = 95% history).
    pub blend_factor: f32,
    /// Sub-pixel jitter scale for the camera (default 1.0).
    pub jitter_scale: f32,
}

impl Default for TaaConfig {
    fn default() -> Self {
        Self {
            blend_factor: 0.05,
            jitter_scale: 1.0,
        }
    }
}

/// GPU uniform for TAA parameters (16 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct TaaUniform {
    pub blend_factor: f32,
    pub jitter_scale: f32,
    pub _pad: [f32; 2],
}

/// TAA render targets.
pub struct TaaTarget {
    /// History buffer (previous resolved frame).
    pub history_texture: wgpu::Texture,
    pub history_view: wgpu::TextureView,
    /// Motion vector buffer (Rg16Float).
    pub motion_texture: wgpu::Texture,
    pub motion_view: wgpu::TextureView,
    /// Resolved output (ping-pong with history).
    pub resolved_texture: wgpu::Texture,
    pub resolved_view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

impl TaaTarget {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let (history_texture, history_view) =
            Self::create_rgba16f(device, width, height, "taa_history");
        let (motion_texture, motion_view) = Self::create_rg16f(device, width, height, "taa_motion");
        let (resolved_texture, resolved_view) =
            Self::create_rgba16f(device, width, height, "taa_resolved");

        Self {
            history_texture,
            history_view,
            motion_texture,
            motion_view,
            resolved_texture,
            resolved_view,
            width,
            height,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        let (history_texture, history_view) =
            Self::create_rgba16f(device, width, height, "taa_history");
        let (motion_texture, motion_view) = Self::create_rg16f(device, width, height, "taa_motion");
        let (resolved_texture, resolved_view) =
            Self::create_rgba16f(device, width, height, "taa_resolved");
        self.history_texture = history_texture;
        self.history_view = history_view;
        self.motion_texture = motion_texture;
        self.motion_view = motion_view;
        self.resolved_texture = resolved_texture;
        self.resolved_view = resolved_view;
    }

    /// Swap history and resolved buffers for next frame.
    pub fn swap(&mut self) {
        std::mem::swap(&mut self.history_texture, &mut self.resolved_texture);
        std::mem::swap(&mut self.history_view, &mut self.resolved_view);
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

    fn create_rg16f(
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
            format: wgpu::TextureFormat::Rg16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }
}

/// TAA resolve pass.
pub struct TaaPass {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_buffer: wgpu::Buffer,
    pub sampler: wgpu::Sampler,
    pub config: TaaConfig,
}

impl TaaPass {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("taa_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "pipeline/taa.wgsl"
            ))),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("taa_bind_group_layout"),
            entries: &[
                // @binding(0): current frame
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
                // @binding(1): history buffer
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
                // @binding(2): motion vectors
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
                // @binding(4): uniform
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
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("taa_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("taa_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_taa"),
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

        let config = TaaConfig::default();
        let uniform = TaaUniform {
            blend_factor: config.blend_factor,
            jitter_scale: config.jitter_scale,
            _pad: [0.0; 2],
        };
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("taa_uniform"),
            size: std::mem::size_of::<TaaUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("taa_sampler"),
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

    /// Update TAA configuration and upload to GPU.
    pub fn set_config(&mut self, queue: &wgpu::Queue, config: TaaConfig) {
        self.config = config;
        let uniform = TaaUniform {
            blend_factor: self.config.blend_factor,
            jitter_scale: self.config.jitter_scale,
            _pad: [0.0; 2],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        current_view: &wgpu::TextureView,
        history_view: &wgpu::TextureView,
        motion_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
        device: &wgpu::Device,
    ) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("taa_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(current_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(history_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(motion_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("taa_pass"),
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

/// Complete TAA effect.
pub struct TaaEffect {
    pub pass: TaaPass,
    pub target: TaaTarget,
}

impl TaaEffect {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) -> Self {
        let pass = TaaPass::new(device, queue);
        let target = TaaTarget::new(device, width, height);
        Self { pass, target }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.target.resize(device, width, height);
    }

    /// Execute TAA: blend current frame with history using motion vectors.
    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        current_view: &wgpu::TextureView,
        motion_view: &wgpu::TextureView,
        device: &wgpu::Device,
    ) {
        self.pass.execute(
            encoder,
            current_view,
            &self.target.history_view,
            motion_view,
            &self.target.resolved_view,
            device,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_taa_config_default() {
        let config = TaaConfig::default();
        assert!((config.blend_factor - 0.05).abs() < 1e-6);
        assert!((config.jitter_scale - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_taa_uniform_size() {
        assert_eq!(std::mem::size_of::<TaaUniform>(), 16);
    }
}
