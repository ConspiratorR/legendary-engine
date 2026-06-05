//! # engine-math
//!
//! Core math types and operations for the RustEngine.
//!
//! Provides vector, matrix, and quaternion types backed by glam.
//! Includes extension traits for additional functionality like
//! interpolation, angle conversions, and geometric operations.
//!
//! ## Quick Start
//!
//! ```rust
//! use engine_math::{Vec3, Mat4, Quat};
//!
//! let position = Vec3::new(1.0, 2.0, 3.0);
//! let rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
//! let transform = Mat4::from_rotation_translation(rotation, position);
//! ```

pub mod error;
pub use error::MathError;

pub use glam::{EulerRot, Mat4, Quat, Vec2, Vec3, Vec4};

/// Extension trait for [`Vec3`] providing game-utility methods.
pub trait Vec3Ext {
    /// Extend a 3D vector to a 4D vector with the given `w` component.
    ///
    /// # Arguments
    ///
    /// * `w` - The w component for the resulting 4D vector.
    ///
    /// # Returns
    ///
    /// A [`Vec4`] with the original x, y, z values and the specified `w`.
    ///
    /// # Examples
    ///
    /// ```
    /// use engine_math::{Vec3, Vec3Ext, Vec4};
    ///
    /// let v = Vec3::new(1.0, 2.0, 3.0);
    /// let v4 = v.extend_with_w(1.0);
    /// assert_eq!(v4, Vec4::new(1.0, 2.0, 3.0, 1.0));
    /// ```
    fn extend_with_w(self, w: f32) -> Vec4;
}

impl Vec3Ext for Vec3 {
    fn extend_with_w(self, w: f32) -> Vec4 {
        Vec4::new(self.x, self.y, self.z, w)
    }
}

/// Extension trait for [`Mat4`] providing left-handed look-at utilities.
pub trait Mat4Ext {
    /// Build a left-handed look-at matrix from eye, target, and up vectors.
    ///
    /// # Arguments
    ///
    /// * `eye` - Position of the camera.
    /// * `target` - Point the camera looks at.
    /// * `up` - Up direction (typically `Vec3::Y`).
    ///
    /// # Returns
    ///
    /// A [`Mat4`] view matrix for a left-handed coordinate system.
    ///
    /// # Examples
    ///
    /// ```
    /// use engine_math::{Vec3, Mat4, Mat4Ext};
    ///
    /// let view = Mat4::look_at_lh(
    ///     Vec3::new(0.0, 0.0, -5.0),
    ///     Vec3::ZERO,
    ///     Vec3::Y,
    /// );
    /// ```
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
