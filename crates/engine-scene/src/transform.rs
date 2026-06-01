use engine_math::{Mat4, Quat, Vec3};

/// Local transform of a scene node (translation, rotation, scale).
#[derive(Debug, Clone)]
pub struct Transform {
    /// Position relative to the parent.
    pub translation: Vec3,
    /// Rotation relative to the parent.
    pub rotation: Quat,
    /// Scale relative to the parent.
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    /// Create a transform at the given position with identity rotation and unit scale.
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self {
            translation: Vec3::new(x, y, z),
            ..Default::default()
        }
    }

    /// Convert to a 4×4 transformation matrix.
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

/// The world-space transform of a scene node, computed by
/// [`SceneManager::sync_transforms`](super::scene_manager::SceneManager::sync_transforms).
#[derive(Debug, Clone)]
pub struct GlobalTransform(pub Mat4);

impl Default for GlobalTransform {
    fn default() -> Self {
        Self(Mat4::IDENTITY)
    }
}

#[cfg(test)]
mod tests {
    use crate::transform::Transform;
    use engine_math::Vec3;

    #[test]
    fn test_transform_identity() {
        let t = Transform::default();
        assert_eq!(t.translation, Vec3::ZERO);
        assert_eq!(t.scale, Vec3::ONE);
    }

    #[test]
    fn test_transform_from_xyz() {
        let t = Transform::from_xyz(1.0, 2.0, 3.0);
        assert_eq!(t.translation.x, 1.0);
    }
}
