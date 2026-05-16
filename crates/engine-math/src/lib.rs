//! Math primitives for the engine.
//!
//! Re-exports `glam` types and provides extension traits for common
//! game engine operations.

pub use glam::{Vec2, Vec3, Vec4, Mat4, Quat, EulerRot};

/// Extension trait for [`Vec3`] providing game-utility methods.
pub trait Vec3Ext {
    /// Extend a 3D vector to a 4D vector with the given `w` component.
    fn extend_with_w(self, w: f32) -> Vec4;
}

impl Vec3Ext for Vec3 {
    fn extend_with_w(self, w: f32) -> Vec4 {
        Vec4::new(self.x, self.y, self.z, w)
    }
}

/// Extension trait for [`Mat4`] providing left-handed look-at.
pub trait Mat4Ext {
    /// Build a left-handed look-at matrix from eye, target, and up vectors.
    fn look_at_lh(eye: Vec3, target: Vec3, up: Vec3) -> Self;
}

impl Mat4Ext for Mat4 {
    fn look_at_lh(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        Mat4::look_at_lh(eye, target, up)
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_vec3_extension() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
    }

    #[test]
    fn test_vec3_normalize() {
        let v = Vec3::new(3.0, 0.0, 0.0);
        let n = v.normalize();
        assert!((n.x - 1.0).abs() < 1e-6);
        assert!((n.y).abs() < 1e-6);
        assert!((n.z).abs() < 1e-6);
    }

    #[test]
    fn test_mat4_identity() {
        let m = Mat4::IDENTITY;
        let v = Vec4::new(1.0, 2.0, 3.0, 1.0);
        assert_eq!(m * v, v);
    }

    #[test]
    fn test_extend_with_w() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        let v4 = v.extend_with_w(1.0);
        assert_eq!(v4, Vec4::new(1.0, 2.0, 3.0, 1.0));
    }

    #[test]
    fn test_look_at_lh() {
        let eye = Vec3::new(0.0, 0.0, 10.0);
        let target = Vec3::ZERO;
        let up = Vec3::Y;
        let m = Mat4::look_at_lh(eye, target, up);
        // The translation row should have eye position (inverted)
        assert_eq!(m.w_axis[0], 0.0);
        assert_eq!(m.w_axis[1], 0.0);
        assert_eq!(m.w_axis[2], 10.0);
    }
}
