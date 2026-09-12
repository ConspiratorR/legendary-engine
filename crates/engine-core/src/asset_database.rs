use std::any::Any;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::asset_handle::AssetHandle;
use crate::scriptable_asset::{
    self, AssetMeta, GuidIndex, SoAssetError, load_scriptable_object, save_scriptable_object,
};
use crate::scriptable_object::ScriptableObject;

/// A centralized asset database for managing ScriptableObject assets.
///
/// Assets are stored by name and type, providing a simple registry
/// for creating, querying, and managing assets at runtime.
/// Disk-backed assets also get a stable GUID via `.meta` sidecars.
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
    /// GUID → (path, meta) for assets saved or scanned from disk.
    guid_index: GuidIndex,
    /// Default directory for `save_asset_to_disk` / `scan_directory`.
    assets_root: Option<PathBuf>,
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
            guid_index: GuidIndex::new(),
            assets_root: None,
        }
    }

    /// Create a new asset database with a specified capacity hint.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(capacity),
            guid_index: GuidIndex::new(),
            assets_root: None,
        }
    }

    /// Set the root directory used for disk save/load/scan.
    pub fn set_assets_root(&mut self, root: impl Into<PathBuf>) {
        self.assets_root = Some(root.into());
    }

    /// Get the assets root directory, if set.
    pub fn assets_root(&self) -> Option<&Path> {
        self.assets_root.as_deref()
    }

    /// GUID index (path lookups for disk-backed assets).
    pub fn guid_index(&self) -> &GuidIndex {
        &self.guid_index
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
        self.guid_index = GuidIndex::new();
    }

    // ============================================================
    // Disk-backed ScriptableObject assets (Unity .asset + .meta)
    // ============================================================

    /// Save an in-memory asset to disk and index it by GUID.
    ///
    /// Writes `name.asset` + `name.asset.meta` under `dir` (or `assets_root`).
    pub fn save_asset_to_disk<T: ScriptableObject + Clone + 'static>(
        &mut self,
        dir: Option<&Path>,
        name: &str,
        asset: &T,
    ) -> Result<AssetMeta, SoAssetError> {
        let root = dir
            .map(|p| p.to_path_buf())
            .or_else(|| self.assets_root.clone())
            .ok_or_else(|| SoAssetError::InvalidPath("no assets root or dir set".into()))?;
        let path = root.join(format!("{name}.asset"));
        let meta = save_scriptable_object(&path, name, asset)?;
        self.guid_index.insert(path, meta.clone());
        // Also keep a runtime copy
        let _ = self.create_asset(name, asset.clone());
        Ok(meta)
    }

    /// Load a ScriptableObject from disk by file path and register it.
    pub fn load_asset_from_path<T: ScriptableObject + Clone + 'static>(
        &mut self,
        path: &Path,
        name: &str,
    ) -> Result<AssetHandle<T>, SoAssetError> {
        let data: T = load_scriptable_object(path)?;
        if let Some(meta) = scriptable_asset::load_asset_meta(path)? {
            self.guid_index.insert(path.to_path_buf(), meta);
        }
        Ok(self.create_asset(name, data))
    }

    /// Load an asset by GUID (must have been saved/scanned so the index knows the path).
    pub fn load_asset_by_guid<T: ScriptableObject + Clone + 'static>(
        &mut self,
        guid: &str,
    ) -> Result<AssetHandle<T>, SoAssetError> {
        let (path, meta) = self
            .guid_index
            .get_by_guid(guid)
            .map(|(p, m)| (p.clone(), m.clone()))
            .ok_or_else(|| SoAssetError::NotFound(format!("guid {guid}")))?;
        let data: T = load_scriptable_object(&path)?;
        Ok(self.create_asset(&meta.name, data))
    }

    /// Scan a directory for `*.asset` files and index their GUIDs.
    ///
    /// Does not load payloads into memory; use [`load_asset_by_guid`] after.
    pub fn scan_directory(&mut self, dir: &Path) -> Result<usize, SoAssetError> {
        let found = scriptable_asset::scan_asset_directory(dir)?;
        let n = found.len();
        for (path, meta) in found {
            self.guid_index.insert(path, meta);
        }
        Ok(n)
    }

    /// Scan the configured assets root (if any).
    pub fn scan_assets_root(&mut self) -> Result<usize, SoAssetError> {
        let root = self
            .assets_root
            .clone()
            .ok_or_else(|| SoAssetError::InvalidPath("assets_root not set".into()))?;
        self.scan_directory(&root)
    }

    /// Resolve a name to a GUID via the index.
    pub fn guid_for_name(&self, name: &str) -> Option<&str> {
        self.guid_index.guid_for_name(name)
    }

    /// Get the disk path for a GUID.
    pub fn path_for_guid(&self, guid: &str) -> Option<&Path> {
        self.guid_index.get_by_guid(guid).map(|(p, _)| p.as_path())
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

    #[test]
    fn test_save_and_load_by_guid() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = AssetDatabase::new();
        db.set_assets_root(dir.path());

        let asset = make_test_asset("Goblin", 80);
        let meta = db.save_asset_to_disk(None, "Goblin", &asset).unwrap();
        assert_eq!(meta.name, "Goblin");
        assert!(db.has_asset("Goblin"));
        assert_eq!(db.guid_for_name("Goblin"), Some(meta.guid.as_str()));

        // Fresh database, load by GUID
        let mut db2 = AssetDatabase::new();
        let n = db2.scan_directory(dir.path()).unwrap();
        assert_eq!(n, 1);
        let handle = db2.load_asset_by_guid::<TestAsset>(&meta.guid).unwrap();
        assert_eq!(handle.get().value, 80);
        assert!(db2.path_for_guid(&meta.guid).is_some());
    }

    #[test]
    fn test_load_asset_from_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Hero.asset");
        let asset = make_test_asset("Hero", 100);
        crate::scriptable_asset::save_scriptable_object(&path, "Hero", &asset).unwrap();

        let mut db = AssetDatabase::new();
        let handle = db.load_asset_from_path::<TestAsset>(&path, "Hero").unwrap();
        assert_eq!(handle.get().value, 100);
        assert!(db.guid_for_name("Hero").is_some());
    }
}
