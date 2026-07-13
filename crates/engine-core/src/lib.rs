//! # engine-core
//!
//! Core engine systems for the RustEngine — a Rust implementation of Unity's architecture.
//!
//! ## Architecture
//!
//! This crate follows Unity's documented architecture:
//! - [`GameObject`](gameobject::GameObject) is the fundamental building block
//! - [`Component`](component::Component) is the base trait for all components
//! - [`Behaviour`](behaviour::Behaviour) extends Component with enabled state
//! - [`MonoBehaviour`](monobehaviour::MonoBehaviour) is the base class for user scripts
//! - [`Transform`](transform::Transform) is built-in to every GameObject (mandatory, cannot be removed)
//! - [`ScriptableObject`](scriptable_object::ScriptableObject) is for data containers
//! - [`World`](world::World) is the central container for all GameObjects
//!
//! ## Class Hierarchy
//!
//! ```text
//! Object (trait)
//!   +-- Component (trait)
//!   |     +-- Transform (built-in, not user-attached)
//!   |     +-- Behaviour (trait)
//!   |     |     +-- MonoBehaviour (user scripts)
//!   |     |     +-- Camera
//!   |     |     +-- Collider (BoxCollider, SphereCollider, etc.)
//!   |     |     +-- Renderer (MeshRenderer, SpriteRenderer)
//!   |     |     +-- Rigidbody
//!   |     |     +-- AudioSource
//!   |     |     +-- Light
//!   |     |     +-- Animator
//!   |     +-- Rigidbody
//!   |     +-- Collider
//!   +-- ScriptableObject (data containers)
//! ```
//!
//! ## Quick Start
//!
//! ```rust
//! use engine_core::world::World;
//! use engine_core::gameobject::GameObjectHandle;
//!
//! let mut world = World::new();
//!
//! // Create a GameObject (matches Unity's new GameObject("name"))
//! let player = world.CreateGameObject("Player");
//!
//! // Set tag (matches Unity's GameObject.tag = "Player")
//! world.SetTag(player, "Player");
//!
//! // Set parent (matches Unity's Transform.SetParent)
//! let root = world.CreateGameObject("Root");
//! world.SetParent(player, Some(root));
//!
//! // Find objects (matches Unity's GameObject.FindWithTag)
//! let found = world.FindWithTag("Player");
//! assert_eq!(found, Some(player));
//! ```

// ============================================================
// New Unity-style modules
// ============================================================

/// Object trait — base class for all Unity objects.
pub mod object;

/// Component trait — base class for all components attached to GameObjects.
pub mod component;

/// Behaviour trait — base class for components that can be enabled/disabled.
pub mod behaviour;

// ============================================================
// Rewritten modules
// ============================================================

/// Transform — built-in component for position, rotation, scale, and hierarchy.
pub mod transform;

/// GameObject — fundamental building block of Unity scenes.
pub mod gameobject;

/// MonoBehaviour — base class for all user scripts.
pub mod monobehaviour;

/// ScriptableObject — data container for sharing data.
pub mod scriptable_object;

/// Unity-style built-in components (Rigidbody, Collider, Camera, Light, etc.).
pub mod components;

/// World — central container for all GameObjects.
pub mod world;

/// Hierarchy utilities — helper functions for working with Transform hierarchy.
pub mod hierarchy;

/// MonoBehaviour lifecycle runner.
pub mod monobehaviour_runner;

// ============================================================
// Existing modules (to be refactored in later phases)
// ============================================================

pub mod app;
pub mod asset_database;
pub mod asset_handle;
pub mod color;
pub mod config;
pub mod context;
pub mod debug;
pub mod engine;
pub mod error;
pub mod event;
pub mod events;
pub mod logger;
pub mod math_utils;
pub mod memory;
pub mod player_loop;
pub mod plugin;
pub mod plugin_loader;
pub mod plugins;
pub mod prefab;
pub mod profiler;
pub mod resource;
pub mod serialization;
pub mod system;
pub mod time;
pub mod undo;

// Re-export for convenience
pub use object::{Object, ObjectUtil};
pub use component::Component;
pub use behaviour::{Behaviour, BehaviourState};
pub use transform::{Space, Transform};
pub use gameobject::{GameObject, GameObjectHandle};
pub use monobehaviour::{MonoBehaviour, MonoBehaviourHolder, CoroutineHandle};
pub use scriptable_object::ScriptableObject;
pub use world::World;
pub use hierarchy::{get_ancestors, get_depth, get_root, is_ancestor, sync_transforms};
pub use monobehaviour_runner::MonoBehaviourRunner;
pub use context::Context;
pub use event::{Event, EventBus, EventBusExt, EventHandler};
pub use events::*;
pub use player_loop::{Phase, PlayerLoop};
pub use system::System;
pub use time::Time;
pub use app::AppBuilder;

// Re-export macros - impl_component is defined in component.rs with #[macro_export]
// It's automatically available at crate root

#[cfg(target_os = "android")]
pub mod android;
