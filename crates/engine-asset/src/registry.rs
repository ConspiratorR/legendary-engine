use crate::asset::{Asset, Handle};
use std::any::Any;
use std::collections::HashMap;

/// Type alias for asset constructor functions used in hot-reload.
type AssetConstructor = Box<dyn Fn(Box<dyn Any + Send + Sync>) -> Option<Box<dyn Any>>>;

/// A type-erased asset store keyed by string paths.
pub struct Registry {
    assets: HashMap<String, Box<dyn Any>>,
    /// Type-erased constructors for hot-reload: take `Box<dyn Any + Send + Sync>`,
    /// produce a new `Box<dyn Any>` containing a `Handle<T>`.
    constructors: HashMap<String, AssetConstructor>,
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
            constructors: HashMap::new(),
        }
    }

    /// Store an asset under `key` and return a [`Handle`] to it.
    ///
    /// Also registers a hot-reload constructor so the asset can be updated
    /// when the source file changes.
    pub fn store<T: Asset + Send + Sync + Clone + 'static>(
        &mut self,
        key: &str,
        asset: T,
    ) -> Handle<T> {
        let handle = Handle::new(asset);
        let handle_clone = handle.clone();
        self.assets
            .insert(key.to_string(), Box::new(handle.clone()));

        // Register a constructor that creates a new Handle<T> from imported data
        self.constructors.insert(
            key.to_string(),
            Box::new(|new_data: Box<dyn Any + Send + Sync>| {
                let asset = new_data.downcast::<T>().ok()?;
                Some(Box::new(Handle::new(*asset)))
            }),
        );

        handle_clone
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

    /// Replace an asset entry with new imported data (type-erased).
    ///
    /// Uses the registered constructor to create a new `Handle<T>` from the
    /// imported data, then swaps it into the assets map.
    ///
    /// Returns `true` if the key existed and was replaced.
    pub fn replace(&mut self, key: &str, new_asset: Box<dyn Any + Send + Sync>) -> bool {
        if let Some(constructor) = self.constructors.get(key)
            && let Some(new_entry) = constructor(new_asset)
        {
            self.assets.insert(key.to_string(), new_entry);
            return true;
        }
        false
    }

    /// Remove an asset entry and its constructor.
    pub fn remove(&mut self, key: &str) -> bool {
        self.constructors.remove(key);
        self.assets.remove(key).is_some()
    }

    /// Get all stored keys.
    pub fn keys(&self) -> Vec<&str> {
        self.assets.keys().map(|s| s.as_str()).collect()
    }
}
