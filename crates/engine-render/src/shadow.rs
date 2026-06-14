//! Directional light shadow mapping with cascaded shadow map support.

use engine_math::{Mat4, Vec3};

/// Configuration for shadow map generation.
#[derive(Debug, Clone)]
pub struct ShadowMapConfig {
    /// Resolution of the shadow map texture (width and height in pixels).
    pub resolution: u32,
    /// Number of cascades for CSM (1 = single shadow map).
    pub cascade_count: u32,
    /// Depth bias to reduce shadow acne.
    pub shadow_bias: f32,
    /// Normal-based bias to reduce acne on angled surfaces.
    pub normal_bias: f32,
    /// Number of PCF samples for soft shadows (0 = hard shadows).
    pub pcf_samples: u32,
}

impl Default for ShadowMapConfig {
    fn default() -> Self {
        Self {
            resolution: 2048,
            cascade_count: 1,
            shadow_bias: 0.005,
            normal_bias: 0.02,
            pcf_samples: 4,
        }
    }
}

/// Axis-aligned bounding box for scene fitting.
#[derive(Debug, Clone, Copy)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    /// Create a new AABB from min and max corners.
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Compute the center of the AABB.
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Compute the half-extents of the AABB.
    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }
}

/// GPU uniform data for shadow sampling in the PBR shader.
///
/// Layout matches the WGSL `ShadowUniform` struct.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShadowUniform {
    /// Light view-projection matrix (64 bytes).
    pub light_vp: [[f32; 4]; 4],
    /// Depth bias for shadow acne mitigation (4 bytes).
    pub shadow_bias: f32,
    /// Normal bias for angled surface acne mitigation (4 bytes).
    pub normal_bias: f32,
    /// Number of cascades (4 bytes).
    pub cascade_count: u32,
    /// Padding for 16-byte alignment (4 bytes).
    pub _pad: f32,
}

impl Default for ShadowUniform {
    fn default() -> Self {
        Self {
            light_vp: Mat4::IDENTITY.to_cols_array_2d(),
            shadow_bias: 0.005,
            normal_bias: 0.02,
            cascade_count: 1,
            _pad: 0.0,
        }
    }
}

/// Per-cascade data for cascaded shadow maps.
#[derive(Debug, Clone)]
pub struct CascadeShadow {
    /// View-projection matrix for this cascade.
    pub view_proj: Mat4,
    /// Near-to-far split distance for this cascade.
    pub split_distance: f32,
}

/// Shadow render pass managing depth texture, pipeline, and light matrix computation.
pub struct ShadowPass {
    pub config: ShadowMapConfig,
    pub depth_texture: wgpu::Texture,
    pub depth_texture_view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub pipeline: wgpu::RenderPipeline,
}

impl ShadowPass {
    /// Create a new shadow pass with the given device, configuration, and bind group layout.
    pub fn new(
        device: &wgpu::Device,
        config: ShadowMapConfig,
        bind_group_layout: wgpu::BindGroupLayout,
    ) -> Self {
        let depth_texture = Self::create_depth_texture(device, &config);
        let depth_texture_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "pipeline/shadow.wgsl"
            ))),
        });

        let push_constant_ranges = [wgpu::PushConstantRange {
            stages: wgpu::ShaderStages::VERTEX,
            range: 0..64,
        }];

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow_pipeline_layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &push_constant_ranges,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[crate::resource::mesh::MeshVertex::desc()],
            },
            fragment: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Front),
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

        Self {
            config,
            depth_texture,
            depth_texture_view,
            sampler,
            bind_group_layout,
            pipeline,
        }
    }

    /// Create the bind group layout for shadow sampling (depth texture + comparison sampler + uniform).
    pub fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow_bind_group_layout"),
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
        })
    }

    /// Create the depth texture for shadow mapping.
    fn create_depth_texture(device: &wgpu::Device, config: &ShadowMapConfig) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow_depth_texture"),
            size: wgpu::Extent3d {
                width: config.resolution,
                height: config.resolution,
                depth_or_array_layers: config.cascade_count,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }

    /// Create a bind group for the lighting pass that includes the shadow uniform buffer.
    pub fn create_lighting_bind_group(
        &self,
        device: &wgpu::Device,
        uniform_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow_lighting_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.depth_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// Compute the light view-projection matrix for a directional light.
    ///
    /// Uses orthographic projection fitted to the given scene AABB.
    pub fn compute_light_matrices(light_direction: Vec3, scene_aabb: AABB) -> Mat4 {
        let light_dir = light_direction.normalize();
        let scene_center = scene_aabb.center();
        let half_extents = scene_aabb.half_extents();
        let scene_radius = half_extents.length();

        // Position the light camera looking at the scene center
        let light_pos = scene_center - light_dir * scene_radius * 2.0;
        let up = if light_dir.y.abs() > 0.99 {
            Vec3::Z
        } else {
            Vec3::Y
        };

        let view = Mat4::look_at_lh(light_pos, scene_center, up);

        // Orthographic projection sized to the scene
        let left = -scene_radius;
        let right = scene_radius;
        let bottom = -scene_radius;
        let top = scene_radius;
        let near = 0.01;
        let far = scene_radius * 4.0;

        let projection = Mat4::orthographic_lh(left, right, bottom, top, near, far);

        projection * view
    }

    /// Compute cascade split distances using a practical split scheme.
    ///
    /// Returns a vector of `cascade_count` split distances in view space.
    pub fn compute_cascade_splits(
        cascade_count: u32,
        near: f32,
        far: f32,
        lambda: f32,
    ) -> Vec<f32> {
        let mut splits = Vec::with_capacity(cascade_count as usize);
        let n = cascade_count as f32;

        for i in 0..cascade_count {
            let i_f = i as f32;
            // Logarithmic split
            let log_split = near * (far / near).powf((i_f + 1.0) / n);
            // Uniform split
            let uniform_split = near + (far - near) * ((i_f + 1.0) / n);
            // Practical split (lerp between log and uniform), clamped to far
            let split = (lambda * log_split + (1.0 - lambda) * uniform_split).min(far);
            splits.push(split);
        }

        splits
    }

    /// Compute per-cascade view-projection matrices for cascaded shadow maps.
    pub fn compute_cascade_matrices(
        light_direction: Vec3,
        camera_position: Vec3,
        camera_forward: Vec3,
        cascade_splits: &[f32],
    ) -> Vec<CascadeShadow> {
        let light_dir = light_direction.normalize();
        let up = if light_dir.y.abs() > 0.99 {
            Vec3::Z
        } else {
            Vec3::Y
        };

        let mut cascades = Vec::with_capacity(cascade_splits.len());
        let mut prev_split = 0.01;

        for &split_distance in cascade_splits {
            let near = prev_split;
            let far = split_distance;

            // Approximate the frustum slice center and radius
            let slice_center = camera_position + camera_forward * (near + far) * 0.5;
            let slice_radius = (far - near) * 0.5;

            let light_pos = slice_center - light_dir * slice_radius * 2.0;
            let view = Mat4::look_at_lh(light_pos, slice_center, up);

            let left = -slice_radius;
            let right = slice_radius;
            let bottom = -slice_radius;
            let top = slice_radius;
            let proj_near = 0.01;
            let proj_far = slice_radius * 4.0;

            let projection = Mat4::orthographic_lh(left, right, bottom, top, proj_near, proj_far);

            cascades.push(CascadeShadow {
                view_proj: projection * view,
                split_distance,
            });

            prev_split = split_distance;
        }

        cascades
    }

    /// Recreate the depth texture (e.g. after resolution change).
    pub fn resize(&mut self, device: &wgpu::Device, resolution: u32) {
        self.config.resolution = resolution;
        self.depth_texture = Self::create_depth_texture(device, &self.config);
        self.depth_texture_view = self
            .depth_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_math::Vec4;

    #[test]
    fn test_shadow_map_config_defaults() {
        let config = ShadowMapConfig::default();
        assert_eq!(config.resolution, 2048);
        assert_eq!(config.cascade_count, 1);
        assert!((config.shadow_bias - 0.005).abs() < 1e-6);
        assert!((config.normal_bias - 0.02).abs() < 1e-6);
        assert_eq!(config.pcf_samples, 4);
    }

    #[test]
    fn test_shadow_uniform_size() {
        // 64 bytes (mat4) + 4 (f32) + 4 (f32) + 4 (u32) + 4 (pad) = 80 bytes
        assert_eq!(std::mem::size_of::<ShadowUniform>(), 80);
    }

    #[test]
    fn test_shadow_uniform_default() {
        let u = ShadowUniform::default();
        assert!((u.shadow_bias - 0.005).abs() < 1e-6);
        assert!((u.normal_bias - 0.02).abs() < 1e-6);
        assert_eq!(u.cascade_count, 1);
    }

    #[test]
    fn test_light_vp_matrix_valid() {
        let light_dir = Vec3::new(0.3, -1.0, -0.5);
        let aabb = AABB::new(Vec3::new(-10.0, -10.0, -10.0), Vec3::new(10.0, 10.0, 10.0));

        let vp = ShadowPass::compute_light_matrices(light_dir, aabb);

        // The matrix should not be identity (it should have actual values)
        let identity = Mat4::IDENTITY;
        assert_ne!(vp, identity);

        // The matrix should be finite (no NaN/Inf)
        let cols = vp.to_cols_array();
        for v in cols {
            assert!(v.is_finite(), "Matrix contains non-finite value: {v}");
        }

        // Verify the matrix transforms a point in the scene to clip space
        let test_point = Vec4::new(0.0, 0.0, 0.0, 1.0);
        let projected = vp * test_point;
        // The projected point should be in clip space (w > 0)
        assert!(projected.w > 0.0);
    }

    #[test]
    fn test_cascade_split_computation() {
        let splits = ShadowPass::compute_cascade_splits(4, 0.1, 1000.0, 0.5);
        assert_eq!(splits.len(), 4);

        // Splits should be monotonically increasing
        for i in 1..splits.len() {
            assert!(
                splits[i] > splits[i - 1],
                "Cascade splits should be monotonically increasing"
            );
        }

        // First split should be > near
        assert!(splits[0] > 0.1);
        // Last split should be <= far
        assert!(splits[3] <= 1000.0);
    }

    #[test]
    fn test_cascade_split_lambda() {
        // With lambda=1.0 (pure logarithmic), splits should be closer together near the camera
        let log_splits = ShadowPass::compute_cascade_splits(3, 0.1, 1000.0, 1.0);
        // With lambda=0.0 (pure uniform), splits should be evenly spaced
        let uniform_splits = ShadowPass::compute_cascade_splits(3, 0.1, 1000.0, 0.0);

        // Logarithmic splits should have a smaller first split than uniform
        assert!(log_splits[0] < uniform_splits[0]);
    }

    #[test]
    fn test_aabb_center() {
        let aabb = AABB::new(Vec3::new(-10.0, -20.0, -30.0), Vec3::new(10.0, 20.0, 30.0));
        let center = aabb.center();
        assert!((center.x).abs() < 1e-6);
        assert!((center.y).abs() < 1e-6);
        assert!((center.z).abs() < 1e-6);
    }

    #[test]
    fn test_aabb_half_extents() {
        let aabb = AABB::new(Vec3::new(-5.0, -10.0, -15.0), Vec3::new(5.0, 10.0, 15.0));
        let he = aabb.half_extents();
        assert!((he.x - 5.0).abs() < 1e-6);
        assert!((he.y - 10.0).abs() < 1e-6);
        assert!((he.z - 15.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_cascade_matrices() {
        let light_dir = Vec3::new(0.5, -1.0, -0.3);
        let camera_pos = Vec3::new(0.0, 5.0, 10.0);
        let camera_forward = Vec3::new(0.0, 0.0, -1.0);
        let splits = vec![10.0, 50.0, 200.0];

        let cascades =
            ShadowPass::compute_cascade_matrices(light_dir, camera_pos, camera_forward, &splits);

        assert_eq!(cascades.len(), 3);

        for (i, cascade) in cascades.iter().enumerate() {
            assert!(
                cascade
                    .view_proj
                    .to_cols_array()
                    .iter()
                    .all(|v| v.is_finite()),
                "Cascade {i} matrix contains non-finite values"
            );
            assert!(
                (cascade.split_distance - splits[i]).abs() < 1e-6,
                "Cascade {i} split distance mismatch"
            );
        }
    }
}
