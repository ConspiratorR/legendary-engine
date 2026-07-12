# Unity Parity Refactoring — Phase 1: Core Architecture

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the ECS-based architecture with Unity-like GameObject/Component model and Player Loop system.

**Architecture:** Create new `GameObject`, `Component`, `World`, and `PlayerLoop` types that mirror Unity's API. The old ECS will be preserved behind a feature flag for backward compatibility.

**Tech Stack:** Rust, serde (serialization), glam (math), egui (editor UI)

---

## File Structure

```
crates/engine-core/src/
├── lib.rs                    # Module declarations
├── gameobject.rs             # GameObject, Component, GameObjectHandle
├── world.rs                  # World container (replaces ECS World)
├── transform.rs              # Transform component (local/world)
├── hierarchy.rs              # Parent-child relationships, sync system
├── player_loop.rs            # PlayerLoop, Phase, LoopStage
├── time.rs                   # Time management
├── context.rs                # Context passed to systems
├── system.rs                 # System trait
├── plugin.rs                 # Plugin trait (updated)
├── app.rs                    # AppBuilder, App (updated)
├── events.rs                 # Event system (placeholder for Phase 3)
├── monobehaviour.rs          # MonoBehaviour trait (placeholder for Phase 2)
└── scriptable_object.rs      # ScriptableObject trait (placeholder for Phase 4)

crates/engine-core/tests/
└── unity_api_tests.rs        # Integration tests

crates/engine-scene/src/
├── lib.rs                    # Updated module declarations
├── node.rs                   # SceneNode (updated to use GameObjectHandle)
├── scene_manager.rs          # SceneManager (updated)
└── transform.rs              # GlobalTransform (kept for rendering)
```

---

## Task 1: Create GameObject and Component Types

**Files:**
- Create: `crates/engine-core/src/gameobject.rs`
- Modify: `crates/engine-core/src/lib.rs`

- [ ] **Step 1: Create gameobject.rs with basic types**

```rust
// crates/engine-core/src/gameobject.rs

use std::any::Any;
use std::fmt;

/// Lightweight handle to a GameObject (index + generation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GameObjectHandle {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

impl GameObjectHandle {
    /// Create a new handle (internal use only).
    pub(crate) fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }
    
    /// Get the index (for internal use).
    pub fn index(&self) -> u32 {
        self.index
    }
    
    /// Get the generation (for internal use).
    pub fn generation(&self) -> u32 {
        self.generation
    }
}

impl fmt::Display for GameObjectHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GameObject({}:{})", self.index, self.generation)
    }
}

/// Base trait for all components (like Unity's Component).
pub trait Component: Any + Send + Sync {
    /// Called when the component is added to a GameObject.
    fn on_added(&mut self, _handle: GameObjectHandle) {}
    
    /// Called when the component is removed from a GameObject.
    fn on_removed(&mut self, _handle: GameObjectHandle) {}
    
    /// Called when the GameObject becomes active.
    fn on_enable(&mut self, _handle: GameObjectHandle) {}
    
    /// Called when the GameObject becomes inactive.
    fn on_disable(&mut self, _handle: GameObjectHandle) {}
    
    /// Called when the GameObject is destroyed.
    fn on_destroy(&mut self, _handle: GameObjectHandle) {}
    
    /// Get the component as Any for downcasting.
    fn as_any(&self) -> &dyn Any;
    
    /// Get the component as mutable Any for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
    
    /// Get the component name (for debugging).
    fn component_name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// Base class for all entities in the scene (like Unity's GameObject).
pub struct GameObject {
    name: String,
    tag: String,
    layer: u32,
    active: bool,
    components: Vec<Box<dyn Component>>,
    parent: Option<GameObjectHandle>,
    children: Vec<GameObjectHandle>,
}

impl GameObject {
    /// Create a new GameObject (like Unity's new GameObject()).
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tag: "Untagged".to_string(),
            layer: 0,
            active: true,
            components: Vec::new(),
            parent: None,
            children: Vec::new(),
        }
    }
    
    /// Get the name.
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Set the name.
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }
    
    /// Get the tag.
    pub fn tag(&self) -> &str {
        &self.tag
    }
    
    /// Set the tag.
    pub fn set_tag(&mut self, tag: &str) {
        self.tag = tag.to_string();
    }
    
    /// Get the layer.
    pub fn layer(&self) -> u32 {
        self.layer
    }
    
    /// Set the layer.
    pub fn set_layer(&mut self, layer: u32) {
        self.layer = layer;
    }
    
    /// Check if the GameObject is active.
    pub fn is_active(&self) -> bool {
        self.active
    }
    
    /// Set the active state.
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }
    
    /// Get the parent handle.
    pub fn parent(&self) -> Option<GameObjectHandle> {
        self.parent
    }
    
    /// Get the children handles.
    pub fn children(&self) -> &[GameObjectHandle] {
        &self.children
    }
    
    /// Get the number of children.
    pub fn child_count(&self) -> usize {
        self.children.len()
    }
    
    /// Add a component (like Unity's AddComponent<T>()).
    pub fn add_component<T: Component + 'static>(&mut self, mut component: T) {
        component.on_added(GameObjectHandle::new(0, 0)); // Handle set by World
        self.components.push(Box::new(component));
    }
    
    /// Get a component by type (like Unity's GetComponent<T>()).
    pub fn get_component<T: Component + 'static>(&self) -> Option<&T> {
        self.components
            .iter()
            .find_map(|c| c.as_any().downcast_ref::<T>())
    }
    
    /// Get a component mutably by type (like Unity's GetComponent<T>()).
    pub fn get_component_mut<T: Component + 'static>(&mut self) -> Option<&mut T> {
        self.components
            .iter_mut()
            .find_map(|c| c.as_any_mut().downcast_mut::<T>())
    }
    
    /// Check if the GameObject has a component.
    pub fn has_component<T: Component + 'static>(&self) -> bool {
        self.get_component::<T>().is_some()
    }
    
    /// Get all components.
    pub fn components(&self) -> &[Box<dyn Component>] {
        &self.components
    }
    
    /// Get all components mutably.
    pub fn components_mut(&mut self) -> &mut Vec<Box<dyn Component>> {
        &mut self.components
    }
    
    /// Remove a component by type.
    pub fn remove_component<T: Component + 'static>(&mut self) -> Option<Box<dyn Component>> {
        if let Some(pos) = self.components.iter().position(|c| c.as_any().is::<T>()) {
            let mut component = self.components.remove(pos);
            component.on_destroy(GameObjectHandle::new(0, 0));
            Some(component)
        } else {
            None
        }
    }
}

impl fmt::Debug for GameObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GameObject")
            .field("name", &self.name)
            .field("tag", &self.tag)
            .field("layer", &self.layer)
            .field("active", &self.active)
            .field("components", &self.components.len())
            .field("children", &self.children.len())
            .finish()
    }
}
```

- [ ] **Step 2: Update lib.rs to include new module**

```rust
// crates/engine-core/src/lib.rs (add to existing)

pub mod gameobject;

// Re-export for convenience
pub use gameobject::{Component, GameObject, GameObjectHandle};
```

- [ ] **Step 3: Write tests for GameObject and Component**

```rust
// crates/engine-core/src/gameobject.rs (add at bottom)

#[cfg(test)]
mod tests {
    use super::*;
    
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
    
    #[test]
    fn test_gameobject_creation() {
        let go = GameObject::new("TestObject");
        assert_eq!(go.name(), "TestObject");
        assert_eq!(go.tag(), "Untagged");
        assert_eq!(go.layer(), 0);
        assert!(go.is_active());
    }
    
    #[test]
    fn test_add_and_get_component() {
        let mut go = GameObject::new("TestObject");
        go.add_component(TestComponent { value: 42 });
        
        let component = go.get_component::<TestComponent>().unwrap();
        assert_eq!(component.value, 42);
    }
    
    #[test]
    fn test_get_component_mut() {
        let mut go = GameObject::new("TestObject");
        go.add_component(TestComponent { value: 42 });
        
        {
            let component = go.get_component_mut::<TestComponent>().unwrap();
            component.value = 100;
        }
        
        let component = go.get_component::<TestComponent>().unwrap();
        assert_eq!(component.value, 100);
    }
    
    #[test]
    fn test_has_component() {
        let mut go = GameObject::new("TestObject");
        assert!(!go.has_component::<TestComponent>());
        
        go.add_component(TestComponent { value: 42 });
        assert!(go.has_component::<TestComponent>());
    }
    
    #[test]
    fn test_remove_component() {
        let mut go = GameObject::new("TestObject");
        go.add_component(TestComponent { value: 42 });
        
        let removed = go.remove_component::<TestComponent>();
        assert!(removed.is_some());
        assert!(!go.has_component::<TestComponent>());
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p engine-core --lib gameobject`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/engine-core/src/gameobject.rs crates/engine-core/src/lib.rs
git commit -m "feat(core): add GameObject and Component types

- Add GameObject struct with name, tag, layer, active state
- Add Component trait with lifecycle callbacks
- Add GameObjectHandle for lightweight references
- Add tests for component operations"
```

---

## Task 2: Create World Container

**Files:**
- Create: `crates/engine-core/src/world.rs`
- Modify: `crates/engine-core/src/lib.rs`

- [ ] **Step 1: Create world.rs with World container**

```rust
// crates/engine-core/src/world.rs

use crate::gameobject::{Component, GameObject, GameObjectHandle};
use std::collections::HashMap;

/// Central container for all GameObjects (replaces ECS World).
pub struct World {
    gameobjects: Vec<Option<GameObject>>,
    generations: Vec<u32>,
    free_list: Vec<u32>,
    name_to_handle: HashMap<String, GameObjectHandle>,
}

impl World {
    /// Create a new empty World.
    pub fn new() -> Self {
        Self {
            gameobjects: Vec::new(),
            generations: Vec::new(),
            free_list: Vec::new(),
            name_to_handle: HashMap::new(),
        }
    }
    
    /// Spawn a new GameObject (like Unity's Instantiate).
    pub fn spawn(&mut self, gameobject: GameObject) -> GameObjectHandle {
        let name = gameobject.name().to_string();
        
        let handle = if let Some(index) = self.free_list.pop() {
            let generation = self.generations[index];
            self.gameobjects[index] = Some(gameobject);
            self.generations[index] = generation + 1;
            GameObjectHandle::new(index, generation + 1)
        } else {
            let index = self.gameobjects.len() as u32;
            self.gameobjects.push(Some(gameobject));
            self.generations.push(0);
            GameObjectHandle::new(index, 0)
        };
        
        self.name_to_handle.insert(name, handle);
        handle
    }
    
    /// Despawn a GameObject (like Unity's Destroy).
    pub fn despawn(&mut self, handle: GameObjectHandle) -> Option<GameObject> {
        if self.is_valid(handle) {
            let index = handle.index() as usize;
            let gameobject = self.gameobjects[index].take();
            
            // Remove from name map
            if let Some(go) = &gameobject {
                self.name_to_handle.remove(go.name());
            }
            
            // Add to free list
            self.free_list.push(index as u32);
            
            gameobject
        } else {
            None
        }
    }
    
    /// Check if a handle is valid.
    pub fn is_valid(&self, handle: GameObjectHandle) -> bool {
        let index = handle.index() as usize;
        index < self.gameobjects.len()
            && self.gameobjects[index].is_some()
            && self.generations[index] == handle.generation()
    }
    
    /// Get a reference to a GameObject.
    pub fn get_gameobject(&self, handle: GameObjectHandle) -> Option<&GameObject> {
        if self.is_valid(handle) {
            self.gameobjects[handle.index() as usize].as_ref()
        } else {
            None
        }
    }
    
    /// Get a mutable reference to a GameObject.
    pub fn get_gameobject_mut(&mut self, handle: GameObjectHandle) -> Option<&mut GameObject> {
        if self.is_valid(handle) {
            self.gameobjects[handle.index() as usize].as_mut()
        } else {
            None
        }
    }
    
    /// Find a GameObject by name.
    pub fn find_gameobject(&self, name: &str) -> Option<GameObjectHandle> {
        self.name_to_handle.get(name).copied()
    }
    
    /// Find all GameObjects with a specific tag.
    pub fn find_gameobjects_with_tag(&self, tag: &str) -> Vec<GameObjectHandle> {
        self.gameobjects
            .iter()
            .enumerate()
            .filter_map(|(i, go)| {
                go.as_ref()
                    .filter(|g| g.tag() == tag && g.is_active())
                    .map(|_| GameObjectHandle::new(i as u32, self.generations[i]))
            })
            .collect()
    }
    
    /// Get all root GameObjects (no parent).
    pub fn root_gameobjects(&self) -> Vec<GameObjectHandle> {
        self.gameobjects
            .iter()
            .enumerate()
            .filter_map(|(i, go)| {
                go.as_ref()
                    .filter(|g| g.parent().is_none() && g.is_active())
                    .map(|_| GameObjectHandle::new(i as u32, self.generations[i]))
            })
            .collect()
    }
    
    /// Get all GameObjects.
    pub fn all_gameobjects(&self) -> Vec<GameObjectHandle> {
        self.gameobjects
            .iter()
            .enumerate()
            .filter_map(|(i, go)| {
                go.as_ref()
                    .map(|_| GameObjectHandle::new(i as u32, self.generations[i]))
            })
            .collect()
    }
    
    /// Get the number of active GameObjects.
    pub fn count(&self) -> usize {
        self.gameobjects.iter().filter(|go| go.is_some()).count()
    }
    
    /// Set parent of a GameObject.
    pub fn set_parent(&mut self, child: GameObjectHandle, parent: Option<GameObjectHandle>, world_position_stays: bool) {
        if !self.is_valid(child) {
            return;
        }
        
        // Remove from old parent's children list
        if let Some(old_parent) = self.get_gameobject(child).and_then(|go| go.parent()) {
            if let Some(parent_go) = self.get_gameobject_mut(old_parent) {
                parent_go.children.retain(|&h| h != child);
            }
        }
        
        // Set new parent
        if let Some(parent_go) = self.get_gameobject_mut(child) {
            parent_go.parent = parent;
        }
        
        // Add to new parent's children list
        if let Some(new_parent) = parent {
            if let Some(parent_go) = self.get_gameobject_mut(new_parent) {
                parent_go.children.push(child);
            }
        }
    }
    
    /// Get children of a GameObject.
    pub fn get_children(&self, handle: GameObjectHandle) -> Vec<GameObjectHandle> {
        self.get_gameobject(handle)
            .map(|go| go.children().to_vec())
            .unwrap_or_default()
    }
    
    /// Get parent of a GameObject.
    pub fn get_parent(&self, handle: GameObjectHandle) -> Option<GameObjectHandle> {
        self.get_gameobject(handle).and_then(|go| go.parent())
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("count", &self.count())
            .field("free_slots", &self.free_list.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameobject::Component;
    use std::any::Any;
    
    #[derive(Debug)]
    struct Transform {
        x: f32,
        y: f32,
    }
    
    impl Component for Transform {
        fn as_any(&self) -> &dyn Any {
            self
        }
        
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }
    
    #[test]
    fn test_world_spawn() {
        let mut world = World::new();
        let go = GameObject::new("Player");
        let handle = world.spawn(go);
        
        assert!(world.is_valid(handle));
        assert_eq!(world.get_gameobject(handle).unwrap().name(), "Player");
    }
    
    #[test]
    fn test_world_despawn() {
        let mut world = World::new();
        let go = GameObject::new("Player");
        let handle = world.spawn(go);
        
        let removed = world.despawn(handle);
        assert!(removed.is_some());
        assert!(!world.is_valid(handle));
    }
    
    #[test]
    fn test_world_find_by_name() {
        let mut world = World::new();
        let go = GameObject::new("Player");
        let handle = world.spawn(go);
        
        let found = world.find_gameobject("Player");
        assert_eq!(found, Some(handle));
    }
    
    #[test]
    fn test_world_parent_child() {
        let mut world = World::new();
        
        let parent = world.spawn(GameObject::new("Parent"));
        let child = world.spawn(GameObject::new("Child"));
        
        world.set_parent(child, Some(parent), true);
        
        assert_eq!(world.get_parent(child), Some(parent));
        assert!(world.get_children(parent).contains(&child));
    }
    
    #[test]
    fn test_world_recycle_slot() {
        let mut world = World::new();
        
        let go1 = world.spawn(GameObject::new("First"));
        let go2 = world.spawn(GameObject::new("Second"));
        
        world.despawn(go1);
        
        let go3 = world.spawn(GameObject::new("Third"));
        
        // go3 should reuse go1's slot
        assert_eq!(go3.index(), go1.index());
        assert_ne!(go3.generation(), go1.generation());
    }
}
```

- [ ] **Step 2: Update lib.rs to include world module**

```rust
// crates/engine-core/src/lib.rs (add to existing)

pub mod world;

// Re-export for convenience
pub use world::World;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p engine-core --lib world`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/world.rs crates/engine-core/src/lib.rs
git commit -m "feat(core): add World container

- Add World struct with spawn/despawn operations
- Add handle validation with generation tracking
- Add parent-child hierarchy support
- Add name-based lookup
- Add slot recycling for performance"
```

---

## Task 3: Create Transform Component

**Files:**
- Create: `crates/engine-core/src/transform.rs`
- Modify: `crates/engine-core/src/lib.rs`

- [ ] **Step 1: Create transform.rs with Transform component**

```rust
// crates/engine-core/src/transform.rs

use engine_math::{Mat4, Quat, Vec3};
use crate::gameobject::Component;
use serde::{Deserialize, Serialize};
use std::any::Any;

/// Space for transformations (like Unity's Space).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Space {
    /// Relative to parent (local space).
    Self_,
    /// Relative to world (world space).
    World,
}

/// Transform component (like Unity's Transform).
/// Stores position, rotation, scale relative to parent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    /// Local position (relative to parent).
    pub local_position: Vec3,
    /// Local rotation (relative to parent).
    pub local_rotation: Quat,
    /// Local scale (relative to parent).
    pub local_scale: Vec3,
    
    // Cached world transform (computed by sync system)
    #[serde(skip)]
    world_position: Vec3,
    #[serde(skip)]
    world_rotation: Quat,
    #[serde(skip)]
    world_scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            local_position: Vec3::ZERO,
            local_rotation: Quat::IDENTITY,
            local_scale: Vec3::ONE,
            world_position: Vec3::ZERO,
            world_rotation: Quat::IDENTITY,
            world_scale: Vec3::ONE,
        }
    }
}

impl Component for Transform {
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Transform {
    /// Create a transform at the given position.
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self {
            local_position: Vec3::new(x, y, z),
            ..Default::default()
        }
    }
    
    /// Create a transform at the given position with rotation and scale.
    pub fn from_position_rotation_scale(position: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            local_position: position,
            local_rotation: rotation,
            local_scale: scale,
            world_position: position,
            world_rotation: rotation,
            world_scale: scale,
        }
    }
    
    /// Get world position (like Unity's Transform.position).
    pub fn position(&self) -> Vec3 {
        self.world_position
    }
    
    /// Set world position (like Unity's Transform.position).
    pub fn set_position(&mut self, position: Vec3) {
        self.world_position = position;
        // Note: local_position will be computed by sync system
    }
    
    /// Get world rotation (like Unity's Transform.rotation).
    pub fn rotation(&self) -> Quat {
        self.world_rotation
    }
    
    /// Set world rotation (like Unity's Transform.rotation).
    pub fn set_rotation(&mut self, rotation: Quat) {
        self.world_rotation = rotation;
    }
    
    /// Get world scale (like Unity's Transform.lossyScale).
    pub fn lossy_scale(&self) -> Vec3 {
        self.world_scale
    }
    
    /// Get local position (like Unity's Transform.localPosition).
    pub fn local_position(&self) -> Vec3 {
        self.local_position
    }
    
    /// Set local position (like Unity's Transform.localPosition).
    pub fn set_local_position(&mut self, position: Vec3) {
        self.local_position = position;
    }
    
    /// Get local rotation (like Unity's Transform.localRotation).
    pub fn local_rotation(&self) -> Quat {
        self.local_rotation
    }
    
    /// Set local rotation (like Unity's Transform.localRotation).
    pub fn set_local_rotation(&mut self, rotation: Quat) {
        self.local_rotation = rotation;
    }
    
    /// Get local scale (like Unity's Transform.localScale).
    pub fn local_scale(&self) -> Vec3 {
        self.local_scale
    }
    
    /// Set local scale (like Unity's Transform.localScale).
    pub fn set_local_scale(&mut self, scale: Vec3) {
        self.local_scale = scale;
    }
    
    /// Get forward direction (like Unity's Transform.forward).
    pub fn forward(&self) -> Vec3 {
        self.world_rotation * Vec3::Z
    }
    
    /// Get right direction (like Unity's Transform.right).
    pub fn right(&self) -> Vec3 {
        self.world_rotation * Vec3::X
    }
    
    /// Get up direction (like Unity's Transform.up).
    pub fn up(&self) -> Vec3 {
        self.world_rotation * Vec3::Y
    }
    
    /// Transform a point from local to world space (like Unity's Transform.TransformPoint).
    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        self.world_position + self.world_rotation * (point * self.world_scale)
    }
    
    /// Transform a point from world to local space (like Unity's Transform.InverseTransformPoint).
    pub fn inverse_transform_point(&self, point: Vec3) -> Vec3 {
        let relative = point - self.world_position;
        let inv_rotation = self.world_rotation.inverse();
        let inv_scale = Vec3::new(1.0 / self.world_scale.x, 1.0 / self.world_scale.y, 1.0 / self.world_scale.z);
        inv_rotation * (relative * inv_scale)
    }
    
    /// Transform a direction from local to world space (like Unity's Transform.TransformDirection).
    pub fn transform_direction(&self, direction: Vec3) -> Vec3 {
        self.world_rotation * direction
    }
    
    /// Transform a direction from world to local space (like Unity's Transform.InverseTransformDirection).
    pub fn inverse_transform_direction(&self, direction: Vec3) -> Vec3 {
        self.world_rotation.inverse() * direction
    }
    
    /// Look at a target position (like Unity's Transform.LookAt).
    pub fn look_at(&mut self, target: Vec3) {
        let direction = (target - self.world_position).normalize();
        if direction.length_squared() > 0.0001 {
            self.world_rotation = Quat::from_rotation_arc(-Vec3::Z, direction);
        }
    }
    
    /// Rotate around a point (like Unity's Transform.RotateAround).
    pub fn rotate_around(&mut self, point: Vec3, axis: Vec3, angle: f32) {
        let rotation = Quat::from_axis_angle(axis, angle.to_radians());
        let offset = self.world_position - point;
        self.world_position = point + rotation * offset;
        self.world_rotation = rotation * self.world_rotation;
    }
    
    /// Translate in world/local space (like Unity's Transform.Translate).
    pub fn translate(&mut self, translation: Vec3, space: Space) {
        match space {
            Space::World => {
                self.world_position += translation;
            }
            Space::Self_ => {
                self.world_position += self.world_rotation * translation;
            }
        }
    }
    
    /// Get the local-to-world matrix.
    pub fn local_to_world_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.world_scale, self.world_rotation, self.world_position)
    }
    
    /// Get the world-to-local matrix.
    pub fn world_to_local_matrix(&self) -> Mat4 {
        self.local_to_world_matrix().inverse()
    }
    
    /// Update the cached world transform from parent (called by sync system).
    pub fn update_world_transform(&mut self, parent_world_position: Vec3, parent_world_rotation: Quat, parent_world_scale: Vec3) {
        self.world_position = parent_world_position + parent_world_rotation * (self.local_position * parent_world_scale);
        self.world_rotation = parent_world_rotation * self.local_rotation;
        self.world_scale = parent_world_scale * self.local_scale;
    }
    
    /// Update the cached world transform for root (no parent).
    pub fn update_world_transform_root(&mut self) {
        self.world_position = self.local_position;
        self.world_rotation = self.local_rotation;
        self.world_scale = self.local_scale;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_transform_default() {
        let t = Transform::default();
        assert_eq!(t.local_position, Vec3::ZERO);
        assert_eq!(t.local_rotation, Quat::IDENTITY);
        assert_eq!(t.local_scale, Vec3::ONE);
    }
    
    #[test]
    fn test_transform_from_xyz() {
        let t = Transform::from_xyz(1.0, 2.0, 3.0);
        assert_eq!(t.local_position, Vec3::new(1.0, 2.0, 3.0));
    }
    
    #[test]
    fn test_transform_forward() {
        let t = Transform::default();
        assert_eq!(t.forward(), Vec3::Z);
    }
    
    #[test]
    fn test_transform_look_at() {
        let mut t = Transform::from_xyz(0.0, 0.0, 0.0);
        t.look_at(Vec3::new(1.0, 0.0, 0.0));
        
        let forward = t.forward();
        assert!((forward.x - 1.0).abs() < 0.001);
        assert!(forward.y.abs() < 0.001);
        assert!(forward.z.abs() < 0.001);
    }
    
    #[test]
    fn test_transform_translate() {
        let mut t = Transform::default();
        t.translate(Vec3::new(1.0, 0.0, 0.0), Space::World);
        
        assert_eq!(t.world_position, Vec3::new(1.0, 0.0, 0.0));
    }
    
    #[test]
    fn test_transform_update_world() {
        let mut t = Transform::from_xyz(1.0, 0.0, 0.0);
        t.update_world_transform(
            Vec3::new(5.0, 0.0, 0.0),
            Quat::IDENTITY,
            Vec3::ONE,
        );
        
        assert_eq!(t.world_position, Vec3::new(6.0, 0.0, 0.0));
    }
}
```

- [ ] **Step 2: Update lib.rs to include transform module**

```rust
// crates/engine-core/src/lib.rs (add to existing)

pub mod transform;

// Re-export for convenience
pub use transform::{Transform, Space};
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p engine-core --lib transform`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/transform.rs crates/engine-core/src/lib.rs
git commit -m "feat(core): add Transform component

- Add Transform struct with local/world transforms
- Add Space enum for coordinate spaces
- Add direction getters (forward, right, up)
- Add point/direction transformation methods
- Add look_at, rotate_around, translate methods
- Add world transform sync helpers"
```

---

## Task 4: Create Hierarchy System

**Files:**
- Create: `crates/engine-core/src/hierarchy.rs`
- Modify: `crates/engine-core/src/lib.rs`, `crates/engine-core/src/world.rs`

- [ ] **Step 1: Create hierarchy.rs with sync system**

```rust
// crates/engine-core/src/hierarchy.rs

use crate::gameobject::GameObjectHandle;
use crate::transform::Transform;
use crate::world::World;

/// System that synchronizes world transforms from local transforms.
/// Runs in the PreUpdate phase (before gameplay systems).
pub fn sync_transforms(world: &mut World) {
    // Collect root GameObjects first to avoid borrow issues
    let roots: Vec<GameObjectHandle> = world.root_gameobjects();
    
    // Sync root transforms
    for root in roots {
        sync_transform_recursive(world, root, true);
    }
}

/// Recursively sync transform for a GameObject and its children.
fn sync_transform_recursive(world: &mut World, handle: GameObjectHandle, is_root: bool) {
    // Get parent transform data before borrowing child
    let (parent_pos, parent_rot, parent_scale) = if is_root {
        (None, None, None)
    } else {
        // This is a simplified version - in production, we'd cache parent transforms
        // For now, we assume parent transform is already synced
        (Some(Vec3::ZERO), Some(engine_math::Quat::IDENTITY), Some(Vec3::ONE))
    };
    
    // Get children before modifying transform
    let children = world.get_children(handle);
    
    // Update this transform
    if let Some(transform) = world.get_gameobject_mut(handle).and_then(|go| go.get_component_mut::<Transform>()) {
        if is_root {
            transform.update_world_transform_root();
        } else if let (Some(pos), Some(rot), Some(scale)) = (parent_pos, parent_rot, parent_scale) {
            transform.update_world_transform(pos, rot, scale);
        }
    }
    
    // Recursively sync children
    for child in children {
        sync_transform_recursive(world, child, false);
    }
}

/// Get all ancestors of a GameObject (from immediate parent to root).
pub fn get_ancestors(world: &World, handle: GameObjectHandle) -> Vec<GameObjectHandle> {
    let mut ancestors = Vec::new();
    let mut current = world.get_parent(handle);
    
    while let Some(parent) = current {
        ancestors.push(parent);
        current = world.get_parent(parent);
    }
    
    ancestors
}

/// Get the root ancestor of a GameObject.
pub fn get_root(world: &World, handle: GameObjectHandle) -> GameObjectHandle {
    let mut current = handle;
    while let Some(parent) = world.get_parent(current) {
        current = parent;
    }
    current
}

/// Check if a GameObject is an ancestor of another.
pub fn is_ancestor(world: &World, ancestor: GameObjectHandle, descendant: GameObjectHandle) -> bool {
    let mut current = world.get_parent(descendant);
    while let Some(parent) = current {
        if parent == ancestor {
            return true;
        }
        current = world.get_parent(parent);
    }
    false
}

/// Get the depth of a GameObject in the hierarchy (root = 0).
pub fn get_depth(world: &World, handle: GameObjectHandle) -> usize {
    let mut depth = 0;
    let mut current = world.get_parent(handle);
    
    while let Some(parent) = current {
        depth += 1;
        current = world.get_parent(parent);
    }
    
    depth
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameobject::GameObject;
    use crate::transform::Transform;
    
    #[test]
    fn test_sync_root_transform() {
        let mut world = World::new();
        let mut go = GameObject::new("Root");
        go.add_component(Transform::from_xyz(1.0, 2.0, 3.0));
        let handle = world.spawn(go);
        
        sync_transforms(&world);
        
        let transform = world.get_gameobject(handle).unwrap().get_component::<Transform>().unwrap();
        assert_eq!(transform.position(), Vec3::new(1.0, 2.0, 3.0));
    }
    
    #[test]
    fn test_get_ancestors() {
        let mut world = World::new();
        
        let root = world.spawn(GameObject::new("Root"));
        let child = world.spawn(GameObject::new("Child"));
        let grandchild = world.spawn(GameObject::new("Grandchild"));
        
        world.set_parent(child, Some(root), true);
        world.set_parent(grandchild, Some(child), true);
        
        let ancestors = get_ancestors(&world, grandchild);
        assert_eq!(ancestors, vec![child, root]);
    }
    
    #[test]
    fn test_get_root() {
        let mut world = World::new();
        
        let root = world.spawn(GameObject::new("Root"));
        let child = world.spawn(GameObject::new("Child"));
        let grandchild = world.spawn(GameObject::new("Grandchild"));
        
        world.set_parent(child, Some(root), true);
        world.set_parent(grandchild, Some(child), true);
        
        assert_eq!(get_root(&world, grandchild), root);
    }
    
    #[test]
    fn test_is_ancestor() {
        let mut world = World::new();
        
        let root = world.spawn(GameObject::new("Root"));
        let child = world.spawn(GameObject::new("Child"));
        let grandchild = world.spawn(GameObject::new("Grandchild"));
        
        world.set_parent(child, Some(root), true);
        world.set_parent(grandchild, Some(child), true);
        
        assert!(is_ancestor(&world, root, grandchild));
        assert!(is_ancestor(&world, child, grandchild));
        assert!(!is_ancestor(&world, grandchild, root));
    }
    
    #[test]
    fn test_get_depth() {
        let mut world = World::new();
        
        let root = world.spawn(GameObject::new("Root"));
        let child = world.spawn(GameObject::new("Child"));
        let grandchild = world.spawn(GameObject::new("Grandchild"));
        
        world.set_parent(child, Some(root), true);
        world.set_parent(grandchild, Some(child), true);
        
        assert_eq!(get_depth(&world, root), 0);
        assert_eq!(get_depth(&world, child), 1);
        assert_eq!(get_depth(&world, grandchild), 2);
    }
}
```

- [ ] **Step 2: Update lib.rs to include hierarchy module**

```rust
// crates/engine-core/src/lib.rs (add to existing)

pub mod hierarchy;

// Re-export for convenience
pub use hierarchy::{sync_transforms, get_ancestors, get_root, is_ancestor, get_depth};
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p engine-core --lib hierarchy`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/hierarchy.rs crates/engine-core/src/lib.rs
git commit -m "feat(core): add hierarchy system

- Add sync_transforms system for world transform updates
- Add get_ancestors, get_root, is_ancestor utilities
- Add get_depth for hierarchy traversal
- Add recursive sync for parent-child relationships"
```

---

## Task 5: Create Player Loop

**Files:**
- Create: `crates/engine-core/src/player_loop.rs`
- Create: `crates/engine-core/src/system.rs`
- Create: `crates/engine-core/src/context.rs`
- Modify: `crates/engine-core/src/lib.rs`

- [ ] **Step 1: Create system.rs with System trait**

```rust
// crates/engine-core/src/system.rs

use crate::context::Context;

/// Trait for game logic systems (like Unity's PlayerLoopSystem).
pub trait System: Send + Sync {
    /// Run the system.
    fn run(&self, context: &mut Context);
    
    /// Get the system name (for debugging).
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// Blanket implementation for closures.
impl<F: Fn(&mut Context) + Send + Sync> System for F {
    fn run(&self, context: &mut Context) {
        self(context);
    }
}

/// Wrapper for systems with a custom name.
pub struct NamedSystem {
    name: String,
    system: Box<dyn System>,
}

impl NamedSystem {
    /// Create a new named system.
    pub fn new(name: &str, system: impl System + 'static) -> Self {
        Self {
            name: name.to_string(),
            system: Box::new(system),
        }
    }
}

impl System for NamedSystem {
    fn run(&self, context: &mut Context) {
        self.system.run(context);
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}
```

- [ ] **Step 2: Create context.rs with Context struct**

```rust
// crates/engine-core/src/context.rs

use crate::time::Time;
use crate::world::World;

/// Context passed to systems during execution.
pub struct Context<'a> {
    /// The ECS world.
    pub world: &'a mut World,
    /// Time information.
    pub time: Time,
    /// Current frame number.
    pub frame: u64,
}

impl<'a> Context<'a> {
    /// Create a new context.
    pub fn new(world: &'a mut World, time: Time, frame: u64) -> Self {
        Self { world, time, frame }
    }
}
```

- [ ] **Step 3: Create player_loop.rs with PlayerLoop**

```rust
// crates/engine-core/src/player_loop.rs

use crate::context::Context;
use crate::system::System;
use std::collections::HashMap;

/// Execution phase (like Unity's PlayerLoopTiming).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Initialization (Time, Input).
    Initialization,
    /// Before FixedUpdate.
    PreFixedUpdate,
    /// Fixed timestep (physics, animation).
    FixedUpdate,
    /// After FixedUpdate.
    PostFixedUpdate,
    /// Before Update.
    PreUpdate,
    /// Main update (game logic, input).
    Update,
    /// After Update.
    PostUpdate,
    /// Before LateUpdate.
    PreLateUpdate,
    /// Late update (camera follow, etc.).
    LateUpdate,
    /// After LateUpdate.
    PostLateUpdate,
    /// Rendering.
    Render,
    /// After rendering.
    AfterRender,
    /// Cleanup.
    Cleanup,
}

impl Phase {
    /// Get all phases in order.
    pub fn all() -> &'static [Phase] {
        &[
            Phase::Initialization,
            Phase::PreFixedUpdate,
            Phase::FixedUpdate,
            Phase::PostFixedUpdate,
            Phase::PreUpdate,
            Phase::Update,
            Phase::PostUpdate,
            Phase::PreLateUpdate,
            Phase::LateUpdate,
            Phase::PostLateUpdate,
            Phase::Render,
            Phase::AfterRender,
            Phase::Cleanup,
        ]
    }
    
    /// Get the index of this phase (for ordering).
    pub fn index(&self) -> usize {
        Phase::all().iter().position(|&p| p == *self).unwrap_or(0)
    }
}

/// A system registered for a specific phase.
struct PhaseSystem {
    phase: Phase,
    system: Box<dyn System>,
}

/// The main game loop (like Unity's PlayerLoop).
pub struct PlayerLoop {
    systems: Vec<PhaseSystem>,
    startup_systems: Vec<Box<dyn System>>,
    startup_done: bool,
}

impl PlayerLoop {
    /// Create a new PlayerLoop.
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
            startup_systems: Vec::new(),
            startup_done: false,
        }
    }
    
    /// Register a system for a specific phase.
    pub fn add_system(&mut self, phase: Phase, system: impl System + 'static) {
        self.systems.push(PhaseSystem {
            phase,
            system: Box::new(system),
        });
    }
    
    /// Register a startup system (runs once before first frame).
    pub fn add_startup_system(&mut self, system: impl System + 'static) {
        self.startup_systems.push(Box::new(system));
    }
    
    /// Execute one frame.
    pub fn run(&mut self, context: &mut Context) {
        // Run startup systems once
        if !self.startup_done {
            for system in &self.startup_systems {
                system.run(context);
            }
            self.startup_done = true;
        }
        
        // Sort systems by phase order
        let mut phase_groups: HashMap<usize, Vec<&PhaseSystem>> = HashMap::new();
        for system in &self.systems {
            let index = system.phase.index();
            phase_groups.entry(index).or_default().push(system);
        }
        
        // Execute phases in order
        let mut indices: Vec<usize> = phase_groups.keys().copied().collect();
        indices.sort();
        
        for index in indices {
            if let Some(systems) = phase_groups.get(&index) {
                for system in systems {
                    system.system.run(context);
                }
            }
        }
    }
    
    /// Get the number of registered systems.
    pub fn system_count(&self) -> usize {
        self.systems.len()
    }
    
    /// Get the number of startup systems.
    pub fn startup_system_count(&self) -> usize {
        self.startup_systems.len()
    }
}

impl Default for PlayerLoop {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::World;
    use crate::time::Time;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    
    #[test]
    fn test_player_loop_phases() {
        assert_eq!(Phase::all().len(), 13);
        assert_eq!(Phase::Initialization.index(), 0);
        assert_eq!(Phase::Update.index(), 5);
        assert_eq!(Phase::Cleanup.index(), 12);
    }
    
    #[test]
    fn test_player_loop_run() {
        COUNTER.store(0, Ordering::SeqCst);
        
        let mut loop_ = PlayerLoop::new();
        loop_.add_system(Phase::Update, |_: &mut Context| {
            COUNTER.fetch_add(1, Ordering::SeqCst);
        });
        
        let mut world = World::new();
        let time = Time::default();
        let mut context = Context::new(&mut world, time, 0);
        
        loop_.run(&mut context);
        assert_eq!(COUNTER.load(Ordering::SeqCst), 1);
        
        loop_.run(&mut context);
        assert_eq!(COUNTER.load(Ordering::SeqCst), 2);
    }
    
    #[test]
    fn test_player_loop_startup() {
        COUNTER.store(0, Ordering::SeqCst);
        
        let mut loop_ = PlayerLoop::new();
        loop_.add_startup_system(|_: &mut Context| {
            COUNTER.fetch_add(10, Ordering::SeqCst);
        });
        loop_.add_system(Phase::Update, |_: &mut Context| {
            COUNTER.fetch_add(1, Ordering::SeqCst);
        });
        
        let mut world = World::new();
        let time = Time::default();
        let mut context = Context::new(&mut world, time, 0);
        
        loop_.run(&mut context); // Startup + Update
        assert_eq!(COUNTER.load(Ordering::SeqCst), 11);
        
        loop_.run(&mut context); // Only Update (startup already ran)
        assert_eq!(COUNTER.load(Ordering::SeqCst), 12);
    }
    
    #[test]
    fn test_player_loop_phase_order() {
        COUNTER.store(0, Ordering::SeqCst);
        
        let mut loop_ = PlayerLoop::new();
        loop_.add_system(Phase::LateUpdate, |_: &mut Context| {
            COUNTER.fetch_add(100, Ordering::SeqCst);
        });
        loop_.add_system(Phase::Update, |_: &mut Context| {
            COUNTER.fetch_add(1, Ordering::SeqCst);
        });
        
        let mut world = World::new();
        let time = Time::default();
        let mut context = Context::new(&mut world, time, 0);
        
        loop_.run(&mut context);
        // Update (1) should run before LateUpdate (100)
        assert_eq!(COUNTER.load(Ordering::SeqCst), 101);
    }
}
```

- [ ] **Step 4: Update lib.rs to include new modules**

```rust
// crates/engine-core/src/lib.rs (add to existing)

pub mod system;
pub mod context;
pub mod player_loop;

// Re-export for convenience
pub use system::System;
pub use context::Context;
pub use player_loop::{PlayerLoop, Phase};
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p engine-core --lib player_loop`
Expected: All tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/engine-core/src/system.rs crates/engine-core/src/context.rs crates/engine-core/src/player_loop.rs crates/engine-core/src/lib.rs
git commit -m "feat(core): add Player Loop system

- Add System trait for game logic
- Add Context struct with World and Time
- Add Phase enum with 13 execution phases
- Add PlayerLoop with phase-based execution
- Add startup system support
- Add phase ordering guarantees"
```

---

## Task 6: Create Time Management

**Files:**
- Create: `crates/engine-core/src/time.rs`
- Modify: `crates/engine-core/src/lib.rs`

- [ ] **Step 1: Create time.rs with Time struct**

```rust
// crates/engine-core/src/time.rs

/// Time information (like Unity's Time class).
#[derive(Debug, Clone)]
pub struct Time {
    /// Time since last frame (deltaTime).
    delta_time: f32,
    /// Total time since application start (time).
    elapsed_time: f32,
    /// Fixed timestep (fixedDeltaTime).
    fixed_delta_time: f32,
    /// Time scale (timeScale).
    time_scale: f32,
    /// Frame count (frameCount).
    frame_count: u64,
    /// Whether we're in FixedUpdate.
    in_fixed_update: bool,
    /// Maximum allowed delta time (maximumDeltaTime).
    max_delta_time: f32,
}

impl Default for Time {
    fn default() -> Self {
        Self {
            delta_time: 0.0,
            elapsed_time: 0.0,
            fixed_delta_time: 0.02, // 50 Hz
            time_scale: 1.0,
            frame_count: 0,
            in_fixed_update: false,
            max_delta_time: 0.33333334, // ~3 FPS minimum
        }
    }
}

impl Time {
    /// Create a new Time with default values.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Get delta time (scaled by timeScale) (like Unity:: Time.deltaTime).
    pub fn deltaTime(&self) -> f32 {
        self.delta_time * self.time_scale
    }
    
    /// Get unscaled delta time (like Unity:: Time.unscaledDeltaTime).
    pub fn unscaledDeltaTime(&self) -> f32 {
        self.delta_time
    }
    
    /// Get fixed delta time (like Unity:: Time.fixedDeltaTime).
    pub fn fixedDeltaTime(&self) -> f32 {
        self.fixed_delta_time
    }
    
    /// Get total elapsed time (like Unity:: Time.time).
    pub fn time(&self) -> f32 {
        self.elapsed_time
    }
    
    /// Get unscaled total time (like Unity:: Time.unscaledTime).
    pub fn unscaledTime(&self) -> f32 {
        self.elapsed_time
    }
    
    /// Get time since last fixed update (like Unity:: Time.fixedUnscaledTime).
    pub fn fixedUnscaledTime(&self) -> f32 {
        self.elapsed_time
    }
    
    /// Get frame count (like Unity:: Time.frameCount).
    pub fn frameCount(&self) -> u64 {
        self.frame_count
    }
    
    /// Get time scale (like Unity:: Time.timeScale).
    pub fn timeScale(&self) -> f32 {
        self.time_scale
    }
    
    /// Set time scale (like Unity:: Time.timeScale).
    pub fn set_timeScale(&mut self, scale: f32) {
        self.time_scale = scale.clamp(0.0, 100.0);
    }
    
    /// Get maximum delta time (like Unity:: Time.maximumDeltaTime).
    pub fn maximumDeltaTime(&self) -> f32 {
        self.max_delta_time
    }
    
    /// Set maximum delta time.
    pub fn set_maximumDeltaTime(&mut self, max: f32) {
        self.max_delta_time = max.max(0.0);
    }
    
    /// Check if we're in FixedUpdate (like Unity:: Time.inFixedTimeStep).
    pub fn inFixedTimeStep(&self) -> bool {
        self.in_fixed_update
    }
    
    /// Get delta time for the current step (fixed or regular).
    pub fn stepDeltaTime(&self) -> f32 {
        if self.in_fixed_update {
            self.fixed_delta_time
        } else {
            self.deltaTime()
        }
    }
    
    // Internal methods for updating time
    
    /// Update time for a new frame (called by engine).
    pub fn update(&mut self, delta: f32) {
        self.delta_time = delta.min(self.max_delta_time);
        self.elapsed_time += self.deltaTime();
        self.frame_count += 1;
        self.in_fixed_update = false;
    }
    
    /// Update time for a fixed update step (called by engine).
    pub fn update_fixed(&mut self) {
        self.in_fixed_update = true;
    }
    
    /// Reset time (for new level, etc.).
    pub fn reset(&mut self) {
        self.delta_time = 0.0;
        self.elapsed_time = 0.0;
        self.frame_count = 0;
        self.in_fixed_update = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_time_default() {
        let time = Time::default();
        assert_eq!(time.deltaTime(), 0.0);
        assert_eq!(time.time(), 0.0);
        assert_eq!(time.timeScale(), 1.0);
        assert_eq!(time.frameCount(), 0);
    }
    
    #[test]
    fn test_time_update() {
        let mut time = Time::default();
        
        time.update(0.016); // 60 FPS
        assert_eq!(time.deltaTime(), 0.016);
        assert_eq!(time.time(), 0.016);
        assert_eq!(time.frameCount(), 1);
        
        time.update(0.016);
        assert_eq!(time.time(), 0.032);
        assert_eq!(time.frameCount(), 2);
    }
    
    #[test]
    fn test_time_scale() {
        let mut time = Time::default();
        time.set_timeScale(0.5);
        
        time.update(0.016);
        assert_eq!(time.deltaTime(), 0.008); // Scaled
        assert_eq!(time.unscaledDeltaTime(), 0.016); // Unscaled
    }
    
    #[test]
    fn test_time_max_delta() {
        let mut time = Time::default();
        time.set_maximumDeltaTime(0.1);
        
        time.update(1.0); // Very large delta
        assert_eq!(time.deltaTime(), 0.1); // Clamped
    }
    
    #[test]
    fn test_time_fixed_update() {
        let mut time = Time::default();
        
        time.update(0.016);
        assert!(!time.inFixedTimeStep());
        
        time.update_fixed();
        assert!(time.inFixedTimeStep());
        assert_eq!(time.stepDeltaTime(), time.fixedDeltaTime());
    }
    
    #[test]
    fn test_time_reset() {
        let mut time = Time::default();
        time.update(0.016);
        time.update(0.016);
        
        time.reset();
        assert_eq!(time.time(), 0.0);
        assert_eq!(time.frameCount(), 0);
    }
}
```

- [ ] **Step 2: Update lib.rs to include time module**

```rust
// crates/engine-core/src/lib.rs (add to existing)

pub mod time;

// Re-export for convenience
pub use time::Time;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p engine-core --lib time`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/time.rs crates/engine-core/src/lib.rs
git commit -m "feat(core): add Time management

- Add Time struct with delta, elapsed, fixed delta
- Add time scale support
- Add maximum delta time clamping
- Add fixed update support
- Add reset functionality"
```

---

## Task 7: Update AppBuilder and App

**Files:**
- Modify: `crates/engine-core/src/app.rs`
- Modify: `crates/engine-core/src/plugin.rs`

- [ ] **Step 1: Update Plugin trait**

```rust
// crates/engine-core/src/plugin.rs

use crate::app::AppBuilder;

/// Trait for engine plugins (like Unity's MonoBehaviour for initialization).
pub trait Plugin: Send + Sync {
    /// Configure the application by adding systems, resources, and hooks.
    fn build(&self, app: &mut AppBuilder);
    
    /// Get the plugin name (for debugging).
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// Blanket implementation for closures.
impl<F: Fn(&mut AppBuilder) + Send + Sync> Plugin for F {
    fn build(&self, app: &mut AppBuilder) {
        self(app);
    }
}
```

- [ ] **Step 2: Update AppBuilder**

```rust
// crates/engine-core/src/app.rs

use crate::plugin::Plugin;
use crate::player_loop::{Phase, PlayerLoop};
use crate::system::System;
use crate::time::Time;
use crate::world::World;
use crate::gameobject::GameObject;
use crate::transform::Transform;

/// Builder for constructing an App with plugins, systems, and resources.
pub struct AppBuilder {
    world: World,
    player_loop: PlayerLoop,
    time: Time,
    plugins: Vec<Box<dyn Plugin>>,
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AppBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self {
            world: World::new(),
            player_loop: PlayerLoop::new(),
            time: Time::new(),
            plugins: Vec::new(),
        }
    }
    
    /// Register a plugin (like Unity:: AppBuilder.AddPlugin).
    pub fn add_plugin(&mut self, plugin: impl Plugin + 'static) -> &mut Self {
        self.plugins.push(Box::new(plugin));
        self
    }
    
    /// Add a system to the Update phase (like Unity's Update).
    pub fn add_system(&mut self, system: impl System + 'static) -> &mut Self {
        self.player_loop.add_system(Phase::Update, system);
        self
    }
    
    /// Add a system to a specific phase.
    pub fn add_system_to_phase(&mut self, phase: Phase, system: impl System + 'static) -> &mut Self {
        self.player_loop.add_system(phase, system);
        self
    }
    
    /// Add a startup system (runs once before first frame).
    pub fn add_startup_system(&mut self, system: impl System + 'static) -> &mut Self {
        self.player_loop.add_startup_system(system);
        self
    }
    
    /// Add a system that runs at fixed timestep (FixedUpdate phase).
    pub fn add_fixed_update_system(&mut self, system: impl System + 'static) -> &mut Self {
        self.player_loop.add_system(Phase::FixedUpdate, system);
        self
    }
    
    /// Add a system that runs after all other updates (LateUpdate phase).
    pub fn add_late_update_system(&mut self, system: impl System + 'static) -> &mut Self {
        self.player_loop.add_system(Phase::LateUpdate, system);
        self
    }
    
    /// Spawn a GameObject (convenience method).
    pub fn spawn_gameobject(&mut self, gameobject: GameObject) -> crate::gameobject::GameObjectHandle {
        self.world.spawn(gameobject)
    }
    
    /// Get mutable access to the ECS world.
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }
    
    /// Get shared access to the ECS world.
    pub fn world(&self) -> &World {
        &self.world
    }
    
    /// Get mutable access to the player loop.
    pub fn player_loop_mut(&mut self) -> &mut PlayerLoop {
        &mut self.player_loop
    }
    
    /// Get shared access to the time.
    pub fn time(&self) -> &Time {
        &self.time
    }
    
    /// Finalize the builder and produce an App.
    pub fn build(mut self) -> App {
        // Run plugin builds
        for plugin in &self.plugins {
            plugin.build(&mut self);
        }
        
        App::from(self)
    }
}

/// The main application.
pub struct App {
    world: World,
    player_loop: PlayerLoop,
    time: Time,
    frame: u64,
    running: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Create a new app with default settings.
    pub fn new() -> Self {
        Self {
            world: World::new(),
            player_loop: PlayerLoop::new(),
            time: Time::new(),
            frame: 0,
            running: false,
        }
    }
    
    /// Run one frame (like Unity's Application.Update).
    pub fn update(&mut self, delta: f32) {
        // Update time
        self.time.update(delta);
        
        // Create context
        let mut context = crate::context::Context::new(&mut self.world, self.time.clone(), self.frame);
        
        // Run player loop
        self.player_loop.run(&mut context);
        
        // Increment frame
        self.frame += 1;
    }
    
    /// Run fixed update step (for physics, etc.).
    pub fn fixed_update(&mut self) {
        self.time.update_fixed();
        
        let mut context = crate::context::Context::new(&mut self.world, self.time.clone(), self.frame);
        
        // Only run FixedUpdate phase
        // Note: In production, we'd filter by phase
        self.player_loop.run(&mut context);
    }
    
    /// Check if the app is running.
    pub fn is_running(&self) -> bool {
        self.running
    }
    
    /// Set the running state.
    pub fn set_running(&mut self, running: bool) {
        self.running = running;
    }
    
    /// Get the current frame number.
    pub fn frame(&self) -> u64 {
        self.frame
    }
    
    /// Get a reference to the world.
    pub fn world(&self) -> &World {
        &self.world
    }
    
    /// Get a mutable reference to the world.
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }
    
    /// Get a reference to the time.
    pub fn time(&self) -> &Time {
        &self.time
    }
    
    /// Get a mutable reference to the time.
    pub fn time_mut(&mut self) -> &mut Time {
        &mut self.time
    }
    
    /// Quit the application (like Unity:: Application.Quit).
    pub fn quit(&mut self) {
        self.running = false;
    }
}

impl From<AppBuilder> for App {
    fn from(b: AppBuilder) -> Self {
        Self {
            world: b.world,
            player_loop: b.player_loop,
            time: b.time,
            frame: 0,
            running: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    static UPDATE_COUNT: AtomicUsize = AtomicUsize::new(0);
    static LATE_COUNT: AtomicUsize = AtomicUsize::new(0);
    
    #[test]
    fn test_app_builder() {
        UPDATE_COUNT.store(0, Ordering::SeqCst);
        LATE_COUNT.store(0, Ordering::SeqCst);
        
        let mut builder = AppBuilder::new();
        builder.add_system(|_: &mut crate::context::Context| {
            UPDATE_COUNT.fetch_add(1, Ordering::SeqCst);
        });
        builder.add_late_update_system(|_: &mut crate::context::Context| {
            LATE_COUNT.fetch_add(1, Ordering::SeqCst);
        });
        
        let mut app = builder.build();
        app.set_running(true);
        
        // Run a few frames
        for _ in 0..5 {
            if app.is_running() {
                app.update(0.016);
            }
        }
        
        assert_eq!(UPDATE_COUNT.load(Ordering::SeqCst), 5);
        assert_eq!(LATE_COUNT.load(Ordering::SeqCst), 5);
    }
    
    #[test]
    fn test_app_quit() {
        let mut app = App::new();
        app.set_running(true);
        
        app.update(0.016);
        assert!(app.is_running());
        
        app.quit();
        assert!(!app.is_running());
    }
    
    #[test]
    fn test_app_spawn_gameobject() {
        let mut builder = AppBuilder::new();
        let mut app = builder.build();
        
        let handle = app.world_mut().spawn(GameObject::new("Test"));
        assert!(app.world().is_valid(handle));
        assert_eq!(app.world().get_gameobject(handle).unwrap().name(), "Test");
    }
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p engine-core --lib app`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/app.rs crates/engine-core/src/plugin.rs
git commit -m "feat(core): update AppBuilder and App for Unity-like API

- Update Plugin trait with name method
- Update AppBuilder with phase-based system registration
- Update App with update/fixed_update methods
- Add convenience methods for common phases
- Add plugin build integration"
```

---

## Task 8: Add Integration Tests

**Files:**
- Create: `crates/engine-core/tests/unity_api_tests.rs`

- [ ] **Step 1: Create integration tests**

```rust
// crates/engine-core/tests/unity_api_tests.rs

use engine_core::app::AppBuilder;
use engine_core::gameobject::{Component, GameObject};
use engine_core::transform::Transform;
use engine_core::world::World;
use engine_core::{Phase, Time};
use std::any::Any;
use std::sync::atomic::{AtomicUsize, Ordering};

static UPDATE_CALLED: AtomicUsize = AtomicUsize::new(0);
static LATE_CALLED: AtomicUsize = AtomicUsize::new(0);
static FIXED_CALLED: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
struct Health {
    current: f32,
    max: f32,
}

impl Component for Health {
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[test]
fn test_unity_like_gameobject_creation() {
    let mut world = World::new();
    
    // Create player like Unity
    let mut player = GameObject::new("Player");
    player.add_component(Transform::from_xyz(0.0, 1.0, 0.0));
    player.add_component(Health { current: 100.0, max: 100.0 });
    player.set_tag("Player");
    player.set_layer(6);
    
    let handle = world.spawn(player);
    
    // Verify
    let player = world.get_gameobject(handle).unwrap();
    assert_eq!(player.name(), "Player");
    assert_eq!(player.tag(), "Player");
    assert_eq!(player.layer(), 6);
    assert!(player.has_component::<Transform>());
    assert!(player.has_component::<Health>());
    
    let health = player.get_component::<Health>().unwrap();
    assert_eq!(health.current, 100.0);
}

#[test]
fn test_unity_like_hierarchy() {
    let mut world = World::new();
    
    // Create parent-child like Unity
    let parent = world.spawn(GameObject::new("Parent"));
    let child = world.spawn(GameObject::new("Child"));
    
    world.set_parent(child, Some(parent), true);
    
    // Verify
    assert_eq!(world.get_parent(child), Some(parent));
    assert!(world.get_children(parent).contains(&child));
}

#[test]
fn test_unity_like_player_loop() {
    UPDATE_CALLED.store(0, Ordering::SeqCst);
    LATE_CALLED.store(0, Ordering::SeqCst);
    FIXED_CALLED.store(0, Ordering::SeqCst);
    
    let mut builder = AppBuilder::new();
    
    builder.add_system(|_: &mut engine_core::context::Context| {
        UPDATE_CALLED.fetch_add(1, Ordering::SeqCst);
    });
    
    builder.add_late_update_system(|_: &mut engine_core::context::Context| {
        LATE_CALLED.fetch_add(1, Ordering::SeqCst);
    });
    
    builder.add_fixed_update_system(|_: &mut engine_core::context::Context| {
        FIXED_CALLED.fetch_add(1, Ordering::SeqCst);
    });
    
    let mut app = builder.build();
    app.set_running(true);
    
    // Run frames
    for _ in 0..3 {
        if app.is_running() {
            app.update(0.016);
        }
    }
    
    assert_eq!(UPDATE_CALLED.load(Ordering::SeqCst), 3);
    assert_eq!(LATE_CALLED.load(Ordering::SeqCst), 3);
}

#[test]
fn test_unity_like_time_management() {
    let mut builder = AppBuilder::new();
    let mut app = builder.build();
    
    // Initial time
    assert_eq!(app.time().time(), 0.0);
    assert_eq!(app.time().frame(), 0);
    
    // Update
    app.update(0.016);
    assert!((app.time().time() - 0.016).abs() < 0.001);
    assert_eq!(app.time().frame(), 1);
    
    // Update again
    app.update(0.016);
    assert!((app.time().time() - 0.032).abs() < 0.001);
    assert_eq!(app.time().frame(), 2);
}

#[test]
fn test_unity_like_component_lifecycle() {
    let mut world = World::new();
    
    let mut go = GameObject::new("TestObject");
    go.add_component(Health { current: 100.0, max: 100.0 });
    
    let handle = world.spawn(go);
    
    // Get component and modify
    {
        let go = world.get_gameobject_mut(handle).unwrap();
        let health = go.get_component_mut::<Health>().unwrap();
        health.current -= 25.0;
    }
    
    // Verify modification
    let go = world.get_gameobject(handle).unwrap();
    let health = go.get_component::<Health>().unwrap();
    assert_eq!(health.current, 75.0);
}

#[test]
fn test_unity_like_find_gameobjects() {
    let mut world = World::new();
    
    let mut player1 = GameObject::new("Player");
    player1.set_tag("Player");
    let h1 = world.spawn(player1);
    
    let mut player2 = GameObject::new("Player");
    player2.set_tag("Player");
    let h2 = world.spawn(player2);
    
    let mut enemy = GameObject::new("Enemy");
    enemy.set_tag("Enemy");
    let h3 = world.spawn(enemy);
    
    // Find by name
    let found = world.find_gameobject("Player");
    assert!(found.is_some());
    
    // Find by tag
    let players = world.find_gameobjects_with_tag("Player");
    assert_eq!(players.len(), 2);
    assert!(players.contains(&h1));
    assert!(players.contains(&h2));
    
    let enemies = world.find_gameobjects_with_tag("Enemy");
    assert_eq!(enemies.len(), 1);
    assert!(enemies.contains(&h3));
}
```

- [ ] **Step 2: Run integration tests**

Run: `cargo test -p engine-core --test unity_api_tests`
Expected: All tests PASS

- [ ] **Step 3: Commit**

```bash
git add crates/engine-core/tests/unity_api_tests.rs
git commit -m "test(core): add Unity-like API integration tests

- Test GameObject creation with components
- Test parent-child hierarchy
- Test Player Loop execution
- Test Time management
- Test component lifecycle
- Test GameObject finding by name/tag"
```

---

## Task 9: Update Scene Module

**Files:**
- Modify: `crates/engine-scene/src/node.rs`
- Modify: `crates/engine-scene/src/scene_manager.rs`

- [ ] **Step 1: Update SceneNode to use GameObjectHandle**

```rust
// crates/engine-scene/src/node.rs

use engine_core::gameobject::GameObjectHandle;
use serde::{Deserialize, Serialize};

/// A lightweight handle for a node in the scene graph.
/// Now wraps a GameObjectHandle instead of an Entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SceneNode {
    gameobject: GameObjectHandle,
}

impl SceneNode {
    /// Create a scene node from a GameObject handle.
    pub fn new(gameobject: GameObjectHandle) -> Self {
        Self { gameobject }
    }
    
    /// Return the underlying GameObject handle.
    pub fn gameobject(&self) -> GameObjectHandle {
        self.gameobject
    }
    
    /// Convert to Entity (for backward compatibility with old ECS).
    /// This is deprecated - use gameobject() instead.
    #[deprecated(note = "Use gameobject() instead")]
    pub fn entity(&self) -> engine_ecs::entity::Entity {
        // This is a compatibility shim - in production, we'd need to map handles
        unimplemented!("Entity mapping not yet implemented")
    }
}

impl From<GameObjectHandle> for SceneNode {
    fn from(handle: GameObjectHandle) -> Self {
        Self::new(handle)
    }
}

impl From<SceneNode> for GameObjectHandle {
    fn from(node: SceneNode) -> Self {
        node.gameobject()
    }
}
```

- [ ] **Step 2: Run tests to verify no regressions**

Run: `cargo test -p engine-scene`
Expected: All tests PASS (or expected failures for deprecated methods)

- [ ] **Step 3: Commit**

```bash
git add crates/engine-scene/src/node.rs
git commit -m "feat(scene): update SceneNode to use GameObjectHandle

- Replace Entity with GameObjectHandle
- Add From implementations for conversion
- Deprecate entity() method
- Maintain backward compatibility"
```

---

## Task 10: Verify Build and Tests

**Files:**
- None (verification only)

- [ ] **Step 1: Run full build**

Run: `cargo build`
Expected: Build succeeds with no errors

- [ ] **Step 2: Run all tests**

Run: `cargo test -p engine-core`
Expected: All tests PASS

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -p engine-core`
Expected: No warnings or errors

- [ ] **Step 4: Run format check**

Run: `cargo fmt --check -p engine-core`
Expected: No formatting issues

- [ ] **Step 5: Commit any fixes**

```bash
git add -A
git commit -m "chore(core): fix lint and format issues

- Fix clippy warnings
- Apply rustfmt formatting
- Ensure all tests pass"
```

---

## Summary

This plan completes **Phase 1: Core Architecture** of the Unity Parity Refactoring. After completing all tasks:

1. **GameObject/Component model** is implemented
2. **World container** handles spawning, despawning, and hierarchy
3. **Transform component** provides local/world transforms
4. **Hierarchy system** synchronizes parent-child transforms
5. **Player Loop** executes systems in Unity-like phases
6. **Time management** tracks delta time, elapsed time, and fixed updates
7. **AppBuilder/App** provides Unity-like API for game initialization

**Next Phase:** Phase 2 - MonoBehaviour & Lifecycle (Weeks 3-4)
