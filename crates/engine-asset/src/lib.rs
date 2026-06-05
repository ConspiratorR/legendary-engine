//! # engine-asset
//!
//! Asset loading and management for the RustEngine.
//!
//! Provides handle-based asset management with reference counting,
//! type registration, file system scanning, and loaders for
//! images, glTF models, and audio files.
//!
//! ## Architecture
//!
//! The crate is organized in layers:
//!
//! 1. **Types** ([`types`]) — concrete asset types: [`Texture`](types::Texture),
//!    [`AudioClip`](types::AudioClip), [`Mesh`](types::Mesh), [`Material`](types::Material),
//!    [`Script`](types::Script).
//!
//! 2. **Handle System** ([`asset`]) — [`Handle<T>`](asset::Handle) provides reference-counted
//!    access to assets. Cloning a handle increments the ref count; dropping decrements it.
//!    [`HandleId`](asset::HandleId) uniquely identifies a handle via its inner `Arc` pointer.
//!
//! 3. **Registry** ([`registry`]) — type-erased store keyed by string paths.
//!    Stores `Handle<T>` values and supports hot-reload via registered constructors.
//!
//! 4. **Import Pipeline** ([`pipeline`]) — [`AssetImporter`](pipeline::AssetImporter) trait
//!    for format-specific importers. [`ImportPipeline`](pipeline::ImportPipeline) registers
//!    importers by extension and runs imports from files or raw bytes.
//!
//! 5. **Cache** ([`cache`]) — [`AssetCache`](cache::AssetCache) tracks content hashes
//!    for incremental re-import. Dependencies between assets are tracked so that
//!    changing a texture triggers re-import of materials that reference it.
//!
//! 6. **File Watcher** ([`watcher`]) — [`FileWatcher`](watcher::FileWatcher) monitors
//!    directories with debounced events. The OS-level [`create_notify_watcher`](watcher::create_notify_watcher)
//!    helper spawns a background thread using the `notify` crate.
//!
//! 7. **Async Loader** ([`async_loader`]) — [`AsyncLoader`](async_loader::AsyncLoader)
//!    runs imports on a thread pool, returning results through a channel.
//!
//! 8. **Streaming** ([`streaming`]) — LOD selection, memory budgeting, and priority-based
//!    stream requests for large assets.
//!
//! 9. **Manager** ([`manager`]) — [`AssetManager`](manager::AssetManager) ties everything
//!    together: pipeline + registry + cache + watcher. Call [`update`](manager::AssetManager::update)
//!    each frame to process file changes.
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
