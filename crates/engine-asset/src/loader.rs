use crate::asset::Asset;
use crate::registry::Registry;

/// Trait for asset loaders that can load a specific asset type from a path.
pub trait Loader {
    /// The asset type this loader produces.
    type AssetType: Asset;
    /// Load the asset at `path` into the `registry`.
    fn load(&self, path: &str, registry: &mut Registry);
}

/// Convenience function to store an asset in a registry at the given path.
pub fn load_asset<T: Asset>(registry: &mut Registry, path: &str, asset: T) {
    registry.store(path, asset);
}
