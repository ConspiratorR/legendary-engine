use engine_ecs::entity::Entity;
use engine_ecs::world::World;
use engine_math::{Mat4, Vec3, Vec4};

use crate::camera_system::CameraRenderData;
use crate::culling::CullingBounds;
use crate::instancing::{InstanceBatch, InstanceKey, collect_instance_batches};
use crate::lod::LodConfig;
use crate::occlusion::OcclusionCuller;

/// Result of the culling + LOD pipeline for a single entity.
#[derive(Debug, Clone)]
pub struct CullResult {
    pub entity: Entity,
    pub world_position: Vec3,
    pub model_matrix: Mat4,
    pub mesh_id: u64,
    pub instance_key: InstanceKey,
}

/// Configuration for the culling system.
#[derive(Debug, Clone)]
pub struct CullingConfig {
    /// Maximum draw distance for occlusion culling.
    pub max_draw_distance: f32,
}

impl Default for CullingConfig {
    fn default() -> Self {
        Self {
            max_draw_distance: 500.0,
        }
    }
}

/// Global resource storing the occlusion culler state between frames.
///
/// Insert this into the World before running the culling system.
pub type CullingState = OcclusionCuller;

/// Run the full culling + LOD + instancing pipeline for one camera.
///
/// This function:
/// 1. Collects all entities with `CullingBounds` + `LodConfig` + transform
/// 2. Applies frustum culling (from `camera.frustum`)
/// 3. Applies distance-based occlusion culling
/// 4. Selects LOD based on camera distance
/// 5. Groups results into instanced batches
///
/// # Arguments
///
/// * `world` — ECS world with entity components
/// * `camera` — camera render data (frustum, position)
/// * `model_matrices` — per-entity world model matrix (parallel to entity list)
/// * `material_ids` — per-entity material identifier (parallel to entity list)
///
/// Returns `(visible_entities, instance_batches)`.
pub fn run_culling_pipeline(
    world: &World,
    camera: &CameraRenderData,
    config: &CullingConfig,
    entity_model_matrices: &[(Entity, Mat4)],
    material_ids: &[u64],
) -> (Vec<CullResult>, Vec<InstanceBatch>) {
    // Extract camera world position from the view-projection matrix.
    let cam_world_pos = extract_camera_position(&camera.view_projection);

    let mut occlusion = OcclusionCuller::new(config.max_draw_distance);
    occlusion.set_viewer_position(cam_world_pos);

    let mut results: Vec<CullResult> = Vec::new();

    for (i, (entity, model)) in entity_model_matrices.iter().enumerate() {
        // Extract world position from model matrix
        let world_pos = model.transform_point3(Vec3::ZERO);

        // Get culling bounds
        let bounds = match world.get::<CullingBounds>(*entity) {
            Some(b) => b,
            None => continue,
        };

        // Get LOD config
        let lod_config = world.get::<LodConfig>(*entity);

        // Compute world-space AABB
        let (aabb_min, aabb_max) = bounds.world_aabb(world_pos);

        // Frustum cull
        if !camera.frustum.test_aabb(aabb_min, aabb_max) {
            continue;
        }

        // Occlusion (distance) cull
        if !occlusion.test_aabb(aabb_min, aabb_max) {
            continue;
        }

        // LOD selection
        let mesh_id = if let Some(lod) = lod_config {
            let dist = (world_pos - cam_world_pos).length();
            lod.select(dist).unwrap_or(0)
        } else {
            // No LOD config — use mesh_id 0 as default
            0
        };

        let material_id = material_ids.get(i).copied().unwrap_or(0);

        results.push(CullResult {
            entity: *entity,
            world_position: world_pos,
            model_matrix: *model,
            mesh_id,
            instance_key: InstanceKey::new(mesh_id, material_id),
        });
    }

    // Group into instanced batches
    let keys: Vec<InstanceKey> = results.iter().map(|r| r.instance_key).collect();
    let transforms: Vec<Mat4> = results.iter().map(|r| r.model_matrix).collect();
    let batches = collect_instance_batches(&keys, &transforms);

    (results, batches)
}

/// Extract the camera world position from a view-projection matrix.
///
/// Computes `(VP)^-1` and returns the translation column (the point that
/// maps to clip-space origin).
fn extract_camera_position(vp: &Mat4) -> Vec3 {
    let inv = vp.inverse();
    // The camera position in world space maps to clip-space (0,0,0,1)
    // through VP.  So (VP)^-1 * (0,0,0,1) = last column of (VP)^-1.
    let pos = inv * Vec4::new(0.0, 0.0, 0.0, 1.0);
    if pos.w.abs() > 1e-6 {
        Vec3::new(pos.x / pos.w, pos.y / pos.w, pos.z / pos.w)
    } else {
        Vec3::new(pos.x, pos.y, pos.z)
    }
}

/// ECS system: collect visible entities for all active cameras.
///
/// This is the high-level entry point that should be called each frame.
/// It runs the culling pipeline for each camera in the `CameraStack` and
/// stores the results as a `CullingResults` resource.
pub fn culling_system(
    world: &mut World,
    config: &CullingConfig,
    entity_model_matrices: &[(Entity, Mat4)],
    material_ids: &[u64],
) -> CullingResults {
    let mut all_results = CullingResults::new();

    // Get camera stack (produced by sort_cameras_system)
    let camera_stack = world
        .get_resource::<crate::camera_system::CameraStack>()
        .cloned();

    if let Some(stack) = camera_stack {
        for camera in stack.cameras() {
            let (results, batches) =
                run_culling_pipeline(world, camera, config, entity_model_matrices, material_ids);
            all_results.add_camera_results(camera.entity, results, batches);
        }
    }

    world.insert_resource(all_results.clone());
    all_results
}

/// Resource storing per-camera culling results.
#[derive(Debug, Clone, Default)]
pub struct CullingResults {
    /// Per-camera visible entities and instance batches.
    camera_results: Vec<CameraCullingResult>,
}

/// Culling results for a single camera.
#[derive(Debug, Clone)]
pub struct CameraCullingResult {
    pub camera_entity: Entity,
    pub visible: Vec<CullResult>,
    pub batches: Vec<InstanceBatch>,
}

impl CullingResults {
    pub fn new() -> Self {
        Self {
            camera_results: Vec::new(),
        }
    }

    pub fn add_camera_results(
        &mut self,
        camera: Entity,
        visible: Vec<CullResult>,
        batches: Vec<InstanceBatch>,
    ) {
        self.camera_results.push(CameraCullingResult {
            camera_entity: camera,
            visible,
            batches,
        });
    }

    /// Get results for all cameras.
    pub fn cameras(&self) -> &[CameraCullingResult] {
        &self.camera_results
    }

    /// Get results for a specific camera entity.
    pub fn for_camera(&self, camera: Entity) -> Option<&CameraCullingResult> {
        self.camera_results
            .iter()
            .find(|r| r.camera_entity == camera)
    }

    /// Total visible entity count across all cameras.
    pub fn total_visible(&self) -> usize {
        self.camera_results.iter().map(|r| r.visible.len()).sum()
    }

    /// Total draw calls (instance batches) across all cameras.
    pub fn total_draw_calls(&self) -> usize {
        self.camera_results.iter().map(|r| r.batches.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::{Camera, Projection};
    use crate::camera_system::sort_cameras_system;
    use crate::lod::LodLevel;

    fn setup_world_with_entities() -> (World, Vec<(Entity, Mat4)>, Vec<u64>) {
        let mut world = World::new();

        // Camera entity
        let cam_entity = world.spawn();
        let mut cam = Camera::new(Projection::perspective(1.0, 0.1, 1000.0));
        cam.view = Mat4::IDENTITY;
        cam.is_active = true;
        world.add_component(cam_entity, cam);

        // Visible entity close to camera
        let e1 = world.spawn();
        world.add_component(
            e1,
            CullingBounds::from_half_extent(Vec3::new(1.0, 1.0, 1.0)),
        );
        world.add_component(
            e1,
            LodConfig::new(vec![
                LodLevel::new(0.0, 50.0, 0),
                LodLevel::new(50.0, 200.0, 1),
                LodLevel::new(200.0, f32::MAX, 2),
            ]),
        );

        // Far entity — should be culled by distance
        let e2 = world.spawn();
        world.add_component(
            e2,
            CullingBounds::from_half_extent(Vec3::new(1.0, 1.0, 1.0)),
        );

        let entities = vec![
            (e1, Mat4::from_translation(Vec3::new(0.0, 0.0, -10.0))),
            (e2, Mat4::from_translation(Vec3::new(0.0, 0.0, -600.0))),
        ];
        let materials = vec![0u64, 0];

        (world, entities, materials)
    }

    #[test]
    fn test_culling_system_basic() {
        let (mut world, entities, materials) = setup_world_with_entities();
        sort_cameras_system(&mut world, 800, 600);

        let config = CullingConfig {
            max_draw_distance: 500.0,
        };
        let results = culling_system(&mut world, &config, &entities, &materials);

        // Only e1 should be visible (e2 is 600 units away, beyond 500 draw distance)
        assert_eq!(results.total_visible(), 1);
    }

    #[test]
    fn test_lod_selection_integration() {
        let mut world = World::new();

        let cam_entity = world.spawn();
        let mut cam = Camera::new(Projection::perspective(1.0, 0.1, 1000.0));
        cam.view = Mat4::IDENTITY;
        cam.is_active = true;
        world.add_component(cam_entity, cam);

        let e = world.spawn();
        world.add_component(e, CullingBounds::from_half_extent(Vec3::ONE));
        world.add_component(
            e,
            LodConfig::new(vec![
                LodLevel::new(0.0, 50.0, 100),
                LodLevel::new(50.0, 200.0, 200),
                LodLevel::new(200.0, f32::MAX, 300),
            ]),
        );

        let entities = vec![(e, Mat4::from_translation(Vec3::new(0.0, 0.0, -80.0)))];
        let materials = vec![0u64];

        sort_cameras_system(&mut world, 800, 600);
        let config = CullingConfig::default();
        let results = culling_system(&mut world, &config, &entities, &materials);

        assert_eq!(results.total_visible(), 1);
        let visible = &results.cameras()[0].visible[0];
        // At distance ~80, should select LOD 1 (mesh_id=200)
        assert_eq!(visible.mesh_id, 200);
    }

    #[test]
    fn test_instancing_groups_same_mesh() {
        let mut world = World::new();

        let cam_entity = world.spawn();
        let mut cam = Camera::new(Projection::perspective(1.0, 0.1, 1000.0));
        cam.view = Mat4::IDENTITY;
        cam.is_active = true;
        world.add_component(cam_entity, cam);

        // 5 entities at the same position with the same mesh
        let mut entities = Vec::new();
        let mut materials = Vec::new();
        for i in 0..5 {
            let e = world.spawn();
            world.add_component(e, CullingBounds::from_half_extent(Vec3::ONE));
            entities.push((
                e,
                Mat4::from_translation(Vec3::new(i as f32 * 2.0, 0.0, -10.0)),
            ));
            materials.push(0u64);
        }

        sort_cameras_system(&mut world, 800, 600);
        let config = CullingConfig::default();
        let results = culling_system(&mut world, &config, &entities, &materials);

        assert_eq!(results.total_visible(), 5);
        // All 5 should be in 1 instanced batch (same mesh_id=0, same material=0)
        assert_eq!(results.total_draw_calls(), 1);
        assert_eq!(results.cameras()[0].batches[0].instance_count(), 5);
    }

    #[test]
    fn test_extract_camera_position_identity() {
        let pos = extract_camera_position(&Mat4::IDENTITY);
        assert!((pos.x).abs() < 1e-3);
        assert!((pos.y).abs() < 1e-3);
        assert!((pos.z).abs() < 1e-3);
    }

    #[test]
    fn test_culling_results_api() {
        let results = CullingResults::new();
        assert_eq!(results.total_visible(), 0);
        assert_eq!(results.total_draw_calls(), 0);
        assert!(results.for_camera(Entity::new(0, 0)).is_none());
    }
}
