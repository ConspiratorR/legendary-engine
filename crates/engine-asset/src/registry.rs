use crate::asset::{Asset, Handle};
use std::collections::HashMap;

/// A type-erased asset store keyed by string paths.
pub struct Registry {
    assets: HashMap<String, Box<dyn std::any::Any>>,
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }

    /// Store an asset under `key` and return a [`Handle`] to it.
    pub fn store<T: Asset>(&mut self, key: &str, asset: T) -> Handle<T> {
        let handle = Handle::new(asset);
        self.assets
            .insert(key.to_string(), Box::new(handle.clone()));
        handle
    }

    /// Retrieve a shared reference to the asset at `key`.
    pub fn get<T: Asset>(&self, key: &str) -> Option<&T> {
        self.assets
            .get(key)?
            .downcast_ref::<Handle<T>>()
            .map(|h| h.get())
    }

    /// Returns `true` if an asset exists at `key`.
    pub fn contains(&self, key: &str) -> bool {
        self.assets.contains_key(key)
    }

    /// Return references to all stored handles of a given asset type.
    pub fn get_handles_of_type<T: Asset + 'static>(&self) -> Vec<&Handle<T>> {
        self.assets
            .values()
            .filter_map(|boxed| boxed.downcast_ref::<Handle<T>>())
            .collect()
    }
}
