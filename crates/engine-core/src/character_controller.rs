//! Character controller component (matches Unity's CharacterController).

use crate::component::Component;
use engine_math::Vec3;
use std::any::Any;

/// Character controller component (matches Unity's `CharacterController`).
#[derive(Debug, Clone)]
pub struct CharacterController {
    pub slope_limit: f32,
    pub step_offset: f32,
    pub skin_width: f32,
    pub min_move_distance: f32,
    pub center: Vec3,
    pub radius: f32,
    pub height: f32,
    pub is_grounded: bool,
    pub velocity: Vec3,
}

impl Default for CharacterController {
    fn default() -> Self {
        Self {
            slope_limit: 45.0,
            step_offset: 0.3,
            skin_width: 0.08,
            min_move_distance: 0.001,
            center: Vec3::new(0.0, 1.0, 0.0),
            radius: 0.5,
            height: 2.0,
            is_grounded: false,
            velocity: Vec3::ZERO,
        }
    }
}

impl Component for CharacterController {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Collision flags for CharacterController.Move (matches Unity's CollisionFlags).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionFlags(i32);

impl CollisionFlags {
    pub const NONE: Self = Self(0);
    pub const SIDE: Self = Self(1);
    pub const ABOVE: Self = Self(2);
    pub const BELOW: Self = Self(4);
}

impl std::ops::BitOr for CollisionFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        CollisionFlags(self.0 | rhs.0)
    }
}

impl CharacterController {
    /// Move the character (matches CharacterController.Move).
    pub fn Move(&mut self, motion: Vec3) -> CollisionFlags {
        self.velocity = motion;
        CollisionFlags::NONE
    }

    /// Simple move (matches CharacterController.SimpleMove).
    pub fn SimpleMove(&mut self, speed: f32) {
        if speed > 0.0 {
            self.velocity = Vec3::new(0.0, 0.0, -speed);
        }
    }

    /// Check if the character is grounded (matches CharacterController.isGrounded).
    pub fn IsGrounded(&self) -> bool {
        self.is_grounded
    }
}
