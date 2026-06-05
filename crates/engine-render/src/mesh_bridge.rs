use crate::renderer::GpuDevice;
use crate::resource::mesh::{MeshStore, MeshVertex};
use engine_asset::asset::{Handle, HandleId};
use engine_asset::types::Mesh as AssetMesh;
use engine_ecs::entity::Entity;
use engine_ecs::world::World;
use std::collections::HashSet;

/// 将实体链接到 GPU Mesh
#[derive(Debug, Clone)]
pub struct MeshRenderer {
    pub mesh_id: u64,
    pub material_id: u64,
    pub cast_shadow: bool,
}

/// 将 asset Vertex 转换为 render MeshVertex
fn to_mesh_vertex(v: &engine_asset::types::Vertex) -> MeshVertex {
    MeshVertex {
        position: v.position,
        normal: v.normal,
        uv: v.tex_coord,
    }
}

/// Mesh 上传系统：检测新 Handle<Mesh>，上传 GPU，添加 MeshRenderer
pub fn mesh_upload_system(world: &mut World) {
    // Find entities with Handle<Mesh> but no MeshRenderer
    let handle_indices = world.component_entities::<Handle<AssetMesh>>();

    // Collect entities that need uploading (index, handle clone, mesh data)
    let mut to_upload: Vec<(u32, Vec<MeshVertex>, Vec<u32>)> = Vec::new();

    // Track already-processed handle IDs to avoid duplicate uploads
    let mut seen_handles: HashSet<HandleId> = HashSet::new();

    for idx in handle_indices {
        let entity = Entity::new(idx, 0);

        // Skip entities that already have a MeshRenderer
        if world.get::<MeshRenderer>(entity).is_some() {
            continue;
        }

        // Get the Handle<Mesh> and extract mesh data
        let Some(handle) = world.get::<Handle<AssetMesh>>(entity) else {
            continue;
        };

        let handle_id = HandleId::from_handle(handle);
        if seen_handles.contains(&handle_id) {
            continue;
        }
        seen_handles.insert(handle_id);

        let asset_mesh = handle.get();
        let vertices: Vec<MeshVertex> = asset_mesh.vertices.iter().map(to_mesh_vertex).collect();
        let indices = asset_mesh.indices.clone();

        to_upload.push((idx, vertices, indices));
    }

    if to_upload.is_empty() {
        return;
    }

    // Get GPU device resource
    let device = match world.get_resource::<GpuDevice>() {
        Some(d) => d.clone(),
        None => return,
    };

    // Ensure MeshStore exists
    if world.get_resource::<MeshStore>().is_none() {
        world.insert_resource(MeshStore::new());
    }

    // Upload meshes and add MeshRenderer components
    for (idx, vertices, indices) in to_upload {
        let mesh_id = {
            let mesh_store = world
                .get_resource_mut::<MeshStore>()
                .expect("MeshStore must be inserted before uploading meshes");
            let indices_opt = if indices.is_empty() {
                None
            } else {
                Some(indices.as_slice())
            };
            mesh_store.upload(&device, &vertices, indices_opt)
        };

        let entity = Entity::new(idx, 0);
        world.add_component(
            entity,
            MeshRenderer {
                mesh_id,
                material_id: 0,
                cast_shadow: true,
            },
        );
    }
}
