use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// A complete scene containing entities and global settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub name: String,
    pub entities: Vec<SceneEntity>,
    pub settings: SceneSettings,
}

/// A single entity in a scene with transform, components, and hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEntity {
    pub id: u64,
    pub name: String,
    pub transform: TransformData,
    pub components: Vec<ComponentData>,
    pub children: Vec<u64>,
    pub parent: Option<u64>,
    pub active: bool,
}

/// Transform data (translation, rotation as quaternion, scale).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformData {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

impl Default for TransformData {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

/// Serialized component data with typed properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentData {
    pub type_name: String,
    pub properties: HashMap<String, PropertyValue>,
}

/// A typed property value for serialized components.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
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

/// Global scene rendering settings (ambient color, fog).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneSettings {
    pub ambient_color: [f32; 4],
    pub fog_enabled: bool,
    pub fog_color: [f32; 4],
    pub fog_near: f32,
    pub fog_far: f32,
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

impl Scene {
    /// Creates an empty scene with default settings.
    pub fn new(name: String) -> Self {
        Self {
            name,
            entities: Vec::new(),
            settings: SceneSettings::default(),
        }
    }

    /// Adds an entity to the scene.
    pub fn add_entity(&mut self, entity: SceneEntity) {
        self.entities.push(entity);
    }

    /// Removes an entity by ID, returning it if found.
    pub fn remove_entity(&mut self, id: u64) -> Option<SceneEntity> {
        if let Some(pos) = self.entities.iter().position(|e| e.id == id) {
            Some(self.entities.remove(pos))
        } else {
            None
        }
    }

    /// Returns a reference to the entity with the given ID.
    pub fn get_entity(&self, id: u64) -> Option<&SceneEntity> {
        self.entities.iter().find(|e| e.id == id)
    }

    /// Returns a mutable reference to the entity with the given ID.
    pub fn get_entity_mut(&mut self, id: u64) -> Option<&mut SceneEntity> {
        self.entities.iter_mut().find(|e| e.id == id)
    }

    /// Returns a human-readable summary of the scene.
    pub fn to_string_pretty(&self) -> String {
        let mut output = format!("Scene: {}\n", self.name);
        output += "Settings:\n";
        output += &format!("  Ambient Color: {:?}\n", self.settings.ambient_color);
        output += &format!("  Fog Enabled: {}\n", self.settings.fog_enabled);
        output += &format!("\nEntities ({}):\n", self.entities.len());

        for entity in &self.entities {
            output += &format!(
                "  Entity {}: {} (active: {})\n",
                entity.id, entity.name, entity.active
            );
            output += &format!(
                "    Transform: pos={:?} rot={:?} scale={:?}\n",
                entity.transform.translation, entity.transform.rotation, entity.transform.scale
            );
            if !entity.components.is_empty() {
                output += "    Components:\n";
                for component in &entity.components {
                    output += &format!("      - {}\n", component.type_name);
                }
            }
            if !entity.children.is_empty() {
                output += &format!("    Children: {:?}\n", entity.children);
            }
        }

        output
    }
}

impl SceneEntity {
    /// Creates a new entity with default transform and no components.
    pub fn new(id: u64, name: String) -> Self {
        Self {
            id,
            name,
            transform: TransformData::default(),
            components: Vec::new(),
            children: Vec::new(),
            parent: None,
            active: true,
        }
    }

    /// Adds a component to this entity.
    pub fn add_component(&mut self, component: ComponentData) {
        self.components.push(component);
    }

    /// Removes and returns the first component with the given type name.
    pub fn remove_component(&mut self, type_name: &str) -> Option<ComponentData> {
        self.components
            .iter()
            .position(|c| c.type_name == type_name)
            .map(|pos| self.components.remove(pos))
    }
}

impl ComponentData {
    /// Creates a new component with the given type name and no properties.
    pub fn new(type_name: String) -> Self {
        Self {
            type_name,
            properties: HashMap::new(),
        }
    }

    /// Adds a property to this component (builder pattern).
    pub fn with_property(mut self, key: &str, value: PropertyValue) -> Self {
        self.properties.insert(key.to_string(), value);
        self
    }
}

/// Manages scene creation, loading, saving, and modification tracking.
pub struct SceneManager {
    current_scene: Option<Scene>,
    scene_path: Option<PathBuf>,
    is_modified: bool,
}

impl fmt::Debug for SceneManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SceneManager")
            .field("has_scene", &self.current_scene.is_some())
            .field("scene_path", &self.scene_path)
            .field("is_modified", &self.is_modified)
            .finish()
    }
}

impl Clone for SceneManager {
    fn clone(&self) -> Self {
        Self {
            current_scene: self.current_scene.clone(),
            scene_path: self.scene_path.clone(),
            is_modified: self.is_modified,
        }
    }
}

impl SceneManager {
    /// Creates a new scene manager with no loaded scene.
    pub fn new() -> Self {
        Self {
            current_scene: None,
            scene_path: None,
            is_modified: false,
        }
    }

    /// Creates a new empty scene with the given name.
    pub fn create_scene(&mut self, name: String) {
        self.current_scene = Some(Scene::new(name));
        self.scene_path = None;
        self.is_modified = false;
    }

    /// Returns a reference to the current scene, if loaded.
    pub fn current_scene(&self) -> Option<&Scene> {
        self.current_scene.as_ref()
    }

    /// Returns a mutable reference to the current scene (marks as modified).
    pub fn current_scene_mut(&mut self) -> Option<&mut Scene> {
        self.is_modified = true;
        self.current_scene.as_mut()
    }

    /// Returns the file path of the current scene, if saved.
    pub fn scene_path(&self) -> Option<&Path> {
        self.scene_path.as_deref()
    }

    /// Returns `true` if the scene has unsaved changes.
    pub fn is_modified(&self) -> bool {
        self.is_modified
    }

    /// Marks the scene as having unsaved changes.
    pub fn mark_modified(&mut self) {
        self.is_modified = true;
    }

    /// Marks the scene as saved (clears modified flag).
    pub fn mark_saved(&mut self) {
        self.is_modified = false;
    }

    /// Creates a new entity in the current scene. Returns its ID, or `None` if no scene is loaded.
    pub fn new_entity(&mut self, name: String) -> Option<u64> {
        if let Some(ref mut scene) = self.current_scene {
            let id = scene.entities.iter().map(|e| e.id).max().unwrap_or(0) + 1;
            scene.add_entity(SceneEntity::new(id, name));
            self.is_modified = true;
            Some(id)
        } else {
            None
        }
    }

    /// Prints the current scene to stdout.
    pub fn print_scene(&self) {
        if let Some(ref scene) = self.current_scene {
            println!("{}", scene.to_string_pretty());
        }
    }

    /// Saves the current scene to the given file path (JSON format).
    pub fn save_scene(&mut self, path: &Path) -> Result<()> {
        let scene = self.current_scene.as_ref().context("No scene loaded")?;

        let json = serde_json::to_string_pretty(scene).context("Failed to serialize scene")?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        fs::write(path, json)
            .with_context(|| format!("Failed to write scene file: {}", path.display()))?;

        self.scene_path = Some(path.to_path_buf());
        self.is_modified = false;
        Ok(())
    }

    /// Saves the current scene to its previously-set path.
    pub fn save_current_scene(&mut self) -> Result<()> {
        let path = self
            .scene_path
            .clone()
            .context("No scene path set. Use save_scene_as first.")?;
        self.save_scene(&path)
    }

    /// Loads a scene from a JSON file.
    pub fn load_scene(&mut self, path: &Path) -> Result<()> {
        let json = fs::read_to_string(path)
            .with_context(|| format!("Failed to read scene file: {}", path.display()))?;

        let scene: Scene = serde_json::from_str(&json)
            .with_context(|| format!("Failed to parse scene file: {}", path.display()))?;

        self.current_scene = Some(scene);
        self.scene_path = Some(path.to_path_buf());
        self.is_modified = false;
        Ok(())
    }
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let mut scene = Scene::new("TestScene".to_string());
        scene.settings.fog_enabled = true;

        let mut entity = SceneEntity::new(1, "Player".to_string());
        entity.transform.translation = [1.0, 2.0, 3.0];
        entity.transform.scale = [2.0, 2.0, 2.0];
        entity.add_component(
            ComponentData::new("MeshRenderer".to_string())
                .with_property("mesh", PropertyValue::String("cube.obj".to_string()))
                .with_property("visible", PropertyValue::Bool(true)),
        );
        scene.add_entity(entity);

        let json = serde_json::to_string_pretty(&scene).unwrap();
        let loaded: Scene = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.name, "TestScene");
        assert!(loaded.settings.fog_enabled);
        assert_eq!(loaded.entities.len(), 1);
        assert_eq!(loaded.entities[0].name, "Player");
        assert_eq!(loaded.entities[0].transform.translation, [1.0, 2.0, 3.0]);
        assert_eq!(loaded.entities[0].components.len(), 1);
    }

    #[test]
    fn test_save_and_load_scene() {
        let dir = std::env::temp_dir().join("rust_engine_test_scene");
        let path = dir.join("test_scene.json");

        let mut mgr = SceneManager::new();
        mgr.create_scene("SaveTest".to_string());
        mgr.new_entity("Cube".to_string());
        mgr.new_entity("Light".to_string());

        mgr.save_scene(&path).unwrap();
        assert!(!mgr.is_modified());
        assert_eq!(mgr.scene_path(), Some(path.as_path()));

        let mut mgr2 = SceneManager::new();
        mgr2.load_scene(&path).unwrap();

        let scene = mgr2.current_scene().unwrap();
        assert_eq!(scene.name, "SaveTest");
        assert_eq!(scene.entities.len(), 2);
        assert_eq!(scene.entities[0].name, "Cube");
        assert_eq!(scene.entities[1].name, "Light");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_current_without_path_fails() {
        let mut mgr = SceneManager::new();
        mgr.create_scene("NoPath".to_string());
        assert!(mgr.save_current_scene().is_err());
    }
}
