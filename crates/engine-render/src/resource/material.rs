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
    uniform_buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    next_id: u64,
}

impl MaterialStore {
    pub fn new(device: &wgpu::Device) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material_bind_group_layout"),
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

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("material_uniform_buffer"),
            size: 48 * 64, // 48 bytes per material, up to 64 materials
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            materials: HashMap::new(),
            bind_groups: HashMap::new(),
            uniform_buffer,
            bind_group_layout,
            next_id: 1,
        }
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
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

        let uniform = MaterialUniform::from(&material);
        let offset = (id - 1) * 48;
        queue.write_buffer(&self.uniform_buffer, offset, bytemuck::bytes_of(&uniform));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("material_bind_group_{}", id)),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &self.uniform_buffer,
                    offset,
                    size: Some(std::num::NonZeroU64::new(48).unwrap()),
                }),
            }],
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
