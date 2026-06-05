//! # engine-asset
//!
//! Asset loading and management for the RustEngine.
//!
//! Provides handle-based asset management with reference counting,
//! type registration, file system scanning, and loaders for
//! images, glTF models, and audio files.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use engine_asset::manager::AssetManager;
//!
//! let mut assets = AssetManager::with_defaults();
//! // let texture = assets.import::<Texture>(Path::new("textures/player.png"))?;
//! ```
//!
//! ## Core Concepts
//!
//! - [`asset::Handle<T>`] — reference-counted asset handles
//! - [`registry::Registry`] — type-erased asset store keyed by path
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
