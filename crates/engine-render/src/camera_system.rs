use crate::camera::{Camera, RenderTarget};
use crate::frustum::Frustum;
use engine_ecs::entity::Entity;
use engine_ecs::world::World;
use engine_math::{Mat4, Vec3};

/// Per-camera render data extracted for the rendering pipeline.
#[derive(Debug, Clone)]
pub struct CameraRenderData {
    pub entity: Entity,
    pub view_projection: Mat4,
    pub frustum: Frustum,
    pub priority: i32,
    pub viewport_x: u32,
    pub viewport_y: u32,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub render_target: RenderTarget,
    pub clear_color: Option<crate::camera::Color>,
}

/// Sorted camera list, computed each frame.
///
/// Cameras are sorted by priority (ascending — lower priority renders first).
/// This resource should be inserted into the World by a system each frame.
#[derive(Debug, Clone, Default)]
pub struct CameraStack {
    cameras: Vec<CameraRenderData>,
}

impl CameraStack {
    pub fn new() -> Self {
        Self {
            cameras: Vec::new(),
        }
    }

    /// All cameras in render order (lowest priority first).
    pub fn cameras(&self) -> &[CameraRenderData] {
        &self.cameras
    }

    /// Mutable access for custom reordering.
    pub fn cameras_mut(&mut self) -> &mut Vec<CameraRenderData> {
        &mut self.cameras
    }

    /// Find the primary camera (lowest priority, active, renders to screen).
    pub fn primary(&self) -> Option<&CameraRenderData> {
        self.cameras
            .iter()
            .find(|c| matches!(c.render_target, RenderTarget::Screen))
    }
}

/// ECS system: collect all active `Camera` entities, extract their render data,
/// sort by priority, and store in a `CameraStack` resource.
///
/// Requires each camera entity to also have a `Transform`-like component whose
/// world matrix is stored alongside the camera.  For now, `Camera.view` is used
/// directly (caller is responsible for keeping it up-to-date).
///
/// `target_width` / `target_height` are the render target dimensions needed to
/// resolve relative viewports.
pub fn sort_cameras_system(world: &mut World, target_width: u32, target_height: u32) {
    let mut entries: Vec<(Entity, CameraRenderData)> = Vec::new();

    // Iterate all entities that have a Camera component.
    let entities = world.component_entities::<Camera>();
    for idx in entities {
        let entity = Entity::new(idx, 0); // generation 0 for iteration
        let Some(camera) = world.get::<Camera>(entity) else {
            continue;
        };
        if !camera.is_active {
            continue;
        }

        let (vx, vy, vw, vh) = camera.viewport.to_absolute(target_width, target_height);
        let aspect = if vh > 0 { vw as f32 / vh as f32 } else { 1.0 };
        let vp = camera.view_projection(aspect);
        let frustum = Frustum::from_view_projection(&vp);

        let data = CameraRenderData {
            entity,
            view_projection: vp,
            frustum,
            priority: camera.priority,
            viewport_x: vx,
            viewport_y: vy,
            viewport_width: vw,
            viewport_height: vh,
            render_target: camera.render_target.clone(),
            clear_color: camera.clear_color,
        };
        entries.push((entity, data));
    }

    // Sort by priority ascending (lower priority renders first → on top later).
    entries.sort_by_key(|(_, d)| d.priority);

    let cameras: Vec<CameraRenderData> = entries.into_iter().map(|(_, d)| d).collect();
    world.insert_resource(CameraStack { cameras });
}

/// Frustum-cull a list of world-space AABBs against a camera.
///
/// Returns the indices of items that passed the cull test.
pub fn frustum_cull_aabbs(frustum: &Frustum, mins: &[Vec3], maxs: &[Vec3]) -> Vec<usize> {
    mins.iter()
        .zip(maxs.iter())
        .enumerate()
        .filter(|(_, (min, max))| frustum.test_aabb(**min, **max))
        .map(|(i, _)| i)
        .collect()
}

/// Frustum-cull a list of world-space spheres against a camera.
///
/// Returns the indices of items that passed the cull test.
pub fn frustum_cull_spheres(frustum: &Frustum, centers: &[Vec3], radii: &[f32]) -> Vec<usize> {
    centers
        .iter()
        .zip(radii.iter())
        .enumerate()
        .filter(|(_, (c, r))| frustum.test_sphere(**c, **r))
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::{Projection, Viewport};

    fn make_camera_entity(world: &mut World, priority: i32, active: bool) -> Entity {
        let e = world.spawn();
        let mut cam = Camera::new(Projection::perspective(1.0, 0.1, 100.0));
        cam.priority = priority;
        cam.is_active = active;
        world.add_component(e, cam);
        e
    }

    #[test]
    fn test_sort_cameras_by_priority() {
        let mut world = World::new();
        make_camera_entity(&mut world, 10, true);
        make_camera_entity(&mut world, 0, true);
        make_camera_entity(&mut world, 5, true);

        sort_cameras_system(&mut world, 800, 600);

        let stack = world.get_resource::<CameraStack>().unwrap();
        assert_eq!(stack.cameras().len(), 3);
        assert_eq!(stack.cameras()[0].priority, 0);
        assert_eq!(stack.cameras()[1].priority, 5);
        assert_eq!(stack.cameras()[2].priority, 10);
    }

    #[test]
    fn test_inactive_cameras_excluded() {
        let mut world = World::new();
        make_camera_entity(&mut world, 0, true);
        make_camera_entity(&mut world, 1, false);
        make_camera_entity(&mut world, 2, true);

        sort_cameras_system(&mut world, 800, 600);

        let stack = world.get_resource::<CameraStack>().unwrap();
        assert_eq!(stack.cameras().len(), 2);
    }

    #[test]
    fn test_primary_camera_is_first_screen_target() {
        let mut world = World::new();
        make_camera_entity(&mut world, 5, true);
        make_camera_entity(&mut world, 0, true);

        sort_cameras_system(&mut world, 800, 600);

        let stack = world.get_resource::<CameraStack>().unwrap();
        let primary = stack.primary().unwrap();
        assert_eq!(primary.priority, 0);
    }

    #[test]
    fn test_camera_viewport_resolution() {
        let mut world = World::new();
        let e = world.spawn();
        let mut cam = Camera::new(Projection::perspective(1.0, 0.1, 100.0));
        cam.viewport = Viewport::Relative {
            x: 0.0,
            y: 0.0,
            width: 0.5,
            height: 1.0,
        };
        world.add_component(e, cam);

        sort_cameras_system(&mut world, 800, 600);

        let stack = world.get_resource::<CameraStack>().unwrap();
        let data = &stack.cameras()[0];
        assert_eq!(data.viewport_width, 400);
        assert_eq!(data.viewport_height, 600);
    }

    #[test]
    fn test_frustum_cull_aabbs() {
        let proj = Mat4::orthographic_rh(0.0, 800.0, 600.0, 0.0, -1.0, 1.0);
        let vp = proj * Mat4::IDENTITY;
        let frustum = Frustum::from_view_projection(&vp);

        let mins = vec![
            Vec3::new(100.0, 100.0, -0.5),
            Vec3::new(1000.0, 100.0, -0.5),
        ];
        let maxs = vec![Vec3::new(200.0, 200.0, 0.5), Vec3::new(1100.0, 200.0, 0.5)];

        let visible = frustum_cull_aabbs(&frustum, &mins, &maxs);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0], 0);
    }

    #[test]
    fn test_frustum_cull_spheres() {
        let proj = Mat4::orthographic_rh(0.0, 800.0, 600.0, 0.0, -1.0, 1.0);
        let vp = proj * Mat4::IDENTITY;
        let frustum = Frustum::from_view_projection(&vp);

        let centers = vec![Vec3::new(400.0, 300.0, 0.0), Vec3::new(2000.0, 300.0, 0.0)];
        let radii = vec![10.0, 10.0];

        let visible = frustum_cull_spheres(&frustum, &centers, &radii);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0], 0);
    }

    #[test]
    fn test_empty_world_no_cameras() {
        let mut world = World::new();
        sort_cameras_system(&mut world, 800, 600);
        let stack = world.get_resource::<CameraStack>().unwrap();
        assert!(stack.cameras().is_empty());
        assert!(stack.primary().is_none());
    }
}
