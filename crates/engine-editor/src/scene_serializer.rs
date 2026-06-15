//! Scene serialization — JSON-based save/load for editor scenes including
//! entities, components, and global settings.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::state::EditorState;

/// A complete scene containing entities and global settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: u32,
    pub entities: Vec<SceneEntity>,
    pub settings: SceneSettings,
}

fn default_version() -> u32 {
    1
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
    #[serde(default)]
    pub material: Option<MaterialDataSer>,
    #[serde(default)]
    pub light: Option<LightDataSer>,
    #[serde(default)]
    pub sprite: Option<SpriteDataSer>,
    #[serde(default)]
    pub particle: Option<ParticleDataSer>,
    #[serde(default)]
    pub audio: Option<AudioDataSer>,
    #[serde(default)]
    pub script: Option<ScriptDataSer>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub physics: Option<PhysicsDataSer>,
    #[serde(default)]
    pub render: Option<RenderDataSer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialDataSer {
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub ao: f32,
    pub emissive: [f32; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightDataSer {
    pub light_type: String,
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: f32,
    pub direction: [f32; 3],
    pub inner_angle: f32,
    pub outer_angle: f32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteDataSer {
    pub texture: String,
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub flip_x: bool,
    pub flip_y: bool,
    pub uv_region: [f32; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleDataSer {
    pub emitter_type: String,
    pub rate: f32,
    pub lifetime: f32,
    pub speed: f32,
    pub size_start: f32,
    pub size_end: f32,
    pub color_start: [f32; 4],
    pub color_end: [f32; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDataSer {
    pub source: String,
    pub volume: f32,
    pub looping: bool,
    pub spatial: bool,
    pub attenuation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptDataSer {
    pub script_path: String,
    pub enabled: bool,
    pub properties: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsDataSer {
    pub body_type: String,
    pub collider_type: String,
    pub mass: f32,
    pub friction: f32,
    pub restitution: f32,
    pub is_sensor: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderDataSer {
    pub material_name: String,
    pub mesh_name: String,
    pub cast_shadow: bool,
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
            version: 1,
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
            material: None,
            light: None,
            sprite: None,
            particle: None,
            audio: None,
            script: None,
            tags: Vec::new(),
            physics: None,
            render: None,
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

    /// Sets the current scene (e.g., after syncing from EditorState).
    pub fn set_current_scene(&mut self, scene: Scene) {
        self.current_scene = Some(scene);
        self.is_modified = true;
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

impl EditorState {
    pub fn to_scene(&self, name: &str) -> Scene {
        let mut scene = Scene::new(name.to_string());
        for node in &self.scene_tree.nodes {
            let transform = self
                .node_transforms
                .get(&node.id)
                .copied()
                .unwrap_or([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
            let mut entity = SceneEntity::new(node.id, node.name.clone());

            // Convert Euler angles (radians) to quaternion for serialization
            let quat = engine_math::Quat::from_euler(
                engine_math::EulerRot::XYZ,
                transform[3],
                transform[4],
                transform[5],
            );

            entity.transform = TransformData {
                translation: [transform[0], transform[1], transform[2]],
                rotation: [quat.x, quat.y, quat.z, quat.w],
                scale: [transform[6], transform[7], transform[8]],
            };
            entity.parent = node.parent;
            entity.children = node.children.clone();

            if let Some(mat) = self.node_materials.get(&node.id) {
                entity.material = Some(MaterialDataSer {
                    base_color: mat.base_color,
                    metallic: mat.metallic,
                    roughness: mat.roughness,
                    ao: mat.ao,
                    emissive: mat.emissive,
                });
            }
            if let Some(light) = self.node_lights.get(&node.id) {
                entity.light = Some(LightDataSer {
                    light_type: format!("{:?}", light.light_type).to_lowercase(),
                    color: light.color,
                    intensity: light.intensity,
                    range: light.range,
                    direction: light.direction,
                    inner_angle: light.inner_angle,
                    outer_angle: light.outer_angle,
                    enabled: light.enabled,
                });
            }
            if let Some(sprite) = self.node_sprites.get(&node.id) {
                entity.sprite = Some(SpriteDataSer {
                    texture: sprite.texture.clone(),
                    size: sprite.size,
                    color: sprite.color,
                    flip_x: sprite.flip_x,
                    flip_y: sprite.flip_y,
                    uv_region: sprite.uv_region,
                });
            }
            if let Some(particle) = self.node_particles.get(&node.id) {
                entity.particle = Some(ParticleDataSer {
                    emitter_type: particle.emitter_type.clone(),
                    rate: particle.rate,
                    lifetime: particle.lifetime,
                    speed: particle.speed,
                    size_start: particle.size_start,
                    size_end: particle.size_end,
                    color_start: particle.color_start,
                    color_end: particle.color_end,
                });
            }
            if let Some(audio) = self.node_audio.get(&node.id) {
                entity.audio = Some(AudioDataSer {
                    source: audio.source.clone(),
                    volume: audio.volume,
                    looping: audio.looping,
                    spatial: audio.spatial,
                    attenuation: audio.attenuation.clone(),
                });
            }
            if let Some(script) = self.node_scripts.get(&node.id) {
                entity.script = Some(ScriptDataSer {
                    script_path: script.script_path.clone(),
                    enabled: script.enabled,
                    properties: script.properties.clone(),
                });
            }
            if let Some(tags) = self.node_tags.get(&node.id) {
                entity.tags = tags.clone();
            }
            if let Some((body, col)) = self.node_physics.get(&node.id) {
                entity.physics = Some(PhysicsDataSer {
                    body_type: body.clone(),
                    collider_type: col.clone(),
                    mass: 1.0,
                    friction: 0.5,
                    restitution: 0.3,
                    is_sensor: false,
                });
            }
            if let Some((mat_name, mesh_name, cast_shadow)) = self.node_render.get(&node.id) {
                entity.render = Some(RenderDataSer {
                    material_name: mat_name.clone(),
                    mesh_name: mesh_name.clone(),
                    cast_shadow: *cast_shadow,
                });
            }
            scene.add_entity(entity);
        }
        scene
    }

    pub fn load_from_scene(&mut self, scene: &Scene) {
        self.scene_tree = crate::state::SceneTree {
            nodes: Vec::new(),
            root_ids: Vec::new(),
            next_id: 1,
        };
        self.node_transforms.clear();
        self.node_materials.clear();
        self.node_lights.clear();
        self.node_sprites.clear();
        self.node_particles.clear();
        self.node_audio.clear();
        self.node_scripts.clear();
        self.node_tags.clear();
        self.node_render.clear();
        self.node_physics.clear();
        self.selected_nodes.clear();

        let mut next_id = 1u64;
        for entity in &scene.entities {
            let node = crate::state::TreeNode {
                id: entity.id,
                name: entity.name.clone(),
                icon: "📦".into(),
                expanded: false,
                parent: entity.parent,
                children: entity.children.clone(),
            };
            self.scene_tree.nodes.push(node);
            if entity.parent.is_none() {
                self.scene_tree.root_ids.push(entity.id);
            }
            if entity.id >= next_id {
                next_id = entity.id + 1;
            }

            // Convert quaternion back to Euler angles (radians) for editor storage
            let quat = engine_math::Quat::from_xyzw(
                entity.transform.rotation[0],
                entity.transform.rotation[1],
                entity.transform.rotation[2],
                entity.transform.rotation[3],
            );
            let (rx, ry, rz) = quat.to_euler(engine_math::EulerRot::XYZ);

            self.node_transforms.insert(
                entity.id,
                [
                    entity.transform.translation[0],
                    entity.transform.translation[1],
                    entity.transform.translation[2],
                    rx,
                    ry,
                    rz,
                    entity.transform.scale[0],
                    entity.transform.scale[1],
                    entity.transform.scale[2],
                ],
            );

            if let Some(ref mat) = entity.material {
                self.node_materials.insert(
                    entity.id,
                    crate::state::MaterialData {
                        base_color: mat.base_color,
                        metallic: mat.metallic,
                        roughness: mat.roughness,
                        ao: mat.ao,
                        emissive: mat.emissive,
                    },
                );
            }
            if let Some(ref light) = entity.light {
                let lt = match light.light_type.as_str() {
                    "directional" => crate::state::LightType::Directional,
                    "point" => crate::state::LightType::Point,
                    "spot" => crate::state::LightType::Spot,
                    _ => crate::state::LightType::Directional,
                };
                self.node_lights.insert(
                    entity.id,
                    crate::state::LightData {
                        light_type: lt,
                        color: light.color,
                        intensity: light.intensity,
                        range: light.range,
                        direction: light.direction,
                        inner_angle: light.inner_angle,
                        outer_angle: light.outer_angle,
                        enabled: light.enabled,
                    },
                );
            }
            if let Some(ref sprite) = entity.sprite {
                self.node_sprites.insert(
                    entity.id,
                    crate::state::SpriteData {
                        texture: sprite.texture.clone(),
                        size: sprite.size,
                        color: sprite.color,
                        flip_x: sprite.flip_x,
                        flip_y: sprite.flip_y,
                        uv_region: sprite.uv_region,
                    },
                );
            }
            if let Some(ref particle) = entity.particle {
                self.node_particles.insert(
                    entity.id,
                    crate::state::ParticleData {
                        emitter_type: particle.emitter_type.clone(),
                        rate: particle.rate,
                        lifetime: particle.lifetime,
                        speed: particle.speed,
                        size_start: particle.size_start,
                        size_end: particle.size_end,
                        color_start: particle.color_start,
                        color_end: particle.color_end,
                    },
                );
            }
            if let Some(ref audio) = entity.audio {
                self.node_audio.insert(
                    entity.id,
                    crate::state::AudioData {
                        source: audio.source.clone(),
                        volume: audio.volume,
                        looping: audio.looping,
                        spatial: audio.spatial,
                        attenuation: audio.attenuation.clone(),
                    },
                );
            }
            if let Some(ref script) = entity.script {
                self.node_scripts.insert(
                    entity.id,
                    crate::state::ScriptData {
                        script_path: script.script_path.clone(),
                        enabled: script.enabled,
                        properties: script.properties.clone(),
                    },
                );
            }
            if !entity.tags.is_empty() {
                self.node_tags.insert(entity.id, entity.tags.clone());
            }
            if let Some(ref physics) = entity.physics {
                self.node_physics.insert(
                    entity.id,
                    (physics.body_type.clone(), physics.collider_type.clone()),
                );
            }
            if let Some(ref render) = entity.render {
                self.node_render.insert(
                    entity.id,
                    (
                        render.material_name.clone(),
                        render.mesh_name.clone(),
                        render.cast_shadow,
                    ),
                );
            } else {
                self.node_render
                    .insert(entity.id, ("Default".into(), "Cube".into(), true));
            }
        }
        self.scene_tree.next_id = next_id;
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

    #[test]
    fn test_extended_entity_serialization() {
        let mut scene = Scene::new("ExtendedTest".to_string());
        let mut entity = SceneEntity::new(1, "Player".to_string());
        entity.material = Some(MaterialDataSer {
            base_color: [0.8, 0.2, 0.1, 1.0],
            metallic: 0.5,
            roughness: 0.3,
            ao: 1.0,
            emissive: [0.0; 3],
        });
        entity.light = Some(LightDataSer {
            light_type: "point".into(),
            color: [1.0, 1.0, 1.0],
            intensity: 2.0,
            range: 10.0,
            direction: [0.0, -1.0, 0.0],
            inner_angle: 15.0,
            outer_angle: 30.0,
            enabled: true,
        });
        entity.sprite = Some(SpriteDataSer {
            texture: "player.png".into(),
            size: [64.0, 64.0],
            color: [1.0, 1.0, 1.0, 1.0],
            flip_x: false,
            flip_y: false,
            uv_region: [0.0, 0.0, 1.0, 1.0],
        });
        entity.tags = vec!["player".into(), "entity".into()];
        scene.add_entity(entity);

        let json = serde_json::to_string_pretty(&scene).unwrap();
        let loaded: Scene = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.entities.len(), 1);
        let e = &loaded.entities[0];
        assert!(e.material.is_some());
        assert!(e.light.is_some());
        assert!(e.sprite.is_some());
        assert_eq!(e.tags, vec!["player", "entity"]);
        assert_eq!(e.material.as_ref().unwrap().metallic, 0.5);
        assert_eq!(e.light.as_ref().unwrap().intensity, 2.0);
    }
}
