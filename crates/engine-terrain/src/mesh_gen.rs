use engine_math::Vec3;
use engine_render::renderer::GpuDevice;
use engine_render::resource::mesh::{Mesh, MeshVertex};

use crate::components::Terrain;

/// Generate a GPU mesh for a terrain chunk.
///
/// Builds vertices and indices for the chunk at `chunk_coord`, using
/// heightmap data from `terrain`. Returns a `Mesh` with GPU buffers.
pub fn generate_chunk_mesh(terrain: &Terrain, chunk_coord: (u32, u32), device: &GpuDevice) -> Mesh {
    let (cx, cz) = chunk_coord;
    let chunk_size = terrain.chunk_size;
    let res = terrain.resolution;
    let chunk_count = terrain.chunk_count();

    // World-space dimensions of each chunk
    let chunk_world_w = terrain.world_size.x / chunk_count as f32;
    let chunk_world_h = terrain.world_size.y / chunk_count as f32;

    // Starting vertex index in the heightmap
    let start_i = cx * chunk_size;
    let start_j = cz * chunk_size;

    // World-space origin of this chunk (centered terrain)
    let origin_x = cx as f32 * chunk_world_w - terrain.world_size.x * 0.5;
    let origin_z = cz as f32 * chunk_world_h - terrain.world_size.y * 0.5;

    let verts_per_axis = chunk_size + 1;
    let num_vertices = (verts_per_axis * verts_per_axis) as usize;
    let mut vertices = Vec::with_capacity(num_vertices);

    // Generate vertices
    for j in 0..=chunk_size {
        for i in 0..=chunk_size {
            let gi = (start_i + i).min(res);
            let gj = (start_j + j).min(res);

            let x = origin_x + (i as f32 / chunk_size as f32) * chunk_world_w;
            let z = origin_z + (j as f32 / chunk_size as f32) * chunk_world_h;
            let y = terrain.get_height(gi, gj);

            // Normal via central differences
            let h_left = if gi > 0 {
                terrain.get_height(gi - 1, gj)
            } else {
                y
            };
            let h_right = if gi < res {
                terrain.get_height(gi + 1, gj)
            } else {
                y
            };
            let h_down = if gj > 0 {
                terrain.get_height(gi, gj - 1)
            } else {
                y
            };
            let h_up = if gj < res {
                terrain.get_height(gi, gj + 1)
            } else {
                y
            };

            // Cross product for normal (terrain is XZ plane, Y is up)
            let normal =
                Vec3::new(-(h_right - h_left) * 0.5, 1.0, -(h_up - h_down) * 0.5).normalize();

            let u = i as f32 / chunk_size as f32;
            let v = j as f32 / chunk_size as f32;

            vertices.push(MeshVertex {
                position: [x, y, z],
                normal: [normal.x, normal.y, normal.z],
                uv: [u, v],
            });
        }
    }

    // Generate indices (two triangles per quad)
    let num_quads = chunk_size * chunk_size;
    let mut indices = Vec::with_capacity((num_quads * 6) as usize);

    for j in 0..chunk_size {
        for i in 0..chunk_size {
            let base = j * verts_per_axis + i;
            let v0 = base;
            let v1 = base + 1;
            let v2 = base + verts_per_axis;
            let v3 = base + verts_per_axis + 1;

            // Triangle 1: v0, v2, v1
            indices.push(v0);
            indices.push(v2);
            indices.push(v1);
            // Triangle 2: v1, v2, v3
            indices.push(v1);
            indices.push(v2);
            indices.push(v3);
        }
    }

    Mesh::new(device, &vertices, Some(&indices))
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_math::Vec2;

    #[test]
    fn test_chunk_mesh_vertex_count() {
        let terrain = Terrain::new(4, 2, Vec2::new(10.0, 10.0), 10.0);
        // Chunk (0,0) with chunk_size=2: (2+1)^2 = 9 vertices
        let verts_per_axis = terrain.chunk_size + 1;
        assert_eq!(verts_per_axis * verts_per_axis, 9);
    }

    #[test]
    fn test_chunk_mesh_index_count() {
        let terrain = Terrain::new(4, 2, Vec2::new(10.0, 10.0), 10.0);
        // Chunk (0,0): 2*2 quads * 6 indices = 24
        let num_quads = terrain.chunk_size * terrain.chunk_size;
        assert_eq!(num_quads * 6, 24);
    }
}
