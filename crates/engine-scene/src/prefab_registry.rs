//! Registry for managing loaded prefab definitions.
//!
//! [`PrefabRegistry`] stores [`PrefabDef`]s keyed by name and supports
//! loading/saving in JSON, RON, and binary formats via the existing
//! [`serialization`](super::serialization) infrastructure.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::prefab::{PrefabDef, PrefabError};
use super::serialization::SceneFormat;

// ── Prefab File Container ───────────────────────────────────────────

/// Wrapper for serializing multiple prefabs to a single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabFile {
    /// All prefabs in this file.
    pub prefabs: Vec<PrefabDef>,
}

// ── Registry ────────────────────────────────────────────────────────

/// Central store for loaded [`PrefabDef`]s.
///
/// Prefabs are registered by name and can be looked up, loaded from disk,
/// or saved back out.
///
/// # Example
///
/// ```rust,no_run
/// use engine_scene::prefab_registry::PrefabRegistry;
///
/// let mut registry = PrefabRegistry::new();
/// registry.load("assets/prefabs/enemy.ron").unwrap();
/// let enemy = registry.get("Enemy").unwrap();
/// ```
pub struct PrefabRegistry {
    prefabs: HashMap<String, PrefabDef>,
}

impl PrefabRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            prefabs: HashMap::new(),
        }
    }

    /// Register a prefab definition.
    ///
    /// If a prefab with the same name already exists it is replaced.
    pub fn register(&mut self, prefab: PrefabDef) {
        self.prefabs.insert(prefab.name.clone(), prefab);
    }

    /// Remove a prefab by name, returning it if present.
    pub fn remove(&mut self, name: &str) -> Option<PrefabDef> {
        self.prefabs.remove(name)
    }

    /// Look up a prefab by name.
    pub fn get(&self, name: &str) -> Option<&PrefabDef> {
        self.prefabs.get(name)
    }

    /// Return whether a prefab with the given name exists.
    pub fn contains(&self, name: &str) -> bool {
        self.prefabs.contains_key(name)
    }

    /// Return the number of registered prefabs.
    pub fn len(&self) -> usize {
        self.prefabs.len()
    }

    /// Return whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.prefabs.is_empty()
    }

    /// Iterate over all registered prefab names.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.prefabs.keys().map(|s| s.as_str())
    }

    /// Load prefabs from a file, auto-detecting format from extension.
    ///
    /// The file must contain a [`PrefabFile`] (which may hold multiple prefabs).
    /// All prefabs are registered by name.
    pub fn load(&mut self, path: impl AsRef<Path>) -> Result<(), PrefabError> {
        let path = path.as_ref();
        let format = SceneFormat::from_path(path)?;
        let data = fs::read(path)?;
        self.load_from_bytes(&data, format)
    }

    /// Load prefabs from raw bytes in the given format.
    pub fn load_from_bytes(&mut self, data: &[u8], format: SceneFormat) -> Result<(), PrefabError> {
        let file: PrefabFile = match format {
            SceneFormat::Json => serde_json::from_slice(data)?,
            SceneFormat::Ron => {
                let s = std::str::from_utf8(data)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                ron::from_str(s)?
            }
            SceneFormat::Bin => bincode::deserialize(data)?,
        };
        for prefab in file.prefabs {
            self.register(prefab);
        }
        Ok(())
    }

    /// Save a single prefab to a file, auto-detecting format from extension.
    pub fn save(&self, name: &str, path: impl AsRef<Path>) -> Result<(), PrefabError> {
        let prefab = self
            .prefabs
            .get(name)
            .ok_or_else(|| PrefabError::NotFound(name.to_string()))?;
        let path = path.as_ref();
        let format = SceneFormat::from_path(path)?;
        let file = PrefabFile {
            prefabs: vec![prefab.clone()],
        };
        let data = serialize_prefab_file(&file, format)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, data)?;
        Ok(())
    }

    /// Save all registered prefabs to a single file.
    pub fn save_all(&self, path: impl AsRef<Path>) -> Result<(), PrefabError> {
        let path = path.as_ref();
        let format = SceneFormat::from_path(path)?;
        let file = PrefabFile {
            prefabs: self.prefabs.values().cloned().collect(),
        };
        let data = serialize_prefab_file(&file, format)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, data)?;
        Ok(())
    }
}

impl Default for PrefabRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Serialization Helpers ───────────────────────────────────────────

fn serialize_prefab_file(file: &PrefabFile, format: SceneFormat) -> Result<Vec<u8>, PrefabError> {
    match format {
        SceneFormat::Json => {
            let json = serde_json::to_string_pretty(file)?;
            Ok(json.into_bytes())
        }
        SceneFormat::Ron => {
            let ron = ron::ser::to_string_pretty(file, ron::ser::PrettyConfig::default())?;
            Ok(ron.into_bytes())
        }
        SceneFormat::Bin => {
            let bin = bincode::serialize(file)?;
            Ok(bin)
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefab::{ComponentTemplate, PrefabNode};
    use crate::serialization::{PropertyValue, TransformData};

    fn sample_prefab(name: &str) -> PrefabDef {
        let mut prefab = PrefabDef::new(name);
        prefab.root = PrefabNode::new("root")
            .with_component(
                ComponentTemplate::new("MeshRenderer")
                    .with_property("mesh", PropertyValue::String("cube.obj".into())),
            )
            .with_child(PrefabNode::new("child").with_transform(TransformData {
                translation: [0.0, 1.0, 0.0],
                ..Default::default()
            }));
        prefab
    }

    #[test]
    fn test_registry_register_and_get() {
        let mut registry = PrefabRegistry::new();
        registry.register(sample_prefab("Enemy"));

        assert!(registry.contains("Enemy"));
        assert_eq!(registry.len(), 1);
        let enemy = registry.get("Enemy").unwrap();
        assert_eq!(enemy.name, "Enemy");
    }

    #[test]
    fn test_registry_remove() {
        let mut registry = PrefabRegistry::new();
        registry.register(sample_prefab("Enemy"));
        let removed = registry.remove("Enemy");
        assert!(removed.is_some());
        assert!(!registry.contains("Enemy"));
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_names() {
        let mut registry = PrefabRegistry::new();
        registry.register(sample_prefab("A"));
        registry.register(sample_prefab("B"));

        let mut names: Vec<&str> = registry.names().collect();
        names.sort();
        assert_eq!(names, vec!["A", "B"]);
    }

    #[test]
    fn test_registry_overwrite() {
        let mut registry = PrefabRegistry::new();
        registry.register(sample_prefab("Enemy"));
        let mut updated = sample_prefab("Enemy");
        updated.root.components.clear();
        registry.register(updated);

        let enemy = registry.get("Enemy").unwrap();
        assert!(enemy.root.components.is_empty());
    }

    #[test]
    fn test_save_and_load_json() {
        let dir = std::env::temp_dir().join("rust_engine_prefab_test_json");
        let path = dir.join("enemy.json");

        let mut registry = PrefabRegistry::new();
        registry.register(sample_prefab("Enemy"));
        registry.save("Enemy", &path).unwrap();

        let mut registry2 = PrefabRegistry::new();
        registry2.load(&path).unwrap();

        assert!(registry2.contains("Enemy"));
        let enemy = registry2.get("Enemy").unwrap();
        assert_eq!(enemy.root.children.len(), 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_and_load_ron() {
        let dir = std::env::temp_dir().join("rust_engine_prefab_test_ron");
        let path = dir.join("enemy.ron");

        let mut registry = PrefabRegistry::new();
        registry.register(sample_prefab("Enemy"));
        registry.save("Enemy", &path).unwrap();

        let mut registry2 = PrefabRegistry::new();
        registry2.load(&path).unwrap();

        assert!(registry2.contains("Enemy"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_and_load_bin() {
        let dir = std::env::temp_dir().join("rust_engine_prefab_test_bin");
        let path = dir.join("enemy.bin");

        let mut registry = PrefabRegistry::new();
        registry.register(sample_prefab("Enemy"));
        registry.save("Enemy", &path).unwrap();

        let mut registry2 = PrefabRegistry::new();
        registry2.load(&path).unwrap();

        assert!(registry2.contains("Enemy"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_all_and_load() {
        let dir = std::env::temp_dir().join("rust_engine_prefab_test_saveall");
        let path = dir.join("all.json");

        let mut registry = PrefabRegistry::new();
        registry.register(sample_prefab("Enemy"));
        registry.register(sample_prefab("Player"));
        registry.save_all(&path).unwrap();

        let mut registry2 = PrefabRegistry::new();
        registry2.load(&path).unwrap();

        assert!(registry2.contains("Enemy"));
        assert!(registry2.contains("Player"));
        assert_eq!(registry2.len(), 2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_nonexistent_prefab() {
        let registry = PrefabRegistry::new();
        let result = registry.save("Missing", "/tmp/test.json");
        assert!(result.is_err());
    }
}
