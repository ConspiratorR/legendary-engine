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
}
