//! Rigid body component for physics simulation.
use engine_math::Vec3;

/// Type of rigid body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyType {
    /// Static body - cannot move.
    Static,
    /// Kinematic body - can be moved by code, not affected by forces.
    Kinematic,
    /// Dynamic body - fully simulated with forces and collisions.
    Dynamic,
}

/// Rigid body component.
#[derive(Debug, Clone)]
pub struct RigidBody {
    pub body_type: BodyType,
    pub mass: f32,
    pub linear_velocity: Vec3,
    pub angular_velocity: Vec3,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub gravity_scale: f32,
    pub is_sleeping: bool,
    /// Time the body has been at rest (for sleep system).
    pub rest_time: f32,
}

impl Default for RigidBody {
    fn default() -> Self {
        Self {
            body_type: BodyType::Dynamic,
            mass: 1.0,
            linear_velocity: Vec3::new(0.0, 0.0, 0.0),
            angular_velocity: Vec3::new(0.0, 0.0, 0.0),
            linear_damping: 0.0,
            angular_damping: 0.0,
            gravity_scale: 1.0,
            is_sleeping: false,
            rest_time: 0.0,
        }
    }
}

impl RigidBody {
    /// Create a new dynamic body (default: mass 1, no damping, gravity scale 1).
    pub fn new_dynamic() -> Self {
        Self::default()
    }

    /// Create a new static body that cannot be moved by forces or collisions.
    pub fn new_static() -> Self {
        Self {
            body_type: BodyType::Static,
            ..Default::default()
        }
    }

    /// Create a new kinematic body that can be moved by code but is unaffected by forces.
    pub fn new_kinematic() -> Self {
        Self {
            body_type: BodyType::Kinematic,
            ..Default::default()
        }
    }

    /// Apply a continuous force (in Newtons) for one frame.
    ///
    /// Only affects dynamic bodies with positive mass. The force is converted to
    /// a velocity change via `F = ma` (impulse-style, not accumulated).
    pub fn apply_force(&mut self, force: Vec3) {
        if self.body_type == BodyType::Dynamic && self.mass > 0.0 {
            self.linear_velocity += force / self.mass;
        }
    }

    /// Apply an instantaneous impulse (in kg·m/s).
    ///
    /// Only affects dynamic bodies with positive mass. Directly modifies velocity
    /// without waiting for the next integration step.
    pub fn apply_impulse(&mut self, impulse: Vec3) {
        if self.body_type == BodyType::Dynamic && self.mass > 0.0 {
            self.linear_velocity += impulse / self.mass;
        }
    }

    /// Set the linear velocity directly.
    ///
    /// Ignored for static bodies. For kinematic and dynamic bodies this
    /// overrides any existing velocity.
    pub fn set_linear_velocity(&mut self, vel: Vec3) {
        if self.body_type != BodyType::Static {
            self.linear_velocity = vel;
        }
    }

    /// Set the angular velocity directly.
    ///
    /// Ignored for static bodies. For kinematic and dynamic bodies this
    /// overrides any existing angular velocity.
    pub fn set_angular_velocity(&mut self, vel: Vec3) {
        if self.body_type != BodyType::Static {
            self.angular_velocity = vel;
        }
    }
}
