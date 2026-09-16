use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::component::Component;
use crate::gameobject::{GameObject, GameObjectHandle};
use crate::scriptable_asset::AssetRef;
use crate::transform::Transform;
use crate::world::World;
use engine_math::{Quat, Vec3};

/// Serialized component data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComponentData {
    pub type_name: String,
    pub properties: HashMap<String, serde_json::Value>,
}

impl ComponentData {
    pub fn new(type_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            properties: HashMap::new(),
        }
    }

    pub fn with_property(mut self, key: &str, value: serde_json::Value) -> Self {
        self.properties.insert(key.to_string(), value);
        self
    }

    /// Store an [`AssetRef`] GUID under `key` (P3.3 scene asset references).
    pub fn insert_asset_ref(&mut self, key: &str, r: &AssetRef) {
        self.properties
            .insert(key.to_string(), serde_json::json!({ "guid": r.guid }));
    }

    /// Read an [`AssetRef`] from `key`; missing/null becomes `AssetRef::null()`.
    pub fn get_asset_ref(&self, key: &str) -> Option<AssetRef> {
        let v = self.properties.get(key)?;
        if v.is_null() {
            return Some(AssetRef::null());
        }
        if let Some(guid) = v.as_str() {
            return Some(if guid.is_empty() {
                AssetRef::null()
            } else {
                AssetRef::from_guid(guid)
            });
        }
        let guid = v.get("guid")?.as_str()?;
        Some(if guid.is_empty() {
            AssetRef::null()
        } else {
            AssetRef::from_guid(guid)
        })
    }
}

/// Serialized Transform data (built-in, not a Component).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransformData {
    pub local_position: Vec3,
    pub local_rotation: Quat,
    pub local_scale: Vec3,
}

/// Serialized GameObject data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameObjectData {
    pub name: String,
    pub tag: String,
    pub layer: i32,
    pub active: bool,
    pub transform: TransformData,
    pub components: Vec<ComponentData>,
    pub children: Vec<GameObjectData>,
}

/// Complete scene data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneData {
    pub name: String,
    pub version: u32,
    pub game_objects: Vec<GameObjectData>,
}

/// Trait for formatting components during serialization.
pub trait ComponentFormatter: Send + Sync {
    fn format(&self, component: &dyn Component) -> Option<ComponentData>;
    fn type_name(&self) -> &str;
}

/// Trait for deserializing components during scene loading.
pub trait ComponentDeserializer: Send + Sync {
    fn deserialize(&self, data: &ComponentData) -> Option<Box<dyn Component>>;
    fn type_name(&self) -> &str;
}

/// Scene serializer for saving and loading scenes.
pub struct SceneSerializer {
    formatters: Vec<Box<dyn ComponentFormatter>>,
    deserializers: HashMap<String, Box<dyn ComponentDeserializer>>,
}

impl SceneSerializer {
    /// Serializer with built-in Material / SpriteRenderer formatters.
    pub fn new() -> Self {
        let mut s = Self {
            formatters: Vec::new(),
            deserializers: HashMap::new(),
        };
        s.AddFormatter(Box::new(MaterialFormatter));
        s.AddFormatter(Box::new(SpriteRendererFormatter));
        s.AddDeserializer(Box::new(MaterialDeserializer));
        s.AddDeserializer(Box::new(SpriteRendererDeserializer));
        s
    }

    pub fn AddFormatter(&mut self, formatter: Box<dyn ComponentFormatter>) {
        self.formatters.push(formatter);
    }

    pub fn AddDeserializer(&mut self, deserializer: Box<dyn ComponentDeserializer>) {
        self.deserializers
            .insert(deserializer.type_name().to_string(), deserializer);
    }

    /// Serialize a World into SceneData.
    pub fn Save(&self, world: &World, name: &str) -> SceneData {
        let roots = world.GetRootGameObjects();
        let game_objects = roots
            .iter()
            .map(|&handle| self.SerializeGameObject(world, handle))
            .collect();

        SceneData {
            name: name.to_string(),
            version: 1,
            game_objects,
        }
    }

    /// Serialize a single GameObject and its children.
    fn SerializeGameObject(&self, world: &World, handle: GameObjectHandle) -> GameObjectData {
        let name = world.GetName(handle).to_string();
        let tag = world.GetTag(handle).to_string();
        let layer = world.GetLayer(handle);
        let active = world.IsActive(handle);

        // Serialize Transform (built-in)
        let transform_data = if let Some(t) = world.GetTransform(handle) {
            TransformData {
                local_position: t.LocalPosition(),
                local_rotation: t.LocalRotation(),
                local_scale: t.LocalScale(),
            }
        } else {
            TransformData {
                local_position: Vec3::ZERO,
                local_rotation: Quat::IDENTITY,
                local_scale: Vec3::ONE,
            }
        };

        // Serialize components via registered formatters (Material, SpriteRenderer, …)
        let components = world
            .get_gameobject(handle)
            .map(|go| {
                let mut out = Vec::new();
                for c in go.Components() {
                    for f in &self.formatters {
                        if let Some(data) = f.format(c.as_ref()) {
                            out.push(data);
                        }
                    }
                }
                out
            })
            .unwrap_or_default();

        // Serialize children
        let children = world
            .GetChildren(handle)
            .iter()
            .map(|&child_handle| self.SerializeGameObject(world, child_handle))
            .collect();

        GameObjectData {
            name,
            tag,
            layer,
            active,
            transform: transform_data,
            components,
            children,
        }
    }

    /// Deserialize SceneData into a World.
    pub fn Load(&self, scene: &SceneData, world: &mut World) -> Vec<GameObjectHandle> {
        scene
            .game_objects
            .iter()
            .map(|go_data| self.SpawnGameObject(world, go_data))
            .collect()
    }

    /// Spawn a GameObject from serialized data.
    fn SpawnGameObject(&self, world: &mut World, data: &GameObjectData) -> GameObjectHandle {
        let handle = world.CreateGameObject(&data.name);
        world.SetTag(handle, &data.tag);
        world.SetLayer(handle, data.layer);
        world.SetActive(handle, data.active);

        // Set Transform
        if let Some(t) = world.GetTransformMut(handle) {
            t.SetLocalPosition(data.transform.local_position);
            t.SetLocalRotation(data.transform.local_rotation);
            t.SetLocalScale(data.transform.local_scale);
        }

        // Restore components
        for cd in &data.components {
            if let Some(d) = self.deserializers.get(cd.type_name.as_str())
                && let Some(c) = d.deserialize(cd)
            {
                world.AddComponentBoxed(handle, c);
            }
        }

        // Spawn children and attach them
        for child_data in &data.children {
            let child_handle = self.SpawnGameObject(world, child_data);
            world.SetParent(child_handle, Some(handle));
        }

        handle
    }

    /// Save a scene (snake_case alias for Save).
    pub fn save(&self, world: &World, name: &str) -> SceneData {
        self.Save(world, name)
    }

    /// Load a scene (snake_case alias for Load).
    pub fn load(&self, scene: &SceneData, world: &mut World) -> Vec<GameObjectHandle> {
        self.Load(scene, world)
    }
}

impl Default for SceneSerializer {
    fn default() -> Self {
        Self::new()
    }
}

/// Save a scene to JSON string.
pub fn SaveSceneJson(world: &World, name: &str) -> Result<String, serde_json::Error> {
    let serializer = SceneSerializer::new();
    let scene = serializer.Save(world, name);
    serde_json::to_string_pretty(&scene)
}

/// Built-in formatter for [`crate::components::Material`].
struct MaterialFormatter;

impl ComponentFormatter for MaterialFormatter {
    fn type_name(&self) -> &str {
        "Material"
    }

    fn format(&self, component: &dyn Component) -> Option<ComponentData> {
        let m = component
            .as_any()
            .downcast_ref::<crate::components::Material>()?;
        let mut data = ComponentData::new("Material");
        data.properties
            .insert("base_color".into(), serde_json::json!(m.base_color));
        data.properties
            .insert("metallic".into(), serde_json::json!(m.metallic));
        data.properties
            .insert("smoothness".into(), serde_json::json!(m.smoothness));
        if !m.normal_map.is_empty() {
            data.properties
                .insert("normal_map".into(), serde_json::json!(m.normal_map));
        }
        Some(data)
    }
}

struct MaterialDeserializer;

impl ComponentDeserializer for MaterialDeserializer {
    fn type_name(&self) -> &str {
        "Material"
    }

    fn deserialize(&self, data: &ComponentData) -> Option<Box<dyn Component>> {
        let mut m = crate::components::Material::default();
        if let Some(c) = data.properties.get("base_color")
            && let Ok(arr) = serde_json::from_value::<[f32; 4]>(c.clone())
        {
            m.base_color = arr;
        }
        if let Some(v) = data.properties.get("metallic").and_then(|v| v.as_f64()) {
            m.metallic = v as f32;
        }
        if let Some(v) = data.properties.get("smoothness").and_then(|v| v.as_f64()) {
            m.smoothness = v as f32;
        }
        if let Some(v) = data.properties.get("normal_map").and_then(|v| v.as_str()) {
            m.normal_map = v.to_string();
        }
        Some(Box::new(m))
    }
}

/// Built-in formatter for [`crate::components::SpriteRenderer`].
struct SpriteRendererFormatter;

impl ComponentFormatter for SpriteRendererFormatter {
    fn type_name(&self) -> &str {
        "SpriteRenderer"
    }

    fn format(&self, component: &dyn Component) -> Option<ComponentData> {
        let sr = component
            .as_any()
            .downcast_ref::<crate::components::SpriteRenderer>()?;
        let mut data = ComponentData::new("SpriteRenderer");
        data.properties
            .insert("sprite".into(), serde_json::json!(sr.sprite));
        data.properties
            .insert("color".into(), serde_json::json!(sr.color));
        data.properties
            .insert("flip_x".into(), serde_json::json!(sr.flip_x));
        data.properties
            .insert("flip_y".into(), serde_json::json!(sr.flip_y));
        data.properties
            .insert("sorting_order".into(), serde_json::json!(sr.sorting_order));
        Some(data)
    }
}

struct SpriteRendererDeserializer;

impl ComponentDeserializer for SpriteRendererDeserializer {
    fn type_name(&self) -> &str {
        "SpriteRenderer"
    }

    fn deserialize(&self, data: &ComponentData) -> Option<Box<dyn Component>> {
        let mut sr = crate::components::SpriteRenderer::default();
        if let Some(v) = data.properties.get("sprite").and_then(|v| v.as_str()) {
            sr.sprite = v.to_string();
        }
        if let Some(c) = data.properties.get("color")
            && let Ok(arr) = serde_json::from_value::<[f32; 4]>(c.clone())
        {
            sr.color = arr;
        }
        if let Some(v) = data.properties.get("flip_x").and_then(|v| v.as_bool()) {
            sr.flip_x = v;
        }
        if let Some(v) = data.properties.get("flip_y").and_then(|v| v.as_bool()) {
            sr.flip_y = v;
        }
        if let Some(v) = data
            .properties
            .get("sorting_order")
            .and_then(|v| v.as_i64())
        {
            sr.sorting_order = v as i32;
        }
        Some(Box::new(sr))
    }
}

/// Load a scene from JSON string.
pub fn LoadSceneJson(
    json: &str,
    world: &mut World,
) -> Result<Vec<GameObjectHandle>, serde_json::Error> {
    let scene: SceneData = serde_json::from_str(json)?;
    let serializer = SceneSerializer::new();
    Ok(serializer.Load(&scene, world))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_serializer_new() {
        let s = SceneSerializer::new();
        // Built-in Material + SpriteRenderer
        assert_eq!(s.formatters.len(), 2);
        assert_eq!(s.deserializers.len(), 2);
    }

    #[test]
    fn test_material_sprite_roundtrip() {
        use crate::components::{Material, SpriteRenderer};

        let mut world = World::new();
        let go = world.CreateGameObject("Prop");
        world.AddComponent(
            go,
            Material {
                base_color: [1.0, 0.5, 0.25, 1.0],
                metallic: 0.3,
                smoothness: 0.7,
                ..Default::default()
            },
        );
        world.AddComponent(
            go,
            SpriteRenderer {
                sprite: "hero.png".into(),
                color: [0.9, 0.1, 0.1, 1.0],
                flip_x: true,
                flip_y: false,
                sorting_order: 3,
            },
        );

        let s = SceneSerializer::new();
        let scene = s.Save(&world, "Props");
        assert_eq!(scene.game_objects[0].components.len(), 2);

        let mut world2 = World::new();
        s.Load(&scene, &mut world2);
        let go2 = world2.Find("Prop").unwrap();
        let m = world2.GetComponent::<Material>(go2).unwrap();
        assert_eq!(m.base_color, [1.0, 0.5, 0.25, 1.0]);
        assert!((m.metallic - 0.3).abs() < 1e-5);
        let sr = world2.GetComponent::<SpriteRenderer>(go2).unwrap();
        assert_eq!(sr.sprite, "hero.png");
        assert!(sr.flip_x);
        assert_eq!(sr.sorting_order, 3);
    }

    #[test]
    fn test_component_data_asset_ref_property() {
        let mut data = ComponentData::new("EnemyBrain");
        let r = AssetRef::from_guid("deadbeefcafe");
        data.insert_asset_ref("data", &r);
        let back = data.get_asset_ref("data").unwrap();
        assert_eq!(back.guid(), "deadbeefcafe");

        let mut null_data = ComponentData::new("X");
        null_data.insert_asset_ref("data", &AssetRef::null());
        assert!(null_data.get_asset_ref("data").unwrap().is_null());
    }

    #[test]
    fn test_save_empty_world() {
        let world = World::new();
        let s = SceneSerializer::new();
        let scene = s.Save(&world, "EmptyScene");

        assert_eq!(scene.name, "EmptyScene");
        assert_eq!(scene.version, 1);
        assert!(scene.game_objects.is_empty());
    }

    #[test]
    fn test_save_single_root() {
        let mut world = World::new();
        world.CreateGameObject("Player");

        let s = SceneSerializer::new();
        let scene = s.Save(&world, "TestScene");

        assert_eq!(scene.game_objects.len(), 1);
        assert_eq!(scene.game_objects[0].name, "Player");
        assert_eq!(scene.game_objects[0].tag, "Untagged");
        assert_eq!(scene.game_objects[0].layer, 0);
        assert!(scene.game_objects[0].active);
    }

    #[test]
    fn test_save_hierarchy() {
        let mut world = World::new();
        let root = world.CreateGameObject("Root");
        let child1 = world.CreateGameObject("Child1");
        let child2 = world.CreateGameObject("Child2");
        world.SetParent(child1, Some(root));
        world.SetParent(child2, Some(root));

        let s = SceneSerializer::new();
        let scene = s.Save(&world, "Hierarchy");

        assert_eq!(scene.game_objects.len(), 1);
        assert_eq!(scene.game_objects[0].children.len(), 2);
        assert_eq!(scene.game_objects[0].children[0].name, "Child1");
        assert_eq!(scene.game_objects[0].children[1].name, "Child2");
    }

    #[test]
    fn test_save_only_serializes_roots() {
        let mut world = World::new();
        let root = world.CreateGameObject("Root");
        let child = world.CreateGameObject("Child");
        world.SetParent(child, Some(root));

        let s = SceneSerializer::new();
        let scene = s.Save(&world, "Test");

        assert_eq!(scene.game_objects.len(), 1);
        assert_eq!(scene.game_objects[0].children.len(), 1);
    }

    #[test]
    fn test_load_empty_scene() {
        let mut world = World::new();
        let scene = SceneData {
            name: "Empty".into(),
            version: 1,
            game_objects: vec![],
        };

        let s = SceneSerializer::new();
        let handles = s.Load(&scene, &mut world);

        assert!(handles.is_empty());
    }

    #[test]
    fn test_load_single_object() {
        let mut world = World::new();
        let scene = SceneData {
            name: "Test".into(),
            version: 1,
            game_objects: vec![GameObjectData {
                name: "LoadedObj".into(),
                tag: "Enemy".into(),
                layer: 5,
                active: false,
                transform: TransformData {
                    local_position: Vec3::new(1.0, 2.0, 3.0),
                    local_rotation: Quat::IDENTITY,
                    local_scale: Vec3::ONE,
                },
                components: vec![],
                children: vec![],
            }],
        };

        let s = SceneSerializer::new();
        let handles = s.Load(&scene, &mut world);

        assert_eq!(handles.len(), 1);
        assert_eq!(world.GetName(handles[0]), "LoadedObj");
        assert_eq!(world.GetTag(handles[0]), "Enemy");
        assert_eq!(world.GetLayer(handles[0]), 5);
        assert!(!world.IsActive(handles[0]));

        // Verify Transform
        let t = world.GetTransform(handles[0]).unwrap();
        assert_eq!(t.LocalPosition(), Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_load_hierarchy() {
        let mut world = World::new();
        let scene = SceneData {
            name: "Hierarchy".into(),
            version: 1,
            game_objects: vec![GameObjectData {
                name: "Root".into(),
                tag: "Untagged".into(),
                layer: 0,
                active: true,
                transform: TransformData {
                    local_position: Vec3::ZERO,
                    local_rotation: Quat::IDENTITY,
                    local_scale: Vec3::ONE,
                },
                components: vec![],
                children: vec![
                    GameObjectData {
                        name: "Child1".into(),
                        tag: "".into(),
                        layer: 0,
                        active: true,
                        transform: TransformData {
                            local_position: Vec3::ZERO,
                            local_rotation: Quat::IDENTITY,
                            local_scale: Vec3::ONE,
                        },
                        components: vec![],
                        children: vec![],
                    },
                    GameObjectData {
                        name: "Child2".into(),
                        tag: "".into(),
                        layer: 0,
                        active: true,
                        transform: TransformData {
                            local_position: Vec3::ZERO,
                            local_rotation: Quat::IDENTITY,
                            local_scale: Vec3::ONE,
                        },
                        components: vec![],
                        children: vec![],
                    },
                ],
            }],
        };

        let s = SceneSerializer::new();
        let handles = s.Load(&scene, &mut world);

        assert_eq!(handles.len(), 1);
        assert_eq!(world.GetName(handles[0]), "Root");
        assert_eq!(world.GetChildren(handles[0]).len(), 2);

        let children = world.GetChildren(handles[0]);
        assert_eq!(world.GetName(children[0]), "Child1");
        assert_eq!(world.GetName(children[1]), "Child2");
    }

    #[test]
    fn test_roundtrip_json() {
        let mut world = World::new();
        let root = world.CreateGameObject("Player");
        let child = world.CreateGameObject("Gun");
        world.SetParent(child, Some(root));

        let json = SaveSceneJson(&world, "GameScene").unwrap();
        assert!(json.contains("Player"));
        assert!(json.contains("Gun"));

        let mut world2 = World::new();
        let handles = LoadSceneJson(&json, &mut world2).unwrap();

        assert_eq!(handles.len(), 1);
        assert_eq!(world2.GetName(handles[0]), "Player");
        assert_eq!(world2.GetChildren(handles[0]).len(), 1);

        let children = world2.GetChildren(handles[0]);
        assert_eq!(world2.GetName(children[0]), "Gun");
    }

    #[test]
    fn test_save_preserves_transform() {
        let mut world = World::new();
        let handle = world.CreateGameObject("Player");
        if let Some(t) = world.GetTransformMut(handle) {
            t.SetLocalPosition(Vec3::new(1.0, 2.0, 3.0));
            t.SetLocalRotation(Quat::from_rotation_y(1.57));
            t.SetLocalScale(Vec3::new(2.0, 2.0, 2.0));
        }

        let s = SceneSerializer::new();
        let scene = s.Save(&world, "Test");

        let data = &scene.game_objects[0];
        assert_eq!(data.transform.local_position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(data.transform.local_scale, Vec3::new(2.0, 2.0, 2.0));
    }

    #[test]
    fn test_default_impl() {
        let s = SceneSerializer::default();
        assert_eq!(s.formatters.len(), 2);
        assert_eq!(s.deserializers.len(), 2);
    }
}
