//! # engine-physics
//!
//! A 3D physics simulation crate for the RustEngine.
//!
//! Provides rigid body dynamics, collision detection (sphere, box, capsule, cylinder),
//! a spatial-hash broadphase, iterative contact solving with warm starting, joints,
//! continuous collision detection (CCD), and an ECS integration plugin.
//!
//! ## Quick Start
//!
//! ```rust
//! use engine_physics::{PhysicsWorld, RigidBody, Collider};
//! use engine_core::transform::Transform;
//! use engine_ecs::world::World;
//! use engine_math::Vec3;
//!
//! // Create a physics world with default gravity (0, -9.81, 0)
//! let mut physics = PhysicsWorld::new();
//!
//! // Create an ECS world and spawn a dynamic entity
//! let mut world = World::new();
//! let entity = world.spawn();
//! world.add_component(entity, Transform::from_xyz(0.0, 10.0, 0.0));
//! world.add_component(entity, RigidBody::new_dynamic());
//! world.add_component(entity, Collider::sphere(0.5));
//!
//! // Spawn a static floor
//! let floor = world.spawn();
//! world.add_component(floor, Transform::from_xyz(0.0, -0.5, 0.0));
//! world.add_component(floor, RigidBody::new_static());
//! world.add_component(floor, Collider::cuboid(50.0, 0.5, 50.0));
//!
//! // Step the simulation (integrates gravity, detects & resolves collisions)
//! physics.step(&mut world);
//! ```
//!
//! ## Modules
//!
//! | Module | Description |
//! |--------|-------------|
//! | [`body`] | [`RigidBody`] component and [`BodyType`] enum |
//! | [`collider`] | [`Collider`] component, [`ColliderShape`] variants, and narrow-phase functions |
//! | [`world`] | [`PhysicsWorld`] orchestrator with broadphase, CCD, and contact resolution |
//! | [`contact`] | [`ContactPoint`], [`ContactManifold`], and [`ContactSolver`] |
//! | [`broadphase`] | Spatial-hash broadphase for O(n) candidate pair generation |
//! | [`joint`] | [`Joint`] constraints (ball-socket, hinge, spring) and [`JointSolver`] |
//! | [`ccd`] | Continuous collision detection sweeps (sphere-sphere, sphere-AABB) |
//! | [`plugin`] | [`PhysicsPlugin`] for ECS app integration |
//! | [`error`] | [`PhysicsError`] types |

pub mod error;
pub use error::PhysicsError;

pub mod body;
pub mod broadphase;
pub mod ccd;
pub mod collider;
pub mod contact;
pub mod joint;
pub mod physics_2d;
pub mod plugin;
pub mod world;

pub use body::RigidBody;
pub use collider::{
    Collider, check_box_box, check_capsule_capsule, check_collision, check_cylinder_aabb,
    check_cylinder_sphere, check_obb_capsule, check_obb_obb, check_sphere_box,
    check_sphere_capsule, check_sphere_obb, check_sphere_sphere,
};
pub use contact::{ContactManifold, ContactPoint, ContactSolver};
pub use plugin::{Physics2DPlugin, PhysicsPlugin};
pub use world::{CollisionEvent, PhysicsWorld, SensorEvent};
