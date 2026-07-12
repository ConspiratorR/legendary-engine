// crates/engine-core/src/transform.rs

use crate::gameobject::Component;
use engine_math::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::any::Any;

/// Space for transformations (like Unity's Space).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Space {
    /// Relative to parent (local space).
    Self_,
    /// Relative to world (world space).
    World,
}

/// Transform component (like Unity's Transform).
/// Stores position, rotation, scale relative to parent.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Transform {
    /// Local position (relative to parent).
    pub local_position: Vec3,
    /// Local rotation (relative to parent).
    pub local_rotation: Quat,
    /// Local scale (relative to parent).
    pub local_scale: Vec3,

    // Cached world transform (computed by sync system)
    #[serde(skip)]
    world_position: Vec3,
    #[serde(skip)]
    world_rotation: Quat,
    #[serde(skip)]
    world_scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            local_position: Vec3::ZERO,
            local_rotation: Quat::IDENTITY,
            local_scale: Vec3::ONE,
            world_position: Vec3::ZERO,
            world_rotation: Quat::IDENTITY,
            world_scale: Vec3::ONE,
        }
    }
}

impl Component for Transform {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Transform {
    /// Create a transform at the given position.
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self {
            local_position: Vec3::new(x, y, z),
            ..Default::default()
        }
    }

    /// Create a transform at the given position with rotation and scale.
    pub fn from_position_rotation_scale(position: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            local_position: position,
            local_rotation: rotation,
            local_scale: scale,
            world_position: position,
            world_rotation: rotation,
            world_scale: scale,
        }
    }

    /// Get world position (like Unity's Transform.position).
    pub fn position(&self) -> Vec3 {
        self.world_position
    }

    /// Set world position (like Unity's Transform.position).
    pub fn set_position(&mut self, position: Vec3) {
        self.world_position = position;
        // Note: local_position will be computed by sync system
    }

    /// Get world rotation (like Unity's Transform.rotation).
    pub fn rotation(&self) -> Quat {
        self.world_rotation
    }

    /// Set world rotation (like Unity's Transform.rotation).
    pub fn set_rotation(&mut self, rotation: Quat) {
        self.world_rotation = rotation;
    }

    /// Get world scale (like Unity's Transform.lossyScale).
    pub fn lossy_scale(&self) -> Vec3 {
        self.world_scale
    }

    /// Get local position (like Unity's Transform.localPosition).
    pub fn local_position(&self) -> Vec3 {
        self.local_position
    }

    /// Set local position (like Unity's Transform.localPosition).
    pub fn set_local_position(&mut self, position: Vec3) {
        self.local_position = position;
    }

    /// Get local rotation (like Unity's Transform.localRotation).
    pub fn local_rotation(&self) -> Quat {
        self.local_rotation
    }

    /// Set local rotation (like Unity's Transform.localRotation).
    pub fn set_local_rotation(&mut self, rotation: Quat) {
        self.local_rotation = rotation;
    }

    /// Get local scale (like Unity's Transform.localScale).
    pub fn local_scale(&self) -> Vec3 {
        self.local_scale
    }

    /// Set local scale (like Unity's Transform.localScale).
    pub fn set_local_scale(&mut self, scale: Vec3) {
        self.local_scale = scale;
    }

    /// Get forward direction (like Unity's Transform.forward).
    pub fn forward(&self) -> Vec3 {
        self.world_rotation * Vec3::Z
    }

    /// Get right direction (like Unity's Transform.right).
    pub fn right(&self) -> Vec3 {
        self.world_rotation * Vec3::X
    }

    /// Get up direction (like Unity's Transform.up).
    pub fn up(&self) -> Vec3 {
        self.world_rotation * Vec3::Y
    }

    /// Transform a point from local to world space (like Unity's Transform.TransformPoint).
    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        self.world_position + self.world_rotation * (point * self.world_scale)
    }

    /// Transform a point from world to local space (like Unity's Transform.InverseTransformPoint).
    pub fn inverse_transform_point(&self, point: Vec3) -> Vec3 {
        let relative = point - self.world_position;
        let inv_rotation = self.world_rotation.inverse();
        let inv_scale = Vec3::new(
            1.0 / self.world_scale.x,
            1.0 / self.world_scale.y,
            1.0 / self.world_scale.z,
        );
        inv_rotation * (relative * inv_scale)
    }

    /// Transform a direction from local to world space (like Unity's Transform.TransformDirection).
    pub fn transform_direction(&self, direction: Vec3) -> Vec3 {
        self.world_rotation * direction
    }

    /// Transform a direction from world to local space (like Unity's Transform.InverseTransformDirection).
    pub fn inverse_transform_direction(&self, direction: Vec3) -> Vec3 {
        self.world_rotation.inverse() * direction
    }

    /// Look at a target position (like Unity's Transform.LookAt).
    pub fn look_at(&mut self, target: Vec3) {
        let direction = (target - self.world_position).normalize();
        if direction.length_squared() > 0.0001 {
            self.world_rotation = Quat::from_rotation_arc(Vec3::Z, direction);
        }
    }

    /// Rotate around a point (like Unity's Transform.RotateAround).
    pub fn rotate_around(&mut self, point: Vec3, axis: Vec3, angle: f32) {
        let rotation = Quat::from_axis_angle(axis, angle.to_radians());
        let offset = self.world_position - point;
        self.world_position = point + rotation * offset;
        self.world_rotation = rotation * self.world_rotation;
    }

    /// Translate in world/local space (like Unity's Transform.Translate).
    pub fn translate(&mut self, translation: Vec3, space: Space) {
        match space {
            Space::World => {
                self.world_position += translation;
            }
            Space::Self_ => {
                self.world_position += self.world_rotation * translation;
            }
        }
    }

    /// Get the local-to-world matrix.
    pub fn local_to_world_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(
            self.world_scale,
            self.world_rotation,
            self.world_position,
        )
    }

    /// Get the world-to-local matrix.
    pub fn world_to_local_matrix(&self) -> Mat4 {
        self.local_to_world_matrix().inverse()
    }

    /// Update the cached world transform from parent (called by sync system).
    pub fn update_world_transform(
        &mut self,
        parent_world_position: Vec3,
        parent_world_rotation: Quat,
        parent_world_scale: Vec3,
    ) {
        self.world_position = parent_world_position
            + parent_world_rotation * (self.local_position * parent_world_scale);
        self.world_rotation = parent_world_rotation * self.local_rotation;
        self.world_scale = parent_world_scale * self.local_scale;
    }

    /// Update the cached world transform for root (no parent).
    pub fn update_world_transform_root(&mut self) {
        self.world_position = self.local_position;
        self.world_rotation = self.local_rotation;
        self.world_scale = self.local_scale;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_default() {
        let t = Transform::default();
        assert_eq!(t.local_position, Vec3::ZERO);
        assert_eq!(t.local_rotation, Quat::IDENTITY);
        assert_eq!(t.local_scale, Vec3::ONE);
    }

    #[test]
    fn test_transform_from_xyz() {
        let t = Transform::from_xyz(1.0, 2.0, 3.0);
        assert_eq!(t.local_position, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_transform_forward() {
        let t = Transform::default();
        assert_eq!(t.forward(), Vec3::Z);
    }

    #[test]
    fn test_transform_look_at() {
        let mut t = Transform::from_xyz(0.0, 0.0, 0.0);
        t.look_at(Vec3::new(1.0, 0.0, 0.0));

        let forward = t.forward();
        assert!((forward.x - 1.0).abs() < 0.001);
        assert!(forward.y.abs() < 0.001);
        assert!(forward.z.abs() < 0.001);
    }

    #[test]
    fn test_transform_translate() {
        let mut t = Transform::default();
        t.translate(Vec3::new(1.0, 0.0, 0.0), Space::World);

        assert_eq!(t.world_position, Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_transform_update_world() {
        let mut t = Transform::from_xyz(1.0, 0.0, 0.0);
        t.update_world_transform(Vec3::new(5.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE);

        assert_eq!(t.world_position, Vec3::new(6.0, 0.0, 0.0));
    }
}
