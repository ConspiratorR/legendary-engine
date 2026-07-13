//! Unity-style built-in components.
//!
//! These components follow Unity's documented API patterns.

use crate::component::Component;
use engine_math::{Quat, Vec3};
use std::any::Any;

// ============================================================
// Rigidbody (Unity: UnityEngine.Rigidbody)
// ============================================================

/// Physics body component (matches Unity's `Rigidbody`).
///
/// # Unity Documentation
/// <https://docs.unity3d.com/ScriptReference/Rigidbody.html>
#[derive(Debug, Clone)]
pub struct Rigidbody {
    /// Mass of the rigidbody (default: 1.0).
    pub mass: f32,
    /// Drag for linear motion (default: 0.0).
    pub drag: f32,
    /// Drag for angular motion (default: 0.05).
    pub angular_drag: f32,
    /// Whether gravity affects this body (default: true).
    pub use_gravity: bool,
    /// Whether this body is kinematic (default: false).
    pub is_kinematic: bool,
    /// Linear velocity.
    pub velocity: Vec3,
    /// Angular velocity.
    pub angular_velocity: Vec3,
}

impl Default for Rigidbody {
    fn default() -> Self {
        Self {
            mass: 1.0,
            drag: 0.0,
            angular_drag: 0.05,
            use_gravity: true,
            is_kinematic: false,
            velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
        }
    }
}

impl Component for Rigidbody {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

impl Rigidbody {
    pub fn AddForce(&mut self, force: Vec3) {
        self.velocity += force / self.mass;
    }
    pub fn AddTorque(&mut self, torque: Vec3) {
        self.angular_velocity += torque / self.mass;
    }
    pub fn Sleep(&mut self) {
        self.velocity = Vec3::ZERO;
        self.angular_velocity = Vec3::ZERO;
    }
    pub fn IsSleeping(&self) -> bool {
        self.velocity.length_squared() < 0.001 && self.angular_velocity.length_squared() < 0.001
    }
}

// ============================================================
// Colliders (Unity: UnityEngine.Collider)
// ============================================================

/// Base trait for all collider components.
pub trait ColliderTrait: Component {
    /// Get the collider's bounds (min, max).
    fn Bounds(&self) -> (Vec3, Vec3);

    /// Check if this is a trigger (matches `Collider.isTrigger`).
    fn IsTrigger(&self) -> bool;

    /// Set trigger state (matches `Collider.isTrigger`).
    fn SetIsTrigger(&mut self, trigger: bool);
}

/// Box collider (matches Unity's `BoxCollider`).
///
/// # Unity Documentation
/// <https://docs.unity3d.com/ScriptReference/BoxCollider.html>
#[derive(Debug, Clone)]
pub struct BoxCollider {
    /// Center offset (matches `BoxCollider.center`).
    pub center: Vec3,
    /// Size (matches `BoxCollider.size`).
    pub size: Vec3,
    /// Whether this is a trigger (matches `Collider.isTrigger`).
    pub is_trigger: bool,
}

impl Default for BoxCollider {
    fn default() -> Self {
        Self { center: Vec3::ZERO, size: Vec3::ONE, is_trigger: false }
    }
}

impl Component for BoxCollider {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

impl ColliderTrait for BoxCollider {
    fn Bounds(&self) -> (Vec3, Vec3) {
        let half = self.size * 0.5;
        (self.center - half, self.center + half)
    }
    fn IsTrigger(&self) -> bool { self.is_trigger }
    fn SetIsTrigger(&mut self, trigger: bool) { self.is_trigger = trigger; }
}

/// Sphere collider (matches Unity's `SphereCollider`).
///
/// # Unity Documentation
/// <https://docs.unity3d.com/ScriptReference/SphereCollider.html>
#[derive(Debug, Clone)]
pub struct SphereCollider {
    /// Center offset (matches `SphereCollider.center`).
    pub center: Vec3,
    /// Radius (matches `SphereCollider.radius`).
    pub radius: f32,
    /// Whether this is a trigger (matches `Collider.isTrigger`).
    pub is_trigger: bool,
}

impl Default for SphereCollider {
    fn default() -> Self {
        Self { center: Vec3::ZERO, radius: 0.5, is_trigger: false }
    }
}

impl Component for SphereCollider {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

impl ColliderTrait for SphereCollider {
    fn Bounds(&self) -> (Vec3, Vec3) {
        (self.center - Vec3::splat(self.radius), self.center + Vec3::splat(self.radius))
    }
    fn IsTrigger(&self) -> bool { self.is_trigger }
    fn SetIsTrigger(&mut self, trigger: bool) { self.is_trigger = trigger; }
}

/// Capsule collider (matches Unity's `CapsuleCollider`).
///
/// # Unity Documentation
/// <https://docs.unity3d.com/ScriptReference/CapsuleCollider.html>
#[derive(Debug, Clone)]
pub struct CapsuleCollider {
    /// Center offset (matches `CapsuleCollider.center`).
    pub center: Vec3,
    /// Radius (matches `CapsuleCollider.radius`).
    pub radius: f32,
    /// Height (matches `CapsuleCollider.height`).
    pub height: f32,
    /// Direction: 0=X, 1=Y, 2=Z (matches `CapsuleCollider.direction`).
    pub direction: i32,
    /// Whether this is a trigger (matches `Collider.isTrigger`).
    pub is_trigger: bool,
}

impl Default for CapsuleCollider {
    fn default() -> Self {
        Self { center: Vec3::ZERO, radius: 0.5, height: 2.0, direction: 1, is_trigger: false }
    }
}

impl Component for CapsuleCollider {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

impl ColliderTrait for CapsuleCollider {
    fn Bounds(&self) -> (Vec3, Vec3) {
        match self.direction {
            0 => (self.center - Vec3::new(self.height * 0.5, self.radius, self.radius),
                  self.center + Vec3::new(self.height * 0.5, self.radius, self.radius)),
            1 => (self.center - Vec3::new(self.radius, self.height * 0.5, self.radius),
                  self.center + Vec3::new(self.radius, self.height * 0.5, self.radius)),
            _ => (self.center - Vec3::new(self.radius, self.radius, self.height * 0.5),
                  self.center + Vec3::new(self.radius, self.radius, self.height * 0.5)),
        }
    }
    fn IsTrigger(&self) -> bool { self.is_trigger }
    fn SetIsTrigger(&mut self, trigger: bool) { self.is_trigger = trigger; }
}

// ============================================================
// Camera (Unity: UnityEngine.Camera)
// ============================================================

/// Camera component (matches Unity's `Camera`).
///
/// # Unity Documentation
/// <https://docs.unity3d.com/ScriptReference/Camera.html>
#[derive(Debug, Clone)]
pub struct Camera {
    /// Field of view in degrees (matches `Camera.fieldOfView`).
    pub field_of_view: f32,
    /// Near clipping plane (matches `Camera.nearClipPlane`).
    pub near_clip: f32,
    /// Far clipping plane (matches `Camera.farClipPlane`).
    pub far_clip: f32,
    /// Whether this is an orthographic camera (matches `Camera.orthographic`).
    pub orthographic: bool,
    /// Orthographic size (matches `Camera.orthographicSize`).
    pub orthographic_size: f32,
    /// Aspect ratio (matches `Camera.aspect`).
    pub aspect: f32,
    /// Background color (matches `Camera.backgroundColor`).
    pub background_color: [f32; 4],
    /// Culling mask (matches `Camera.cullingMask`).
    pub culling_mask: i32,
    /// Depth for rendering order (matches `Camera.depth`).
    pub depth: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            field_of_view: 60.0,
            near_clip: 0.1,
            far_clip: 1000.0,
            orthographic: false,
            orthographic_size: 5.0,
            aspect: 16.0 / 9.0,
            background_color: [0.1, 0.1, 0.2, 1.0],
            culling_mask: -1, // All layers
            depth: 0.0,
        }
    }
}

impl Component for Camera {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Camera {
    /// Get the projection matrix (matches `Camera.projectionMatrix`).
    pub fn ProjectionMatrix(&self) -> engine_math::Mat4 {
        if self.orthographic {
            engine_math::Mat4::orthographic_rh(
                -self.orthographic_size * self.aspect,
                self.orthographic_size * self.aspect,
                -self.orthographic_size,
                self.orthographic_size,
                self.near_clip,
                self.far_clip,
            )
        } else {
            engine_math::Mat4::perspective_infinite_reverse_rh(
                self.field_of_view.to_radians(),
                self.aspect,
                self.near_clip,
            )
        }
    }

    /// Get the view matrix (matches `Camera.worldToCameraMatrix`).
    pub fn ViewMatrix(&self, position: Vec3, rotation: Quat) -> engine_math::Mat4 {
        let forward = rotation * Vec3::Z;
        let up = rotation * Vec3::Y;
        engine_math::Mat4::look_to_rh(position, forward, up)
    }

    /// Convert screen point to world ray (matches `Camera.ScreenPointToRay`).
    pub fn ScreenPointToRay(
        &self,
        screen_x: f32,
        screen_y: f32,
        screen_width: f32,
        screen_height: f32,
        position: Vec3,
        rotation: Quat,
    ) -> (Vec3, Vec3) {
        let ndc_x = (2.0 * screen_x / screen_width - 1.0) * self.aspect;
        let ndc_y = 1.0 - 2.0 * screen_y / screen_height;

        let view = self.ViewMatrix(position, rotation);
        let proj = self.ProjectionMatrix();
        let inv_vp = (proj * view).inverse();

        let near = inv_vp * engine_math::Vec4::new(ndc_x, ndc_y, -1.0, 1.0);
        let far = inv_vp * engine_math::Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

        let near_point = near.truncate() / near.w;
        let far_point = far.truncate() / far.w;

        let direction = (far_point - near_point).normalize();
        (near_point, direction)
    }
}

// ============================================================
// Light (Unity: UnityEngine.Light)
// ============================================================

/// Light type (matches Unity's `LightType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightType {
    /// Directional light (matches `LightType.Directional`).
    Directional,
    /// Point light (matches `LightType.Point`).
    Point,
    /// Spot light (matches `LightType.Spot`).
    Spot,
}

/// Light component (matches Unity's `Light`).
///
/// # Unity Documentation
/// <https://docs.unity3d.com/ScriptReference/Light.html>
#[derive(Debug, Clone)]
pub struct Light {
    /// Light type (matches `Light.type`).
    pub light_type: LightType,
    /// Light color (matches `Light.color`).
    pub color: [f32; 3],
    /// Light intensity (matches `Light.intensity`).
    pub intensity: f32,
    /// Light range (matches `Light.range`).
    pub range: f32,
    /// Spot light inner angle in degrees (matches `Light.innerSpotAngle`).
    pub inner_angle: f32,
    /// Spot light outer angle in degrees (matches `Light.spotAngle`).
    pub outer_angle: f32,
    /// Shadow type (matches `Light.shadows`).
    pub shadows: bool,
    /// Shadow strength (matches `Light.shadowStrength`).
    pub shadow_strength: f32,
}

impl Default for Light {
    fn default() -> Self {
        Self {
            light_type: LightType::Point,
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            range: 10.0,
            inner_angle: 30.0,
            outer_angle: 60.0,
            shadows: true,
            shadow_strength: 1.0,
        }
    }
}

impl Component for Light {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ============================================================
// MeshRenderer (Unity: UnityEngine.MeshRenderer)
// ============================================================

/// Mesh renderer component (matches Unity's `MeshRenderer`).
///
/// # Unity Documentation
/// <https://docs.unity3d.com/ScriptReference/MeshRenderer.html>
#[derive(Debug, Clone)]
pub struct MeshRenderer {
    /// Mesh name (matches `MeshFilter.mesh`).
    pub mesh: String,
    /// Material name (matches `Renderer.material`).
    pub material: String,
    /// Whether to cast shadows (matches `Renderer.shadowCastingMode`).
    pub cast_shadows: bool,
    /// Whether to receive shadows (matches `Renderer.receiveShadows`).
    pub receive_shadows: bool,
}

impl Default for MeshRenderer {
    fn default() -> Self {
        Self {
            mesh: "Cube".to_string(),
            material: "Default".to_string(),
            cast_shadows: true,
            receive_shadows: true,
        }
    }
}

impl Component for MeshRenderer {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ============================================================
// SpriteRenderer (Unity: UnityEngine.SpriteRenderer)
// ============================================================

/// Sprite renderer component (matches Unity's `SpriteRenderer`).
///
/// # Unity Documentation
/// <https://docs.unity3d.com/ScriptReference/SpriteRenderer.html>
#[derive(Debug, Clone)]
pub struct SpriteRenderer {
    /// Sprite texture path.
    pub sprite: String,
    /// Sprite color (matches `SpriteRenderer.color`).
    pub color: [f32; 4],
    /// Whether to flip on X axis (matches `SpriteRenderer.flipX`).
    pub flip_x: bool,
    /// Whether to flip on Y axis (matches `SpriteRenderer.flipY`).
    pub flip_y: bool,
    /// Sorting order (matches `Renderer.sortingOrder`).
    pub sorting_order: i32,
}

impl Default for SpriteRenderer {
    fn default() -> Self {
        Self {
            sprite: String::new(),
            color: [1.0, 1.0, 1.0, 1.0],
            flip_x: false,
            flip_y: false,
            sorting_order: 0,
        }
    }
}

impl Component for SpriteRenderer {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ============================================================
// AudioSource (Unity: UnityEngine.AudioSource)
// ============================================================

/// Audio source component (matches Unity's `AudioSource`).
///
/// # Unity Documentation
/// <https://docs.unity3d.com/ScriptReference/AudioSource.html>
#[derive(Debug, Clone)]
pub struct AudioSource {
    /// Audio clip path.
    pub clip: String,
    /// Volume (0.0 to 1.0, matches `AudioSource.volume`).
    pub volume: f32,
    /// Pitch (matches `AudioSource.pitch`).
    pub pitch: f32,
    /// Whether to loop (matches `AudioSource.loop`).
    pub loop_playing: bool,
    /// Whether to play on awake (matches `AudioSource.playOnAwake`).
    pub play_on_awake: bool,
    /// Spatial blend: 0.0 = 2D, 1.0 = 3D (matches `AudioSource.spatialBlend`).
    pub spatial_blend: f32,
    /// Doppler level (matches `AudioSource.dopplerLevel`).
    pub doppler_level: f32,
    /// Min distance for 3D audio (matches `AudioSource.minDistance`).
    pub min_distance: f32,
    /// Max distance for 3D audio (matches `AudioSource.maxDistance`).
    pub max_distance: f32,
}

impl Default for AudioSource {
    fn default() -> Self {
        Self {
            clip: String::new(),
            volume: 1.0,
            pitch: 1.0,
            loop_playing: false,
            play_on_awake: true,
            spatial_blend: 0.0,
            doppler_level: 1.0,
            min_distance: 1.0,
            max_distance: 500.0,
        }
    }
}

impl Component for AudioSource {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
