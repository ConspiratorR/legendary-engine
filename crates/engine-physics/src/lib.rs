//! Physics engine module for game engine.
//!
//! This module provides physics simulation capabilities including:
//! - Rigid body dynamics
//! - Collision detection
//! - Simple physics shapes
//! - Gravity and other forces

pub mod body;
pub mod collider;
pub mod plugin;
pub mod world;

pub use body::RigidBody;
pub use collider::Collider;
pub use plugin::PhysicsPlugin;
pub use world::PhysicsWorld;
