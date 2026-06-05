//! Lightweight 2D physics for platformer-style games.
//!
//! Provides AABB collision, simple gravity, ground detection, and trigger support.
//! Designed for tile-based 2D games — no rotation, no circle collision, no constraints.

use engine_math::Vec2;

/// Axis-aligned bounding box in 2D.
#[derive(Debug, Clone, Copy)]
pub struct AABB2D {
    pub min: Vec2,
    pub max: Vec2,
}

impl AABB2D {
    /// Create a new AABB from min and max corners.
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    /// Create an AABB centered at a point with given half-extents.
    pub fn from_center(center: Vec2, half_extents: Vec2) -> Self {
        Self {
            min: center - half_extents,
            max: center + half_extents,
        }
    }

    /// Check overlap with another AABB.
    pub fn overlaps(&self, other: &AABB2D) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
    }

    /// Compute the overlap (penetration) between two AABBs.
    /// Returns None if no overlap.
    pub fn intersection(&self, other: &AABB2D) -> Option<(Vec2, f32)> {
        let overlap_x = (self.max.x - other.min.x).min(other.max.x - self.min.x);
        let overlap_y = (self.max.y - other.min.y).min(other.max.y - self.min.y);

        if overlap_x <= 0.0 || overlap_y <= 0.0 {
            return None;
        }

        // Minimum separation axis
        if overlap_x < overlap_y {
            let sign = if self.min.x + self.max.x < other.min.x + other.max.x {
                -1.0
            } else {
                1.0
            };
            Some((Vec2::new(sign, 0.0), overlap_x))
        } else {
            let sign = if self.min.y + self.max.y < other.min.y + other.max.y {
                -1.0
            } else {
                1.0
            };
            Some((Vec2::new(0.0, sign), overlap_y))
        }
    }

    /// Width of the AABB.
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    /// Height of the AABB.
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    /// Center point.
    pub fn center(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }
}

/// Body type for 2D physics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyType2D {
    /// Immovable — ground, walls.
    Static,
    /// Moved by code only — moving platforms, doors.
    Kinematic,
    /// Fully simulated — player, enemies.
    Dynamic,
}

/// 2D rigid body component.
#[derive(Debug, Clone)]
pub struct RigidBody2D {
    pub body_type: BodyType2D,
    pub velocity: Vec2,
    pub gravity_scale: f32,
    pub grounded: bool,
    pub linear_damping: f32,
}

impl Default for RigidBody2D {
    fn default() -> Self {
        Self {
            body_type: BodyType2D::Dynamic,
            velocity: Vec2::ZERO,
            gravity_scale: 1.0,
            grounded: false,
            linear_damping: 0.0,
        }
    }
}

impl RigidBody2D {
    pub fn new_dynamic() -> Self {
        Self::default()
    }

    pub fn new_static() -> Self {
        Self {
            body_type: BodyType2D::Static,
            velocity: Vec2::ZERO,
            gravity_scale: 0.0,
            grounded: false,
            linear_damping: 0.0,
        }
    }

    pub fn new_kinematic() -> Self {
        Self {
            body_type: BodyType2D::Kinematic,
            velocity: Vec2::ZERO,
            gravity_scale: 0.0,
            grounded: false,
            linear_damping: 0.0,
        }
    }
}

/// 2D collider component.
#[derive(Debug, Clone)]
pub struct Collider2D {
    /// Local offset from entity position.
    pub offset: Vec2,
    /// Half-extents of the AABB (half-width, half-height).
    pub half_extents: Vec2,
    pub friction: f32,
    pub restitution: f32,
    pub is_trigger: bool,
    pub collision_layers: u32,
    pub collision_mask: u32,
}

impl Default for Collider2D {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            half_extents: Vec2::new(0.5, 0.5),
            friction: 0.5,
            restitution: 0.0,
            is_trigger: false,
            collision_layers: 0xFFFF_FFFF,
            collision_mask: 0xFFFF_FFFF,
        }
    }
}

impl Collider2D {
    /// Create a solid AABB collider with given half-extents.
    pub fn aabb(half_x: f32, half_y: f32) -> Self {
        Self {
            half_extents: Vec2::new(half_x, half_y),
            ..Default::default()
        }
    }

    /// Create a trigger (sensor) AABB collider.
    pub fn trigger(half_x: f32, half_y: f32) -> Self {
        Self {
            half_extents: Vec2::new(half_x, half_y),
            is_trigger: true,
            ..Default::default()
        }
    }

    /// Compute the world-space AABB given an entity position.
    pub fn world_aabb(&self, position: Vec2) -> AABB2D {
        let center = position + self.offset;
        AABB2D::from_center(center, self.half_extents)
    }
}

/// Contact result from 2D collision detection.
#[derive(Debug, Clone)]
pub struct Contact2D {
    pub entity_a: u32,
    pub entity_b: u32,
    pub normal: Vec2,
    pub penetration: f32,
    pub is_trigger: bool,
}
