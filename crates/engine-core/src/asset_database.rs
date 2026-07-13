use std::any::Any;
use std::collections::HashMap;

use crate::asset_handle::AssetHandle;
use crate::scriptable_object::ScriptableObject;

/// A centralized asset database for managing ScriptableObject assets.
///
/// Assets are stored by name and type, providing a simple registry
/// for creating, querying, and managing assets at runtime.
///
/// # Examples
///
/// ```rust
/// use engine_core::asset_database::AssetDatabase;
///
/// let mut db = AssetDatabase::new();
/// // db.create_asset("player_data", PlayerData { health: 100 });
/// // assert_eq!(db.asset_count(), 1);
/// // assert!(db.has_asset("player_data"));
/// ```
pub struct AssetDatabase {
    /// Stores assets as type-erased trait objects keyed by name.
    entries: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl Default for AssetDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetDatabase {
    /// Create a new empty asset database.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Create a new asset database with a specified capacity hint.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(capacity),
        }
    }

    /// Create an asset instance and register it in the database.
    ///
    /// Returns an [`AssetHandle`] referencing the stored asset.
    /// If an asset with the same name already exists, it is replaced.
    pub fn create_asset<T: ScriptableObject + Send + Sync + Clone + 'static>(
        &mut self,
        name: &str,
        asset: T,
    ) -> AssetHandle<T> {
        let handle = AssetHandle::new(asset);
        let handle_clone = handle.clone();
        self.entries.insert(name.to_string(), Box::new(handle));
        handle_clone
    }

    /// Get the total number of assets in the database.
    pub fn asset_count(&self) -> usize {
        self.entries.len()
    }

    /// Check if an asset with the given name exists.
    pub fn has_asset(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    /// Retrieve a reference to an asset by name.
    ///
    /// Returns `None` if no asset with that name exists or if the
    /// type does not match.
    pub fn get_asset<T: ScriptableObject + 'static>(&self, name: &str) -> Option<&AssetHandle<T>> {
        self.entries.get(name)?.downcast_ref::<AssetHandle<T>>()
    }

    /// Retrieve a mutable reference to an asset by name.
    ///
    /// Returns `None` if no asset with that name exists or if the
    /// type does not match.
    pub fn get_asset_mut<T: ScriptableObject + 'static>(
        &mut self,
        name: &str,
    ) -> Option<&mut AssetHandle<T>> {
        self.entries.get_mut(name)?.downcast_mut::<AssetHandle<T>>()
    }

    /// Remove an asset by name.
    ///
    /// Returns `true` if the asset was found and removed, `false` otherwise.
    pub fn remove_asset(&mut self, name: &str) -> bool {
        self.entries.remove(name).is_some()
    }

    /// Get a list of all asset names in the database.
    pub fn asset_names(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }

    /// Create a new asset instance from default (like Unity's ScriptableObject.CreateInstance<T>()).
    pub fn create_instance<T: ScriptableObject + Default + Send + Sync + Clone + 'static>(
        &mut self,
        name: &str,
    ) -> AssetHandle<T> {
        let asset = T::default();
        self.create_asset(name, asset)
    }

    /// Remove all assets from the database.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::Component;
    use crate::object::{InstanceId, Object};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestAsset {
        name: String,
        value: i32,
    }

    impl Default for TestAsset {
        fn default() -> Self {
            Self {
                name: String::new(),
                value: 0,
            }
        }
    }

    impl Object for TestAsset {
        fn Name(&self) -> &str {
            &self.name
        }

        fn SetName(&mut self, name: &str) {
            self.name = name.to_string();
        }

        fn GetInstanceID(&self) -> InstanceId {
            0
        }
    }

    impl Component for TestAsset {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    impl ScriptableObject for TestAsset {}

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct AnotherAsset {
        data: String,
    }

    impl Object for AnotherAsset {
        fn Name(&self) -> &str {
            &self.data
        }

        fn SetName(&mut self, name: &str) {
            self.data = name.to_string();
        }

        fn GetInstanceID(&self) -> InstanceId {
            0
        }
    }

    impl Component for AnotherAsset {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    impl ScriptableObject for AnotherAsset {}

    fn make_test_asset(name: &str, value: i32) -> TestAsset {
        TestAsset {
            name: name.to_string(),
            value,
        }
    }

    #[test]
    fn test_create_empty_database() {
        let db = AssetDatabase::new();
        assert_eq!(db.asset_count(), 0);
    }

    #[test]
    fn test_create_asset() {
        let mut db = AssetDatabase::new();
        let handle = db.create_asset("player", make_test_asset("player", 100));
        assert_eq!(handle.get().Name(), "player");
        assert_eq!(db.asset_count(), 1);
    }

    #[test]
    fn test_has_asset() {
        let mut db = AssetDatabase::new();
        assert!(!db.has_asset("missing"));
        db.create_asset("exists", make_test_asset("exists", 1));
        assert!(db.has_asset("exists"));
    }

    #[test]
    fn test_get_asset() {
        let mut db = AssetDatabase::new();
        db.create_asset("data", make_test_asset("data", 42));

        let handle = db.get_asset::<TestAsset>("data").unwrap();
        assert_eq!(handle.get().value, 42);
    }

    #[test]
    fn test_get_asset_wrong_type() {
        let mut db = AssetDatabase::new();
        db.create_asset("data", make_test_asset("data", 42));

        // Try to get as wrong type - should return None
        let result = db.get_asset::<AnotherAsset>("data");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_asset_nonexistent() {
        let db = AssetDatabase::new();
        assert!(db.get_asset::<TestAsset>("missing").is_none());
    }

    #[test]
    fn test_remove_asset() {
        let mut db = AssetDatabase::new();
        db.create_asset("to_remove", make_test_asset("to_remove", 1));
        assert_eq!(db.asset_count(), 1);

        assert!(db.remove_asset("to_remove"));
        assert_eq!(db.asset_count(), 0);
        assert!(!db.has_asset("to_remove"));
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut db = AssetDatabase::new();
        assert!(!db.remove_asset("missing"));
    }

    #[test]
    fn test_clear() {
        let mut db = AssetDatabase::new();
        db.create_asset("a", make_test_asset("a", 1));
        db.create_asset("b", make_test_asset("b", 2));
        assert_eq!(db.asset_count(), 2);

        db.clear();
        assert_eq!(db.asset_count(), 0);
    }

    #[test]
    fn test_asset_names() {
        let mut db = AssetDatabase::new();
        db.create_asset("alpha", make_test_asset("alpha", 1));
        db.create_asset("beta", make_test_asset("beta", 2));

        let mut names = db.asset_names();
        names.sort();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn test_replace_asset() {
        let mut db = AssetDatabase::new();
        db.create_asset("key", make_test_asset("key", 1));
        db.create_asset("key", make_test_asset("key", 2));

        assert_eq!(db.asset_count(), 1);
        let handle = db.get_asset::<TestAsset>("key").unwrap();
        assert_eq!(handle.get().value, 2);
    }

    #[test]
    fn test_create_instance() {
        let mut db = AssetDatabase::new();
        let handle = db.create_instance::<TestAsset>("instance");
        assert!(db.has_asset("instance"));
        assert_eq!(db.asset_count(), 1);
        assert_eq!(handle.get().name, "");
        assert_eq!(handle.get().value, 0);
    }

    #[test]
    fn test_multiple_types() {
        let mut db = AssetDatabase::new();
        db.create_asset("test_asset", make_test_asset("test_asset", 42));
        db.create_asset(
            "another",
            AnotherAsset {
                data: "hello".into(),
            },
        );

        assert_eq!(db.asset_count(), 2);
        assert!(db.get_asset::<TestAsset>("test_asset").is_some());
        assert!(db.get_asset::<AnotherAsset>("another").is_some());
    }

    #[test]
    fn test_with_capacity() {
        let db = AssetDatabase::with_capacity(100);
        assert_eq!(db.asset_count(), 0);
    }
}
