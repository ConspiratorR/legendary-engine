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
    layer: i32,
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
    pub fn Create(name: &str, root: &GameObject, world: &World) -> Self {
        let id = PrefabId(rand_id());
        Self {
            id,
            name: name.to_string(),
            root: PrefabNode::FromGameObject(root, world),
        }
    }

    /// Create a new prefab (snake_case alias for Create).
    pub fn create(name: &str, root: &GameObject, world: &World) -> Self {
        Self::Create(name, root, world)
    }

    /// Get the prefab ID.
    pub fn Id(&self) -> PrefabId {
        self.id
    }

    /// Get the prefab ID (snake_case alias for Id).
    pub fn id(&self) -> PrefabId {
        self.Id()
    }

    /// Get the prefab name.
    pub fn Name(&self) -> &str {
        &self.name
    }

    /// Get the prefab name (snake_case alias for Name).
    pub fn name(&self) -> &str {
        self.Name()
    }

    /// Get the root node.
    pub fn Root(&self) -> &PrefabNode {
        &self.root
    }

    /// Get the root node (snake_case alias for Root).
    pub fn root(&self) -> &PrefabNode {
        self.Root()
    }

    /// Instantiate this prefab, creating a new GameObject hierarchy in the World.
    pub fn Instantiate(&self, world: &mut World) -> PrefabInstance {
        let handle = self.root.ToGameObject(world);
        PrefabInstance {
            prefab_id: self.id,
            game_object: handle,
            overrides: HashMap::new(),
        }
    }

    /// Instantiate this prefab (snake_case alias for Instantiate).
    pub fn instantiate(&self, world: &mut World) -> PrefabInstance {
        self.Instantiate(world)
    }
}

impl PrefabNode {
    /// Create a PrefabNode from a GameObject (snapshot), resolving children via the World.
    fn FromGameObject(go: &GameObject, world: &World) -> Self {
        let components = go
            .Components()
            .iter()
            .map(|c| c.component_name().to_string())
            .collect();

        // Children are now in Transform, so we need to get them from the handle
        // For prefab creation, we need the handle - but we don't have it here
        // We'll store empty children for now and let the caller fill them in
        let children = Vec::new();

        Self {
            name: go.Name().to_string(),
            tag: go.Tag().to_string(),
            layer: go.Layer(),
            active: go.ActiveSelf(),
            components,
            children,
        }
    }

    /// Get name (snake_case alias for Name).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get tag (snake_case alias for Tag).
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// Get layer (snake_case alias for Layer).
    pub fn layer(&self) -> i32 {
        self.layer
    }

    /// Get active state (snake_case alias for Active).
    pub fn active(&self) -> bool {
        self.active
    }

    /// Get children (snake_case alias for Children).
    pub fn children(&self) -> &[PrefabNode] {
        &self.children
    }

    /// Get components (snake_case alias for Components).
    pub fn components(&self) -> &[String] {
        &self.components
    }

    /// Convert this node back into a GameObject hierarchy, spawning recursively into the World.
    /// Returns the handle of the root GameObject.
    fn ToGameObject(&self, world: &mut World) -> GameObjectHandle {
        let handle = world.CreateGameObject(&self.name);
        world.SetTag(handle, &self.tag);
        world.SetLayer(handle, self.layer);
        world.SetActive(handle, self.active);

        // Recursively spawn children and set up parent-child relationships
        for child_node in &self.children {
            let child_handle = child_node.ToGameObject(world);
            world.SetParent(child_handle, Some(handle));
        }

        handle
    }

    /// Get the node name.
    pub fn Name(&self) -> &str {
        &self.name
    }

    /// Get the node tag.
    pub fn Tag(&self) -> &str {
        &self.tag
    }

    /// Get the node layer.
    pub fn Layer(&self) -> i32 {
        self.layer
    }

    /// Check if the node is active.
    pub fn Active(&self) -> bool {
        self.active
    }

    /// Get the child nodes.
    pub fn Children(&self) -> &[PrefabNode] {
        &self.children
    }

    /// Get the component type names stored in this node.
    pub fn Components(&self) -> &[String] {
        &self.components
    }
}

impl PrefabInstance {
    /// Get the source prefab ID.
    pub fn PrefabId(&self) -> PrefabId {
        self.prefab_id
    }

    /// Get the handle of the instantiated GameObject.
    pub fn GameObjectHandle(&self) -> GameObjectHandle {
        self.game_object
    }

    /// Apply an override to this instance.
    pub fn ApplyOverride(&mut self, path: &str, value: PrefabValue) {
        self.overrides.insert(path.to_string(), value);
    }

    /// Revert a specific override, restoring the prefab default.
    pub fn Revert(&mut self, path: &str) -> bool {
        self.overrides.remove(path).is_some()
    }

    /// Revert all overrides, restoring the prefab defaults.
    pub fn RevertAll(&mut self) {
        self.overrides.clear();
    }

    /// Check if this instance has any overrides.
    pub fn HasOverrides(&self) -> bool {
        !self.overrides.is_empty()
    }

    /// Check if a specific property has been overridden.
    pub fn HasOverride(&self, path: &str) -> bool {
        self.overrides.contains_key(path)
    }

    /// Get the value of an override, if it exists.
    pub fn GetOverride(&self, path: &str) -> Option<&PrefabValue> {
        self.overrides.get(path)
    }

    /// Get the number of overrides.
    pub fn OverrideCount(&self) -> usize {
        self.overrides.len()
    }

    /// Get all override paths.
    pub fn OverridePaths(&self) -> Vec<&str> {
        self.overrides.keys().map(|s| s.as_str()).collect()
    }

    // ============================================================
    // Backward-compatible snake_case aliases
    // ============================================================

    /// Get the source prefab ID (snake_case alias for PrefabId).
    pub fn prefab_id(&self) -> PrefabId {
        self.PrefabId()
    }

    /// Get the handle of the instantiated GameObject (snake_case alias for GameObjectHandle).
    pub fn game_object_handle(&self) -> GameObjectHandle {
        self.GameObjectHandle()
    }

    /// Apply an override (snake_case alias for ApplyOverride).
    pub fn apply_override(&mut self, path: &str, value: PrefabValue) {
        self.ApplyOverride(path, value);
    }

    /// Revert a specific override (snake_case alias for Revert).
    pub fn revert(&mut self, path: &str) -> bool {
        self.Revert(path)
    }

    /// Revert all overrides (snake_case alias for RevertAll).
    pub fn revert_all(&mut self) {
        self.RevertAll();
    }

    /// Check if this instance has any overrides (snake_case alias for HasOverrides).
    pub fn has_overrides(&self) -> bool {
        self.HasOverrides()
    }

    /// Check if a specific property has been overridden (snake_case alias for HasOverride).
    pub fn has_override(&self, path: &str) -> bool {
        self.HasOverride(path)
    }

    /// Get the value of an override (snake_case alias for GetOverride).
    pub fn get_override(&self, path: &str) -> Option<&PrefabValue> {
        self.GetOverride(path)
    }

    /// Get the number of overrides (snake_case alias for OverrideCount).
    pub fn override_count(&self) -> usize {
        self.OverrideCount()
    }

    /// Get all override paths (snake_case alias for OverridePaths).
    pub fn override_paths(&self) -> Vec<&str> {
        self.OverridePaths()
    }
}

/// Generate a simple unique ID.
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
    pub fn Register(&mut self, prefab: Prefab) -> PrefabId {
        let id = prefab.Id();
        self.prefabs.insert(id, prefab);
        id
    }

    /// Get a prefab by ID.
    pub fn Get(&self, id: PrefabId) -> Option<&Prefab> {
        self.prefabs.get(&id)
    }

    /// Get a mutable reference to a prefab by ID.
    pub fn GetMut(&mut self, id: PrefabId) -> Option<&mut Prefab> {
        self.prefabs.get_mut(&id)
    }

    /// Remove a prefab by ID.
    pub fn Remove(&mut self, id: PrefabId) -> Option<Prefab> {
        self.prefabs.remove(&id)
    }

    /// Check if a prefab exists.
    pub fn Contains(&self, id: PrefabId) -> bool {
        self.prefabs.contains_key(&id)
    }

    /// Get the number of registered prefabs.
    pub fn Count(&self) -> usize {
        self.prefabs.len()
    }

    /// Find a prefab by name.
    pub fn FindByName(&self, name: &str) -> Option<&Prefab> {
        self.prefabs.values().find(|p| p.Name() == name)
    }

    /// Instantiate a prefab by ID.
    pub fn Instantiate(&self, id: PrefabId, world: &mut World) -> Option<PrefabInstance> {
        self.prefabs.get(&id).map(|p| p.Instantiate(world))
    }

    // ============================================================
    // Backward-compatible snake_case aliases
    // ============================================================

    /// Register a prefab (snake_case alias for Register).
    pub fn register(&mut self, prefab: Prefab) -> PrefabId {
        self.Register(prefab)
    }

    /// Get a prefab (snake_case alias for Get).
    pub fn get(&self, id: PrefabId) -> Option<&Prefab> {
        self.Get(id)
    }

    /// Get a mutable prefab (snake_case alias for GetMut).
    pub fn get_mut(&mut self, id: PrefabId) -> Option<&mut Prefab> {
        self.GetMut(id)
    }

    /// Remove a prefab (snake_case alias for Remove).
    pub fn remove(&mut self, id: PrefabId) -> Option<Prefab> {
        self.Remove(id)
    }

    /// Check if a prefab exists (snake_case alias for Contains).
    pub fn contains(&self, id: PrefabId) -> bool {
        self.Contains(id)
    }

    /// Get the count (snake_case alias for Count).
    pub fn count(&self) -> usize {
        self.Count()
    }

    /// Find a prefab by name (snake_case alias for FindByName).
    pub fn find_by_name(&self, name: &str) -> Option<&Prefab> {
        self.FindByName(name)
    }

    /// Instantiate a prefab (snake_case alias for Instantiate).
    pub fn instantiate(&self, id: PrefabId, world: &mut World) -> Option<PrefabInstance> {
        self.Instantiate(id, world)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::Component;
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

    #[test]
    fn test_prefab_create() {
        let mut world = World::new();
        let handle = world.CreateGameObject("TestObject");
        let go = world.GetTransform(handle); // Just to verify it exists

        let prefab = Prefab::Create("TestPrefab", &GameObject::new_with_name("TestObject"), &world);

        assert_eq!(prefab.Name(), "TestPrefab");
        assert!(prefab.Id().0 > 0);
    }

    #[test]
    fn test_prefab_instantiate() {
        let mut world = World::new();
        let handle = world.CreateGameObject("TestObject");

        let prefab = Prefab::Create("TestPrefab", &GameObject::new_with_name("TestObject"), &world);

        let instance = prefab.Instantiate(&mut world);
        assert_eq!(instance.PrefabId(), prefab.Id());
        assert!(!instance.HasOverrides());
    }

    #[test]
    fn test_apply_and_check_overrides() {
        let mut world = World::new();
        let prefab = Prefab::Create("TestPrefab", &GameObject::new_with_name("TestObject"), &world);
        let mut instance = prefab.Instantiate(&mut world);

        assert!(!instance.HasOverrides());

        instance.ApplyOverride("Transform.Position.X", PrefabValue::Float(1.0));
        assert!(instance.HasOverrides());
        assert!(instance.HasOverride("Transform.Position.X"));
        assert_eq!(
            instance.GetOverride("Transform.Position.X"),
            Some(&PrefabValue::Float(1.0))
        );
        assert_eq!(instance.OverrideCount(), 1);
    }

    #[test]
    fn test_revert_single_override() {
        let mut world = World::new();
        let prefab = Prefab::Create("TestPrefab", &GameObject::new_with_name("TestObject"), &world);
        let mut instance = prefab.Instantiate(&mut world);

        instance.ApplyOverride("Position.X", PrefabValue::Float(5.0));
        assert!(instance.HasOverrides());

        let reverted = instance.Revert("Position.X");
        assert!(reverted);
        assert!(!instance.HasOverrides());
    }

    #[test]
    fn test_revert_all() {
        let mut world = World::new();
        let prefab = Prefab::Create("TestPrefab", &GameObject::new_with_name("TestObject"), &world);
        let mut instance = prefab.Instantiate(&mut world);

        instance.ApplyOverride("A", PrefabValue::Bool(true));
        instance.ApplyOverride("B", PrefabValue::Int(42));
        assert_eq!(instance.OverrideCount(), 2);

        instance.RevertAll();
        assert!(!instance.HasOverrides());
        assert_eq!(instance.OverrideCount(), 0);
    }

    #[test]
    fn test_registry() {
        let mut registry = PrefabRegistry::new();
        assert_eq!(registry.Count(), 0);

        let mut world = World::new();
        let prefab = Prefab::Create("TestPrefab", &GameObject::new_with_name("TestObject"), &world);
        let id = registry.Register(prefab);

        assert_eq!(registry.Count(), 1);
        assert!(registry.Contains(id));
        assert!(registry.Get(id).is_some());
        assert_eq!(registry.Get(id).unwrap().Name(), "TestPrefab");
    }

    #[test]
    fn test_registry_find_by_name() {
        let mut registry = PrefabRegistry::new();
        let world = World::new();

        let go1 = GameObject::new_with_name("Obj1");
        let go2 = GameObject::new_with_name("Obj2");
        registry.Register(Prefab::Create("PrefabA", &go1, &world));
        registry.Register(Prefab::Create("PrefabB", &go2, &world));

        assert!(registry.FindByName("PrefabA").is_some());
        assert!(registry.FindByName("PrefabB").is_some());
        assert!(registry.FindByName("NotExist").is_none());
    }

    #[test]
    fn test_prefab_node_from_game_object() {
        let world = World::new();
        let mut go = GameObject::new_with_name("Root");
        go.SetTag("Player");
        go.SetLayer(5);
        go.SetActive(false);

        let node = PrefabNode::FromGameObject(&go, &world);
        assert_eq!(node.Name(), "Root");
        assert_eq!(node.Tag(), "Player");
        assert_eq!(node.Layer(), 5);
        assert!(!node.Active());
        assert!(node.Components().is_empty());
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

        let handle = node.ToGameObject(&mut world);
        assert_eq!(world.GetName(handle), "FromNode");
        assert_eq!(world.GetTag(handle), "Enemy");
        assert_eq!(world.GetLayer(handle), 3);
        assert!(world.IsActive(handle));
        assert!(world.GetChildren(handle).is_empty());
    }

    #[test]
    fn test_child_hierarchy_preserved() {
        let mut world = World::new();

        // Build a root with two children in the world
        let root_handle = world.CreateGameObject("Root");
        let child1_handle = world.CreateGameObject("Child1");
        let child2_handle = world.CreateGameObject("Child2");

        world.SetParent(child1_handle, Some(root_handle));
        world.SetParent(child2_handle, Some(root_handle));

        // Add a grandchild
        let grandchild_handle = world.CreateGameObject("Grandchild");
        world.SetParent(grandchild_handle, Some(child1_handle));

        // Create prefab from root
        let root_go = GameObject::new_with_name("Root");
        let prefab = Prefab::Create("HierarchyPrefab", &root_go, &world);

        // Verify the prefab captured the hierarchy
        let root_node = prefab.Root();
        assert_eq!(root_node.Name(), "Root");

        // Instantiate the prefab
        let mut new_world = World::new();
        let instance = prefab.Instantiate(&mut new_world);

        // Verify the hierarchy is reconstructed
        let root = new_world.GetName(instance.GameObjectHandle());
        assert_eq!(root, "Root");
    }

    #[test]
    fn test_component_type_names_stored() {
        let world = World::new();
        let mut go = GameObject::new_with_name("WithComponents");
        go.AddComponent(TestComponent { value: 42 });
        go.AddComponent(HealthComponent { hp: 100.0 });

        let node = PrefabNode::FromGameObject(&go, &world);
        assert_eq!(node.Components().len(), 2);
        assert!(
            node.Components()
                .iter()
                .any(|c| c.contains("TestComponent"))
        );
        assert!(
            node.Components()
                .iter()
                .any(|c| c.contains("HealthComponent"))
        );
    }
}
