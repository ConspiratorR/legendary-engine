use bytemuck::{Pod, Zeroable};
use std::collections::HashMap;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl MeshVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 12,
                    shader_location: 1,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 24,
                    shader_location: 2,
                },
            ],
        }
    }
}

pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: Option<wgpu::Buffer>,
    pub num_vertices: u32,
    pub num_indices: u32,
}

impl Mesh {
    pub fn new(device: &wgpu::Device, vertices: &[MeshVertex], indices: Option<&[u32]>) -> Self {
        use wgpu::util::DeviceExt;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_vertex_buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let (index_buffer, num_indices) = if let Some(indices) = indices {
            let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("mesh_index_buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            (Some(buf), indices.len() as u32)
        } else {
            (None, 0)
        };
        Self {
            vertex_buffer,
            index_buffer,
            num_vertices: vertices.len() as u32,
            num_indices,
        }
    }
}

/// GPU Mesh 缓存，作为 World 资源存储
pub struct MeshStore {
    meshes: HashMap<u64, Mesh>,
    next_id: u64,
}

impl Default for MeshStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MeshStore {
    pub fn new() -> Self {
        Self {
            meshes: HashMap::new(),
            next_id: 1,
        }
    }

    /// 上传顶点/索引数据到 GPU，返回 mesh_id
    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        vertices: &[MeshVertex],
        indices: Option<&[u32]>,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mesh = Mesh::new(device, vertices, indices);
        self.meshes.insert(id, mesh);
        id
    }

    pub fn get(&self, id: u64) -> Option<&Mesh> {
        self.meshes.get(&id)
    }

    pub fn remove(&mut self, id: u64) -> Option<Mesh> {
        self.meshes.remove(&id)
    }

    pub fn len(&self) -> usize {
        self.meshes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.meshes.is_empty()
    }
}
