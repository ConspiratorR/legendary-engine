//! Asset loading, storage, and format handling.
//!
//! Provides [`Handle<T>](asset::Handle) for reference-counted asset
//! handles, [`Registry`](registry::Registry) for asset storage, and
//! typed asset definitions in [`types`](types) (textures, meshes,
//! materials, audio clips, scripts).

pub mod asset;
pub mod filesystem;
pub mod format;
pub mod loader;
pub mod registry;
pub mod types;
