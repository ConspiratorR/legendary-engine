//! Simple asset loading utilities.
//!
//! Provides the [`Loader`] trait for path-based asset loading and
//! a convenience [`load_asset`] function for storing assets in a registry.

use crate::asset::Asset;
use crate::registry::Registry;

/// Trait for asset loaders that can load a specific asset type from a path.
///
/// Implementors define how to read and deserialize an asset from a string path
/// and store it in a [`Registry`].
pub trait Loader {
    /// The asset type this loader produces.
    type AssetType: Asset;
    /// Load the asset at `path` into the `registry`.
    fn load(&self, path: &str, registry: &mut Registry);
}

/// Convenience function to store an asset in a registry at the given path.
///
/// This is a simple wrapper around [`Registry::store`] for cases where
/// the asset is already constructed and just needs to be registered.
pub fn load_asset<T: Asset + Send + Sync + Clone + 'static>(
    registry: &mut Registry,
    path: &str,
    asset: T,
) {
    registry.store(path, asset);
}
