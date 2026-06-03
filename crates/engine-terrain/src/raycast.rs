use engine_math::{Mat4, Vec2, Vec3, Vec4};

use crate::components::Terrain;

/// A ray in world space.
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

/// Result of a successful terrain raycast.
#[derive(Debug, Clone, Copy)]
pub struct RaycastHit {
    /// World-space intersection point.
    pub point: Vec3,
    /// Heightmap grid coordinate (i, j) at the hit.
    pub grid_coord: (u32, u32),
    /// Distance from ray origin to the hit point.
    pub distance: f32,
}

/// Convert screen position to a world-space ray using the inverse view-projection matrix.
pub fn screen_to_ray(screen_pos: Vec2, viewport_size: Vec2, view_proj_inv: Mat4) -> Ray {
    let (origin, direction) = unproject(screen_pos, viewport_size, view_proj_inv);
    Ray { origin, direction }
}

/// Unproject a screen position into a world-space origin and direction.
pub fn unproject(screen_pos: Vec2, viewport_size: Vec2, view_proj_inv: Mat4) -> (Vec3, Vec3) {
    let ndc_x = (screen_pos.x / viewport_size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (screen_pos.y / viewport_size.y) * 2.0;

    let near = view_proj_inv * Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
    let far = view_proj_inv * Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

    let near_world = Vec3::new(near.x, near.y, near.z) / near.w;
    let far_world = Vec3::new(far.x, far.y, far.z) / far.w;

    let direction = (far_world - near_world).normalize();

    (near_world, direction)
}

/// Test a ray against an axis-aligned bounding box.
///
/// Returns `(t_near, t_far)` if the ray intersects, where both values are
/// non-negative distances along the ray direction.
pub fn ray_aabb_intersect(ray: &Ray, aabb_min: Vec3, aabb_max: Vec3) -> Option<(f32, f32)> {
    let inv_dir = Vec3::new(
        1.0 / ray.direction.x,
        1.0 / ray.direction.y,
        1.0 / ray.direction.z,
    );

    let t1 = (aabb_min - ray.origin) * inv_dir;
    let t2 = (aabb_max - ray.origin) * inv_dir;

    let t_min = t1.min(t2);
    let t_max = t1.max(t2);

    let t_near = t_min.x.max(t_min.y).max(t_min.z);
    let t_far = t_max.x.min(t_max.y).min(t_max.z);

    if t_near <= t_far && t_far >= 0.0 {
        Some((t_near.max(0.0), t_far))
    } else {
        None
    }
}

/// Sample terrain height at an arbitrary world-space XZ position using bilinear interpolation.
pub fn sample_terrain_height(terrain: &Terrain, world_x: f32, world_z: f32) -> f32 {
    let half_w = terrain.world_size.x * 0.5;
    let half_h = terrain.world_size.y * 0.5;

    let gx = ((world_x + half_w) / terrain.world_size.x * terrain.resolution as f32)
        .max(0.0)
        .min(terrain.resolution as f32);
    let gz = ((world_z + half_h) / terrain.world_size.y * terrain.resolution as f32)
        .max(0.0)
        .min(terrain.resolution as f32);

    let i0 = gx.floor() as u32;
    let j0 = gz.floor() as u32;
    let i1 = (i0 + 1).min(terrain.resolution);
    let j1 = (j0 + 1).min(terrain.resolution);

    let fx = gx - i0 as f32;
    let fz = gz - j0 as f32;

    let h00 = terrain.get_height(i0, j0);
    let h10 = terrain.get_height(i1, j0);
    let h01 = terrain.get_height(i0, j1);
    let h11 = terrain.get_height(i1, j1);

    let h0 = h00 + (h10 - h00) * fx;
    let h1 = h01 + (h11 - h01) * fx;

    h0 + (h1 - h0) * fz
}

/// Refine a ray-terrain intersection using binary search between two parametric distances.
fn binary_search_intersection(
    terrain: &Terrain,
    ray: &Ray,
    t_start: f32,
    t_end: f32,
) -> Option<RaycastHit> {
    let mut lo = t_start;
    let mut hi = t_end;

    for _ in 0..20 {
        let mid = (lo + hi) * 0.5;
        let point = ray.origin + ray.direction * mid;
        let height = sample_terrain_height(terrain, point.x, point.z);

        if point.y >= height {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    let t = (lo + hi) * 0.5;
    let point = ray.origin + ray.direction * t;

    let half_w = terrain.world_size.x * 0.5;
    let half_h = terrain.world_size.y * 0.5;
    let gx = ((point.x + half_w) / terrain.world_size.x * terrain.resolution as f32) as u32;
    let gz = ((point.z + half_h) / terrain.world_size.y * terrain.resolution as f32) as u32;

    Some(RaycastHit {
        point,
        grid_coord: (gx.min(terrain.resolution), gz.min(terrain.resolution)),
        distance: t,
    })
}

/// Cast a ray against the terrain and return the closest intersection.
///
/// Performs AABB rejection first, then steps along the ray to detect
/// where it crosses below the terrain surface, and refines with binary search.
pub fn raycast_terrain(terrain: &Terrain, ray: Ray, max_distance: f32) -> Option<RaycastHit> {
    let half_w = terrain.world_size.x * 0.5;
    let half_h = terrain.world_size.y * 0.5;

    let min_h = terrain
        .heightmap
        .iter()
        .copied()
        .fold(f32::INFINITY, f32::min)
        * terrain.height_scale;
    let max_h = terrain
        .heightmap
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max)
        * terrain.height_scale;

    let aabb_min = Vec3::new(-half_w, min_h - 1.0, -half_h);
    let aabb_max = Vec3::new(half_w, max_h + 1.0, half_h);

    let (t_enter, t_exit) = ray_aabb_intersect(&ray, aabb_min, aabb_max)?;

    let t_start = t_enter.max(0.0);
    let t_end = t_exit.min(max_distance);

    if t_start > t_end {
        return None;
    }

    let step_size = (terrain.world_size.x / terrain.resolution as f32) * 0.5;
    let num_steps = ((t_end - t_start) / step_size).ceil() as u32;

    let mut prev_t = t_start;
    let prev_point = ray.origin + ray.direction * prev_t;
    let prev_height = sample_terrain_height(terrain, prev_point.x, prev_point.z);
    let mut prev_above = prev_point.y >= prev_height;

    for step in 1..=num_steps {
        let t = t_start + step as f32 * step_size;
        let point = ray.origin + ray.direction * t;

        if point.x < -half_w || point.x > half_w || point.z < -half_h || point.z > half_h {
            prev_above = false;
            continue;
        }

        let height = sample_terrain_height(terrain, point.x, point.z);
        let above = point.y >= height;

        if prev_above && !above {
            return binary_search_intersection(terrain, &ray, prev_t, t);
        }

        prev_t = t;
        prev_above = above;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_math::Vec2;

    #[test]
    fn test_ray_aabb_hit() {
        let ray = Ray {
            origin: Vec3::new(0.0, 10.0, 0.0),
            direction: Vec3::new(0.0, -1.0, 0.0),
        };
        let result =
            ray_aabb_intersect(&ray, Vec3::new(-5.0, -1.0, -5.0), Vec3::new(5.0, 1.0, 5.0));
        assert!(result.is_some());
        let (t_near, t_far) = result.unwrap();
        assert!(t_near >= 0.0);
        assert!(t_far >= t_near);
    }

    #[test]
    fn test_ray_aabb_miss() {
        let ray = Ray {
            origin: Vec3::new(10.0, 10.0, 0.0),
            direction: Vec3::new(0.0, -1.0, 0.0),
        };
        let result =
            ray_aabb_intersect(&ray, Vec3::new(-5.0, -1.0, -5.0), Vec3::new(5.0, 1.0, 5.0));
        assert!(result.is_none());
    }

    #[test]
    fn test_sample_flat_terrain_height() {
        let terrain = Terrain::new(4, 2, Vec2::new(10.0, 10.0), 10.0);
        let h = sample_terrain_height(&terrain, 0.0, 0.0);
        assert!((h - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_raycast_flat_terrain_hit() {
        let terrain = Terrain::new(4, 2, Vec2::new(10.0, 10.0), 10.0);
        let ray = Ray {
            origin: Vec3::new(0.0, 10.0, 0.0),
            direction: Vec3::new(0.0, -1.0, 0.0),
        };
        let hit = raycast_terrain(&terrain, ray, 100.0);
        assert!(hit.is_some());
        let hit = hit.unwrap();
        assert!((hit.point.y).abs() < 0.1);
        assert!(hit.distance > 0.0);
    }

    #[test]
    fn test_raycast_outside_bounds_miss() {
        let terrain = Terrain::new(4, 2, Vec2::new(10.0, 10.0), 10.0);
        let ray = Ray {
            origin: Vec3::new(20.0, 10.0, 20.0),
            direction: Vec3::new(0.0, -1.0, 0.0),
        };
        let hit = raycast_terrain(&terrain, ray, 100.0);
        assert!(hit.is_none());
    }
}
