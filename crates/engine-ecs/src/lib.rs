//! Entity Component System (ECS) foundation.
//!
//! Provides the core ECS primitives: [`entity::Entity`] identifiers,
//! [`component::SparseSet`] storage, [`world::World`] container,
//! [`query::Query`] / [`query::QueryPair`] iteration, [`system::System`]
//! trait, and [`schedule::Schedule`] execution.
//!
//! # Example
//!
//! ```
//! use engine_ecs::world::World;
//! use engine_ecs::query::Query;
//!
//! struct Position { x: f32, y: f32 }
//! struct Velocity { x: f32, y: f32 }
//!
//! let mut world = World::new();
//! let e = world.spawn();
//! world.add_component(e, Position { x: 0.0, y: 0.0 });
//! world.add_component(e, Velocity { x: 1.0, y: 0.5 });
//!
//! let query = Query::<Velocity>::new();
//! for vel in query.iter(&world) {
//!     // read velocity components
//! }
//! ```

pub mod component;
pub mod entity;
pub mod query;
pub mod schedule;
pub mod system;
pub mod world;
