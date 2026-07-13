//! Unity Transform — built-in component for position, rotation, scale, and hierarchy.
//!
//! Maps to `UnityEngine.Transform` in Unity's documentation.
//!
//! # Unity Documentation
//! <https://docs.unity3d.com/ScriptReference/Transform.html>
//!
//! Transform is **mandatory** on every GameObject. It cannot be removed.
//! Parent/child relationships are part of Transform.
//!
//! ## Key Concepts
//! - Every GameObject always has exactly one Transform
//! - Transform stores local position, rotation, scale relative to parent
//! - Transform caches world position, rotation, scale (computed by sync system)
//! - Children inherit parent's position, rotation, scale
//! - The topmost object (no parent) is the "root"

use engine_math::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

use crate::gameobject::GameObjectHandle;

/// Space for transformations (matches Unity's `Space` enum).
///
/// # Unity Documentation
/// <https://docs.unity3d.com/ScriptReference/Space.html>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Space {
    /// Relative to parent (local space) — matches `Space.Self`.
    Self_,
    /// Relative to world (world space) — matches `Space.World`.
    World,
}

/// Built-in Transform component (matches `UnityEngine.Transform`).
///
/// # Unity Documentation
/// <https://docs.unity3d.com/ScriptReference/Transform.html>
///
/// Transform is the only mandatory component on every GameObject.
/// It stores position, rotation, scale, and parent/child relationships.
/// It cannot be removed from a GameObject.
///
/// ## Properties
/// - `position` / `localPosition` — world/local position (Vector3)
/// - `rotation` / `localRotation` — world/local rotation (Quaternion)
/// - `localScale` — scale relative to parent
/// - `lossyScale` — global scale (read-only)
/// - `parent` — parent Transform
/// - `childCount` — number of children
/// - `forward`, `right`, `up` — direction vectors
/// - `eulerAngles` / `localEulerAngles` — euler angles
/// - `hasChanged` — dirty flag
/// - `localToWorldMatrix` / `worldToLocalMatrix` — transformation matrices
///
/// ## Rust Implementation
/// In Unity, Transform is a C# class. In Rust, it's a struct that is
/// stored separately from the component vector. The World owns Transform
/// instances and ensures every GameObject has exactly one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    // === Local space ===
    /// Local position relative to parent (matches `Transform.localPosition`).
    pub(crate) local_position: Vec3,

    /// Local rotation relative to parent (matches `Transform.localRotation`).
    pub(crate) local_rotation: Quat,

    /// Local scale relative to parent (matches `Transform.localScale`).
    pub(crate) local_scale: Vec3,

    // === World space (cached, computed by sync system) ===
    /// Cached world position (matches `Transform.position`).
    #[serde(skip)]
    pub(crate) world_position: Vec3,

    /// Cached world rotation (matches `Transform.rotation`).
    #[serde(skip)]
    pub(crate) world_rotation: Quat,

    /// Cached world scale (matches `Transform.lossyScale`).
    #[serde(skip)]
    pub(crate) world_scale: Vec3,

    // === Hierarchy (built-in, not a separate component) ===
    /// Parent transform handle (matches `Transform.parent`).
    pub(crate) parent: Option<GameObjectHandle>,

    /// Children transform handles (matches `Transform.GetChild`).
    pub(crate) children: Vec<GameObjectHandle>,

    /// Sibling index (matches `Transform.GetSiblingIndex`).
    pub(crate) root_order: usize,

    /// Whether the transform has changed since last sync (matches `Transform.hasChanged`).
    pub(crate) has_changed: bool,
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
            parent: None,
            children: Vec::new(),
            root_order: 0,
            has_changed: false,
        }
    }
}

impl crate::component::Component for Transform {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Transform {
    // ============================================================
    // Constructors
    // ============================================================

    /// Create a transform at the given position.
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self {
            local_position: Vec3::new(x, y, z),
            world_position: Vec3::new(x, y, z),
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
            ..Default::default()
        }
    }

    // ============================================================
    // Properties — Local Space
    // ============================================================

    /// Get local position (matches `Transform.localPosition`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform-localPosition.html>
    pub fn LocalPosition(&self) -> Vec3 {
        self.local_position
    }

    /// Set local position (matches `Transform.localPosition`).
    pub fn SetLocalPosition(&mut self, position: Vec3) {
        self.local_position = position;
        self.has_changed = true;
    }

    /// Get local rotation (matches `Transform.localRotation`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform-localRotation.html>
    pub fn LocalRotation(&self) -> Quat {
        self.local_rotation
    }

    /// Set local rotation (matches `Transform.localRotation`).
    pub fn SetLocalRotation(&mut self, rotation: Quat) {
        self.local_rotation = rotation;
        self.has_changed = true;
    }

    /// Get local scale (matches `Transform.localScale`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform-localScale.html>
    pub fn LocalScale(&self) -> Vec3 {
        self.local_scale
    }

    /// Set local scale (matches `Transform.localScale`).
    pub fn SetLocalScale(&mut self, scale: Vec3) {
        self.local_scale = scale;
        self.has_changed = true;
    }

    /// Get local euler angles in degrees (matches `Transform.localEulerAngles`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform-localEulerAngles.html>
    pub fn LocalEulerAngles(&self) -> Vec3 {
        self.local_rotation.to_eulerANGLES()
    }

    /// Set local euler angles in degrees (matches `Transform.localEulerAngles`).
    pub fn SetLocalEulerAngles(&mut self, eulers: Vec3) {
        self.local_rotation = Quat::from_eulerANGLES(eulers);
        self.has_changed = true;
    }

    /// Get local position and rotation simultaneously (matches `Transform.GetLocalPositionAndRotation`).
    pub fn GetLocalPositionAndRotation(&self) -> (Vec3, Quat) {
        (self.local_position, self.local_rotation)
    }

    /// Set local position and rotation simultaneously (matches `Transform.SetLocalPositionAndRotation`).
    pub fn SetLocalPositionAndRotation(&mut self, position: Vec3, rotation: Quat) {
        self.local_position = position;
        self.local_rotation = rotation;
        self.has_changed = true;
    }

    // ============================================================
    // Properties — World Space
    // ============================================================

    /// Get world position (matches `Transform.position`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform-position.html>
    pub fn Position(&self) -> Vec3 {
        self.world_position
    }

    /// Set world position (matches `Transform.position`).
    pub fn SetPosition(&mut self, position: Vec3) {
        self.world_position = position;
        self.has_changed = true;
        // Note: local_position will be computed by sync system
    }

    /// Get world rotation (matches `Transform.rotation`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform-rotation.html>
    pub fn Rotation(&self) -> Quat {
        self.world_rotation
    }

    /// Set world rotation (matches `Transform.rotation`).
    pub fn SetRotation(&mut self, rotation: Quat) {
        self.world_rotation = rotation;
        self.has_changed = true;
    }

    /// Get global scale (matches `Transform.lossyScale`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform-lossyScale.html>
    ///
    /// Read-only. The global scale of the object. Due to the possibility of
    /// skewed or non-uniform scaling in parent, this may not be exactly equal
    /// to localScale.
    pub fn LossyScale(&self) -> Vec3 {
        self.world_scale
    }

    /// Get world euler angles in degrees (matches `Transform.eulerAngles`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform-eulerAngles.html>
    pub fn EulerAngles(&self) -> Vec3 {
        self.world_rotation.to_eulerANGLES()
    }

    /// Set world euler angles in degrees (matches `Transform.eulerAngles`).
    pub fn SetEulerAngles(&mut self, eulers: Vec3) {
        self.world_rotation = Quat::from_eulerANGLES(eulers);
        self.has_changed = true;
    }

    /// Get world position and rotation simultaneously (matches `Transform.GetPositionAndRotation`).
    pub fn GetPositionAndRotation(&self) -> (Vec3, Quat) {
        (self.world_position, self.world_rotation)
    }

    /// Set world position and rotation simultaneously (matches `Transform.SetPositionAndRotation`).
    pub fn SetPositionAndRotation(&mut self, position: Vec3, rotation: Quat) {
        self.world_position = position;
        self.world_rotation = rotation;
        self.has_changed = true;
    }

    // ============================================================
    // Properties — Direction Vectors
    // ============================================================

    /// Get the blue axis (forward direction) in world space (matches `Transform.forward`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform-forward.html>
    pub fn Forward(&self) -> Vec3 {
        self.world_rotation * Vec3::Z
    }

    /// Set the forward direction (matches `Transform.forward`).
    pub fn SetForward(&mut self, forward: Vec3) {
        if forward.length_squared() > 0.0001 {
            self.world_rotation = Quat::from_rotation_arc(Vec3::Z, forward.normalize());
        }
    }

    /// Get the red axis (right direction) in world space (matches `Transform.right`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform-right.html>
    pub fn Right(&self) -> Vec3 {
        self.world_rotation * Vec3::X
    }

    /// Set the right direction (matches `Transform.right`).
    pub fn SetRight(&mut self, right: Vec3) {
        if right.length_squared() > 0.0001 {
            self.world_rotation = Quat::from_rotation_arc(Vec3::X, right.normalize());
        }
    }

    /// Get the green axis (up direction) in world space (matches `Transform.up`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform-up.html>
    pub fn Up(&self) -> Vec3 {
        self.world_rotation * Vec3::Y
    }

    /// Set the up direction (matches `Transform.up`).
    pub fn SetUp(&mut self, up: Vec3) {
        if up.length_squared() > 0.0001 {
            self.world_rotation = Quat::from_rotation_arc(Vec3::Y, up.normalize());
        }
    }

    // ============================================================
    // Properties — Matrix
    // ============================================================

    /// Get the local-to-world transformation matrix (matches `Transform.localToWorldMatrix`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform-localToWorldMatrix.html>
    pub fn LocalToWorldMatrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(
            self.world_scale,
            self.world_rotation,
            self.world_position,
        )
    }

    /// Get the world-to-local transformation matrix (matches `Transform.worldToLocalMatrix`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform-worldToLocalMatrix.html>
    pub fn WorldToLocalMatrix(&self) -> Mat4 {
        self.LocalToWorldMatrix().inverse()
    }

    // ============================================================
    // Properties — Hierarchy
    // ============================================================

    /// Get the number of children (matches `Transform.childCount`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform-childCount.html>
    pub fn ChildCount(&self) -> usize {
        self.children.len()
    }

    /// Get the parent transform (matches `Transform.parent`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform-parent.html>
    pub fn Parent(&self) -> Option<GameObjectHandle> {
        self.parent
    }

    /// Set the parent transform (matches `Transform.parent`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform-parent.html>
    ///
    /// NOTE: This only sets the parent field. Use `World::SetParent()` to
    /// maintain hierarchy consistency.
    pub fn SetParent(&mut self, parent: Option<GameObjectHandle>) {
        self.parent = parent;
    }

    /// Get the topmost transform in the hierarchy (matches `Transform.root`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform-root.html>
    ///
    /// NOTE: This only checks the parent field. Use `World::GetRoot()` to
    /// traverse the full hierarchy.
    pub fn Root(&self) -> Option<GameObjectHandle> {
        if self.parent.is_some() {
            None // Need World to traverse
        } else {
            None // Need World to return self handle
        }
    }

    // ============================================================
    // Properties — Other
    // ============================================================

    /// Get whether the transform has changed since last sync (matches `Transform.hasChanged`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform-hasChanged.html>
    pub fn HasChanged(&self) -> bool {
        self.has_changed
    }

    /// Set the hasChanged flag (matches `Transform.hasChanged`).
    pub fn SetHasChanged(&mut self, changed: bool) {
        self.has_changed = changed;
    }

    // ============================================================
    // Methods — Space Conversion
    // ============================================================

    /// Transform a point from local to world space (matches `Transform.TransformPoint`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform.TransformPoint.html>
    pub fn TransformPoint(&self, point: Vec3) -> Vec3 {
        self.world_position + self.world_rotation * (point * self.world_scale)
    }

    /// Transform a point from world to local space (matches `Transform.InverseTransformPoint`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform.InverseTransformPoint.html>
    pub fn InverseTransformPoint(&self, point: Vec3) -> Vec3 {
        if self.world_scale.x == 0.0 || self.world_scale.y == 0.0 || self.world_scale.z == 0.0 {
            return point;
        }
        let relative = point - self.world_position;
        let inv_rotation = self.world_rotation.inverse();
        let inv_scale = Vec3::new(
            1.0 / self.world_scale.x,
            1.0 / self.world_scale.y,
            1.0 / self.world_scale.z,
        );
        inv_rotation * (relative * inv_scale)
    }

    /// Transform a direction from local to world space (matches `Transform.TransformDirection`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform.TransformDirection.html>
    pub fn TransformDirection(&self, direction: Vec3) -> Vec3 {
        self.world_rotation * direction
    }

    /// Transform a direction from world to local space (matches `Transform.InverseTransformDirection`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform.InverseTransformDirection.html>
    pub fn InverseTransformDirection(&self, direction: Vec3) -> Vec3 {
        self.world_rotation.inverse() * direction
    }

    /// Transform a vector from local to world space (matches `Transform.TransformVector`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform.TransformVector.html>
    pub fn TransformVector(&self, vector: Vec3) -> Vec3 {
        self.world_rotation * (vector * self.world_scale)
    }

    /// Transform a vector from world to local space (matches `Transform.InverseTransformVector`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform.InverseTransformVector.html>
    pub fn InverseTransformVector(&self, vector: Vec3) -> Vec3 {
        if self.world_scale.x == 0.0 || self.world_scale.y == 0.0 || self.world_scale.z == 0.0 {
            return vector;
        }
        let inv_scale = Vec3::new(
            1.0 / self.world_scale.x,
            1.0 / self.world_scale.y,
            1.0 / self.world_scale.z,
        );
        self.world_rotation.inverse() * (vector * inv_scale)
    }

    // ============================================================
    // Methods — LookAt / Rotate / Translate
    // ============================================================

    /// Rotate to face a target position (matches `Transform.LookAt`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform.LookAt.html>
    pub fn LookAt(&mut self, target: Vec3) {
        self.LookAtWithUp(target, Vec3::Y);
    }

    /// Rotate to face a target position with custom up direction (matches `Transform.LookAt`).
    pub fn LookAtWithUp(&mut self, target: Vec3, world_up: Vec3) {
        let direction = target - self.world_position;
        if direction.length_squared() > 0.0001 {
            let forward = direction.normalize();
            let right = forward.cross(world_up).normalize();
            let up = right.cross(forward);
            self.world_rotation = Quat::from_rotation_arc(Vec3::Z, forward);
        }
    }

    /// Rotate by euler angles (matches `Transform.Rotate`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform.Rotate.html>
    pub fn Rotate(&mut self, eulers: Vec3) {
        let rotation = Quat::from_eulerANGLES(eulers);
        self.world_rotation = rotation * self.world_rotation;
        self.has_changed = true;
    }

    /// Rotate by euler angles in specified space (matches `Transform.Rotate`).
    pub fn RotateWithSpace(&mut self, eulers: Vec3, relative_to: Space) {
        match relative_to {
            Space::World => {
                let rotation = Quat::from_eulerANGLES(eulers);
                self.world_rotation = rotation * self.world_rotation;
            }
            Space::Self_ => {
                let rotation = Quat::from_eulerANGLES(eulers);
                self.world_rotation = self.world_rotation * rotation;
            }
        }
        self.has_changed = true;
    }

    /// Rotate around a world point (matches `Transform.RotateAround`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform.RotateAround.html>
    pub fn RotateAround(&mut self, point: Vec3, axis: Vec3, angle: f32) {
        let rotation = Quat::from_axis_angle(axis.normalize(), angle.to_radians());
        let offset = self.world_position - point;
        self.world_position = point + rotation * offset;
        self.world_rotation = rotation * self.world_rotation;
        self.has_changed = true;
    }

    /// Move in direction/distance (matches `Transform.Translate`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform.Translate.html>
    pub fn Translate(&mut self, translation: Vec3) {
        self.world_position += translation;
        self.has_changed = true;
    }

    /// Move in direction/distance in specified space (matches `Transform.Translate`).
    pub fn TranslateWithSpace(&mut self, translation: Vec3, relative_to: Space) {
        match relative_to {
            Space::World => {
                self.world_position += translation;
            }
            Space::Self_ => {
                self.world_position += self.world_rotation * translation;
            }
        }
        self.has_changed = true;
    }

    /// Move relative to another transform (matches `Transform.Translate`).
    pub fn TranslateRelative(&mut self, translation: Vec3, relative_to: &Transform) {
        self.world_position += relative_to.world_rotation * translation;
        self.has_changed = true;
    }

    // ============================================================
    // Methods — Hierarchy
    // ============================================================

    /// Find a child by name (matches `Transform.Find`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform.Find.html>
    ///
    /// NOTE: This only checks the children list. Use `World::FindInChildren()` to
    /// search the full hierarchy.
    pub fn Find(&self, _name: &str) -> Option<GameObjectHandle> {
        // Need World to look up names
        None
    }

    /// Get a child by index (matches `Transform.GetChild`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform.GetChild.html>
    pub fn GetChild(&self, index: usize) -> Option<GameObjectHandle> {
        self.children.get(index).copied()
    }

    /// Get sibling index (matches `Transform.GetSiblingIndex`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform.GetSiblingIndex.html>
    pub fn GetSiblingIndex(&self) -> usize {
        self.root_order
    }

    /// Set sibling index (matches `Transform.SetSiblingIndex`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform.SetSiblingIndex.html>
    pub fn SetSiblingIndex(&mut self, index: usize) {
        self.root_order = index;
    }

    /// Move to beginning of sibling list (matches `Transform.SetAsFirstSibling`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform.SetAsFirstSibling.html>
    pub fn SetAsFirstSibling(&mut self) {
        self.root_order = 0;
    }

    /// Move to end of sibling list (matches `Transform.SetAsLastSibling`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform.SetAsLastSibling.html>
    pub fn SetAsLastSibling(&mut self) {
        self.root_order = usize::MAX;
    }

    /// Detach all children from parent (matches `Transform.DetachChildren`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform.DetachChildren.html>
    ///
    /// NOTE: This only clears the children list. Use `World::DetachChildren()` to
    /// maintain hierarchy consistency.
    pub fn DetachChildren(&mut self) {
        self.children.clear();
    }

    /// Check if this is a child of a parent (matches `Transform.IsChildOf`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform.IsChildOf.html>
    ///
    /// NOTE: This only checks the direct parent. Use `World::IsChildOf()` to
    /// check the full hierarchy.
    pub fn IsChildOf(&self, parent: GameObjectHandle) -> bool {
        self.parent == Some(parent)
    }

    // ============================================================
    // Internal — World Transform Sync
    // ============================================================

    /// Update the cached world transform from parent (called by sync system).
    pub(crate) fn UpdateWorldTransform(
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
    pub(crate) fn UpdateWorldTransformRoot(&mut self) {
        self.world_position = self.local_position;
        self.world_rotation = self.local_rotation;
        self.world_scale = self.local_scale;
    }

    // ============================================================
    // Backward-compatible snake_case aliases
    // ============================================================

    /// Set local rotation (snake_case alias for SetLocalRotation).
    pub fn set_local_rotation(&mut self, rotation: Quat) {
        self.SetLocalRotation(rotation);
    }

    /// Set local scale (snake_case alias for SetLocalScale).
    pub fn set_local_scale(&mut self, scale: Vec3) {
        self.SetLocalScale(scale);
    }

    /// Set local position (snake_case alias for SetLocalPosition).
    pub fn set_local_position(&mut self, position: Vec3) {
        self.SetLocalPosition(position);
    }

    /// Get local position (snake_case alias for LocalPosition).
    pub fn local_position(&self) -> Vec3 {
        self.LocalPosition()
    }

    /// Get local rotation (snake_case alias for LocalRotation).
    pub fn local_rotation(&self) -> Quat {
        self.LocalRotation()
    }

    /// Get local scale (snake_case alias for LocalScale).
    pub fn local_scale(&self) -> Vec3 {
        self.LocalScale()
    }
}

// ============================================================
// Helper trait for euler angle conversion
// ============================================================

/// Extension trait for Quat to convert to/from euler angles.
trait QuatEuler {
    fn to_eulerANGLES(&self) -> Vec3;
    fn from_eulerANGLES(eulers: Vec3) -> Self;
}

impl QuatEuler for Quat {
    fn to_eulerANGLES(&self) -> Vec3 {
        // Convert quaternion to euler angles (degrees)
        let (x, y, z) = self.to_euler(engine_math::EulerRot::XYZ);
        Vec3::new(x.to_degrees(), y.to_degrees(), z.to_degrees())
    }

    fn from_eulerANGLES(eulers: Vec3) -> Self {
        // Convert euler angles (degrees) to quaternion
        Quat::from_euler(
            engine_math::EulerRot::XYZ,
            eulers.x.to_radians(),
            eulers.y.to_radians(),
            eulers.z.to_radians(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_default() {
        let t = Transform::default();
        assert_eq!(t.LocalPosition(), Vec3::ZERO);
        assert_eq!(t.LocalRotation(), Quat::IDENTITY);
        assert_eq!(t.LocalScale(), Vec3::ONE);
        assert!(t.Parent().is_none());
        assert_eq!(t.ChildCount(), 0);
    }

    #[test]
    fn test_transform_from_xyz() {
        let t = Transform::from_xyz(1.0, 2.0, 3.0);
        assert_eq!(t.LocalPosition(), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(t.Position(), Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_transform_forward() {
        let t = Transform::default();
        assert_eq!(t.Forward(), Vec3::Z);
        assert_eq!(t.Right(), Vec3::X);
        assert_eq!(t.Up(), Vec3::Y);
    }

    #[test]
    fn test_transform_look_at() {
        let mut t = Transform::from_xyz(0.0, 0.0, 0.0);
        t.LookAt(Vec3::new(1.0, 0.0, 0.0));

        let forward = t.Forward();
        assert!((forward.x - 1.0).abs() < 0.001);
        assert!(forward.y.abs() < 0.001);
        assert!(forward.z.abs() < 0.001);
    }

    #[test]
    fn test_transform_translate() {
        let mut t = Transform::default();
        t.Translate(Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(t.Position(), Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_transform_rotate() {
        let mut t = Transform::default();
        t.Rotate(Vec3::new(0.0, 90.0, 0.0));
        let forward = t.Forward();
        // After rotating 90 degrees around Y, forward should point along X
        assert!((forward.x - 1.0).abs() < 0.01 || (forward.x + 1.0).abs() < 0.01);
    }

    #[test]
    fn test_transform_parent_child() {
        let parent_handle = GameObjectHandle::new(0, 0);
        let child_handle = GameObjectHandle::new(1, 0);

        let mut parent = Transform::from_xyz(5.0, 0.0, 0.0);
        parent.children.push(child_handle);

        let mut child = Transform::from_xyz(1.0, 0.0, 0.0);
        child.parent = Some(parent_handle);

        assert_eq!(child.Parent(), Some(parent_handle));
        assert_eq!(parent.ChildCount(), 1);
        assert_eq!(parent.GetChild(0), Some(child_handle));
    }

    #[test]
    fn test_transform_world_sync() {
        let mut parent = Transform::from_xyz(5.0, 0.0, 0.0);
        parent.UpdateWorldTransformRoot();

        let mut child = Transform::from_xyz(1.0, 0.0, 0.0);
        child.UpdateWorldTransform(parent.Position(), parent.Rotation(), parent.LossyScale());

        assert_eq!(child.Position(), Vec3::new(6.0, 0.0, 0.0));
    }

    #[test]
    fn test_transform_transform_point() {
        let mut t = Transform::from_xyz(1.0, 0.0, 0.0);
        t.UpdateWorldTransformRoot();

        let world_point = t.TransformPoint(Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(world_point, Vec3::new(1.0, 0.0, 0.0));

        let local_point = t.InverseTransformPoint(world_point);
        assert!((local_point.x).abs() < 0.001);
    }

    #[test]
    fn test_transform_matrix_roundtrip() {
        let t = Transform::from_position_rotation_scale(
            Vec3::new(1.0, 2.0, 3.0),
            Quat::from_rotation_y(0.5),
            Vec3::new(2.0, 2.0, 2.0),
        );
        let l2w = t.LocalToWorldMatrix();
        let w2l = t.WorldToLocalMatrix();
        let product = l2w * w2l;
        for i in 0..4 {
            for j in 0..4 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((product.col(j)[i] - expected).abs() < 0.001);
            }
        }
    }

    #[test]
    fn test_transform_setters_mark_changed() {
        let mut t = Transform::default();
        assert!(!t.HasChanged());

        t.SetPosition(Vec3::ONE);
        assert!(t.HasChanged());

        t.SetHasChanged(false);
        t.SetRotation(Quat::IDENTITY);
        assert!(t.HasChanged());

        t.SetHasChanged(false);
        t.SetLocalScale(Vec3::ONE);
        assert!(t.HasChanged());
    }
}
