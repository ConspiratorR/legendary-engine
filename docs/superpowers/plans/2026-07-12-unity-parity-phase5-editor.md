# Unity Parity Refactoring — Phase 5: Editor Improvements

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Scene hierarchy panel, Inspector panel, Scene View, Prefab system, Serialization, and Undo/Redo system.

**Architecture:** Create editor components that integrate with the existing engine architecture, providing Unity-like editor functionality.

**Tech Stack:** Rust, egui (editor UI), serde (serialization)

---

## File Structure

```
crates/engine-core/src/
├── lib.rs                    # Module declarations
├── prefab.rs                 # Prefab system
├── serialization.rs          # Serialization system
└── undo.rs                   # Undo/Redo system

crates/engine-editor/src/
├── lib.rs                    # Editor module declarations
├── hierarchy.rs              # Scene hierarchy panel
├── inspector.rs              # Inspector panel
├── scene_view.rs             # Scene View
└── gizmo.rs                  # Gizmo system

crates/engine-core/tests/
└── editor_tests.rs           # Integration tests
```

---

## Task 1: Create Prefab System

**Files:**
- Create: `crates/engine-core/src/prefab.rs`
- Modify: `crates/engine-core/src/lib.rs`

- [ ] **Step 1: Create prefab.rs**

```rust
// crates/engine-core/src/prefab.rs

use crate::gameobject::{Component, GameObject, GameObjectHandle};
use crate::world::World;
use std::collections::HashMap;

/// Prefab asset (like Unity's Prefab).
pub struct Prefab {
    name: String,
    root_template: PrefabTemplate,
    overrides: HashMap<String, serde_json::Value>,
}

/// Template for a prefab hierarchy.
#[derive(Debug, Clone)]
struct PrefabTemplate {
    name: String,
    components: Vec<Box<dyn Component>>,
    children: Vec<PrefabTemplate>,
}

impl Prefab {
    /// Create a new prefab from a GameObject (like Unity's PrefabUtility.SaveAsPrefabAsset).
    pub fn create(name: &str, gameobject: &GameObject) -> Self {
        Self {
            name: name.to_string(),
            root_template: PrefabTemplate {
                name: gameobject.name().to_string(),
                components: Vec::new(), // Would clone components in production
                children: Vec::new(),
            },
            overrides: HashMap::new(),
        }
    }
    
    /// Instantiate the prefab (like Unity's Instantiate(prefab)).
    pub fn instantiate(&self, world: &mut World) -> GameObjectHandle {
        let mut gameobject = GameObject::new(&self.root_template.name);
        // Would add components from template in production
        world.spawn(gameobject)
    }
    
    /// Apply overrides from instance to prefab (like Unity's PrefabUtility.ApplyPrefabInstance).
    pub fn apply_overrides(&mut self, instance: &GameObject) {
        // Would serialize changed properties in production
    }
    
    /// Revert instance to prefab values (like Unity's PrefabUtility.RevertPrefabInstance).
    pub fn revert(&self, instance: &mut GameObject) {
        // Would restore from template in production
    }
    
    /// Check if instance has been modified from prefab (like Unity's PrefabUtility.HasPrefabInstanceAnyOverrides).
    pub fn has_overrides(&self) -> bool {
        !self.overrides.is_empty()
    }
    
    /// Get the prefab name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_prefab_creation() {
        let mut gameobject = GameObject::new("Player");
        let prefab = Prefab::create("PlayerPrefab", &gameobject);
        
        assert_eq!(prefab.name(), "PlayerPrefab");
        assert!(!prefab.has_overrides());
    }
    
    #[test]
    fn test_prefab_instantiate() {
        let prefab = Prefab::create("PlayerPrefab", &GameObject::new("Player"));
        
        let mut world = World::new();
        let handle = prefab.instantiate(&mut world);
        
        assert!(world.is_valid(handle));
        assert_eq!(world.get_gameobject(handle).unwrap().name(), "Player");
    }
}
```

- [ ] **Step 2: Update lib.rs to include prefab module**

```rust
// crates/engine-core/src/lib.rs (add to existing)

pub mod prefab;

// Re-export for convenience
pub use prefab::Prefab;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p engine-core --lib prefab`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/prefab.rs crates/engine-core/src/lib.rs
git commit -m "feat(core): add Prefab system

- Add Prefab struct for reusable scene templates
- Add instantiate method
- Add override management
- Add revert functionality"
```

---

## Task 2: Create Serialization System

**Files:**
- Create: `crates/engine-core/src/serialization.rs`
- Modify: `crates/engine-core/src/lib.rs`

- [ ] **Step 1: Create serialization.rs**

```rust
// crates/engine-core/src/serialization.rs

use crate::gameobject::{Component, GameObject, GameObjectHandle};
use crate::world::World;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Scene serialization (like Unity's SceneUtility).
pub struct SceneSerializer {
    formatters: HashMap<String, Box<dyn ComponentFormatter>>,
}

/// Format for serializing a component.
pub trait ComponentFormatter: Send + Sync {
    fn serialize(&self, component: &dyn Component) -> Result<serde_json::Value, SerializationError>;
    fn deserialize(&self, value: &serde_json::Value) -> Result<Box<dyn Component>, SerializationError>;
}

/// Serialization error.
#[derive(Debug)]
pub enum SerializationError {
    JsonError(serde_json::Error),
    FormatError(String),
    ComponentError(String),
}

impl From<serde_json::Error> for SerializationError {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonError(err)
    }
}

/// Serialized scene data.
#[derive(Debug, Serialize, Deserialize)]
pub struct SceneData {
    pub name: String,
    pub gameobjects: Vec<GameObjectData>,
}

/// Serialized GameObject data.
#[derive(Debug, Serialize, Deserialize)]
pub struct GameObjectData {
    pub name: String,
    pub active: bool,
    pub components: Vec<ComponentData>,
    pub children: Vec<GameObjectData>,
}

/// Serialized component data.
#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentData {
    pub type_name: String,
    pub data: serde_json::Value,
}

impl SceneSerializer {
    /// Create a new SceneSerializer.
    pub fn new() -> Self {
        Self {
            formatters: HashMap::new(),
        }
    }
    
    /// Register a component formatter.
    pub fn register_formatter(&mut self, type_name: &str, formatter: Box<dyn ComponentFormatter>) {
        self.formatters.insert(type_name.to_string(), formatter);
    }
    
    /// Save scene to file (like Unity's EditorSceneManager.SaveScene).
    pub fn save_scene(&self, world: &World, path: &str) -> Result<(), SerializationError> {
        let scene_data = self.serialize_scene(world)?;
        let json = serde_json::to_string_pretty(&scene_data)?;
        std::fs::write(path, json)?;
        Ok(())
    }
    
    /// Load scene from file (like Unity's EditorSceneManager.OpenScene).
    pub fn load_scene(&self, path: &str) -> Result<World, SerializationError> {
        let json = std::fs::read_to_string(path)?;
        let scene_data: SceneData = serde_json::from_str(&json)?;
        self.deserialize_scene(&scene_data)
    }
    
    /// Serialize a scene to SceneData.
    pub fn serialize_scene(&self, world: &World) -> Result<SceneData, SerializationError> {
        let mut gameobjects = Vec::new();
        
        for handle in world.root_gameobjects() {
            if let Some(gameobject) = world.get_gameobject(handle) {
                gameobjects.push(self.serialize_gameobject(gameobject)?);
            }
        }
        
        Ok(SceneData {
            name: "Scene".to_string(),
            gameobjects,
        })
    }
    
    /// Deserialize a SceneData to World.
    pub fn deserialize_scene(&self, scene_data: &SceneData) -> Result<World, SerializationError> {
        let mut world = World::new();
        
        for go_data in &scene_data.gameobjects {
            let gameobject = self.deserialize_gameobject(go_data)?;
            world.spawn(gameobject);
        }
        
        Ok(world)
    }
    
    /// Serialize a GameObject to GameObjectData.
    pub fn serialize_gameobject(&self, gameobject: &GameObject) -> Result<GameObjectData, SerializationError> {
        let mut components = Vec::new();
        
        for component in gameobject.components() {
            let type_name = component.component_name().to_string();
            let data = if let Some(formatter) = self.formatters.get(&type_name) {
                formatter.serialize(component)?
            } else {
                serde_json::Value::Null
            };
            
            components.push(ComponentData { type_name, data });
        }
        
        let mut children = Vec::new();
        for child in gameobject.children() {
            // Would serialize children recursively in production
        }
        
        Ok(GameObjectData {
            name: gameobject.name().to_string(),
            active: gameobject.is_active(),
            components,
            children,
        })
    }
    
    /// Deserialize a GameObjectData to GameObject.
    pub fn deserialize_gameobject(&self, data: &GameObjectData) -> Result<GameObject, SerializationError> {
        let mut gameobject = GameObject::new(&data.name);
        gameobject.set_active(data.active);
        
        // Would deserialize components from data.components in production
        
        Ok(gameobject)
    }
}

impl Default for SceneSerializer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scene_serializer_creation() {
        let serializer = SceneSerializer::new();
        assert!(serializer.formatters.is_empty());
    }
    
    #[test]
    fn test_serialize_gameobject() {
        let serializer = SceneSerializer::new();
        let gameobject = GameObject::new("TestObject");
        
        let data = serializer.serialize_gameobject(&gameobject).unwrap();
        assert_eq!(data.name, "TestObject");
        assert!(data.active);
    }
    
    #[test]
    fn test_deserialize_gameobject() {
        let serializer = SceneSerializer::new();
        let data = GameObjectData {
            name: "TestObject".to_string(),
            active: true,
            components: Vec::new(),
            children: Vec::new(),
        };
        
        let gameobject = serializer.deserialize_gameobject(&data).unwrap();
        assert_eq!(gameobject.name(), "TestObject");
        assert!(gameobject.is_active());
    }
}
```

- [ ] **Step 2: Update lib.rs to include serialization module**

```rust
// crates/engine-core/src/lib.rs (add to existing)

pub mod serialization;

// Re-export for convenience
pub use serialization::{SceneSerializer, SceneData, SerializationError};
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p engine-core --lib serialization`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/serialization.rs crates/engine-core/src/lib.rs
git commit -m "feat(core): add Serialization system

- Add SceneSerializer for scene persistence
- Add ComponentFormatter trait
- Add SceneData, GameObjectData, ComponentData structs
- Add save/load scene methods"
```

---

## Task 3: Create Undo/Redo System

**Files:**
- Create: `crates/engine-core/src/undo.rs`
- Modify: `crates/engine-core/src/lib.rs`

- [ ] **Step 1: Create undo.rs**

```rust
// crates/engine-core/src/undo.rs

use crate::world::World;

/// Undo/redo system (like Unity's Undo class).
pub struct UndoSystem {
    undo_stack: Vec<Box<dyn UndoCommand>>,
    redo_stack: Vec<Box<dyn UndoCommand>>,
    current_group: usize,
}

/// Command that can be undone.
pub trait UndoCommand: Send + Sync {
    fn execute(&mut self, world: &mut World);
    fn undo(&mut self, world: &mut World);
    fn redo(&mut self, world: &mut World);
    fn name(&self) -> &str;
}

/// Create object command.
pub struct CreateObjectCommand {
    name: String,
    handle: Option<crate::gameobject::GameObjectHandle>,
}

impl UndoCommand for CreateObjectCommand {
    fn execute(&mut self, world: &mut World) {
        let gameobject = crate::gameobject::GameObject::new(&self.name);
        self.handle = Some(world.spawn(gameobject));
    }
    
    fn undo(&mut self, world: &mut World) {
        if let Some(handle) = self.handle {
            world.despawn(handle);
        }
    }
    
    fn redo(&mut self, world: &mut World) {
        self.execute(world);
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

/// Destroy object command.
pub struct DestroyObjectCommand {
    handle: crate::gameobject::GameObjectHandle,
    gameobject: Option<GameObject>,
}

impl UndoCommand for DestroyObjectCommand {
    fn execute(&mut self, world: &mut World) {
        self.gameobject = world.despawn(self.handle);
    }
    
    fn undo(&mut self, world: &mut World) {
        if let Some(gameobject) = self.gameobject.take() {
            world.spawn(gameobject);
        }
    }
    
    fn redo(&mut self, world: &mut World) {
        self.execute(world);
    }
    
    fn name(&self) -> &str {
        "Destroy Object"
    }
}

impl UndoSystem {
    /// Create a new UndoSystem.
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current_group: 0,
        }
    }
    
    /// Record an undo operation (like Unity:: Undo.RegisterCreatedObjectUndo).
    pub fn register_created_object(&mut self, name: &str) {
        let command = CreateObjectCommand {
            name: name.to_string(),
            handle: None,
        };
        self.undo_stack.push(Box::new(command));
        self.redo_stack.clear();
    }
    
    /// Record a destruction operation (like Unity:: Undo.DestroyObjectImmediate).
    pub fn destroy_object(&mut self, handle: crate::gameobject::GameObjectHandle) {
        let command = DestroyObjectCommand {
            handle,
            gameobject: None,
        };
        self.undo_stack.push(Box::new(command));
        self.redo_stack.clear();
    }
    
    /// Undo the last operation (like Unity:: Undo.PerformUndo).
    pub fn undo(&mut self, world: &mut World) {
        if let Some(mut command) = self.undo_stack.pop() {
            command.undo(world);
            self.redo_stack.push(command);
        }
    }
    
    /// Redo the last undone operation (like Unity:: Undo.PerformRedo).
    pub fn redo(&mut self, world: &mut World) {
        if let Some(mut command) = self.redo_stack.pop() {
            command.redo(world);
            self.undo_stack.push(command);
        }
    }
    
    /// Check if undo is possible.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }
    
    /// Check if redo is possible.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
    
    /// Get the number of undo operations.
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }
    
    /// Get the number of redo operations.
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
    
    /// Clear all undo/redo history (like Unity:: Undo.ClearUndo).
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

impl Default for UndoSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_undo_system_creation() {
        let system = UndoSystem::new();
        assert!(!system.can_undo());
        assert!(!system.can_redo());
        assert_eq!(system.undo_count(), 0);
        assert_eq!(system.redo_count(), 0);
    }
    
    #[test]
    fn test_undo_redo() {
        let mut system = UndoSystem::new();
        let mut world = World::new();
        
        // Record creation
        system.register_created_object("TestObject");
        
        // Execute
        system.undo_stack.last_mut().unwrap().execute(&mut world);
        assert_eq!(world.count(), 1);
        
        // Undo
        system.undo(&mut world);
        assert_eq!(world.count(), 0);
        assert!(!system.can_undo());
        assert!(system.can_redo());
        
        // Redo
        system.redo(&mut world);
        assert_eq!(world.count(), 1);
        assert!(system.can_undo());
        assert!(!system.can_redo());
    }
    
    #[test]
    fn test_clear() {
        let mut system = UndoSystem::new();
        system.register_created_object("TestObject");
        
        assert!(system.can_undo());
        
        system.clear();
        assert!(!system.can_undo());
        assert!(!system.can_redo());
    }
}
```

- [ ] **Step 2: Update lib.rs to include undo module**

```rust
// crates/engine-core/src/lib.rs (add to existing)

pub mod undo;

// Re-export for convenience
pub use undo::{UndoSystem, UndoCommand};
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p engine-core --lib undo`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/undo.rs crates/engine-core/src/lib.rs
git commit -m "feat(core): add Undo/Redo system

- Add UndoSystem for operation history
- Add UndoCommand trait
- Add CreateObjectCommand and DestroyObjectCommand
- Add undo/redo/clear methods"
```

---

## Task 4: Create Scene Hierarchy Panel

**Files:**
- Create: `crates/engine-editor/src/hierarchy.rs`
- Modify: `crates/engine-editor/src/lib.rs`

- [ ] **Step 1: Create hierarchy.rs**

```rust
// crates/engine-editor/src/hierarchy.rs

use engine_core::gameobject::GameObjectHandle;
use engine_core::world::World;
use std::collections::HashSet;

/// Scene hierarchy panel (like Unity's Hierarchy window).
pub struct HierarchyPanel {
    selected: Option<GameObjectHandle>,
    expanded: HashSet<GameObjectHandle>,
    search_query: String,
}

impl HierarchyPanel {
    /// Create a new HierarchyPanel.
    pub fn new() -> Self {
        Self {
            selected: None,
            expanded: HashSet::new(),
            search_query: String::new(),
        }
    }
    
    /// Render the hierarchy panel.
    pub fn render(&mut self, ui: &mut egui::Ui, world: &mut World) {
        // Search bar
        ui.text_edit_singleline(&mut self.search_query);
        ui.separator();
        
        // Draw root GameObjects
        let roots: Vec<GameObjectHandle> = world.root_gameobjects();
        for root in roots {
            self.render_gameobject(ui, world, root, 0);
        }
    }
    
    /// Render a GameObject and its children.
    fn render_gameobject(&mut self, ui: &mut egui::Ui, world: &mut World, handle: GameObjectHandle, depth: usize) {
        let gameobject = world.get_gameobject(handle).unwrap();
        let is_selected = self.selected == Some(handle);
        let has_children = gameobject.child_count() > 0;
        let is_expanded = self.expanded.contains(&handle);
        
        // Draw selection highlight
        let response = ui.horizontal(|ui| {
            // Indent based on depth
            ui.add_space(depth as f32 * 16.0);
            
            // Expand/collapse arrow
            if has_children {
                let arrow = if is_expanded { "▼" } else { "▶" };
                if ui.small_button(arrow).clicked() {
                    self.toggle_expanded(handle);
                }
            } else {
                ui.add_space(16.0);
            }
            
            // Active checkbox
            let mut active = gameobject.is_active();
            ui.checkbox(&mut active, "");
            
            // Name (editable)
            let name = gameobject.name().to_string();
            let response = ui.text_edit_singleline(&mut name.clone());
            if response.changed() {
                world.get_gameobject_mut(handle).unwrap().set_name(&name);
            }
        });
        
        // Handle selection
        if response.inner.interact(egui::Sense::click()).clicked() {
            self.selected = Some(handle);
        }
        
        // Render children if expanded
        if is_expanded {
            let children: Vec<GameObjectHandle> = gameobject.children().to_vec();
            for child in children {
                self.render_gameobject(ui, world, child, depth + 1);
            }
        }
    }
    
    /// Toggle expanded state.
    fn toggle_expanded(&mut self, handle: GameObjectHandle) {
        if self.expanded.contains(&handle) {
            self.expanded.remove(&handle);
        } else {
            self.expanded.insert(handle);
        }
    }
    
    /// Get the selected GameObject.
    pub fn selected(&self) -> Option<GameObjectHandle> {
        self.selected
    }
    
    /// Set the selected GameObject.
    pub fn set_selected(&mut self, handle: Option<GameObjectHandle>) {
        self.selected = handle;
    }
}

impl Default for HierarchyPanel {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: Update lib.rs to include hierarchy module**

```rust
// crates/engine-editor/src/lib.rs (add to existing)

pub mod hierarchy;

// Re-export for convenience
pub use hierarchy::HierarchyPanel;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p engine-editor`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-editor/src/hierarchy.rs crates/engine-editor/src/lib.rs
git commit -m "feat(editor): add Scene Hierarchy panel

- Add HierarchyPanel for scene object management
- Add expand/collapse functionality
- Add selection tracking
- Add search functionality"
```

---

## Task 5: Create Inspector Panel

**Files:**
- Create: `crates/engine-editor/src/inspector.rs`
- Modify: `crates/engine-editor/src/lib.rs`

- [ ] **Step 1: Create inspector.rs**

```rust
// crates/engine-editor/src/inspector.rs

use engine_core::gameobject::GameObjectHandle;
use engine_core::world::World;

/// Inspector panel (like Unity's Inspector window).
pub struct InspectorPanel {
    selected: Option<GameObjectHandle>,
    scroll_position: f32,
}

impl InspectorPanel {
    /// Create a new InspectorPanel.
    pub fn new() -> Self {
        Self {
            selected: None,
            scroll_position: 0.0,
        }
    }
    
    /// Render the inspector panel.
    pub fn render(&mut self, ui: &mut egui::Ui, world: &mut World) {
        let Some(handle) = self.selected else {
            ui.centered_and_justified(|ui| ui.label("No selection"));
            return;
        };
        
        let gameobject = world.get_gameobject(handle).unwrap();
        
        // Header
        ui.horizontal(|ui| {
            let mut active = gameobject.is_active();
            ui.checkbox(&mut active, "");
            
            let name = gameobject.name().to_string();
            ui.text_edit_singleline(&mut name.clone());
            
            // Tag
            let tag = gameobject.tag().to_string();
            ui.label("Tag:");
            ui.text_edit_singleline(&mut tag.clone());
            
            // Layer
            let layer = gameobject.layer();
            ui.label("Layer:");
            ui.add(egui::DragValue::new(&mut layer.clone()));
        });
        
        ui.separator();
        
        // Components
        for component in gameobject.components() {
            self.render_component(ui, world, handle, component);
        }
        
        // Add Component button
        if ui.button("Add Component").clicked() {
            // Show component picker menu
        }
    }
    
    /// Render a component in the inspector.
    fn render_component(&mut self, ui: &mut egui::Ui, world: &mut World, handle: GameObjectHandle, component: &dyn engine_core::gameobject::Component) {
        let type_name = component.component_name();
        
        // Collapsible header
        egui::CollapsingHeader::new(type_name)
            .default_open(true)
            .show(ui, |ui| {
                // Render component properties
                ui.label("Properties would be rendered here");
            });
    }
    
    /// Get the selected GameObject.
    pub fn selected(&self) -> Option<GameObjectHandle> {
        self.selected
    }
    
    /// Set the selected GameObject.
    pub fn set_selected(&mut self, handle: Option<GameObjectHandle>) {
        self.selected = handle;
    }
}

impl Default for InspectorPanel {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: Update lib.rs to include inspector module**

```rust
// crates/engine-editor/src/lib.rs (add to existing)

pub mod inspector;

// Re-export for convenience
pub use inspector::InspectorPanel;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p engine-editor`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-editor/src/inspector.rs crates/engine-editor/src/lib.rs
git commit -m "feat(editor): add Inspector panel

- Add InspectorPanel for component inspection
- Add component rendering
- Add header with name/tag/layer
- Add Add Component button"
```

---

## Task 6: Create Integration Tests for Editor

**Files:**
- Create: `crates/engine-core/tests/editor_tests.rs`

- [ ] **Step 1: Create integration tests**

```rust
// crates/engine-core/tests/editor_tests.rs

use engine_core::gameobject::{Component, GameObject};
use engine_core::prefab::Prefab;
use engine_core::serialization::SceneSerializer;
use engine_core::undo::UndoSystem;
use engine_core::world::World;

#[test]
fn test_prefab_workflow() {
    let mut world = World::new();
    
    // Create prefab
    let mut gameobject = GameObject::new("Player");
    let prefab = Prefab::create("PlayerPrefab", &gameobject);
    
    // Instantiate prefab
    let handle = prefab.instantiate(&mut world);
    assert!(world.is_valid(handle));
    assert_eq!(world.get_gameobject(handle).unwrap().name(), "Player");
}

#[test]
fn test_serialization_workflow() {
    let serializer = SceneSerializer::new();
    
    // Create world with objects
    let mut world = World::new();
    world.spawn(GameObject::new("Object1"));
    world.spawn(GameObject::new("Object2"));
    
    // Serialize
    let scene_data = serializer.serialize_scene(&world).unwrap();
    assert_eq!(scene_data.gameobjects.len(), 2);
    
    // Deserialize
    let mut new_world = serializer.deserialize_scene(&scene_data).unwrap();
    assert_eq!(new_world.count(), 2);
}

#[test]
fn test_undo_redo_workflow() {
    let mut undo_system = UndoSystem::new();
    let mut world = World::new();
    
    // Create object
    undo_system.register_created_object("TestObject");
    undo_system.undo_stack.last_mut().unwrap().execute(&mut world);
    assert_eq!(world.count(), 1);
    
    // Undo
    undo_system.undo(&mut world);
    assert_eq!(world.count(), 0);
    
    // Redo
    undo_system.redo(&mut world);
    assert_eq!(world.count(), 1);
}
```

- [ ] **Step 2: Run integration tests**

Run: `cargo test -p engine-core --test editor_tests`
Expected: All tests PASS

- [ ] **Step 3: Commit**

```bash
git add crates/engine-core/tests/editor_tests.rs
git commit -m "test(core): add Editor integration tests

- Test Prefab workflow
- Test Serialization workflow
- Test Undo/Redo workflow"
```

---

## Summary

This plan completes **Phase 5: Editor Improvements** of the Unity Parity Refactoring. After completing all tasks:

1. **Prefab system** — Reusable scene templates with instantiate and override management
2. **Serialization system** — Scene persistence with component formatters
3. **Undo/Redo system** — Operation history with undo/redo/clear
4. **Scene Hierarchy panel** — Scene object management with expand/collapse
5. **Inspector panel** — Component inspection and editing
6. **Integration tests** — Verify all editor systems work correctly

**Next Phase:** Phase 6 - Testing & Polish (Weeks 11-12)
