//! Physics engine module for game engine.
//!
//! This module provides physics simulation capabilities including:
//! - Rigid body dynamics
//! - Collision detection
//! - Simple physics shapes
//! - Gravity and other forces

pub mod body;
pub mod ccd;
pub mod collider;
pub mod contact;
pub mod joint;
pub mod plugin;
pub mod world;

pub use body::RigidBody;
pub use collider::{
    Collider, check_box_box, check_capsule_capsule, check_collision, check_obb_capsule,
    check_obb_obb, check_sphere_box, check_sphere_capsule, check_sphere_obb, check_sphere_sphere,
};
pub use plugin::PhysicsPlugin;
pub use world::PhysicsWorld;
