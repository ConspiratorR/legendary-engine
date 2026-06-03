use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;
use engine_ecs::world::World;
use engine_render::renderer::GpuDevice;

use crate::components::{Terrain, TerrainChunk};

/// Plugin that registers terrain systems with the ECS.
pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(terrain_mesh_update_system);
        app.add_system(vegetation_update_system);
    }
}

/// System that rebuilds meshes for dirty terrain chunks.
///
/// Queries all entities with `TerrainChunk` that are marked dirty,
/// generates new meshes, and clears the dirty flag.
fn terrain_mesh_update_system(world: &mut World) {
    // Find terrain data
    let terrain_data: Option<Terrain> = {
        let terrain_entities = world.component_entities::<Terrain>();
        terrain_entities.first().and_then(|&idx| {
            world
                .get::<Terrain>(engine_ecs::entity::Entity::new(idx, 0))
                .cloned()
        })
    };

    let terrain = match terrain_data {
        Some(t) => t,
        None => return,
    };

    // Collect dirty chunk entity indices
    let dirty_indices: Vec<u32> = world
        .component_entities::<TerrainChunk>()
        .into_iter()
        .filter(|&idx| {
            world
                .get::<TerrainChunk>(engine_ecs::entity::Entity::new(idx, 0))
                .map(|c| c.dirty)
                .unwrap_or(false)
        })
        .collect();

    if dirty_indices.is_empty() {
        return;
    }

    // Clone device out of world to release the immutable borrow
    let device = {
        let device_ref = world.get_resource::<GpuDevice>();
        device_ref.cloned()
    };
    let device = match device {
        Some(d) => d,
        None => return,
    };

    for entity_idx in dirty_indices {
        // Get chunk coord before mutable borrow
        let chunk_coord = world
            .get::<TerrainChunk>(engine_ecs::entity::Entity::new(entity_idx, 0))
            .map(|c| c.chunk_coord);

        if let Some(coord) = chunk_coord {
            let new_mesh = crate::mesh_gen::generate_chunk_mesh(&terrain, coord, &device);

            if let Some(chunk) =
                world.get_mut::<TerrainChunk>(engine_ecs::entity::Entity::new(entity_idx, 0))
            {
                chunk.mesh = Some(new_mesh);
                chunk.dirty = false;
            }
        }
    }
}

/// System that regenerates vegetation instances when marked dirty.
fn vegetation_update_system(world: &mut World) {
    let needs_regen = world
        .get_resource::<crate::components::VegetationData>()
        .map(|v| v.dirty)
        .unwrap_or(false);

    if !needs_regen {
        return;
    }

    let terrain_entities = world.component_entities::<Terrain>();
    let terrain = terrain_entities.first().and_then(|&idx| {
        world
            .get::<Terrain>(engine_ecs::entity::Entity::new(idx, 0))
            .cloned()
    });

    if let Some(terrain) = terrain
        && let Some(vegetation) = world.get_resource_mut::<crate::components::VegetationData>()
    {
        crate::vegetation::regenerate_vegetation(&terrain, vegetation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{SplatMap, TerrainTextureLayers, VegetationData};
    use engine_math::Vec2;

    #[test]
    fn test_terrain_plugin_systems_register() {
        let mut app = AppBuilder::new();
        app.add_plugin(TerrainPlugin);
        // Should not panic — systems are registered
    }

    #[test]
    fn test_full_terrain_workflow() {
        let mut world = World::new();

        // Create terrain entity
        let terrain_entity = world.spawn();
        let terrain = Terrain::new(4, 2, Vec2::new(10.0, 10.0), 10.0);
        world.add_component(terrain_entity, terrain);

        // Create chunk entities
        let chunk_count = 2; // 4/2 = 2 chunks per axis
        for cz in 0..chunk_count {
            for cx in 0..chunk_count {
                let chunk_entity = world.spawn();
                world.add_component(chunk_entity, TerrainChunk::new((cx, cz)));
            }
        }

        // Verify terrain was created
        let terrain = world.get::<Terrain>(terrain_entity).unwrap();
        assert_eq!(terrain.resolution, 4);

        // Verify chunks were created
        let chunk_entities = world.component_entities::<TerrainChunk>();
        assert_eq!(chunk_entities.len(), 4);
    }

    #[test]
    fn test_splat_map_resource() {
        let mut world = World::new();
        world.insert_resource(SplatMap::new(4));
        let sm = world.get_resource::<SplatMap>().unwrap();
        assert_eq!(sm.resolution, 4);
    }

    #[test]
    fn test_texture_layers_resource() {
        let mut world = World::new();
        world.insert_resource(TerrainTextureLayers::default());
        let layers = world.get_resource::<TerrainTextureLayers>().unwrap();
        assert_eq!(layers.layers.len(), 1);
    }

    #[test]
    fn test_vegetation_data_resource() {
        let mut world = World::new();
        world.insert_resource(VegetationData::default());
        let veg = world.get_resource::<VegetationData>().unwrap();
        assert!(veg.types.is_empty());
    }
}
