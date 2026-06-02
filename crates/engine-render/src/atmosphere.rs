//! Height Fog post-processing effect.
//!
//! Applies exponential height-based fog using G-Buffer position data.

use bytemuck::{Pod, Zeroable};

/// Height fog configuration.
#[derive(Debug, Clone)]
pub struct HeightFogConfig {
    /// Fog color (default: medium gray).
    pub color: [f32; 3],
    /// Fog density (default: 0.02).
    pub density: f32,
    /// Height falloff rate (default: 0.1).
    pub height_falloff: f32,
    /// Distance at which fog begins (default: 10.0).
    pub start_distance: f32,
}

impl Default for HeightFogConfig {
    fn default() -> Self {
        Self {
            color: [0.5, 0.5, 0.6],
            density: 0.02,
            height_falloff: 0.1,
            start_distance: 10.0,
        }
    }
}

/// GPU uniform for height fog (32 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct HeightFogUniform {
    pub color: [f32; 3],
    pub density: f32,
    pub height_falloff: f32,
    pub start_distance: f32,
    pub _pad: [f32; 2],
}

/// Height fog pass.
pub struct HeightFogPass {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_buffer: wgpu::Buffer,
    pub sampler: wgpu::Sampler,
}

impl HeightFogPass {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("height_fog_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "pipeline/height_fog.wgsl"
            ))),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("height_fog_bind_group_layout"),
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
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
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
            label: Some("height_fog_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("height_fog_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_height_fog"),
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

        let config = HeightFogConfig::default();
        let uniform = HeightFogUniform {
            color: config.color,
            density: config.density,
            height_falloff: config.height_falloff,
            start_distance: config.start_distance,
            _pad: [0.0; 2],
        };
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("height_fog_uniform"),
            size: std::mem::size_of::<HeightFogUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("height_fog_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
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
        scene_view: &wgpu::TextureView,
        position_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
        device: &wgpu::Device,
    ) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("height_fog_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(scene_view),
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
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("height_fog_pass"),
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

/// Volumetric light configuration.
#[derive(Debug, Clone)]
pub struct VolumetricConfig {
    /// Light position in world space.
    pub light_pos: [f32; 3],
    /// Scattering coefficient (default: 0.5).
    pub scattering: f32,
    /// Maximum ray march distance (default: 50.0).
    pub max_distance: f32,
    /// Number of ray march steps (default: 32).
    pub num_steps: u32,
    /// Effect intensity (default: 1.0).
    pub intensity: f32,
}

impl Default for VolumetricConfig {
    fn default() -> Self {
        Self {
            light_pos: [0.0, 100.0, 0.0],
            scattering: 0.5,
            max_distance: 50.0,
            num_steps: 32,
            intensity: 1.0,
        }
    }
}

/// GPU uniform for volumetric light (32 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct VolumetricUniform {
    pub light_pos: [f32; 3],
    pub scattering: f32,
    pub max_distance: f32,
    pub num_steps: u32,
    pub intensity: f32,
    pub _pad: f32,
}

/// SSR configuration.
#[derive(Debug, Clone)]
pub struct SsrConfig {
    /// Maximum ray march steps (default: 64).
    pub max_steps: u32,
    /// Maximum reflection distance (default: 50.0).
    pub max_distance: f32,
    /// Depth thickness for hit detection (default: 0.5).
    pub thickness: f32,
    /// Step stride (default: 1.0).
    pub stride: f32,
}

impl Default for SsrConfig {
    fn default() -> Self {
        Self {
            max_steps: 64,
            max_distance: 50.0,
            thickness: 0.5,
            stride: 1.0,
        }
    }
}

/// GPU uniform for SSR (32 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SsrUniform {
    pub max_steps: u32,
    pub max_distance: f32,
    pub thickness: f32,
    pub stride: f32,
    pub _pad: [f32; 3],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_height_fog_config_default() {
        let config = HeightFogConfig::default();
        assert!((config.density - 0.02).abs() < 1e-6);
        assert!((config.height_falloff - 0.1).abs() < 1e-6);
    }

    #[test]
    fn test_height_fog_uniform_size() {
        assert_eq!(std::mem::size_of::<HeightFogUniform>(), 32);
    }

    #[test]
    fn test_volumetric_config_default() {
        let config = VolumetricConfig::default();
        assert_eq!(config.num_steps, 32);
        assert!((config.scattering - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_volumetric_uniform_size() {
        assert_eq!(std::mem::size_of::<VolumetricUniform>(), 32);
    }

    #[test]
    fn test_ssr_config_default() {
        let config = SsrConfig::default();
        assert_eq!(config.max_steps, 64);
        assert!((config.thickness - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_ssr_uniform_size() {
        assert_eq!(std::mem::size_of::<SsrUniform>(), 32);
    }
}
