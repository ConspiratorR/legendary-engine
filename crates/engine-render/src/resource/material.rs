/// PBR material component for mesh rendering.
///
/// Attach to an entity alongside a `Mesh` to define its surface appearance.
#[derive(Debug, Clone)]
pub struct PbrMaterial {
    /// Base color (RGBA).
    pub base_color: [f32; 4],
    /// Metallic factor (0.0 = dielectric, 1.0 = metal).
    pub metallic: f32,
    /// Roughness factor (0.0 = smooth, 1.0 = rough).
    pub roughness: f32,
    /// Ambient occlusion factor.
    pub ao: f32,
    /// Emissive color multiplier.
    pub emissive: [f32; 3],
    /// Optional base color texture handle (u64 key into TextureStore).
    pub base_color_texture: Option<u64>,
    /// Optional normal map texture handle.
    pub normal_texture: Option<u64>,
    /// Optional metallic-roughness texture handle (G=roughness, B=metallic).
    pub metallic_roughness_texture: Option<u64>,
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            base_color: [0.8, 0.8, 0.8, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            ao: 1.0,
            emissive: [0.0; 3],
            base_color_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
        }
    }
}

impl PbrMaterial {
    pub fn new(base_color: [f32; 4], metallic: f32, roughness: f32) -> Self {
        Self {
            base_color,
            metallic,
            roughness,
            ..Default::default()
        }
    }

    pub fn metallic_color(r: f32, g: f32, b: f32) -> Self {
        Self::new([r, g, b, 1.0], 1.0, 0.2)
    }

    pub fn dielectric_color(r: f32, g: f32, b: f32) -> Self {
        Self::new([r, g, b, 1.0], 0.0, 0.5)
    }
}

/// GPU-friendly material uniform (aligned to 48 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialUniform {
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub ao: f32,
    pub _pad0: f32,
    pub emissive: [f32; 3],
    pub _pad1: f32,
}

/// Stride for material uniforms in the buffer, aligned to 256 bytes
/// (the typical `min_uniform_buffer_offset_alignment` limit).
const MATERIAL_STRIDE: u64 = 256;

impl From<&PbrMaterial> for MaterialUniform {
    fn from(m: &PbrMaterial) -> Self {
        Self {
            base_color: m.base_color,
            metallic: m.metallic,
            roughness: m.roughness,
            ao: m.ao,
            _pad0: 0.0,
            emissive: m.emissive,
            _pad1: 0.0,
        }
    }
}

use std::collections::HashMap;

/// Material GPU management, stored as a World resource.
pub struct MaterialStore {
    materials: HashMap<u64, PbrMaterial>,
    bind_groups: HashMap<u64, wgpu::BindGroup>,
    base_color_buffer: wgpu::Buffer,
    material_params_buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    default_texture: wgpu::Texture,
    default_texture_view: wgpu::TextureView,
    default_sampler: wgpu::Sampler,
    next_id: u64,
}

impl MaterialStore {
    pub fn new(device: &wgpu::Device) -> Self {
        // Match the deferred geometry shader's material bind group layout:
        // @group(2) @binding(0) var<uniform> base_color: vec4<f32>;
        // @group(2) @binding(1) var<uniform> material_params: vec4<f32>;
        // @group(2) @binding(2) var albedo_texture: texture_2d<f32>;
        // @group(2) @binding(3) var albedo_sampler: sampler;
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material_bind_group_layout"),
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
                // b1: material_params uniform
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

        // Two separate uniform buffers, each with MATERIAL_STRIDE per material
        let base_color_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("material_base_color_buffer"),
            size: MATERIAL_STRIDE * 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let material_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("material_params_buffer"),
            size: MATERIAL_STRIDE * 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Default white 1x1 texture
        let default_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("material_default_texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let default_texture_view = default_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let default_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("material_default_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            materials: HashMap::new(),
            bind_groups: HashMap::new(),
            base_color_buffer,
            material_params_buffer,
            bind_group_layout,
            default_texture,
            default_texture_view,
            default_sampler,
            next_id: 1,
        }
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Initialize the default texture with a white pixel. Must be called once with a queue.
    pub fn init_default_texture(&self, queue: &wgpu::Queue) {
        let white: &[u8] = &[255, 255, 255, 255];
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.default_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            white,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Add a material, returns material_id.
    pub fn add(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        material: PbrMaterial,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let offset = (id - 1) * MATERIAL_STRIDE;

        // Write base_color (vec4) to base_color_buffer
        queue.write_buffer(
            &self.base_color_buffer,
            offset,
            bytemuck::bytes_of(&material.base_color),
        );

        // Write material_params (metallic, roughness, ao, pad) to material_params_buffer
        let params: [f32; 4] = [material.metallic, material.roughness, material.ao, 0.0];
        queue.write_buffer(
            &self.material_params_buffer,
            offset,
            bytemuck::bytes_of(&params),
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("material_bind_group_{}", id)),
            layout: &self.bind_group_layout,
            entries: &[
                // b0: base_color uniform
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.base_color_buffer,
                        offset,
                        size: Some(std::num::NonZeroU64::new(16).unwrap()),
                    }),
                },
                // b1: material_params uniform
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.material_params_buffer,
                        offset,
                        size: Some(std::num::NonZeroU64::new(16).unwrap()),
                    }),
                },
                // b2: albedo texture (use default white texture)
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.default_texture_view),
                },
                // b3: albedo sampler
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.default_sampler),
                },
            ],
        });

        self.materials.insert(id, material);
        self.bind_groups.insert(id, bind_group);
        id
    }

    pub fn get(&self, id: u64) -> Option<&PbrMaterial> {
        self.materials.get(&id)
    }

    pub fn get_bind_group(&self, id: u64) -> Option<&wgpu::BindGroup> {
        self.bind_groups.get(&id)
    }

    pub fn len(&self) -> usize {
        self.materials.len()
    }

    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_material() {
        let m = PbrMaterial::default();
        assert!((m.base_color[0] - 0.8).abs() < 1e-6);
        assert!((m.metallic).abs() < 1e-6);
        assert!((m.roughness - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_metallic_factory() {
        let m = PbrMaterial::metallic_color(1.0, 0.8, 0.2);
        assert!((m.metallic - 1.0).abs() < 1e-6);
        assert!((m.roughness - 0.2).abs() < 1e-6);
    }

    #[test]
    fn test_material_to_uniform() {
        let m = PbrMaterial::new([1.0, 0.0, 0.0, 1.0], 0.5, 0.3);
        let u = MaterialUniform::from(&m);
        assert_eq!(u.base_color, [1.0, 0.0, 0.0, 1.0]);
        assert!((u.metallic - 0.5).abs() < 1e-6);
        assert!((u.roughness - 0.3).abs() < 1e-6);
    }

    #[test]
    fn test_material_uniform_size() {
        assert_eq!(std::mem::size_of::<MaterialUniform>(), 48);
    }
}
