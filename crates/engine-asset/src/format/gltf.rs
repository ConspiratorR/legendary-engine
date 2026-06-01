use crate::types;
use gltf::mesh::util::ReadIndices;
use std::path::Path;

/// Error type for glTF loading.
#[derive(Debug, thiserror::Error)]
pub enum GltfError {
    #[error("Failed to load glTF: {0}")]
    Gltf(#[from] gltf::Error),
    #[error("Unsupported component type")]
    UnsupportedComponentType,
    #[error("Missing required accessor: {0}")]
    MissingAccessor(&'static str),
    #[error("Unsupported index format")]
    UnsupportedIndexFormat,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result of loading a glTF file.
pub struct GltfData {
    pub meshes: Vec<GltfMesh>,
}

/// A single mesh from a glTF file with geometry data.
pub struct GltfMesh {
    pub name: String,
    pub vertices: Vec<GltfVertex>,
    pub indices: Vec<u32>,
}

/// A vertex from a glTF mesh.
#[derive(Debug, Clone, Copy)]
pub struct GltfVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
}

impl GltfVertex {
    pub fn zero() -> Self {
        Self {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            tex_coord: [0.0; 2],
        }
    }
}

impl From<GltfVertex> for types::Vertex {
    fn from(v: GltfVertex) -> Self {
        Self {
            position: v.position,
            normal: v.normal,
            tex_coord: v.tex_coord,
        }
    }
}

/// Load all mesh geometry from a glTF or GLB file.
///
/// Returns mesh data with positions, normals, and texture coordinates.
/// Materials and textures are not loaded — only geometry.
pub fn load_gltf(path: &Path) -> Result<GltfData, GltfError> {
    let (document, buffers, _images) = gltf::import(path)?;

    let mut meshes = Vec::new();

    for mesh in document.meshes() {
        let mesh_name = mesh.name().unwrap_or("unnamed").to_string();

        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(|d| &**d));

            // Positions (required)
            let positions = reader
                .read_positions()
                .ok_or(GltfError::MissingAccessor("positions"))?;

            // Normals (optional — generate default if missing)
            let normals: Vec<[f32; 3]> = match reader.read_normals() {
                Some(n) => n.collect(),
                None => positions.clone().map(|_| [0.0, 1.0, 0.0]).collect(),
            };

            // Texture coordinates (optional — default to [0,0])
            let tex_coords: Vec<[f32; 2]> = match reader.read_tex_coords(0) {
                Some(tc) => tc.into_f32().collect(),
                None => positions.clone().map(|_| [0.0, 0.0]).collect(),
            };

            // Build vertices
            let vertices: Vec<GltfVertex> = positions
                .zip(normals)
                .zip(tex_coords)
                .map(|((pos, n), uv)| GltfVertex {
                    position: pos,
                    normal: n,
                    tex_coord: uv,
                })
                .collect();

            // Indices
            let indices: Vec<u32> = match reader.read_indices() {
                Some(ReadIndices::U8(iter)) => iter.map(|i| i as u32).collect(),
                Some(ReadIndices::U16(iter)) => iter.map(|i| i as u32).collect(),
                Some(ReadIndices::U32(iter)) => iter.collect(),
                None => (0..vertices.len() as u32).collect(),
            };

            meshes.push(GltfMesh {
                name: mesh_name.clone(),
                vertices,
                indices,
            });
        }
    }

    Ok(GltfData { meshes })
}

/// Convenience: load a glTF file and convert to engine `types::Mesh` objects.
pub fn load_gltf_as_meshes(path: &Path) -> Result<Vec<types::Mesh>, GltfError> {
    let data = load_gltf(path)?;
    let mut result = Vec::new();

    for (i, gm) in data.meshes.into_iter().enumerate() {
        let id = if gm.name.is_empty() {
            format!("{}_mesh_{}", path.display(), i)
        } else {
            gm.name.clone()
        };

        let vertices: Vec<types::Vertex> = gm.vertices.into_iter().map(Into::into).collect();

        result.push(types::Mesh {
            id,
            vertices,
            indices: gm.indices,
        });
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gltf_vertex_zero() {
        let v = GltfVertex::zero();
        assert_eq!(v.position, [0.0; 3]);
        assert_eq!(v.normal, [0.0, 1.0, 0.0]);
        assert_eq!(v.tex_coord, [0.0; 2]);
    }

    #[test]
    fn test_gltf_vertex_to_types_vertex() {
        let gv = GltfVertex {
            position: [1.0, 2.0, 3.0],
            normal: [0.0, 1.0, 0.0],
            tex_coord: [0.5, 0.5],
        };
        let tv: types::Vertex = gv.into();
        assert_eq!(tv.position, [1.0, 2.0, 3.0]);
        assert_eq!(tv.normal, [0.0, 1.0, 0.0]);
        assert_eq!(tv.tex_coord, [0.5, 0.5]);
    }

    #[test]
    fn test_load_nonexistent_gltf_fails() {
        let result = load_gltf(Path::new("nonexistent.gltf"));
        assert!(result.is_err());
    }
}
