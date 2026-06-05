//! View frustum for visibility culling.
//!
//! The frustum is defined by 6 planes extracted from the view-projection matrix.
//! It is used to test whether objects are visible before rendering them.

use engine_math::{Mat4, Vec3, Vec4};

/// A view frustum defined by 6 planes.
///
/// Planes: 0=left, 1=right, 2=bottom, 3=top, 4=near, 5=far.
/// Each plane is (a, b, c, d) where ax + by + cz + d = 0.
/// The normal points inward (positive half-space is inside).
///
/// # Usage
///
/// ```rust
/// use engine_render::frustum::Frustum;
/// use engine_math::{Mat4, Vec3};
///
/// let vp = Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, 16.0/9.0, 0.1, 1000.0);
/// let frustum = Frustum::from_view_projection(&vp);
///
/// // Test if a point is visible
/// assert!(frustum.test_sphere(Vec3::new(0.0, 0.0, -5.0), 1.0));
/// ```
#[derive(Debug, Clone)]
pub struct Frustum {
    pub planes: [Vec4; 6],
}

impl Frustum {
    /// Extract frustum planes from a combined view-projection matrix.
    ///
    /// Uses the Gribb-Hartmann method for plane extraction.
    pub fn from_view_projection(vp: &Mat4) -> Self {
        let row0 = vp.row(0);
        let row1 = vp.row(1);
        let row2 = vp.row(2);
        let row3 = vp.row(3);

        let planes = [
            normalize_plane(row3 + row0), // left
            normalize_plane(row3 - row0), // right
            normalize_plane(row3 + row1), // bottom
            normalize_plane(row3 - row1), // top
            normalize_plane(row3 + row2), // near
            normalize_plane(row3 - row2), // far
        ];
        Self { planes }
    }

    /// Test if an AABB (axis-aligned bounding box) intersects the frustum.
    ///
    /// Returns `true` if the AABB is at least partially inside the frustum.
    pub fn test_aabb(&self, min: Vec3, max: Vec3) -> bool {
        for plane in &self.planes {
            let p = Vec3::new(
                if plane.x >= 0.0 { max.x } else { min.x },
                if plane.y >= 0.0 { max.y } else { min.y },
                if plane.z >= 0.0 { max.z } else { min.z },
            );
            if plane.x * p.x + plane.y * p.y + plane.z * p.z + plane.w < 0.0 {
                return false;
            }
        }
        true
    }

    /// Test if a sphere intersects the frustum.
    ///
    /// Returns `true` if the sphere is at least partially inside the frustum.
    pub fn test_sphere(&self, center: Vec3, radius: f32) -> bool {
        for plane in &self.planes {
            let dist = plane.x * center.x + plane.y * center.y + plane.z * center.z + plane.w;
            if dist < -radius {
                return false;
            }
        }
        true
    }
}

/// Normalize a plane so that the normal has unit length.
fn normalize_plane(p: Vec4) -> Vec4 {
    let len = (p.x * p.x + p.y * p.y + p.z * p.z).sqrt();
    if len < 1e-10 {
        return p;
    }
    Vec4::new(p.x / len, p.y / len, p.z / len, p.w / len)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_frustum() -> Frustum {
        Frustum::from_view_projection(&Mat4::IDENTITY)
    }

    #[test]
    fn test_frustum_from_identity_has_six_planes() {
        let f = identity_frustum();
        assert_eq!(f.planes.len(), 6);
    }

    #[test]
    fn test_aabb_inside_frustum() {
        let f = identity_frustum();
        assert!(f.test_aabb(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(0.5, 0.5, 0.5)));
    }

    #[test]
    fn test_aabb_outside_frustum() {
        let f = identity_frustum();
        assert!(!f.test_aabb(Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 1.0)));
    }

    #[test]
    fn test_aabb_partially_inside() {
        let f = identity_frustum();
        assert!(f.test_aabb(Vec3::new(0.5, -0.5, -0.5), Vec3::new(1.5, 0.5, 0.5)));
    }

    #[test]
    fn test_aabb_fully_outside_left() {
        let f = identity_frustum();
        assert!(!f.test_aabb(Vec3::new(-3.0, 0.0, 0.0), Vec3::new(-2.0, 1.0, 1.0)));
    }

    #[test]
    fn test_sphere_inside() {
        let f = identity_frustum();
        assert!(f.test_sphere(Vec3::ZERO, 0.5));
    }

    #[test]
    fn test_sphere_outside() {
        let f = identity_frustum();
        assert!(!f.test_sphere(Vec3::new(3.0, 0.0, 0.0), 0.5));
    }

    #[test]
    fn test_sphere_intersecting() {
        let f = identity_frustum();
        assert!(f.test_sphere(Vec3::new(1.2, 0.0, 0.0), 0.5));
    }

    #[test]
    fn test_sphere_fully_outside() {
        let f = identity_frustum();
        assert!(!f.test_sphere(Vec3::new(0.0, 0.0, 3.0), 0.5));
    }

    #[test]
    fn test_orthographic_frustum() {
        let proj = Mat4::orthographic_rh(0.0, 800.0, 600.0, 0.0, -1.0, 1.0);
        let view = Mat4::IDENTITY;
        let vp = proj * view;
        let f = Frustum::from_view_projection(&vp);
        assert!(f.test_sphere(Vec3::new(400.0, 300.0, 0.0), 1.0));
        assert!(!f.test_sphere(Vec3::new(1000.0, 300.0, 0.0), 1.0));
    }
}
