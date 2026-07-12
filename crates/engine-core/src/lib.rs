//! # engine-core
//!
//! Core engine systems for the RustEngine.
//!
//! This is the central crate ("Layer 4") that ties together all engine subsystems:
//! - Application builder and plugin system
//! - Time management
//! - Configuration system
//! - Logging and performance profiling
//! - Memory tracking and object pools
//! - Event channels
//!
//! ## Architecture
//!
//! engine-core follows a plugin-based architecture where each subsystem is
//! integrated as a [`Plugin`](crate::plugin::Plugin) that registers systems,
//! resources, and lifecycle hooks with the [`AppBuilder`](crate::app::AppBuilder).
//!
//! ```text
//! AppBuilder::new()
//!   .add_plugin(CorePlugins)      // registers Time, ActionMap, etc.
//!   .add_plugin(MyGamePlugin)     // registers game-specific systems
//!   .build()                      // produces an App
//!   .run()                        // executes one frame
//! ```
//!
//! ## AppBuilder Pattern
//!
//! [`AppBuilder`](crate::app::AppBuilder) is the primary entry point for
//! constructing an application. It accumulates:
//!
//! - **Plugins** via [`add_plugin`](crate::app::AppBuilder::add_plugin) —
//!   each plugin's `build()` method is called immediately in registration order.
//! - **Systems** via [`add_system`](crate::app::AppBuilder::add_system) —
//!   added to either the sequential or parallel schedule.
//! - **Resources** via [`insert_resource`](crate::app::AppBuilder::insert_resource) —
//!   global singletons accessible from any system.
//! - **Lifecycle hooks** via `add_pre_update_hook` / `add_post_update_hook` —
//!   closures that run before/after the system schedule each frame.
//!
//! Call [`build`](crate::app::AppBuilder::build) to finalize and produce an
//! [`App`](crate::app::App). The `App` holds the ECS world, schedule, and
//! renderer. Call [`run`](crate::app::App::run) each frame to execute:
//!
//! ```text
//! pre-update hooks → input frame advance → systems → post-update hooks
//! ```
//!
//! ## Plugin Lifecycle
//!
//! 1. **Registration**: `app.add_plugin(MyPlugin)` calls `MyPlugin::build(&self, &mut AppBuilder)`.
//! 2. **Build phase**: The plugin inserts resources, registers systems, and adds hooks.
//! 3. **Build order**: Plugins execute `build()` in the order they are registered.
//!    Dependencies between plugins must be handled by registration order.
//! 4. **App finalization**: `app_builder.build()` consumes the builder and
//!    produces an `App` with all registered systems and hooks.
//! 5. **Runtime**: `app.run()` executes the frame cycle. Hooks and systems
//!    see all resources and components registered during the build phase.
//!
//! ## Crate Dependencies
//!
//! engine-core depends on all major engine crates. Mandatory dependencies:
//! - `engine-ecs` — Entity-Component-World and scheduling
//! - `engine-input` — Input manager and action maps
//! - `engine-render` — Renderer (set via `App::set_renderer`)
//! - `engine-math` — Vector math types
//! - `engine-scene`, `engine-asset`, `engine-window` — Error type integration
//!
//! Optional (feature-gated):
//! - `engine-audio` — Audio playback (feature `"audio"`, enabled by default)
//!
//! ## Quick Start
//!
//! ```rust
//! use engine_core::app::AppBuilder;
//! use engine_core::plugin::Plugin;
//!
//! struct MyPlugin;
//! impl Plugin for MyPlugin {
//!     fn build(&self, app: &mut AppBuilder) {
//!         app.add_system(my_system);
//!     }
//! }
//!
//! # fn my_system(_world: &mut engine_ecs::world::World) {}
//! let mut app = AppBuilder::new();
//! app.add_plugin(MyPlugin);
//! let mut app = app.build();
//! app.run();
//! ```

pub mod error;
pub use error::EngineError;

pub mod app;
pub mod color;
pub mod config;
pub mod context;
pub mod debug;
pub mod engine;
pub mod event;
pub mod gameobject;
pub mod hierarchy;
pub mod logger;
pub mod math_utils;
pub mod memory;
pub mod monobehaviour;
pub mod monobehaviour_runner;
pub mod player_loop;
pub mod plugin;
pub mod plugin_loader;
pub mod plugins;
pub mod profiler;
pub mod resource;
pub mod system;
pub mod time;
pub mod transform;
pub mod world;

// Re-export for convenience
pub use context::Context;
pub use event::{Event, EventBus, EventBusExt, EventHandler};
pub use gameobject::{Component, GameObject, GameObjectHandle};
pub use hierarchy::{get_ancestors, get_depth, get_root, is_ancestor, sync_transforms};
pub use monobehaviour::{MonoBehaviour, MonoBehaviourHolder};
pub use monobehaviour_runner::MonoBehaviourRunner;
pub use player_loop::{Phase, PlayerLoop};
pub use system::System;
pub use time::Time;
pub use transform::{Space, Transform};
pub use world::World;

#[cfg(target_os = "android")]
pub mod android;
