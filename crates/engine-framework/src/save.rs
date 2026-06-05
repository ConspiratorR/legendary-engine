use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Serializable game state snapshot.
///
/// Stores arbitrary key-value data organized by category.
/// Use `SaveManager` to persist to / load from disk.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SaveData {
    /// Schema version for migration support.
    pub version: u32,
    /// Human-readable save name.
    pub name: String,
    /// Timestamp (seconds since epoch).
    pub timestamp: u64,
    /// Key-value storage organized by category.
    pub categories: HashMap<String, HashMap<String, SaveValue>>,
}

/// A value that can be saved to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum SaveValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    IntArray(Vec<i64>),
    FloatArray(Vec<f64>),
    StringArray(Vec<String>),
}

impl SaveData {
    /// Create a new save data snapshot with the given name and current timestamp.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            version: 1,
            name: name.into(),
            timestamp: now_timestamp(),
            categories: HashMap::new(),
        }
    }

    /// Set a value in a category.
    pub fn set(&mut self, category: &str, key: &str, value: SaveValue) {
        self.categories
            .entry(category.to_string())
            .or_default()
            .insert(key.to_string(), value);
    }

    /// Get a value from a category.
    pub fn get(&self, category: &str, key: &str) -> Option<&SaveValue> {
        self.categories.get(category)?.get(key)
    }

    /// Get all keys in a category.
    pub fn category_keys(&self, category: &str) -> Vec<&String> {
        self.categories
            .get(category)
            .map(|m| m.keys().collect())
            .unwrap_or_default()
    }

    /// Check if a category exists.
    pub fn has_category(&self, category: &str) -> bool {
        self.categories.contains_key(category)
    }
}

/// Manages save/load operations for game state.
pub struct SaveManager {
    save_dir: PathBuf,
    max_slots: usize,
}

impl SaveManager {
    /// Create a new save manager with the given save directory.
    pub fn new(save_dir: impl Into<PathBuf>) -> Self {
        Self {
            save_dir: save_dir.into(),
            max_slots: 10,
        }
    }

    /// Set the maximum number of save slots.
    pub fn with_max_slots(mut self, max: usize) -> Self {
        self.max_slots = max;
        self
    }

    /// Get the path for a save slot.
    pub fn slot_path(&self, slot: usize) -> PathBuf {
        self.save_dir.join(format!("save_{}.json", slot))
    }

    /// Save data to a slot.
    pub fn save(&self, slot: usize, data: &SaveData) -> Result<(), SaveError> {
        if slot >= self.max_slots {
            return Err(SaveError::InvalidSlot(slot));
        }
        fs::create_dir_all(&self.save_dir).map_err(|e| SaveError::Io(e.to_string()))?;
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| SaveError::Serialization(e.to_string()))?;
        fs::write(self.slot_path(slot), json).map_err(|e| SaveError::Io(e.to_string()))?;
        Ok(())
    }

    /// Load data from a slot.
    pub fn load(&self, slot: usize) -> Result<SaveData, SaveError> {
        if slot >= self.max_slots {
            return Err(SaveError::InvalidSlot(slot));
        }
        let path = self.slot_path(slot);
        if !path.exists() {
            return Err(SaveError::SlotEmpty(slot));
        }
        let json = fs::read_to_string(&path).map_err(|e| SaveError::Io(e.to_string()))?;
        let data: SaveData =
            serde_json::from_str(&json).map_err(|e| SaveError::Deserialization(e.to_string()))?;
        Ok(data)
    }

    /// Check if a slot has save data.
    pub fn slot_exists(&self, slot: usize) -> bool {
        self.slot_path(slot).exists()
    }

    /// Delete a save slot.
    pub fn delete_slot(&self, slot: usize) -> Result<(), SaveError> {
        let path = self.slot_path(slot);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| SaveError::Io(e.to_string()))?;
        }
        Ok(())
    }

    /// List all existing save slots.
    pub fn list_slots(&self) -> Vec<usize> {
        (0..self.max_slots)
            .filter(|&s| self.slot_exists(s))
            .collect()
    }

    /// Save to a file path directly.
    pub fn save_to_path(&self, path: &Path, data: &SaveData) -> Result<(), SaveError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| SaveError::Io(e.to_string()))?;
        }
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| SaveError::Serialization(e.to_string()))?;
        fs::write(path, json).map_err(|e| SaveError::Io(e.to_string()))?;
        Ok(())
    }

    /// Load from a file path directly.
    pub fn load_from_path(&self, path: &Path) -> Result<SaveData, SaveError> {
        let json = fs::read_to_string(path).map_err(|e| SaveError::Io(e.to_string()))?;
        let data: SaveData =
            serde_json::from_str(&json).map_err(|e| SaveError::Deserialization(e.to_string()))?;
        Ok(data)
    }
}

/// Error type for save/load operations.
#[derive(Debug, Clone)]
pub enum SaveError {
    /// The requested slot index exceeds the configured maximum.
    InvalidSlot(usize),
    /// The requested slot does not contain any saved data.
    SlotEmpty(usize),
    /// An I/O error occurred while reading or writing.
    Io(String),
    /// Failed to serialize save data to JSON.
    Serialization(String),
    /// Failed to deserialize JSON into save data.
    Deserialization(String),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSlot(s) => write!(f, "Invalid save slot: {}", s),
            Self::SlotEmpty(s) => write!(f, "Save slot {} is empty", s),
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::Serialization(e) => write!(f, "Serialization error: {}", e),
            Self::Deserialization(e) => write!(f, "Deserialization error: {}", e),
        }
    }
}

impl std::error::Error for SaveError {}

fn now_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_save_manager() -> (SaveManager, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SaveManager::new(dir.path()).with_max_slots(3);
        (mgr, dir)
    }

    #[test]
    fn test_save_data_set_get() {
        let mut data = SaveData::new("Test");
        data.set("player", "health", SaveValue::Int(100));
        data.set("player", "name", SaveValue::String("Hero".to_string()));
        assert!(matches!(
            data.get("player", "health"),
            Some(SaveValue::Int(100))
        ));
        assert!(matches!(data.get("player", "name"), Some(SaveValue::String(s)) if s == "Hero"));
    }

    #[test]
    fn test_save_load_roundtrip() {
        let (mgr, _dir) = temp_save_manager();
        let mut data = SaveData::new("Roundtrip");
        data.set("game", "level", SaveValue::Int(5));
        data.set("game", "score", SaveValue::Float(1234.5));

        mgr.save(0, &data).unwrap();
        let loaded = mgr.load(0).unwrap();
        assert_eq!(loaded.name, "Roundtrip");
        assert!(matches!(
            loaded.get("game", "level"),
            Some(SaveValue::Int(5))
        ));
    }

    #[test]
    fn test_slot_exists_and_delete() {
        let (mgr, _dir) = temp_save_manager();
        assert!(!mgr.slot_exists(0));
        mgr.save(0, &SaveData::new("test")).unwrap();
        assert!(mgr.slot_exists(0));
        mgr.delete_slot(0).unwrap();
        assert!(!mgr.slot_exists(0));
    }

    #[test]
    fn test_invalid_slot() {
        let (mgr, _dir) = temp_save_manager();
        assert!(mgr.save(99, &SaveData::new("test")).is_err());
        assert!(mgr.load(99).is_err());
    }

    #[test]
    fn test_load_empty_slot() {
        let (mgr, _dir) = temp_save_manager();
        assert!(mgr.load(0).is_err());
    }

    #[test]
    fn test_list_slots() {
        let (mgr, _dir) = temp_save_manager();
        mgr.save(0, &SaveData::new("a")).unwrap();
        mgr.save(2, &SaveData::new("b")).unwrap();
        let slots = mgr.list_slots();
        assert_eq!(slots, vec![0, 2]);
    }

    #[test]
    fn test_save_load_via_path() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SaveManager::new(dir.path());
        let path = dir.path().join("custom_save.json");
        let mut data = SaveData::new("Custom");
        data.set("stats", "hp", SaveValue::Int(80));
        mgr.save_to_path(&path, &data).unwrap();
        let loaded = mgr.load_from_path(&path).unwrap();
        assert_eq!(loaded.name, "Custom");
    }

    #[test]
    fn test_save_value_types() {
        let mut data = SaveData::new("Types");
        data.set("t", "bool", SaveValue::Bool(true));
        data.set("t", "vec3", SaveValue::Vec3([1.0, 2.0, 3.0]));
        data.set("t", "arr", SaveValue::IntArray(vec![1, 2, 3]));
        assert!(matches!(data.get("t", "bool"), Some(SaveValue::Bool(true))));
        assert!(matches!(data.get("t", "vec3"), Some(SaveValue::Vec3(_))));
        assert!(matches!(data.get("t", "arr"), Some(SaveValue::IntArray(a)) if a.len() == 3));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut data = SaveData::new("SerTest");
        data.set("k", "v", SaveValue::Float(3.14));
        let json = serde_json::to_string(&data).unwrap();
        let loaded: SaveData = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.name, "SerTest");
    }

    #[test]
    fn test_delete_nonexistent_slot() {
        let (mgr, _dir) = temp_save_manager();
        // Deleting a slot that doesn't exist should succeed (noop)
        assert!(mgr.delete_slot(0).is_ok());
        assert!(!mgr.slot_exists(0));
    }

    #[test]
    fn test_max_slots_boundary() {
        let (mgr, _dir) = temp_save_manager(); // max_slots = 3
        assert!(mgr.save(0, &SaveData::new("a")).is_ok());
        assert!(mgr.save(1, &SaveData::new("b")).is_ok());
        assert!(mgr.save(2, &SaveData::new("c")).is_ok());
        // Slot 3 is out of range
        assert!(mgr.save(3, &SaveData::new("d")).is_err());
        assert!(mgr.load(3).is_err());
    }

    #[test]
    fn test_overwrite_existing_slot() {
        let (mgr, _dir) = temp_save_manager();
        mgr.save(0, &SaveData::new("first")).unwrap();
        mgr.save(0, &SaveData::new("second")).unwrap();
        let loaded = mgr.load(0).unwrap();
        assert_eq!(loaded.name, "second");
    }

    #[test]
    fn test_save_data_category_keys() {
        let mut data = SaveData::new("Keys");
        data.set("a", "x", SaveValue::Int(1));
        data.set("a", "y", SaveValue::Int(2));
        data.set("b", "z", SaveValue::Int(3));
        let mut keys = data.category_keys("a");
        keys.sort();
        assert_eq!(keys, vec!["x", "y"]);
        assert!(data.category_keys("nonexistent").is_empty());
    }

    #[test]
    fn test_save_data_has_category() {
        let mut data = SaveData::new("Cat");
        assert!(!data.has_category("player"));
        data.set("player", "hp", SaveValue::Int(100));
        assert!(data.has_category("player"));
    }

    #[test]
    fn test_save_value_all_variants() {
        let mut data = SaveData::new("All");
        data.set("v", "bool", SaveValue::Bool(false));
        data.set("v", "int", SaveValue::Int(-42));
        data.set("v", "float", SaveValue::Float(2.718));
        data.set("v", "string", SaveValue::String("hello".into()));
        data.set("v", "vec2", SaveValue::Vec2([1.0, 2.0]));
        data.set("v", "vec3", SaveValue::Vec3([1.0, 2.0, 3.0]));
        data.set("v", "vec4", SaveValue::Vec4([1.0, 2.0, 3.0, 4.0]));
        data.set("v", "int_arr", SaveValue::IntArray(vec![10, 20, 30]));
        data.set("v", "float_arr", SaveValue::FloatArray(vec![1.1, 2.2]));
        data.set(
            "v",
            "str_arr",
            SaveValue::StringArray(vec!["a".into(), "b".into()]),
        );

        assert!(matches!(
            data.get("v", "bool"),
            Some(SaveValue::Bool(false))
        ));
        assert!(matches!(data.get("v", "int"), Some(SaveValue::Int(-42))));
        assert!(
            matches!(data.get("v", "float"), Some(SaveValue::Float(f)) if (*f - 2.718).abs() < 1e-10)
        );
        assert!(matches!(data.get("v", "string"), Some(SaveValue::String(s)) if s == "hello"));
        assert!(matches!(data.get("v", "vec2"), Some(SaveValue::Vec2(_))));
        assert!(matches!(data.get("v", "vec3"), Some(SaveValue::Vec3(_))));
        assert!(matches!(data.get("v", "vec4"), Some(SaveValue::Vec4(_))));
        assert!(matches!(data.get("v", "int_arr"), Some(SaveValue::IntArray(a)) if a.len() == 3));
        assert!(
            matches!(data.get("v", "float_arr"), Some(SaveValue::FloatArray(a)) if a.len() == 2)
        );
        assert!(
            matches!(data.get("v", "str_arr"), Some(SaveValue::StringArray(a)) if a.len() == 2)
        );
    }
}
