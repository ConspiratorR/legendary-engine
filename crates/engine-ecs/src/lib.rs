//! Entity Component System (ECS) foundation.
//!
//! Provides the core ECS primitives: [`entity::Entity`] identifiers,
//! [`component::SparseSet`] storage, [`world::World`] container,
//! [`query::Query`] / [`query::QueryPair`] iteration, [`system::System`]
//! trait, and [`schedule::Schedule`] execution.

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
