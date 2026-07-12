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
//! ## Storage Model
//!
//! This ECS uses **sparse-set** storage rather than archetype tables. Each
//! component type gets its own [`SparseSet<T>`](component::SparseSet) — a
//! pair of arrays:
//!
//! - **sparse**: indexed by entity index, maps to dense position (or `None`)
//! - **dense**: packed list of `(entity_index, component_value)` pairs
//!
//! This gives O(1) insert, remove, and lookup. Iteration walks the dense
//! array which is cache-friendly for single-component queries. The trade-off
//! vs archetypes is that multi-component queries must intersect entity-index
//! lists rather than co-iterating struct-of-arrays columns.
//!
//! ## Entity Lifecycle
//!
//! Entities are lightweight handles containing an **index** (slot in the
//! sparse array) and a **generation** counter. When an entity is despawned
//! its generation increments so stale handles never alias a recycled slot.
//!
//! ```text
//! spawn()  → Entity { index: 0, gen: 0 }
//! despawn  → generation[0] becomes 1, index 0 enters free list
//! spawn()  → Entity { index: 0, gen: 1 }  // index reused, gen bumped
//! ```
//!
//! ## Query System
//!
//! Queries iterate over all entities possessing specific component types:
//!
//! - [`Query<T>`](query::Query) — single-component iteration
//! - [`QueryPair<A, B>`](query::QueryPair) — two-component join
//!
//! Both support shared (`iter`) and exclusive (`iter_mut`) access. The join
//! intersects entity-index lists from both sparse sets, yielding only entities
//! that have **both** components.
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
pub mod gameobject;
pub mod par_iter;
pub mod query;
pub mod schedule;
pub mod system;
pub mod world;

pub use error::EcsError;
pub use gameobject::GameObjectHandle;
