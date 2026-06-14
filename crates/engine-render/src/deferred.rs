//! Deferred rendering pipeline with G-Buffer and lighting pass.

use bytemuck::{Pod, Zeroable};

/// Push constant data for the deferred geometry pass (128 bytes).
///
/// Contains model matrix and normal matrix for transforming vertices
/// and normals to world space.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GeometryPassUniform {
    pub model_matrix: [[f32; 4]; 4],
    pub normal_matrix: [[f32; 4]; 4],
}

impl Default for GeometryPassUniform {
    fn default() -> Self {
        Self {
            model_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            normal_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }
}

/// G-Buffer textures for deferred rendering.
///
/// Holds the render targets used during the geometry pass:
/// - albedo: base color RGB + alpha (Rgba8UnormSrgb)
/// - normal: world-space normal packed to \[0,1\] (Rgba16Float)
/// - position: world-space position XYZ (Rgba16Float)
/// - material: metallic R, roughness G, ao B (Rgba8Unorm)
/// - depth: depth buffer (Depth32Float)
pub struct GBufferTextures {
    pub albedo: wgpu::Texture,
    pub albedo_view: wgpu::TextureView,
    pub normal: wgpu::Texture,
    pub normal_view: wgpu::TextureView,
    pub position: wgpu::Texture,
    pub position_view: wgpu::TextureView,
    pub material: wgpu::Texture,
    pub material_view: wgpu::TextureView,
    pub depth: wgpu::Texture,
    pub depth_view: wgpu::TextureView,
}

impl GBufferTextures {
    /// Create G-Buffer textures with the given resolution.
    fn create(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let albedo = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gbuffer_albedo"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let albedo_view = albedo.create_view(&wgpu::TextureViewDescriptor::default());

        let normal = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gbuffer_normal"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let normal_view = normal.create_view(&wgpu::TextureViewDescriptor::default());

        let position = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gbuffer_position"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let position_view = position.create_view(&wgpu::TextureViewDescriptor::default());

        let material = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gbuffer_material"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let material_view = material.create_view(&wgpu::TextureViewDescriptor::default());

        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gbuffer_depth"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            albedo,
            albedo_view,
            normal,
            normal_view,
            position,
            position_view,
            material,
            material_view,
            depth,
            depth_view,
        }
    }
}

/// G-Buffer managing all deferred rendering textures.
pub struct GBuffer {
    pub textures: GBufferTextures,
    pub width: u32,
    pub height: u32,
}

impl GBuffer {
    /// Create a new G-Buffer with the given resolution.
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let textures = GBufferTextures::create(device, width, height);
        Self {
            textures,
            width,
            height,
        }
    }

    /// Recreate all G-Buffer textures with a new resolution.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.textures = GBufferTextures::create(device, width, height);
    }

    /// Create the bind group layout for sampling G-Buffer textures in the lighting pass.
    ///
    /// Layout (5 bindings):
    /// - t0: albedo texture (Fragment)
    /// - t1: normal texture (Fragment)
    /// - t2: position texture (Fragment)
    /// - t3: material texture (Fragment)
    /// - s4: sampler (Fragment)
    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gbuffer_bind_group_layout"),
            entries: &[
                // t0: albedo
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
                // t1: normal
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
                // t2: position
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
                // t3: material
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // s4: sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }

    /// Create a bind group for sampling the G-Buffer textures.
    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("gbuffer_sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gbuffer_bind_group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.textures.albedo_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.textures.normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.textures.position_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.textures.material_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        })
    }
}

/// Deferred rendering pass with geometry and lighting pipelines.
pub struct DeferredPass {
    pub geometry_pipeline: wgpu::RenderPipeline,
    pub lighting_pipeline: wgpu::RenderPipeline,
    pub gbuffer_bind_group_layout: wgpu::BindGroupLayout,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
    pub light_bind_group_layout: wgpu::BindGroupLayout,
}

/// Push constant size for the geometry pass (128 bytes: two mat4x4).
pub const GEOMETRY_PUSH_CONSTANT_SIZE: u32 = 128;

impl DeferredPass {
    /// Create a new deferred pass with geometry and lighting pipelines.
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        shadow_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        // Geometry pass shader
        let geometry_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("deferred_geometry_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "pipeline/deferred_geometry.wgsl"
            ))),
        });

        // Lighting pass shader
        let lighting_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("deferred_lighting_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "pipeline/deferred_lighting.wgsl"
            ))),
        });

        // Camera bind group layout (group 0)
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("deferred_camera_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // Light bind group layout (group 1)
        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("deferred_light_bind_group_layout"),
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

        // G-Buffer bind group layout (group 2 for lighting pass)
        let gbuffer_bind_group_layout = Self::create_gbuffer_bind_group_layout(device);

        // Material bind group layout for geometry pass (group 2)
        let material_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("deferred_material_bind_group_layout"),
                entries: &[
                    // b0: base_color uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // b1: material_params uniform (metallic, roughness, ao, pad)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // b2: albedo texture
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
                    // b3: albedo sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // Geometry pass push constants
        let geometry_push_constants = [wgpu::PushConstantRange {
            stages: wgpu::ShaderStages::VERTEX,
            range: 0..GEOMETRY_PUSH_CONSTANT_SIZE,
        }];

        // Geometry pipeline layout
        let geometry_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("deferred_geometry_layout"),
            bind_group_layouts: &[
                &camera_bind_group_layout,
                &light_bind_group_layout,
                &material_bind_group_layout,
            ],
            push_constant_ranges: &geometry_push_constants,
        });

        // Geometry pipeline: outputs to 4 render targets
        let geometry_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("deferred_geometry_pipeline"),
            layout: Some(&geometry_layout),
            vertex: wgpu::VertexState {
                module: &geometry_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[crate::resource::mesh::MeshVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &geometry_shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[
                    // albedo
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    // normal
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    // position
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    // material
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Lighting pipeline layout
        let lighting_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("deferred_lighting_layout"),
            bind_group_layouts: &[
                &camera_bind_group_layout,
                &light_bind_group_layout,
                &gbuffer_bind_group_layout,
                shadow_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        // Lighting pipeline: full-screen triangle, single output
        let lighting_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("deferred_lighting_pipeline"),
            layout: Some(&lighting_layout),
            vertex: wgpu::VertexState {
                module: &lighting_shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &lighting_shader,
                entry_point: Some("fs_lighting"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
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

        Self {
            geometry_pipeline,
            lighting_pipeline,
            gbuffer_bind_group_layout,
            camera_bind_group_layout,
            light_bind_group_layout,
        }
    }

    /// Create the bind group layout for sampling G-Buffer textures.
    fn create_gbuffer_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("deferred_gbuffer_bind_group_layout"),
            entries: &[
                // t0: albedo
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
                // t1: normal
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
                // t2: position
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
                // t3: material
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // s4: sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a wgpu device for testing.
    fn create_test_device() -> wgpu::Device {
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
        let (device, _queue) = pollster::block_on(adapter.request_device(
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
        .unwrap();
        device
    }

    #[test]
    fn test_geometry_pass_uniform_size() {
        // Two mat4x4 = 128 bytes
        assert_eq!(std::mem::size_of::<GeometryPassUniform>(), 128);
    }

    #[test]
    fn test_geometry_pass_uniform_default() {
        let u = GeometryPassUniform::default();
        // Model matrix diagonal should be 1.0
        assert!((u.model_matrix[0][0] - 1.0).abs() < 1e-6);
        assert!((u.model_matrix[1][1] - 1.0).abs() < 1e-6);
        assert!((u.model_matrix[2][2] - 1.0).abs() < 1e-6);
        assert!((u.model_matrix[3][3] - 1.0).abs() < 1e-6);
        // Normal matrix diagonal should be 1.0
        assert!((u.normal_matrix[0][0] - 1.0).abs() < 1e-6);
        assert!((u.normal_matrix[1][1] - 1.0).abs() < 1e-6);
        assert!((u.normal_matrix[2][2] - 1.0).abs() < 1e-6);
        assert!((u.normal_matrix[3][3] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_gbuffer_creation() {
        let device = create_test_device();
        let width = 1280;
        let height = 720;
        let gbuffer = GBuffer::new(&device, width, height);

        assert_eq!(gbuffer.width, width);
        assert_eq!(gbuffer.height, height);

        // Verify texture dimensions
        assert_eq!(gbuffer.textures.albedo.width(), width);
        assert_eq!(gbuffer.textures.albedo.height(), height);
        assert_eq!(gbuffer.textures.normal.width(), width);
        assert_eq!(gbuffer.textures.normal.height(), height);
        assert_eq!(gbuffer.textures.position.width(), width);
        assert_eq!(gbuffer.textures.position.height(), height);
        assert_eq!(gbuffer.textures.material.width(), width);
        assert_eq!(gbuffer.textures.material.height(), height);
        assert_eq!(gbuffer.textures.depth.width(), width);
        assert_eq!(gbuffer.textures.depth.height(), height);

        // Verify formats
        assert_eq!(
            gbuffer.textures.albedo.format(),
            wgpu::TextureFormat::Rgba8UnormSrgb
        );
        assert_eq!(
            gbuffer.textures.normal.format(),
            wgpu::TextureFormat::Rgba16Float
        );
        assert_eq!(
            gbuffer.textures.position.format(),
            wgpu::TextureFormat::Rgba16Float
        );
        assert_eq!(
            gbuffer.textures.material.format(),
            wgpu::TextureFormat::Rgba8Unorm
        );
        assert_eq!(
            gbuffer.textures.depth.format(),
            wgpu::TextureFormat::Depth32Float
        );
    }

    #[test]
    fn test_gbuffer_resize() {
        let device = create_test_device();
        let mut gbuffer = GBuffer::new(&device, 1280, 720);

        assert_eq!(gbuffer.width, 1280);
        assert_eq!(gbuffer.height, 720);

        gbuffer.resize(&device, 1920, 1080);

        assert_eq!(gbuffer.width, 1920);
        assert_eq!(gbuffer.height, 1080);
        assert_eq!(gbuffer.textures.albedo.width(), 1920);
        assert_eq!(gbuffer.textures.albedo.height(), 1080);
        assert_eq!(gbuffer.textures.normal.width(), 1920);
        assert_eq!(gbuffer.textures.normal.height(), 1080);
        assert_eq!(gbuffer.textures.position.width(), 1920);
        assert_eq!(gbuffer.textures.position.height(), 1080);
        assert_eq!(gbuffer.textures.material.width(), 1920);
        assert_eq!(gbuffer.textures.material.height(), 1080);
        assert_eq!(gbuffer.textures.depth.width(), 1920);
        assert_eq!(gbuffer.textures.depth.height(), 1080);
    }

    #[test]
    fn test_gbuffer_bind_group_layout() {
        let device = create_test_device();
        let layout = GBuffer::bind_group_layout(&device);

        // Verify layout works by creating a bind group
        let gbuffer = GBuffer::new(&device, 64, 64);
        let _bind_group = gbuffer.create_bind_group(&device, &layout);
    }

    #[test]
    fn test_deferred_pass_creation() {
        let device = create_test_device();
        let format = wgpu::TextureFormat::Bgra8UnormSrgb;

        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("test_shadow_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
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

        let deferred = DeferredPass::new(&device, format, &shadow_layout);

        // Verify pipelines were created (they hold valid handles)
        // We can't directly check pipeline validity, but creation succeeding
        // means the shader compiled and layouts were valid
        let _ = deferred.geometry_pipeline;
        let _ = deferred.lighting_pipeline;
    }
}
