//! Physics engine module for game engine.
//!
//! This module provides physics simulation capabilities including:
//! - Rigid body dynamics
//! - Collision detection
//! - Simple physics shapes
//! - Gravity and other forces

pub mod body;
pub mod collider;
pub mod world;
pub mod plugin;

pub use body::RigidBody;
pub use collider::Collider;
pub use world::PhysicsWorld;
pub use plugin::PhysicsPlugin;
