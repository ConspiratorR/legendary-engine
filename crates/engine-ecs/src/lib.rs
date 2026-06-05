//! # engine-ecs
//!
//! Entity Component System for the RustEngine.
//!
//! A high-performance, sparse-set ECS implementation featuring:
//! - Generational entity IDs
//! - Type-erased component storage
//! - Efficient query iteration
//! - Parallel system execution (optional, via engine-jobs)
//!
//! ## Quick Start
//!
//! ```rust
//! use engine_ecs::world::World;
//!
//! struct Position { x: f32, y: f32 }
//! struct Velocity { dx: f32, dy: f32 }
//!
//! let mut world = World::new();
//! let entity = world.spawn();
//! world.add_component(entity, Position { x: 0.0, y: 0.0 });
//! world.add_component(entity, Velocity { dx: 1.0, dy: 0.5 });
//! ```
//!
//! ## Core Primitives
//!
//! - [`entity::Entity`] — lightweight, generational entity identifier
//! - [`component::SparseSet`] — O(1) sparse-set component storage
//! - [`world::World`] — central container owning entities, components, and resources
//! - [`query::Query`] / [`query::QueryPair`] — type-safe component iteration
//! - [`system::System`] — trait for game logic operating on a `World`
//! - [`schedule::Schedule`] / [`schedule::ParallelSchedule`] — ordered system execution

pub mod access;
pub mod component;
pub mod entity;
pub mod error;
pub mod par_iter;
pub mod query;
pub mod schedule;
pub mod system;
pub mod world;

pub use error::EcsError;
