use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::gameobject::{Component, GameObject, GameObjectHandle};
use crate::transform::Transform;
use crate::world::World;
use engine_math::{Quat, Vec3};

/// Serialized component data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComponentData {
    pub type_name: String,
    pub properties: HashMap<String, serde_json::Value>,
}

/// Serialized GameObject data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameObjectData {
    pub name: String,
    pub tag: String,
    pub layer: u32,
    pub active: bool,
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
///
/// Implementors can provide custom serialization logic for specific component types.
pub trait ComponentFormatter: Send + Sync {
    /// Try to serialize a component. Returns `Some(ComponentData)` if this formatter
    /// handles the given component, or `None` to skip it.
    fn format(&self, component: &dyn Component) -> Option<ComponentData>;

    /// The type name this formatter handles.
    fn type_name(&self) -> &str;
}

/// Trait for deserializing components during scene loading.
///
/// Implementors can provide custom deserialization logic for specific component types.
pub trait ComponentDeserializer: Send + Sync {
    /// Try to deserialize a component from serialized data.
    /// Returns `Some(Box<dyn Component>)` if successful, or `None` on failure.
    fn deserialize(&self, data: &ComponentData) -> Option<Box<dyn Component>>;

    /// The type name this deserializer handles.
    fn type_name(&self) -> &str;
}

/// Formatter for Transform components.
pub struct TransformFormatter;

impl ComponentFormatter for TransformFormatter {
    fn format(&self, component: &dyn Component) -> Option<ComponentData> {
        let transform = component.as_any().downcast_ref::<Transform>()?;
        let mut properties = HashMap::new();
        properties.insert(
            "local_position".into(),
            serde_json::json!({
                "x": transform.local_position.x,
                "y": transform.local_position.y,
                "z": transform.local_position.z,
            }),
        );
        properties.insert(
            "local_rotation".into(),
            serde_json::json!({
                "x": transform.local_rotation.x,
                "y": transform.local_rotation.y,
                "z": transform.local_rotation.z,
                "w": transform.local_rotation.w,
            }),
        );
        properties.insert(
            "local_scale".into(),
            serde_json::json!({
                "x": transform.local_scale.x,
                "y": transform.local_scale.y,
                "z": transform.local_scale.z,
            }),
        );
        Some(ComponentData {
            type_name: "Transform".into(),
            properties,
        })
    }

    fn type_name(&self) -> &str {
        "Transform"
    }
}

/// Deserializer for Transform components.
pub struct TransformDeserializer;

impl ComponentDeserializer for TransformDeserializer {
    fn deserialize(&self, data: &ComponentData) -> Option<Box<dyn Component>> {
        let pos = data.properties.get("local_position")?;
        let rot = data.properties.get("local_rotation")?;
        let scale = data.properties.get("local_scale")?;

        let position = Vec3::new(
            pos.get("x")?.as_f64()? as f32,
            pos.get("y")?.as_f64()? as f32,
            pos.get("z")?.as_f64()? as f32,
        );
        let rotation = Quat::from_xyzw(
            rot.get("x")?.as_f64()? as f32,
            rot.get("y")?.as_f64()? as f32,
            rot.get("z")?.as_f64()? as f32,
            rot.get("w")?.as_f64()? as f32,
        );
        let scale = Vec3::new(
            scale.get("x")?.as_f64()? as f32,
            scale.get("y")?.as_f64()? as f32,
            scale.get("z")?.as_f64()? as f32,
        );

        let mut transform = Transform::from_xyz(position.x, position.y, position.z);
        transform.set_local_rotation(rotation);
        transform.set_local_scale(scale);

        Some(Box::new(transform))
    }

    fn type_name(&self) -> &str {
        "Transform"
    }
}

/// Scene serializer for saving and loading scenes.
pub struct SceneSerializer {
    formatters: Vec<Box<dyn ComponentFormatter>>,
    deserializers: HashMap<String, Box<dyn ComponentDeserializer>>,
}

impl SceneSerializer {
    /// Create a new serializer with default formatters and deserializers (Transform).
    pub fn new() -> Self {
        let mut s = Self {
            formatters: Vec::new(),
            deserializers: HashMap::new(),
        };
        s.add_formatter(Box::new(TransformFormatter));
        s.add_deserializer(Box::new(TransformDeserializer));
        s
    }

    /// Register a custom component formatter.
    pub fn add_formatter(&mut self, formatter: Box<dyn ComponentFormatter>) {
        self.formatters.push(formatter);
    }

    /// Register a custom component deserializer.
    pub fn add_deserializer(&mut self, deserializer: Box<dyn ComponentDeserializer>) {
        self.deserializers
            .insert(deserializer.type_name().to_string(), deserializer);
    }

    /// Serialize a World into SceneData.
    pub fn save(&self, world: &World, name: &str) -> SceneData {
        let roots = world.root_gameobjects(true);
        let game_objects = roots
            .iter()
            .filter_map(|&handle| self.serialize_game_object(world, handle))
            .collect();

        SceneData {
            name: name.to_string(),
            version: 1,
            game_objects,
        }
    }

    /// Serialize a single GameObject and its children.
    fn serialize_game_object(
        &self,
        world: &World,
        handle: GameObjectHandle,
    ) -> Option<GameObjectData> {
        let go = world.get_gameobject(handle)?;

        let components = go
            .components()
            .iter()
            .filter_map(|c| self.format_component(c.as_ref()))
            .collect();

        let children = go
            .children()
            .iter()
            .filter_map(|&child_handle| self.serialize_game_object(world, child_handle))
            .collect();

        Some(GameObjectData {
            name: go.name().to_string(),
            tag: go.tag().to_string(),
            layer: go.layer(),
            active: go.is_active(),
            components,
            children,
        })
    }

    /// Format a component using registered formatters.
    fn format_component(&self, component: &dyn Component) -> Option<ComponentData> {
        for formatter in &self.formatters {
            if let Some(data) = formatter.format(component) {
                return Some(data);
            }
        }
        None
    }

    /// Deserialize SceneData into a World, returning handles of spawned root objects.
    pub fn load(&self, scene: &SceneData, world: &mut World) -> Vec<GameObjectHandle> {
        scene
            .game_objects
            .iter()
            .map(|go_data| self.spawn_game_object(world, go_data))
            .collect()
    }

    /// Spawn a GameObject from serialized data and recursively spawn children.
    fn spawn_game_object(&self, world: &mut World, data: &GameObjectData) -> GameObjectHandle {
        let mut go = GameObject::new(&data.name);
        go.set_tag(&data.tag);
        go.set_layer(data.layer);
        go.set_active(data.active);

        // Deserialize components
        for comp_data in &data.components {
            if let Some(deserializer) = self.deserializers.get(&comp_data.type_name)
                && let Some(component) = deserializer.deserialize(comp_data)
            {
                go.add_component_boxed(component);
            }
        }

        let handle = world.spawn(go);

        // Spawn children and attach them
        for child_data in &data.children {
            let child_handle = self.spawn_game_object(world, child_data);
            world.set_parent(child_handle, Some(handle));
        }

        handle
    }
}

impl Default for SceneSerializer {
    fn default() -> Self {
        Self::new()
    }
}

/// Save a scene to JSON string.
pub fn save_scene_json(world: &World, name: &str) -> Result<String, serde_json::Error> {
    let serializer = SceneSerializer::new();
    let scene = serializer.save(world, name);
    serde_json::to_string_pretty(&scene)
}

/// Load a scene from JSON string.
pub fn load_scene_json(
    json: &str,
    world: &mut World,
) -> Result<Vec<GameObjectHandle>, serde_json::Error> {
    let scene: SceneData = serde_json::from_str(json)?;
    let serializer = SceneSerializer::new();
    Ok(serializer.load(&scene, world))
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_math::Quat;

    #[test]
    fn test_transform_formatter() {
        let formatter = TransformFormatter;
        let mut t = Transform::from_xyz(1.0, 2.0, 3.0);
        t.set_local_rotation(Quat::from_rotation_y(1.57));

        let data = formatter.format(&t).unwrap();
        assert_eq!(data.type_name, "Transform");
        assert_eq!(
            data.properties["local_position"],
            serde_json::json!({"x": 1.0, "y": 2.0, "z": 3.0})
        );
    }

    #[test]
    fn test_transform_formatter_wrong_type() {
        use std::any::Any;

        struct DummyComponent;
        impl Component for DummyComponent {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }

        let formatter = TransformFormatter;
        let c = DummyComponent;
        let data = formatter.format(&c);
        assert!(data.is_none());
    }

    #[test]
    fn test_scene_serializer_new_has_transform() {
        let s = SceneSerializer::new();
        assert_eq!(s.formatters.len(), 1);
        assert_eq!(s.formatters[0].type_name(), "Transform");
        assert_eq!(s.deserializers.len(), 1);
        assert_eq!(s.deserializers["Transform"].type_name(), "Transform");
    }

    #[test]
    fn test_save_empty_world() {
        let world = World::new();
        let s = SceneSerializer::new();
        let scene = s.save(&world, "EmptyScene");

        assert_eq!(scene.name, "EmptyScene");
        assert_eq!(scene.version, 1);
        assert!(scene.game_objects.is_empty());
    }

    #[test]
    fn test_save_single_root() {
        let mut world = World::new();
        world.spawn(GameObject::new("Player"));

        let s = SceneSerializer::new();
        let scene = s.save(&world, "TestScene");

        assert_eq!(scene.game_objects.len(), 1);
        assert_eq!(scene.game_objects[0].name, "Player");
        assert_eq!(scene.game_objects[0].tag, "Untagged");
        assert_eq!(scene.game_objects[0].layer, 0);
        assert!(scene.game_objects[0].active);
    }

    #[test]
    fn test_save_hierarchy() {
        let mut world = World::new();
        let root = world.spawn(GameObject::new("Root"));
        let child1 = world.spawn(GameObject::new("Child1"));
        let child2 = world.spawn(GameObject::new("Child2"));
        world.set_parent(child1, Some(root));
        world.set_parent(child2, Some(root));

        let s = SceneSerializer::new();
        let scene = s.save(&world, "Hierarchy");

        assert_eq!(scene.game_objects.len(), 1);
        assert_eq!(scene.game_objects[0].children.len(), 2);
        assert_eq!(scene.game_objects[0].children[0].name, "Child1");
        assert_eq!(scene.game_objects[0].children[1].name, "Child2");
    }

    #[test]
    fn test_save_only_serializes_roots() {
        let mut world = World::new();
        let root = world.spawn(GameObject::new("Root"));
        let child = world.spawn(GameObject::new("Child"));
        world.set_parent(child, Some(root));

        let s = SceneSerializer::new();
        let scene = s.save(&world, "Test");

        // Only root is at top level
        assert_eq!(scene.game_objects.len(), 1);
        // Child is nested
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
        let handles = s.load(&scene, &mut world);

        assert!(handles.is_empty());
        assert_eq!(world.count(), 0);
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
                components: vec![],
                children: vec![],
            }],
        };

        let s = SceneSerializer::new();
        let handles = s.load(&scene, &mut world);

        assert_eq!(handles.len(), 1);
        let go = world.get_gameobject(handles[0]).unwrap();
        assert_eq!(go.name(), "LoadedObj");
        assert_eq!(go.tag(), "Enemy");
        assert_eq!(go.layer(), 5);
        assert!(!go.is_active());
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
                components: vec![],
                children: vec![
                    GameObjectData {
                        name: "Child1".into(),
                        tag: "".into(),
                        layer: 0,
                        active: true,
                        components: vec![],
                        children: vec![],
                    },
                    GameObjectData {
                        name: "Child2".into(),
                        tag: "".into(),
                        layer: 0,
                        active: true,
                        components: vec![],
                        children: vec![],
                    },
                ],
            }],
        };

        let s = SceneSerializer::new();
        let handles = s.load(&scene, &mut world);

        assert_eq!(handles.len(), 1);
        let root = world.get_gameobject(handles[0]).unwrap();
        assert_eq!(root.name(), "Root");
        assert_eq!(root.children().len(), 2);

        let c1 = world.get_gameobject(root.children()[0]).unwrap();
        assert_eq!(c1.name(), "Child1");
        let c2 = world.get_gameobject(root.children()[1]).unwrap();
        assert_eq!(c2.name(), "Child2");
    }

    #[test]
    fn test_roundtrip_json() {
        let mut world = World::new();
        let root = world.spawn(GameObject::new("Player"));
        let child = world.spawn(GameObject::new("Gun"));
        world.set_parent(child, Some(root));

        let json = save_scene_json(&world, "GameScene").unwrap();
        assert!(json.contains("Player"));
        assert!(json.contains("Gun"));

        let mut world2 = World::new();
        let handles = load_scene_json(&json, &mut world2).unwrap();

        assert_eq!(handles.len(), 1);
        let loaded_root = world2.get_gameobject(handles[0]).unwrap();
        assert_eq!(loaded_root.name(), "Player");
        assert_eq!(loaded_root.children().len(), 1);
        let loaded_child = world2.get_gameobject(loaded_root.children()[0]).unwrap();
        assert_eq!(loaded_child.name(), "Gun");
    }

    #[test]
    fn test_scene_data_serializable() {
        let scene = SceneData {
            name: "Test".into(),
            version: 1,
            game_objects: vec![GameObjectData {
                name: "Obj".into(),
                tag: "Tag".into(),
                layer: 3,
                active: true,
                components: vec![ComponentData {
                    type_name: "Transform".into(),
                    properties: HashMap::new(),
                }],
                children: vec![],
            }],
        };

        let json = serde_json::to_string(&scene).unwrap();
        let deserialized: SceneData = serde_json::from_str(&json).unwrap();
        assert_eq!(scene, deserialized);
    }

    #[test]
    fn test_component_data_equality() {
        let mut props1 = HashMap::new();
        props1.insert("x".into(), serde_json::json!(1.0));
        let mut props2 = HashMap::new();
        props2.insert("x".into(), serde_json::json!(1.0));

        let d1 = ComponentData {
            type_name: "Transform".into(),
            properties: props1,
        };
        let d2 = ComponentData {
            type_name: "Transform".into(),
            properties: props2,
        };
        assert_eq!(d1, d2);
    }

    #[test]
    fn test_format_component_returns_none_for_unknown() {
        use std::any::Any;

        struct UnknownComponent;
        impl Component for UnknownComponent {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }

        let s = SceneSerializer::new();
        let c = UnknownComponent;
        let result = s.format_component(&c);
        assert!(result.is_none());
    }

    #[test]
    fn test_custom_formatter() {
        use std::any::Any;

        struct DummyComp;
        impl Component for DummyComp {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }

        struct TestFormatter;

        impl ComponentFormatter for TestFormatter {
            fn format(&self, _component: &dyn Component) -> Option<ComponentData> {
                Some(ComponentData {
                    type_name: "TestComponent".into(),
                    properties: HashMap::from([("value".into(), serde_json::json!(42))]),
                })
            }

            fn type_name(&self) -> &str {
                "TestComponent"
            }
        }

        let mut s = SceneSerializer::new();
        s.add_formatter(Box::new(TestFormatter));

        let c = DummyComp;
        let result = s.format_component(&c).unwrap();
        assert_eq!(result.type_name, "TestComponent");
        assert_eq!(result.properties["value"], serde_json::json!(42));
    }

    #[test]
    fn test_save_preserves_gameobject_properties() {
        let mut world = World::new();
        let mut go = GameObject::new("Hero");
        go.set_tag("Player");
        go.set_layer(10);
        go.set_active(false);
        world.spawn(go);

        let s = SceneSerializer::new();
        let scene = s.save(&world, "Test");

        let data = &scene.game_objects[0];
        assert_eq!(data.name, "Hero");
        assert_eq!(data.tag, "Player");
        assert_eq!(data.layer, 10);
        assert!(!data.active);
    }

    #[test]
    fn test_default_impl() {
        let s = SceneSerializer::default();
        assert_eq!(s.formatters.len(), 1);
        assert_eq!(s.deserializers.len(), 1);
    }

    #[test]
    fn test_roundtrip_with_transform() {
        let mut world = World::new();
        let mut go = GameObject::new("Player");
        let mut t = Transform::from_xyz(1.0, 2.0, 3.0);
        t.set_local_rotation(Quat::from_rotation_y(1.57));
        go.add_component(t);
        world.spawn(go);

        let s = SceneSerializer::new();
        let scene = s.save(&world, "TestScene");

        // Verify component was serialized
        assert_eq!(scene.game_objects[0].components.len(), 1);
        assert_eq!(scene.game_objects[0].components[0].type_name, "Transform");

        // Load into new world
        let mut world2 = World::new();
        let handles = s.load(&scene, &mut world2);

        assert_eq!(handles.len(), 1);
        let loaded_go = world2.get_gameobject(handles[0]).unwrap();
        assert_eq!(loaded_go.name(), "Player");

        // Verify Transform was deserialized
        let loaded_transform = loaded_go.get_component::<Transform>().unwrap();
        assert_eq!(loaded_transform.local_position.x, 1.0);
        assert_eq!(loaded_transform.local_position.y, 2.0);
        assert_eq!(loaded_transform.local_position.z, 3.0);

        // Verify rotation survived roundtrip
        let expected_rot = Quat::from_rotation_y(1.57);
        assert!((loaded_transform.local_rotation.x - expected_rot.x).abs() < 1e-5);
        assert!((loaded_transform.local_rotation.y - expected_rot.y).abs() < 1e-5);
        assert!((loaded_transform.local_rotation.z - expected_rot.z).abs() < 1e-5);
        assert!((loaded_transform.local_rotation.w - expected_rot.w).abs() < 1e-5);
    }

    #[test]
    fn test_roundtrip_json_with_transform() {
        let mut world = World::new();
        let mut go = GameObject::new("Hero");
        let mut t = Transform::from_xyz(5.0, 10.0, 15.0);
        t.set_local_scale(Vec3::new(2.0, 2.0, 2.0));
        go.add_component(t);
        world.spawn(go);

        let json = save_scene_json(&world, "JsonScene").unwrap();

        let mut world2 = World::new();
        let handles = load_scene_json(&json, &mut world2).unwrap();

        let loaded_go = world2.get_gameobject(handles[0]).unwrap();
        let loaded_t = loaded_go.get_component::<Transform>().unwrap();
        assert_eq!(loaded_t.local_position.x, 5.0);
        assert_eq!(loaded_t.local_position.y, 10.0);
        assert_eq!(loaded_t.local_position.z, 15.0);
        assert_eq!(loaded_t.local_scale.x, 2.0);
        assert_eq!(loaded_t.local_scale.y, 2.0);
        assert_eq!(loaded_t.local_scale.z, 2.0);
    }

    #[test]
    fn test_custom_deserializer() {
        use std::any::Any;

        struct DummyComp {
            value: i32,
        }

        impl Component for DummyComp {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }

        struct TestDeserializer;

        impl ComponentDeserializer for TestDeserializer {
            fn deserialize(&self, data: &ComponentData) -> Option<Box<dyn Component>> {
                let v = data.properties.get("value")?.as_i64()?;
                Some(Box::new(DummyComp { value: v as i32 }))
            }

            fn type_name(&self) -> &str {
                "DummyComp"
            }
        }

        let mut s = SceneSerializer::new();
        s.add_deserializer(Box::new(TestDeserializer));

        let data = ComponentData {
            type_name: "DummyComp".into(),
            properties: HashMap::from([("value".into(), serde_json::json!(42))]),
        };

        let comp = s.deserializers["DummyComp"].deserialize(&data).unwrap();
        let dummy = comp.as_any().downcast_ref::<DummyComp>().unwrap();
        assert_eq!(dummy.value, 42);
    }
}
