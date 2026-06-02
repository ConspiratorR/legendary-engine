use engine_math::Vec3;

/// Spatial bounding volume for culling, attached as an ECS component.
///
/// Defines an axis-aligned bounding box (AABB) in local space around an entity.
/// The renderer transforms these bounds to world space before frustum/occlusion tests.
#[derive(Debug, Clone, Copy)]
pub struct CullingBounds {
    /// Center of the AABB in local space.
    pub center: Vec3,
    /// Half-extents of the AABB in each axis.
    pub half_extent: Vec3,
}

impl CullingBounds {
    /// Create bounds from a center and half-extent.
    pub fn new(center: Vec3, half_extent: Vec3) -> Self {
        Self {
            center,
            half_extent,
        }
    }

    /// Create symmetric bounds centered at the origin.
    pub fn from_half_extent(half_extent: Vec3) -> Self {
        Self {
            center: Vec3::ZERO,
            half_extent,
        }
    }

    /// Create bounds from min/max corners of an AABB.
    pub fn from_min_max(min: Vec3, max: Vec3) -> Self {
        Self {
            center: (min + max) * 0.5,
            half_extent: (max - min) * 0.5,
        }
    }

    /// Compute the world-space AABB given a world-space translation.
    pub fn world_aabb(&self, world_pos: Vec3) -> (Vec3, Vec3) {
        let center = world_pos + self.center;
        (center - self.half_extent, center + self.half_extent)
    }

    /// Bounding sphere radius (conservative — uses the longest half-extent).
    pub fn bounding_radius(&self) -> f32 {
        self.half_extent.max_element()
    }

    /// Compute the world-space bounding sphere given a world-space translation.
    pub fn world_sphere(&self, world_pos: Vec3) -> (Vec3, f32) {
        let center = world_pos + self.center;
        (center, self.bounding_radius())
    }
}

impl Default for CullingBounds {
    fn default() -> Self {
        Self {
            center: Vec3::ZERO,
            half_extent: Vec3::ONE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_min_max() {
        let b = CullingBounds::from_min_max(Vec3::new(-1.0, -2.0, -3.0), Vec3::new(1.0, 2.0, 3.0));
        assert!((b.center.x).abs() < 1e-6);
        assert!((b.center.y).abs() < 1e-6);
        assert!((b.center.z).abs() < 1e-6);
        assert!((b.half_extent.x - 1.0).abs() < 1e-6);
        assert!((b.half_extent.y - 2.0).abs() < 1e-6);
        assert!((b.half_extent.z - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_world_aabb() {
        let b = CullingBounds::from_half_extent(Vec3::new(1.0, 1.0, 1.0));
        let (min, max) = b.world_aabb(Vec3::new(5.0, 0.0, 0.0));
        assert!((min.x - 4.0).abs() < 1e-6);
        assert!((max.x - 6.0).abs() < 1e-6);
    }

    #[test]
    fn test_bounding_radius() {
        let b = CullingBounds::from_half_extent(Vec3::new(1.0, 2.0, 3.0));
        assert!((b.bounding_radius() - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_world_sphere() {
        let b = CullingBounds::from_half_extent(Vec3::ONE);
        let (center, radius) = b.world_sphere(Vec3::new(10.0, 0.0, 0.0));
        assert!((center.x - 10.0).abs() < 1e-6);
        assert!((radius - 1.0).abs() < 1e-6);
    }
}
