use crate::asset::{Asset, Handle};
use std::collections::HashMap;

pub struct Registry {
    assets: HashMap<String, Box<dyn std::any::Any>>,
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }

    pub fn store<T: Asset>(&mut self, key: &str, asset: T) -> Handle<T> {
        let handle = Handle::new(asset);
        self.assets
            .insert(key.to_string(), Box::new(handle.clone()));
        handle
    }

    pub fn get<T: Asset>(&self, key: &str) -> Option<&T> {
        self.assets
            .get(key)?
            .downcast_ref::<Handle<T>>()
            .map(|h| h.get())
    }

    pub fn contains(&self, key: &str) -> bool {
        self.assets.contains_key(key)
    }

    /// Returns references to all stored handles of a given asset type.
    /// Iterates all entries and attempts to downcast each to Handle<T>.
    pub fn get_handles_of_type<T: Asset + 'static>(&self) -> Vec<&Handle<T>> {
        self.assets
            .values()
            .filter_map(|boxed| boxed.downcast_ref::<Handle<T>>())
            .collect()
    }
}
