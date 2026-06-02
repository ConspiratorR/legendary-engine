//! Scene serialization supporting JSON, RON, and binary (bincode) formats.
//!
//! Provides [`SceneData`] as the serializable representation of a scene, along with
//! format detection via [`SceneFormat`] and save/load functions.
//!
//! # Example
//!
//! ```rust,no_run
//! use engine_scene::serialization::{SceneData, SceneFormat, save_scene, load_scene};
//!
//! let scene = SceneData::new("my_scene");
//! save_scene(&scene, "output.ron").unwrap();
//! let loaded = load_scene("output.ron").unwrap();
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── Error Type ──────────────────────────────────────────────────────

/// Errors that can occur during scene serialization or deserialization.
#[derive(Error, Debug)]
pub enum SceneError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("RON serialization error: {0}")]
    RonSer(#[from] ron::error::Error),

    #[error("RON deserialization error: {0}")]
    RonDe(#[from] ron::error::SpannedError),

    #[error("BIN error: {0}")]
    Bincode(#[from] bincode::Error),

    #[error("unsupported scene format for extension: '{0}'")]
    UnsupportedFormat(String),

    #[error("path has no file extension")]
    NoExtension,
}

// ── Scene Format ────────────────────────────────────────────────────

/// Supported scene file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneFormat {
    /// JSON (`.json` or `.scene`).
    Json,
    /// RON — Rusty Object Notation (`.ron`).
    Ron,
    /// Binary via bincode (`.bin`).
    Bin,
}

impl SceneFormat {
    /// Detect format from a file path's extension.
    pub fn from_path(path: &Path) -> Result<Self, SceneError> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or(SceneError::NoExtension)?;
        Self::from_extension(ext)
    }

    /// Detect format from a bare extension string (without leading dot).
    pub fn from_extension(ext: &str) -> Result<Self, SceneError> {
        match ext.to_lowercase().as_str() {
            "json" | "scene" => Ok(Self::Json),
            "ron" => Ok(Self::Ron),
            "bin" => Ok(Self::Bin),
            other => Err(SceneError::UnsupportedFormat(other.to_string())),
        }
    }
}

// ── Scene Data Types ────────────────────────────────────────────────

/// Serializable representation of a complete scene.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneData {
    /// Scene name.
    pub name: String,
    /// All entities in the scene.
    pub entities: Vec<SceneEntityData>,
    /// Global scene settings.
    pub settings: SceneSettings,
}

/// Serializable representation of a single entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEntityData {
    /// Unique entity ID.
    pub id: u64,
    /// Human-readable name.
    pub name: String,
    /// Local transform.
    pub transform: TransformData,
    /// Attached components (type name → properties).
    pub components: Vec<ComponentData>,
    /// Child entity IDs.
    pub children: Vec<u64>,
    /// Parent entity ID, or `None` for root-level entities.
    pub parent: Option<u64>,
    /// Whether the entity is active.
    pub active: bool,
}

/// Serializable transform data using raw arrays for portability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformData {
    /// Position `[x, y, z]`.
    pub translation: [f32; 3],
    /// Rotation quaternion `[x, y, z, w]`.
    pub rotation: [f32; 4],
    /// Scale `[x, y, z]`.
    pub scale: [f32; 3],
}

/// Serializable component data with arbitrary properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentData {
    /// The component type name (e.g. `"MeshRenderer"`).
    pub type_name: String,
    /// Key-value properties.
    pub properties: HashMap<String, PropertyValue>,
}

/// A typed property value for component serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropertyValue {
    Float(f32),
    Int(i64),
    Bool(bool),
    String(String),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Color([f32; 4]),
}

/// Global scene settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneSettings {
    /// Ambient light color `[r, g, b, a]`.
    pub ambient_color: [f32; 4],
    /// Whether fog is enabled.
    pub fog_enabled: bool,
    /// Fog color `[r, g, b, a]`.
    pub fog_color: [f32; 4],
    /// Fog near distance.
    pub fog_near: f32,
    /// Fog far distance.
    pub fog_far: f32,
}

// ── Default Implementations ─────────────────────────────────────────

impl Default for TransformData {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

impl Default for SceneSettings {
    fn default() -> Self {
        Self {
            ambient_color: [0.2, 0.2, 0.2, 1.0],
            fog_enabled: false,
            fog_color: [0.5, 0.5, 0.5, 1.0],
            fog_near: 10.0,
            fog_far: 100.0,
        }
    }
}

// ── Constructors ────────────────────────────────────────────────────

impl SceneData {
    /// Create a new empty scene with default settings.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            entities: Vec::new(),
            settings: SceneSettings::default(),
        }
    }

    /// Add an entity to the scene.
    pub fn add_entity(&mut self, entity: SceneEntityData) {
        self.entities.push(entity);
    }

    /// Find an entity by ID.
    pub fn get_entity(&self, id: u64) -> Option<&SceneEntityData> {
        self.entities.iter().find(|e| e.id == id)
    }

    /// Find an entity by ID (mutable).
    pub fn get_entity_mut(&mut self, id: u64) -> Option<&mut SceneEntityData> {
        self.entities.iter_mut().find(|e| e.id == id)
    }

    /// Remove an entity by ID.
    pub fn remove_entity(&mut self, id: u64) -> Option<SceneEntityData> {
        if let Some(pos) = self.entities.iter().position(|e| e.id == id) {
            Some(self.entities.remove(pos))
        } else {
            None
        }
    }
}

impl SceneEntityData {
    /// Create a new entity with default transform and no components.
    pub fn new(id: u64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            transform: TransformData::default(),
            components: Vec::new(),
            children: Vec::new(),
            parent: None,
            active: true,
        }
    }

    /// Add a component to this entity.
    pub fn add_component(&mut self, component: ComponentData) {
        self.components.push(component);
    }
}

impl ComponentData {
    /// Create a new component with the given type name.
    pub fn new(type_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            properties: HashMap::new(),
        }
    }

    /// Add a property to this component.
    pub fn with_property(mut self, key: impl Into<String>, value: PropertyValue) -> Self {
        self.properties.insert(key.into(), value);
        self
    }
}

// ── Serialize / Deserialize ─────────────────────────────────────────

/// Serialize a scene to a string in the given format.
pub fn serialize_scene(scene: &SceneData, format: SceneFormat) -> Result<Vec<u8>, SceneError> {
    match format {
        SceneFormat::Json => {
            let json = serde_json::to_string_pretty(scene)?;
            Ok(json.into_bytes())
        }
        SceneFormat::Ron => {
            let ron = ron::ser::to_string_pretty(scene, ron::ser::PrettyConfig::default())?;
            Ok(ron.into_bytes())
        }
        SceneFormat::Bin => {
            let bin = bincode::serialize(scene)?;
            Ok(bin)
        }
    }
}

/// Deserialize a scene from bytes in the given format.
pub fn deserialize_scene(data: &[u8], format: SceneFormat) -> Result<SceneData, SceneError> {
    match format {
        SceneFormat::Json => {
            let scene = serde_json::from_slice(data)?;
            Ok(scene)
        }
        SceneFormat::Ron => {
            let s = std::str::from_utf8(data)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            let scene = ron::from_str(s)?;
            Ok(scene)
        }
        SceneFormat::Bin => {
            let scene = bincode::deserialize(data)?;
            Ok(scene)
        }
    }
}

// ── File I/O ────────────────────────────────────────────────────────

/// Save a scene to a file, auto-detecting format from the file extension.
pub fn save_scene(scene: &SceneData, path: impl AsRef<Path>) -> Result<(), SceneError> {
    let path = path.as_ref();
    let format = SceneFormat::from_path(path)?;
    let data = serialize_scene(scene, format)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, data)?;
    Ok(())
}

/// Load a scene from a file, auto-detecting format from the file extension.
pub fn load_scene(path: impl AsRef<Path>) -> Result<SceneData, SceneError> {
    let path = path.as_ref();
    let format = SceneFormat::from_path(path)?;
    let data = fs::read(path)?;
    deserialize_scene(&data, format)
}

/// Save a scene to a file with an explicit format (ignores extension).
pub fn save_scene_as(
    scene: &SceneData,
    path: impl AsRef<Path>,
    format: SceneFormat,
) -> Result<(), SceneError> {
    let path = path.as_ref();
    let data = serialize_scene(scene, format)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, data)?;
    Ok(())
}

/// Load a scene from a file with an explicit format (ignores extension).
pub fn load_scene_as(path: impl AsRef<Path>, format: SceneFormat) -> Result<SceneData, SceneError> {
    let path = path.as_ref();
    let data = fs::read(path)?;
    deserialize_scene(&data, format)
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_scene() -> SceneData {
        let mut scene = SceneData::new("TestScene");
        scene.settings.fog_enabled = true;

        let mut entity = SceneEntityData::new(1, "Player");
        entity.transform.translation = [1.0, 2.0, 3.0];
        entity.transform.scale = [2.0, 2.0, 2.0];
        entity.add_component(
            ComponentData::new("MeshRenderer")
                .with_property("mesh", PropertyValue::String("cube.obj".into()))
                .with_property("visible", PropertyValue::Bool(true)),
        );
        scene.add_entity(entity);
        scene
    }

    #[test]
    fn test_format_from_extension() {
        assert_eq!(
            SceneFormat::from_extension("json").unwrap(),
            SceneFormat::Json
        );
        assert_eq!(
            SceneFormat::from_extension("scene").unwrap(),
            SceneFormat::Json
        );
        assert_eq!(
            SceneFormat::from_extension("ron").unwrap(),
            SceneFormat::Ron
        );
        assert_eq!(
            SceneFormat::from_extension("bin").unwrap(),
            SceneFormat::Bin
        );
        assert_eq!(
            SceneFormat::from_extension("RON").unwrap(),
            SceneFormat::Ron
        );
        assert!(SceneFormat::from_extension("xml").is_err());
    }

    #[test]
    fn test_format_from_path() {
        assert_eq!(
            SceneFormat::from_path(Path::new("scene.json")).unwrap(),
            SceneFormat::Json
        );
        assert_eq!(
            SceneFormat::from_path(Path::new("scene.ron")).unwrap(),
            SceneFormat::Ron
        );
        assert_eq!(
            SceneFormat::from_path(Path::new("scene.bin")).unwrap(),
            SceneFormat::Bin
        );
        assert!(SceneFormat::from_path(Path::new("scene")).is_err());
    }

    #[test]
    fn test_json_roundtrip() {
        let scene = sample_scene();
        let data = serialize_scene(&scene, SceneFormat::Json).unwrap();
        let loaded = deserialize_scene(&data, SceneFormat::Json).unwrap();

        assert_eq!(loaded.name, "TestScene");
        assert!(loaded.settings.fog_enabled);
        assert_eq!(loaded.entities.len(), 1);
        assert_eq!(loaded.entities[0].name, "Player");
        assert_eq!(loaded.entities[0].transform.translation, [1.0, 2.0, 3.0]);
        assert_eq!(loaded.entities[0].components.len(), 1);
    }

    #[test]
    fn test_ron_roundtrip() {
        let scene = sample_scene();
        let data = serialize_scene(&scene, SceneFormat::Ron).unwrap();
        let loaded = deserialize_scene(&data, SceneFormat::Ron).unwrap();

        assert_eq!(loaded.name, "TestScene");
        assert!(loaded.settings.fog_enabled);
        assert_eq!(loaded.entities.len(), 1);
        assert_eq!(loaded.entities[0].name, "Player");
        assert_eq!(loaded.entities[0].transform.translation, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_bin_roundtrip() {
        let scene = sample_scene();
        let data = serialize_scene(&scene, SceneFormat::Bin).unwrap();
        let loaded = deserialize_scene(&data, SceneFormat::Bin).unwrap();

        assert_eq!(loaded.name, "TestScene");
        assert!(loaded.settings.fog_enabled);
        assert_eq!(loaded.entities.len(), 1);
        assert_eq!(loaded.entities[0].name, "Player");
        assert_eq!(loaded.entities[0].transform.translation, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_bin_smaller_than_json() {
        let scene = sample_scene();
        let json = serialize_scene(&scene, SceneFormat::Json).unwrap();
        let bin = serialize_scene(&scene, SceneFormat::Bin).unwrap();
        assert!(
            bin.len() < json.len(),
            "binary ({}B) should be smaller than JSON ({}B)",
            bin.len(),
            json.len()
        );
    }

    #[test]
    fn test_file_io_json() {
        let dir = std::env::temp_dir().join("rust_engine_scene_test_json");
        let path = dir.join("test.json");

        let scene = sample_scene();
        save_scene(&scene, &path).unwrap();
        let loaded = load_scene(&path).unwrap();

        assert_eq!(loaded.name, "TestScene");
        assert_eq!(loaded.entities.len(), 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_file_io_ron() {
        let dir = std::env::temp_dir().join("rust_engine_scene_test_ron");
        let path = dir.join("test.ron");

        let scene = sample_scene();
        save_scene(&scene, &path).unwrap();
        let loaded = load_scene(&path).unwrap();

        assert_eq!(loaded.name, "TestScene");
        assert_eq!(loaded.entities.len(), 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_file_io_bin() {
        let dir = std::env::temp_dir().join("rust_engine_scene_test_bin");
        let path = dir.join("test.bin");

        let scene = sample_scene();
        save_scene(&scene, &path).unwrap();
        let loaded = load_scene(&path).unwrap();

        assert_eq!(loaded.name, "TestScene");
        assert_eq!(loaded.entities.len(), 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_scene_as_explicit_format() {
        let dir = std::env::temp_dir().join("rust_engine_scene_test_explicit");
        // Save as RON but with .txt extension
        let path = dir.join("test.txt");

        let scene = sample_scene();
        save_scene_as(&scene, &path, SceneFormat::Ron).unwrap();
        let loaded = load_scene_as(&path, SceneFormat::Ron).unwrap();

        assert_eq!(loaded.name, "TestScene");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_unsupported_format_error() {
        let result = SceneFormat::from_extension("xml");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("xml"));
    }

    #[test]
    fn test_empty_scene_roundtrip() {
        let scene = SceneData::new("Empty");

        for format in [SceneFormat::Json, SceneFormat::Ron, SceneFormat::Bin] {
            let data = serialize_scene(&scene, format).unwrap();
            let loaded = deserialize_scene(&data, format).unwrap();
            assert_eq!(loaded.name, "Empty");
            assert!(loaded.entities.is_empty());
        }
    }

    #[test]
    fn test_scene_with_parent_child() {
        let mut scene = SceneData::new("Hierarchy");

        let mut parent = SceneEntityData::new(1, "Parent");
        parent.children = vec![2, 3];

        let mut child1 = SceneEntityData::new(2, "Child1");
        child1.parent = Some(1);

        let mut child2 = SceneEntityData::new(3, "Child2");
        child2.parent = Some(1);

        scene.add_entity(parent);
        scene.add_entity(child1);
        scene.add_entity(child2);

        for format in [SceneFormat::Json, SceneFormat::Ron, SceneFormat::Bin] {
            let data = serialize_scene(&scene, format).unwrap();
            let loaded = deserialize_scene(&data, format).unwrap();
            assert_eq!(loaded.entities.len(), 3);
            assert_eq!(loaded.entities[0].children, vec![2, 3]);
            assert_eq!(loaded.entities[1].parent, Some(1));
        }
    }
}
