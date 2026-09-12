//! ScriptableObject asset files on disk (Unity `.asset` + `.meta` GUID).
//!
//! # Unity Documentation
//! - <https://docs.unity3d.com/Manual/ScriptableObjects.html>
//! - <https://docs.unity3d.com/Manual/AssetDatabase.html>
//!
//! ## On-disk layout
//! ```text
//! Assets/
//!   EnemyData.asset        // serialized ScriptableObject payload
//!   EnemyData.asset.meta   // GUID + type metadata
//! ```
//!
//! ## File formats
//!
//! `EnemyData.asset`:
//! ```json
//! {
//!   "format": 1,
//!   "type": "EnemyData",
//!   "name": "Goblin",
//!   "data": { "health": 80.0, "speed": 3.5 }
//! }
//! ```
//!
//! `EnemyData.asset.meta`:
//! ```json
//! {
//!   "guid": "9f8a7b6c5d4e3f2a1b0c9d8e7f6a5b4c",
//!   "type": "EnemyData",
//!   "name": "Goblin"
//! }
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::scriptable_object::ScriptableObject;

/// Current asset file format version.
pub const ASSET_FORMAT_VERSION: u32 = 1;

/// Errors from ScriptableObject asset I/O.
#[derive(Debug, thiserror::Error)]
pub enum SoAssetError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("asset type mismatch: expected '{expected}', found '{found}'")]
    TypeMismatch { expected: String, found: String },
    #[error("asset not found: {0}")]
    NotFound(String),
    #[error("invalid asset path: {0}")]
    InvalidPath(String),
}

/// Meta sidecar describing an asset's stable identity (Unity `.meta`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetMeta {
    /// Stable GUID (32 hex chars, no dashes) — never changes when the file moves.
    pub guid: String,
    /// Rust type name of the ScriptableObject.
    #[serde(rename = "type")]
    pub type_name: String,
    /// Display name.
    pub name: String,
}

impl AssetMeta {
    /// Create a new meta with a fresh GUID.
    pub fn new(type_name: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            guid: new_guid(),
            type_name: type_name.into(),
            name: name.into(),
        }
    }
}

/// Payload wrapper for a `.asset` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetFile<T> {
    /// Format version.
    pub format: u32,
    /// Type name (for validation on load).
    #[serde(rename = "type")]
    pub type_name: String,
    /// Display name.
    pub name: String,
    /// Serialized ScriptableObject fields.
    pub data: T,
}

/// Generate a new GUID (32 lowercase hex characters, Unity-style, no dashes).
pub fn new_guid() -> String {
    // Use a simple counter + time-based mix for uniqueness without extra deps.
    // For production, swap in uuid crate; this is adequate for local assets.
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let c = COUNTER.fetch_add(1, Ordering::Relaxed);
    // xorshift-ish mix
    let mut x = t ^ (c.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    format!("{:032x}", x)
}

/// Resolve the `.meta` path for an asset path.
pub fn meta_path_for(asset_path: &Path) -> PathBuf {
    let mut p = asset_path.as_os_str().to_owned();
    p.push(".meta");
    PathBuf::from(p)
}

/// Save a ScriptableObject as `.asset` + `.meta`.
///
/// # Unity Documentation
/// ScriptableObjects are project assets; Unity writes a `.meta` GUID next to them.
pub fn save_scriptable_object<T: ScriptableObject>(
    asset_path: &Path,
    name: &str,
    asset: &T,
) -> Result<AssetMeta, SoAssetError> {
    if let Some(parent) = asset_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let type_name = std::any::type_name::<T>()
        .rsplit("::")
        .next()
        .unwrap_or("Unknown")
        .to_string();

    let file = AssetFile {
        format: ASSET_FORMAT_VERSION,
        type_name: type_name.clone(),
        name: name.to_string(),
        data: asset,
    };
    let json = serde_json::to_string_pretty(&file)?;
    std::fs::write(asset_path, json)?;

    // Reuse existing GUID if meta already exists (stability across saves).
    let meta_path = meta_path_for(asset_path);
    let meta = if meta_path.exists() {
        let text = std::fs::read_to_string(&meta_path)?;
        let mut m: AssetMeta = serde_json::from_str(&text)?;
        m.name = name.to_string();
        m.type_name = type_name;
        m
    } else {
        AssetMeta::new(type_name, name)
    };

    let meta_json = serde_json::to_string_pretty(&meta)?;
    std::fs::write(&meta_path, meta_json)?;
    Ok(meta)
}

/// Load a ScriptableObject from `.asset` (validates type name).
pub fn load_scriptable_object<T: ScriptableObject>(asset_path: &Path) -> Result<T, SoAssetError> {
    let text = std::fs::read_to_string(asset_path)?;
    let file: AssetFile<T> = serde_json::from_str(&text)?;

    let expected = std::any::type_name::<T>()
        .rsplit("::")
        .next()
        .unwrap_or("Unknown");
    if file.type_name != expected {
        return Err(SoAssetError::TypeMismatch {
            expected: expected.to_string(),
            found: file.type_name,
        });
    }
    Ok(file.data)
}

/// Load the `.meta` for an asset, if present.
pub fn load_asset_meta(asset_path: &Path) -> Result<Option<AssetMeta>, SoAssetError> {
    let meta_path = meta_path_for(asset_path);
    if !meta_path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(meta_path)?;
    Ok(Some(serde_json::from_str(&text)?))
}

/// Ensure a `.meta` exists for an asset; creates one if missing.
/// Returns the (possibly new) meta.
pub fn ensure_asset_meta(
    asset_path: &Path,
    type_name: &str,
    name: &str,
) -> Result<AssetMeta, SoAssetError> {
    if let Some(meta) = load_asset_meta(asset_path)? {
        return Ok(meta);
    }
    let meta = AssetMeta::new(type_name, name);
    let json = serde_json::to_string_pretty(&meta)?;
    std::fs::write(meta_path_for(asset_path), json)?;
    Ok(meta)
}

/// Scan a directory for `*.asset` files and return their metas.
///
/// Missing metas are auto-created so every asset has a stable GUID.
pub fn scan_asset_directory(dir: &Path) -> Result<Vec<(PathBuf, AssetMeta)>, SoAssetError> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("asset") {
            continue;
        }
        // Peek type/name from the asset file for meta bootstrap
        let text = std::fs::read_to_string(&path)?;
        let value: serde_json::Value = serde_json::from_str(&text)?;
        let type_name = value
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();
        let name = value
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unnamed")
            .to_string();
        let meta = ensure_asset_meta(&path, &type_name, &name)?;
        out.push((path, meta));
    }
    Ok(out)
}

/// Stable reference to a ScriptableObject by GUID (Unity asset reference).
///
/// Serializes as a plain GUID string in scene/component fields, so references
/// survive file moves when `.meta` is preserved.
///
/// ```json
/// { "guid": "9f8a7b6c5d4e3f2a1b0c9d8e7f6a5b4c" }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct AssetRef {
    /// Empty string = null reference.
    pub guid: String,
}

impl AssetRef {
    /// Null reference (Unity `null` object reference).
    pub fn null() -> Self {
        Self {
            guid: String::new(),
        }
    }

    /// Reference by GUID.
    pub fn from_guid(guid: impl Into<String>) -> Self {
        Self { guid: guid.into() }
    }

    /// From a meta sidecar.
    pub fn from_meta(meta: &AssetMeta) -> Self {
        Self {
            guid: meta.guid.clone(),
        }
    }

    pub fn is_null(&self) -> bool {
        self.guid.is_empty()
    }

    pub fn guid(&self) -> &str {
        &self.guid
    }
}

impl std::fmt::Display for AssetRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_null() {
            write!(f, "AssetRef(null)")
        } else {
            write!(f, "AssetRef({})", self.guid)
        }
    }
}

/// A ScriptableObject asset was reloaded from disk (hot-reload).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetReloadEvent {
    pub guid: String,
    pub name: String,
    pub path: PathBuf,
}

/// In-memory index: GUID → (path, meta). Used by AssetDatabase.
#[derive(Debug, Default, Clone)]
pub struct GuidIndex {
    by_guid: HashMap<String, (PathBuf, AssetMeta)>,
    by_name: HashMap<String, String>, // name → guid
}

impl GuidIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, path: PathBuf, meta: AssetMeta) {
        self.by_name.insert(meta.name.clone(), meta.guid.clone());
        self.by_guid.insert(meta.guid.clone(), (path, meta));
    }

    pub fn get_by_guid(&self, guid: &str) -> Option<&(PathBuf, AssetMeta)> {
        self.by_guid.get(guid)
    }

    pub fn guid_for_name(&self, name: &str) -> Option<&str> {
        self.by_name.get(name).map(|s| s.as_str())
    }

    pub fn len(&self) -> usize {
        self.by_guid.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_guid.is_empty()
    }

    pub fn remove(&mut self, guid: &str) -> Option<(PathBuf, AssetMeta)> {
        let removed = self.by_guid.remove(guid)?;
        self.by_name.remove(&removed.1.name);
        Some(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
    struct EnemyData {
        health: f32,
        speed: f32,
    }

    impl crate::object::Object for EnemyData {
        fn Name(&self) -> &str {
            "EnemyData"
        }
        fn SetName(&mut self, _name: &str) {}
        fn GetInstanceID(&self) -> crate::object::InstanceId {
            0
        }
    }
    impl ScriptableObject for EnemyData {}

    #[test]
    fn test_guid_format() {
        let g = new_guid();
        assert_eq!(g.len(), 32);
        assert!(g.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(new_guid(), g);
    }

    #[test]
    fn test_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Goblin.asset");
        let data = EnemyData {
            health: 80.0,
            speed: 3.5,
        };
        let meta = save_scriptable_object(&path, "Goblin", &data).unwrap();
        assert_eq!(meta.name, "Goblin");
        assert_eq!(meta.type_name, "EnemyData");
        assert_eq!(meta.guid.len(), 32);

        let loaded: EnemyData = load_scriptable_object(&path).unwrap();
        assert_eq!(loaded, data);

        let meta2 = load_asset_meta(&path).unwrap().unwrap();
        assert_eq!(meta2.guid, meta.guid);
    }

    #[test]
    fn test_save_preserves_guid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Shared.asset");
        let m1 = save_scriptable_object(&path, "Shared", &EnemyData::default()).unwrap();
        let m2 = save_scriptable_object(
            &path,
            "Shared",
            &EnemyData {
                health: 1.0,
                speed: 1.0,
            },
        )
        .unwrap();
        assert_eq!(m1.guid, m2.guid);
    }

    #[test]
    fn test_guid_index() {
        let mut idx = GuidIndex::new();
        let meta = AssetMeta::new("EnemyData", "Goblin");
        let guid = meta.guid.clone();
        idx.insert(PathBuf::from("Assets/Goblin.asset"), meta);
        assert_eq!(idx.len(), 1);
        assert_eq!(idx.guid_for_name("Goblin"), Some(guid.as_str()));
        assert!(idx.get_by_guid(&guid).is_some());
    }

    #[test]
    fn test_scan_directory() {
        let dir = tempfile::tempdir().unwrap();
        save_scriptable_object(&dir.path().join("A.asset"), "A", &EnemyData::default()).unwrap();
        save_scriptable_object(
            &dir.path().join("B.asset"),
            "B",
            &EnemyData {
                health: 10.0,
                speed: 1.0,
            },
        )
        .unwrap();
        let found = scan_asset_directory(dir.path()).unwrap();
        assert_eq!(found.len(), 2);
    }
}
