//! Asset loading, storage, and format handling.
//!
//! Provides [`Handle<T>](asset::Handle) for reference-counted asset
//! handles, [`Registry`](registry::Registry) for asset storage, and
//! typed asset definitions in [`types`](types) (textures, meshes,
//! materials, audio clips, scripts).
//!
//! ## v2 Pipeline
//!
//! The new asset pipeline adds:
//! - [`pipeline::ImportPipeline`] — unified import with format-specific importers
//! - [`cache::AssetCache`] — hash-based cache invalidation
//! - [`watcher::FileWatcher`] — debounced file system monitoring
//! - [`async_loader::AsyncLoader`] — background thread pool for non-blocking loads
//! - [`manager::AssetManager`] — unified interface combining all subsystems

pub mod asset;
pub mod async_loader;
pub mod bundle;
pub mod cache;
pub mod error;
pub mod filesystem;
pub mod format;
pub mod loader;
pub mod manager;
pub mod pipeline;
pub mod preview;
pub mod registry;
pub mod streaming;
pub mod types;
pub mod watcher;

pub use error::AssetError;
