use engine_math::Vec3;

use crate::frustum::Frustum;

/// Distance-based occlusion culler.
///
/// Culls objects beyond a maximum draw distance and, optionally, applies
/// a fog-based fade where objects near the far plane are still rendered
/// but could be faded/hidden by atmospheric effects.
///
/// This is the simplest form of occlusion culling — no GPU occlusion queries
/// or hierarchical Z-buffer.  For scenes with heavy overdraw it should be
/// combined with a depth prepass or HiZ-based approach in the future.
#[derive(Debug, Clone)]
pub struct OcclusionCuller {
    /// World-space position of the viewer (camera).
    pub viewer_position: Vec3,
    /// Maximum draw distance. Objects beyond this are culled.
    pub max_draw_distance: f32,
    /// Squared max draw distance (cached for fast comparisons).
    max_draw_distance_sq: f32,
}

impl OcclusionCuller {
    /// Create a new culler with the given maximum draw distance.
    pub fn new(max_draw_distance: f32) -> Self {
        Self {
            viewer_position: Vec3::ZERO,
            max_draw_distance,
            max_draw_distance_sq: max_draw_distance * max_draw_distance,
        }
    }

    /// Update the viewer (camera) position each frame.
    pub fn set_viewer_position(&mut self, pos: Vec3) {
        self.viewer_position = pos;
    }

    /// Change the maximum draw distance.
    pub fn set_max_draw_distance(&mut self, distance: f32) {
        self.max_draw_distance = distance;
        self.max_draw_distance_sq = distance * distance;
    }

    /// Test if a world-space sphere is within draw distance.
    ///
    /// Returns `true` if any part of the sphere could be visible.
    pub fn test_sphere(&self, center: Vec3, radius: f32) -> bool {
        let dist_sq = (center - self.viewer_position).length_squared();
        dist_sq <= (self.max_draw_distance + radius).powi(2)
    }

    /// Test if a world-space AABB is within draw distance.
    ///
    /// Uses the AABB's bounding sphere for the distance check.
    pub fn test_aabb(&self, min: Vec3, max: Vec3) -> bool {
        let center = (min + max) * 0.5;
        let half_extent = (max - min) * 0.5;
        let radius = half_extent.max_element();
        self.test_sphere(center, radius)
    }

    /// Combined frustum + occlusion test for a world-space AABB.
    ///
    /// Returns `true` only if the AABB passes both tests.
    pub fn test_aabb_full(&self, frustum: &Frustum, min: Vec3, max: Vec3) -> bool {
        if !frustum.test_aabb(min, max) {
            return false;
        }
        self.test_aabb(min, max)
    }

    /// Combined frustum + occlusion test for a world-space sphere.
    pub fn test_sphere_full(&self, frustum: &Frustum, center: Vec3, radius: f32) -> bool {
        if !frustum.test_sphere(center, radius) {
            return false;
        }
        self.test_sphere(center, radius)
    }

    /// Batch cull: given world-space AABBs, return indices of visible ones.
    ///
    /// Applies both frustum and distance culling.
    pub fn cull_aabbs(&self, frustum: &Frustum, mins: &[Vec3], maxs: &[Vec3]) -> Vec<usize> {
        mins.iter()
            .zip(maxs.iter())
            .enumerate()
            .filter(|(_, (min, max))| self.test_aabb_full(frustum, **min, **max))
            .map(|(i, _)| i)
            .collect()
    }

    /// Batch cull: given world-space spheres, return indices of visible ones.
    pub fn cull_spheres(&self, frustum: &Frustum, centers: &[Vec3], radii: &[f32]) -> Vec<usize> {
        centers
            .iter()
            .zip(radii.iter())
            .enumerate()
            .filter(|(_, (c, r))| self.test_sphere_full(frustum, **c, **r))
            .map(|(i, _)| i)
            .collect()
    }
}

impl Default for OcclusionCuller {
    fn default() -> Self {
        Self::new(1000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frustum::Frustum;
    use engine_math::Mat4;

    fn ortho_frustum() -> Frustum {
        let proj = Mat4::orthographic_rh(0.0, 800.0, 600.0, 0.0, -1.0, 1.0);
        Frustum::from_view_projection(&(proj * Mat4::IDENTITY))
    }

    #[test]
    fn test_within_draw_distance() {
        let mut culler = OcclusionCuller::new(100.0);
        culler.set_viewer_position(Vec3::ZERO);
        assert!(culler.test_sphere(Vec3::new(50.0, 0.0, 0.0), 1.0));
    }

    #[test]
    fn test_beyond_draw_distance() {
        let mut culler = OcclusionCuller::new(100.0);
        culler.set_viewer_position(Vec3::ZERO);
        assert!(!culler.test_sphere(Vec3::new(200.0, 0.0, 0.0), 1.0));
    }

    #[test]
    fn test_at_boundary_with_radius() {
        let mut culler = OcclusionCuller::new(100.0);
        culler.set_viewer_position(Vec3::ZERO);
        // Center at 100, radius 5 => max extent at 105 > 100, still visible
        assert!(culler.test_sphere(Vec3::new(100.0, 0.0, 0.0), 5.0));
    }

    #[test]
    fn test_aabb_full_frustum_reject() {
        let culler = OcclusionCuller::new(1000.0);
        let frustum = ortho_frustum();
        // Outside ortho frustum (x > 800) but within draw distance
        assert!(!culler.test_aabb_full(
            &frustum,
            Vec3::new(900.0, 300.0, 0.0),
            Vec3::new(950.0, 350.0, 0.0),
        ));
    }

    #[test]
    fn test_aabb_full_both_pass() {
        let culler = OcclusionCuller::new(1000.0);
        let frustum = ortho_frustum();
        assert!(culler.test_aabb_full(
            &frustum,
            Vec3::new(100.0, 100.0, -0.5),
            Vec3::new(200.0, 200.0, 0.5),
        ));
    }

    #[test]
    fn test_batch_cull_aabbs() {
        let culler = OcclusionCuller::new(500.0);
        let frustum = ortho_frustum();
        let mins = vec![
            Vec3::new(100.0, 100.0, -0.5),
            Vec3::new(1000.0, 100.0, -0.5), // outside frustum
            Vec3::new(300.0, 300.0, -0.5),
        ];
        let maxs = vec![
            Vec3::new(200.0, 200.0, 0.5),
            Vec3::new(1100.0, 200.0, 0.5),
            Vec3::new(400.0, 400.0, 0.5),
        ];
        let visible = culler.cull_aabbs(&frustum, &mins, &maxs);
        assert_eq!(visible.len(), 2);
        assert!(visible.contains(&0));
        assert!(visible.contains(&2));
    }

    #[test]
    fn test_set_max_draw_distance() {
        let mut culler = OcclusionCuller::new(100.0);
        culler.set_viewer_position(Vec3::ZERO);
        assert!(!culler.test_sphere(Vec3::new(150.0, 0.0, 0.0), 1.0));
        culler.set_max_draw_distance(200.0);
        assert!(culler.test_sphere(Vec3::new(150.0, 0.0, 0.0), 1.0));
    }
}
