//! # engine-core
//!
//! Core engine systems for the RustEngine.
//!
//! This is the central crate that ties together all engine subsystems:
//! - Application builder and plugin system
//! - Time management
//! - Configuration system
//! - Logging and performance profiling
//!
//! ## Architecture
//!
//! engine-core follows a plugin-based architecture:
//!
//! ```text
//! AppBuilder -> Plugin::build() -> System Registration -> App::run()
//! ```
//!
//! Each subsystem (render, physics, audio, etc.) is integrated
//! as a plugin that registers its systems with the app builder.
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
pub mod debug;
pub mod engine;
pub mod event;
pub mod logger;
pub mod math_utils;
pub mod memory;
pub mod plugin;
pub mod plugins;
pub mod profiler;
pub mod resource;
pub mod time;
pub mod transform;
