use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::gameobject::{GameObject, GameObjectHandle};
use crate::world::World;

/// Unique identifier for prefab instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PrefabId(pub u64);

/// A reusable GameObject template (like Unity's Prefab).
///
/// Prefabs define a hierarchy of GameObjects with components that can be
/// instantiated multiple times. Instances can have overrides that modify
/// properties without changing the original prefab definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prefab {
    /// Unique identifier for this prefab.
    id: PrefabId,
    /// Name of the prefab.
    name: String,
    /// The root GameObject definition.
    root: PrefabNode,
}

/// A node in the prefab hierarchy, representing a GameObject template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabNode {
    /// Name of the GameObject.
    name: String,
    /// Tag of the GameObject.
    tag: String,
    /// Layer of the GameObject.
    layer: u32,
    /// Whether the GameObject is active.
    active: bool,
    /// Type names of components attached to this GameObject.
    components: Vec<String>,
    /// Child nodes.
    children: Vec<PrefabNode>,
}

/// An instance of a prefab in the scene.
#[derive(Debug)]
pub struct PrefabInstance {
    /// Reference to the source prefab.
    prefab_id: PrefabId,
    /// The instantiated GameObject handle.
    game_object: GameObjectHandle,
    /// Property overrides: path -> serialized value.
    overrides: HashMap<String, PrefabValue>,
}

/// A serialized property value for prefab overrides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PrefabValue {
    Bool(bool),
    Int(i32),
    Float(f32),
    String(String),
}

impl Prefab {
    /// Create a new prefab from a root GameObject and its World.
    pub fn create(name: &str, root: &GameObject, world: &World) -> Self {
        let id = PrefabId(rand_id());
        Self {
            id,
            name: name.to_string(),
            root: PrefabNode::from_game_object(root, world),
        }
    }

    /// Get the prefab ID.
    pub fn id(&self) -> PrefabId {
        self.id
    }

    /// Get the prefab name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the root node.
    pub fn root(&self) -> &PrefabNode {
        &self.root
    }

    /// Instantiate this prefab, creating a new GameObject hierarchy in the World.
    pub fn instantiate(&self, world: &mut World) -> PrefabInstance {
        let handle = self.root.to_game_object(world);
        PrefabInstance {
            prefab_id: self.id,
            game_object: handle,
            overrides: HashMap::new(),
        }
    }
}

impl PrefabNode {
    /// Create a PrefabNode from a GameObject (snapshot), resolving children via the World.
    fn from_game_object(go: &GameObject, world: &World) -> Self {
        let components = go
            .components()
            .iter()
            .map(|c| c.component_name().to_string())
            .collect();

        let children = go
            .children()
            .iter()
            .filter_map(|&handle| {
                world
                    .get_gameobject(handle)
                    .map(|child_go| PrefabNode::from_game_object(child_go, world))
            })
            .collect();

        Self {
            name: go.name().to_string(),
            tag: go.tag().to_string(),
            layer: go.layer(),
            active: go.is_active(),
            components,
            children,
        }
    }

    /// Convert this node back into a GameObject hierarchy, spawning recursively into the World.
    /// Returns the handle of the root GameObject.
    fn to_game_object(&self, world: &mut World) -> GameObjectHandle {
        let mut go = GameObject::new(&self.name);
        go.set_tag(&self.tag);
        go.set_layer(self.layer);
        go.set_active(self.active);

        // Spawn root into world to get a handle
        let root_handle = world.spawn(go);

        // Recursively spawn children and set up parent-child relationships
        for child_node in &self.children {
            let child_handle = child_node.to_game_object(world);
            world.set_parent(child_handle, Some(root_handle));
        }

        root_handle
    }

    /// Get the node name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the node tag.
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// Get the node layer.
    pub fn layer(&self) -> u32 {
        self.layer
    }

    /// Check if the node is active.
    pub fn active(&self) -> bool {
        self.active
    }

    /// Get the child nodes.
    pub fn children(&self) -> &[PrefabNode] {
        &self.children
    }

    /// Get the component type names stored in this node.
    pub fn components(&self) -> &[String] {
        &self.components
    }
}

impl PrefabInstance {
    /// Get the source prefab ID.
    pub fn prefab_id(&self) -> PrefabId {
        self.prefab_id
    }

    /// Get the handle of the instantiated GameObject.
    pub fn game_object_handle(&self) -> GameObjectHandle {
        self.game_object
    }

    /// Get a reference to the instantiated GameObject from the World.
    pub fn game_object<'a>(&self, world: &'a World) -> Option<&'a GameObject> {
        world.get_gameobject(self.game_object)
    }

    /// Get a mutable reference to the instantiated GameObject from the World.
    pub fn game_object_mut<'a>(&self, world: &'a mut World) -> Option<&'a mut GameObject> {
        world.get_gameobject_mut(self.game_object)
    }

    /// Apply an override to this instance.
    ///
    /// The `path` identifies the property (e.g., "Transform.Position.X").
    /// The `value` is the new value for that property.
    pub fn apply_override(&mut self, path: &str, value: PrefabValue) {
        self.overrides.insert(path.to_string(), value);
    }

    /// Revert a specific override, restoring the prefab default.
    ///
    /// Returns `true` if the override existed and was removed.
    pub fn revert(&mut self, path: &str) -> bool {
        self.overrides.remove(path).is_some()
    }

    /// Revert all overrides, restoring the prefab defaults.
    pub fn revert_all(&mut self) {
        self.overrides.clear();
    }

    /// Check if this instance has any overrides.
    pub fn has_overrides(&self) -> bool {
        !self.overrides.is_empty()
    }

    /// Check if a specific property has been overridden.
    pub fn has_override(&self, path: &str) -> bool {
        self.overrides.contains_key(path)
    }

    /// Get the value of an override, if it exists.
    pub fn get_override(&self, path: &str) -> Option<&PrefabValue> {
        self.overrides.get(path)
    }

    /// Get the number of overrides.
    pub fn override_count(&self) -> usize {
        self.overrides.len()
    }

    /// Get all override paths.
    pub fn override_paths(&self) -> Vec<&str> {
        self.overrides.keys().map(|s| s.as_str()).collect()
    }
}

/// Generate a simple unique ID (placeholder for a proper UUID/snowflake).
fn rand_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// A registry for managing prefabs.
#[derive(Debug, Default)]
pub struct PrefabRegistry {
    prefabs: HashMap<PrefabId, Prefab>,
}

impl PrefabRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a prefab and return its ID.
    pub fn register(&mut self, prefab: Prefab) -> PrefabId {
        let id = prefab.id();
        self.prefabs.insert(id, prefab);
        id
    }

    /// Get a prefab by ID.
    pub fn get(&self, id: PrefabId) -> Option<&Prefab> {
        self.prefabs.get(&id)
    }

    /// Get a mutable reference to a prefab by ID.
    pub fn get_mut(&mut self, id: PrefabId) -> Option<&mut Prefab> {
        self.prefabs.get_mut(&id)
    }

    /// Remove a prefab by ID.
    pub fn remove(&mut self, id: PrefabId) -> Option<Prefab> {
        self.prefabs.remove(&id)
    }

    /// Check if a prefab exists.
    pub fn contains(&self, id: PrefabId) -> bool {
        self.prefabs.contains_key(&id)
    }

    /// Get the number of registered prefabs.
    pub fn count(&self) -> usize {
        self.prefabs.len()
    }

    /// Find a prefab by name.
    pub fn find_by_name(&self, name: &str) -> Option<&Prefab> {
        self.prefabs.values().find(|p| p.name() == name)
    }

    /// Instantiate a prefab by ID.
    pub fn instantiate(&self, id: PrefabId, world: &mut World) -> Option<PrefabInstance> {
        self.prefabs.get(&id).map(|p| p.instantiate(world))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameobject::Component;
    use std::any::Any;

    #[derive(Debug)]
    struct TestComponent {
        value: i32,
    }

    impl Component for TestComponent {
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[derive(Debug)]
    struct HealthComponent {
        hp: f32,
    }

    impl Component for HealthComponent {
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    fn make_test_game_object() -> (GameObject, World) {
        let world = World::new();
        let go = GameObject::new("TestObject");
        (go, world)
    }

    #[test]
    fn test_prefab_create() {
        let (go, world) = make_test_game_object();
        let prefab = Prefab::create("TestPrefab", &go, &world);

        assert_eq!(prefab.name(), "TestPrefab");
        assert!(prefab.id().0 > 0);
    }

    #[test]
    fn test_prefab_instantiate() {
        let (go, mut world) = make_test_game_object();
        let prefab = Prefab::create("TestPrefab", &go, &world);

        let instance = prefab.instantiate(&mut world);
        assert_eq!(instance.prefab_id(), prefab.id());
        let go_ref = instance.game_object(&world).unwrap();
        assert_eq!(go_ref.name(), "TestObject");
        assert!(!instance.has_overrides());
    }

    #[test]
    fn test_apply_and_check_overrides() {
        let (go, mut world) = make_test_game_object();
        let prefab = Prefab::create("TestPrefab", &go, &world);
        let mut instance = prefab.instantiate(&mut world);

        assert!(!instance.has_overrides());

        instance.apply_override("Transform.Position.X", PrefabValue::Float(1.0));
        assert!(instance.has_overrides());
        assert!(instance.has_override("Transform.Position.X"));
        assert_eq!(
            instance.get_override("Transform.Position.X"),
            Some(&PrefabValue::Float(1.0))
        );
        assert_eq!(instance.override_count(), 1);
    }

    #[test]
    fn test_revert_single_override() {
        let (go, mut world) = make_test_game_object();
        let prefab = Prefab::create("TestPrefab", &go, &world);
        let mut instance = prefab.instantiate(&mut world);

        instance.apply_override("Position.X", PrefabValue::Float(5.0));
        assert!(instance.has_overrides());

        let reverted = instance.revert("Position.X");
        assert!(reverted);
        assert!(!instance.has_overrides());
    }

    #[test]
    fn test_revert_nonexistent_override() {
        let (go, mut world) = make_test_game_object();
        let prefab = Prefab::create("TestPrefab", &go, &world);
        let mut instance = prefab.instantiate(&mut world);

        let reverted = instance.revert("NonExistent");
        assert!(!reverted);
    }

    #[test]
    fn test_revert_all() {
        let (go, mut world) = make_test_game_object();
        let prefab = Prefab::create("TestPrefab", &go, &world);
        let mut instance = prefab.instantiate(&mut world);

        instance.apply_override("A", PrefabValue::Bool(true));
        instance.apply_override("B", PrefabValue::Int(42));
        assert_eq!(instance.override_count(), 2);

        instance.revert_all();
        assert!(!instance.has_overrides());
        assert_eq!(instance.override_count(), 0);
    }

    #[test]
    fn test_override_paths() {
        let (go, mut world) = make_test_game_object();
        let prefab = Prefab::create("TestPrefab", &go, &world);
        let mut instance = prefab.instantiate(&mut world);

        instance.apply_override("X", PrefabValue::Float(1.0));
        instance.apply_override("Y", PrefabValue::Float(2.0));

        let mut paths = instance.override_paths();
        paths.sort();
        assert_eq!(paths, vec!["X", "Y"]);
    }

    #[test]
    fn test_registry() {
        let mut registry = PrefabRegistry::new();
        assert_eq!(registry.count(), 0);

        let (go, world) = make_test_game_object();
        let prefab = Prefab::create("TestPrefab", &go, &world);
        let id = registry.register(prefab);

        assert_eq!(registry.count(), 1);
        assert!(registry.contains(id));
        assert!(registry.get(id).is_some());
        assert_eq!(registry.get(id).unwrap().name(), "TestPrefab");
    }

    #[test]
    fn test_registry_find_by_name() {
        let mut registry = PrefabRegistry::new();
        let world = World::new();

        let go1 = GameObject::new("Obj1");
        let go2 = GameObject::new("Obj2");
        registry.register(Prefab::create("PrefabA", &go1, &world));
        registry.register(Prefab::create("PrefabB", &go2, &world));

        assert!(registry.find_by_name("PrefabA").is_some());
        assert!(registry.find_by_name("PrefabB").is_some());
        assert!(registry.find_by_name("NotExist").is_none());
    }

    #[test]
    fn test_registry_instantiate() {
        let mut registry = PrefabRegistry::new();
        let (go, mut world) = make_test_game_object();
        let prefab = Prefab::create("TestPrefab", &go, &world);
        let id = registry.register(prefab);

        let instance = registry.instantiate(id, &mut world).unwrap();
        assert_eq!(instance.prefab_id(), id);
    }

    #[test]
    fn test_registry_remove() {
        let mut registry = PrefabRegistry::new();
        let (go, world) = make_test_game_object();
        let prefab = Prefab::create("TestPrefab", &go, &world);
        let id = registry.register(prefab);

        let removed = registry.remove(id);
        assert!(removed.is_some());
        assert!(!registry.contains(id));
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_prefab_node_from_game_object() {
        let world = World::new();
        let mut go = GameObject::new("Root");
        go.set_tag("Player");
        go.set_layer(5);
        go.set_active(false);

        let node = PrefabNode::from_game_object(&go, &world);
        assert_eq!(node.name(), "Root");
        assert_eq!(node.tag(), "Player");
        assert_eq!(node.layer(), 5);
        assert!(!node.active());
        assert!(node.components().is_empty());
    }

    #[test]
    fn test_prefab_node_to_game_object() {
        let mut world = World::new();
        let node = PrefabNode {
            name: "FromNode".to_string(),
            tag: "Enemy".to_string(),
            layer: 3,
            active: true,
            components: Vec::new(),
            children: Vec::new(),
        };

        let handle = node.to_game_object(&mut world);
        let go = world.get_gameobject(handle).unwrap();
        assert_eq!(go.name(), "FromNode");
        assert_eq!(go.tag(), "Enemy");
        assert_eq!(go.layer(), 3);
        assert!(go.is_active());
        assert!(go.children().is_empty());
    }

    #[test]
    fn test_multiple_override_types() {
        let (go, mut world) = make_test_game_object();
        let prefab = Prefab::create("TestPrefab", &go, &world);
        let mut instance = prefab.instantiate(&mut world);

        instance.apply_override("BoolProp", PrefabValue::Bool(true));
        instance.apply_override("IntProp", PrefabValue::Int(42));
        instance.apply_override("FloatProp", PrefabValue::Float(3.14));
        instance.apply_override("StringProp", PrefabValue::String("hello".to_string()));

        assert_eq!(instance.override_count(), 4);
        assert!(instance.has_override("BoolProp"));
        assert!(instance.has_override("IntProp"));
        assert!(instance.has_override("FloatProp"));
        assert!(instance.has_override("StringProp"));
    }

    #[test]
    fn test_override_overwrite() {
        let (go, mut world) = make_test_game_object();
        let prefab = Prefab::create("TestPrefab", &go, &world);
        let mut instance = prefab.instantiate(&mut world);

        instance.apply_override("X", PrefabValue::Float(1.0));
        instance.apply_override("X", PrefabValue::Float(2.0));

        assert_eq!(instance.override_count(), 1);
        assert_eq!(instance.get_override("X"), Some(&PrefabValue::Float(2.0)));
    }

    #[test]
    fn test_child_hierarchy_preserved() {
        let mut world = World::new();

        // Build a root with two children in the world
        let root_handle = world.spawn(GameObject::new("Root"));
        let child1_handle = world.spawn(GameObject::new("Child1"));
        let child2_handle = world.spawn(GameObject::new("Child2"));

        world.set_parent(child1_handle, Some(root_handle));
        world.set_parent(child2_handle, Some(root_handle));

        // Add a grandchild
        let grandchild_handle = world.spawn(GameObject::new("Grandchild"));
        world.set_parent(grandchild_handle, Some(child1_handle));

        let root_go = world.get_gameobject(root_handle).unwrap();

        // Create prefab from root
        let prefab = Prefab::create("HierarchyPrefab", root_go, &world);

        // Verify the prefab captured the hierarchy
        let root_node = prefab.root();
        assert_eq!(root_node.name(), "Root");
        assert_eq!(root_node.children().len(), 2);
        assert_eq!(root_node.children()[0].name(), "Child1");
        assert_eq!(root_node.children()[1].name(), "Child2");

        // Verify grandchild is captured too
        assert_eq!(root_node.children()[0].children().len(), 1);
        assert_eq!(root_node.children()[0].children()[0].name(), "Grandchild");

        // Instantiate the prefab
        let mut new_world = World::new();
        let instance = prefab.instantiate(&mut new_world);

        // Verify the hierarchy is reconstructed
        let root = new_world
            .get_gameobject(instance.game_object_handle())
            .unwrap();
        assert_eq!(root.name(), "Root");
        assert_eq!(root.children().len(), 2);

        let c1 = new_world.get_gameobject(root.children()[0]).unwrap();
        assert_eq!(c1.name(), "Child1");
        assert_eq!(c1.children().len(), 1);

        let gc = new_world.get_gameobject(c1.children()[0]).unwrap();
        assert_eq!(gc.name(), "Grandchild");
    }

    #[test]
    fn test_component_type_names_stored() {
        let world = World::new();
        let mut go = GameObject::new("WithComponents");
        go.add_component(TestComponent { value: 42 });
        go.add_component(HealthComponent { hp: 100.0 });

        let node = PrefabNode::from_game_object(&go, &world);
        assert_eq!(node.components().len(), 2);
        assert!(
            node.components()
                .iter()
                .any(|c| c.contains("TestComponent"))
        );
        assert!(
            node.components()
                .iter()
                .any(|c| c.contains("HealthComponent"))
        );
    }

    #[test]
    fn test_empty_children_on_leaf() {
        let world = World::new();
        let go = GameObject::new("Leaf");

        let node = PrefabNode::from_game_object(&go, &world);
        assert!(node.children().is_empty());
    }
}
