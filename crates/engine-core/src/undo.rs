//! Undo/redo command system for the engine core.
//!
//! Provides a [`UndoCommand`] trait and [`UndoSystem`] that maintains an
//! undo/redo stack with configurable history depth. Each engine action that
//! should be reversible implements [`UndoCommand`] with paired `execute`/`undo`
//! methods.

use crate::gameobject::{GameObject, GameObjectHandle};
use crate::world::World;
use std::collections::VecDeque;

/// Trait for undoable engine commands.
pub trait UndoCommand: std::fmt::Debug + Send {
    /// Executes the command (first time or redo).
    fn execute(&mut self, world: &mut World) -> GameObjectHandle;
    /// Reverses the command.
    fn undo(&mut self, world: &mut World);
    /// Re-applies the command after an undo.
    fn redo(&mut self, world: &mut World);
    /// Human-readable description for the undo/redo menu.
    fn description(&self) -> String;
}

/// Manages an undo/redo stack of [`UndoCommand`] instances.
pub struct UndoSystem {
    undo_stack: VecDeque<Box<dyn UndoCommand>>,
    redo_stack: VecDeque<Box<dyn UndoCommand>>,
    max_history: usize,
}

impl std::fmt::Debug for UndoSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UndoSystem")
            .field("undo_count", &self.undo_stack.len())
            .field("redo_count", &self.redo_stack.len())
            .finish()
    }
}

impl UndoSystem {
    /// Creates a new undo system with the given maximum history depth.
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: VecDeque::with_capacity(max_history),
            redo_stack: VecDeque::with_capacity(max_history),
            max_history,
        }
    }

    /// Executes a command, pushing it onto the undo stack and clearing the redo stack.
    pub fn execute(
        &mut self,
        mut command: Box<dyn UndoCommand>,
        world: &mut World,
    ) -> GameObjectHandle {
        let handle = command.execute(world);
        self.undo_stack.push_back(command);
        self.redo_stack.clear();
        while self.undo_stack.len() > self.max_history {
            self.undo_stack.pop_front();
        }
        handle
    }

    /// Undoes the last command.
    pub fn undo(&mut self, world: &mut World) -> Option<()> {
        let mut cmd = self.undo_stack.pop_back()?;
        cmd.undo(world);
        self.redo_stack.push_back(cmd);
        Some(())
    }

    /// Redoes the last undone command.
    pub fn redo(&mut self, world: &mut World) -> Option<()> {
        let mut cmd = self.redo_stack.pop_back()?;
        cmd.redo(world);
        self.undo_stack.push_back(cmd);
        Some(())
    }

    /// Check if there are commands to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if there are commands to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear all undo and redo history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Get the description of the next command to undo.
    pub fn undo_description(&self) -> Option<String> {
        self.undo_stack.back().map(|cmd| cmd.description())
    }

    /// Get the description of the next command to redo.
    pub fn redo_description(&self) -> Option<String> {
        self.redo_stack.back().map(|cmd| cmd.description())
    }
}

impl Default for UndoSystem {
    fn default() -> Self {
        Self::new(100)
    }
}

/// Command to create a new GameObject (undo destroys it, redo recreates it).
#[derive(Debug)]
pub struct CreateObjectCommand {
    name: String,
    tag: String,
    layer: u32,
    active: bool,
    parent: Option<GameObjectHandle>,
    created_handle: Option<GameObjectHandle>,
}

impl CreateObjectCommand {
    /// Create a new command to spawn a GameObject.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tag: "Untagged".to_string(),
            layer: 0,
            active: true,
            parent: None,
            created_handle: None,
        }
    }

    /// Set the tag for the new GameObject.
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tag = tag.to_string();
        self
    }

    /// Set the layer for the new GameObject.
    pub fn with_layer(mut self, layer: u32) -> Self {
        self.layer = layer;
        self
    }

    /// Set the active state for the new GameObject.
    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Set the parent for the new GameObject.
    pub fn with_parent(mut self, parent: GameObjectHandle) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Get the handle of the created GameObject (available after execute).
    pub fn created_handle(&self) -> Option<GameObjectHandle> {
        self.created_handle
    }
}

impl UndoCommand for CreateObjectCommand {
    fn execute(&mut self, world: &mut World) -> GameObjectHandle {
        let mut go = GameObject::new(&self.name);
        go.set_tag(&self.tag);
        go.set_layer(self.layer);
        go.set_active(self.active);

        let handle = world.spawn(go);

        // Set parent if specified
        if let Some(parent) = self.parent {
            world.set_parent(handle, Some(parent));
        }

        self.created_handle = Some(handle);
        handle
    }

    fn undo(&mut self, world: &mut World) {
        if let Some(handle) = self.created_handle {
            world.despawn(handle);
            self.created_handle = None;
        }
    }

    fn redo(&mut self, world: &mut World) {
        self.execute(world);
    }

    fn description(&self) -> String {
        format!("Create '{}'", self.name)
    }
}

/// Command to destroy a GameObject (undo recreates it with its components and children).
#[derive(Debug)]
pub struct DestroyObjectCommand {
    handle: GameObjectHandle,
    name: String,
    parent: Option<GameObjectHandle>,
    children: Vec<GameObjectHandle>,
    /// The parent GameObject captured during execute, with all its components.
    captured_parent: Option<GameObject>,
    /// The child GameObjects captured during execute, with all their components.
    captured_children: Vec<GameObject>,
    recreated_handle: Option<GameObjectHandle>,
}

impl DestroyObjectCommand {
    /// Create a new command to destroy a GameObject.
    /// This captures basic info for display; the full GameObject data is captured during execute.
    pub fn new(world: &World, handle: GameObjectHandle) -> Option<Self> {
        let gameobject = world.get_gameobject(handle)?;

        Some(Self {
            handle,
            name: gameobject.name().to_string(),
            parent: world.get_parent(handle),
            children: world.get_children(handle),
            captured_parent: None,
            captured_children: Vec::new(),
            recreated_handle: None,
        })
    }

    /// Get the handle of the recreated GameObject (available after undo).
    pub fn recreated_handle(&self) -> Option<GameObjectHandle> {
        self.recreated_handle
    }
}

impl UndoCommand for DestroyObjectCommand {
    fn execute(&mut self, world: &mut World) -> GameObjectHandle {
        // Capture children and parent GameObjects with all their components
        self.captured_children.clear();

        // Remove all children first, capturing their data
        for child in self.children.iter().rev() {
            if let Some(child_go) = world.despawn(*child) {
                self.captured_children.push(child_go);
            }
        }
        // Reverse so children are in original order
        self.captured_children.reverse();

        // Remove the parent-child relationship
        world.set_parent(self.handle, None);

        // Destroy the parent GameObject, capturing it
        self.captured_parent = world.despawn(self.handle);

        self.handle
    }

    fn undo(&mut self, world: &mut World) {
        // Recreate the parent GameObject from captured data
        if let Some(mut parent_go) = self.captured_parent.take() {
            // Clear stale parent/children references from captured state
            parent_go.parent = None;
            parent_go.children.clear();

            let new_handle = world.spawn(parent_go);

            // Recreate children from captured data, reattaching to the new parent
            for mut child_go in self.captured_children.drain(..) {
                // Clear stale parent/children references from captured state
                child_go.parent = None;
                child_go.children.clear();

                let new_child_handle = world.spawn(child_go);
                world.set_parent(new_child_handle, Some(new_handle));
            }

            // Reattach to original parent
            if let Some(parent) = self.parent {
                world.set_parent(new_handle, Some(parent));
            }

            self.recreated_handle = Some(new_handle);
            self.handle = new_handle;
        }
    }

    fn redo(&mut self, world: &mut World) {
        self.execute(world);
    }

    fn description(&self) -> String {
        format!("Destroy '{}'", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_system_creation() {
        let system = UndoSystem::new(50);
        assert!(!system.can_undo());
        assert!(!system.can_redo());
    }

    #[test]
    fn test_create_object_command() {
        let mut world = World::new();
        let mut system = UndoSystem::new(10);

        let cmd = CreateObjectCommand::new("TestObject");
        let _handle = system.execute(Box::new(cmd), &mut world);

        assert!(world.is_valid(_handle));
        assert_eq!(world.get_gameobject(_handle).unwrap().name(), "TestObject");
        assert!(system.can_undo());
        assert!(!system.can_redo());
    }

    #[test]
    fn test_undo_create_object() {
        let mut world = World::new();
        let mut system = UndoSystem::new(10);

        let cmd = CreateObjectCommand::new("TestObject");
        let handle = system.execute(Box::new(cmd), &mut world);

        system.undo(&mut world);

        assert!(!world.is_valid(handle));
        assert!(!system.can_undo());
        assert!(system.can_redo());
    }

    #[test]
    fn test_redo_create_object() {
        let mut world = World::new();
        let mut system = UndoSystem::new(10);

        let cmd = CreateObjectCommand::new("TestObject");
        let _handle = system.execute(Box::new(cmd), &mut world);

        system.undo(&mut world);
        system.redo(&mut world);

        // After redo, we need to check if the object was recreated
        // The handle might be different if the slot was reused
        assert!(world.count() > 0);
        assert!(system.can_undo());
        assert!(!system.can_redo());
    }

    #[test]
    fn test_destroy_object_command() {
        let mut world = World::new();
        let mut system = UndoSystem::new(10);

        // Create an object first
        let mut go = GameObject::new("TestObject");
        go.set_tag("enemy");
        go.set_layer(1);
        let handle = world.spawn(go);

        // Create destroy command
        let cmd = DestroyObjectCommand::new(&world, handle).unwrap();
        system.execute(Box::new(cmd), &mut world);

        assert!(!world.is_valid(handle));
    }

    #[test]
    fn test_undo_destroy_object() {
        let mut world = World::new();
        let mut system = UndoSystem::new(10);

        // Create an object first
        let mut go = GameObject::new("TestObject");
        go.set_tag("enemy");
        go.set_layer(1);
        let original_handle = world.spawn(go);

        // Create and execute destroy command
        let cmd = DestroyObjectCommand::new(&world, original_handle).unwrap();
        system.execute(Box::new(cmd), &mut world);

        assert!(!world.is_valid(original_handle));

        // Undo the destroy
        system.undo(&mut world);

        // Object should be recreated (might have a different handle if slot was reused)
        assert!(world.count() > 0);
        assert!(world.find_gameobject("TestObject").is_some());
    }

    #[test]
    fn test_clear_history() {
        let mut world = World::new();
        let mut system = UndoSystem::new(10);

        let cmd = CreateObjectCommand::new("TestObject");
        system.execute(Box::new(cmd), &mut world);

        assert!(system.can_undo());

        system.clear();

        assert!(!system.can_undo());
        assert!(!system.can_redo());
    }

    #[test]
    fn test_max_history() {
        let mut world = World::new();
        let mut system = UndoSystem::new(2); // Only keep 2 commands

        // Execute 3 commands
        system.execute(Box::new(CreateObjectCommand::new("Obj1")), &mut world);
        system.execute(Box::new(CreateObjectCommand::new("Obj2")), &mut world);
        system.execute(Box::new(CreateObjectCommand::new("Obj3")), &mut world);

        // Should only be able to undo 2 times (oldest command was dropped)
        assert!(system.undo(&mut world).is_some());
        assert!(system.undo(&mut world).is_some());
        assert!(system.undo(&mut world).is_none());
    }

    #[test]
    fn test_description() {
        let cmd = CreateObjectCommand::new("Player");
        assert_eq!(cmd.description(), "Create 'Player'");

        let mut world = World::new();
        let mut go = GameObject::new("Enemy");
        go.set_tag("bad");
        let handle = world.spawn(go);
        let cmd = DestroyObjectCommand::new(&world, handle).unwrap();
        assert_eq!(cmd.description(), "Destroy 'Enemy'");
    }

    #[test]
    fn test_undo_destroy_restores_children() {
        let mut world = World::new();
        let mut system = UndoSystem::new(10);

        // Create parent with children
        let parent = world.spawn(GameObject::new("Parent"));
        let child1 = world.spawn(GameObject::new("Child1"));
        let child2 = world.spawn(GameObject::new("Child2"));
        world.set_parent(child1, Some(parent));
        world.set_parent(child2, Some(parent));

        assert_eq!(world.get_children(parent).len(), 2);

        // Execute destroy
        let cmd = DestroyObjectCommand::new(&world, parent).unwrap();
        system.execute(Box::new(cmd), &mut world);

        assert!(!world.is_valid(parent));
        assert!(!world.is_valid(child1));
        assert!(!world.is_valid(child2));

        // Undo should restore parent with children
        system.undo(&mut world);

        assert!(world.count() > 0);
        let restored_parent = world.find_gameobject("Parent").unwrap();
        let restored_children = world.get_children(restored_parent);
        assert_eq!(restored_children.len(), 2);

        // Children should be findable
        assert!(world.find_gameobject("Child1").is_some());
        assert!(world.find_gameobject("Child2").is_some());
    }

    #[test]
    fn test_undo_destroy_restores_components() {
        use std::any::Any;

        #[derive(Debug)]
        struct HealthComponent {
            hp: f32,
        }

        impl crate::gameobject::Component for HealthComponent {
            fn as_any(&self) -> &dyn Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }

        let mut world = World::new();
        let mut system = UndoSystem::new(10);

        // Create object with a component
        let mut go = GameObject::new("Player");
        go.add_component(HealthComponent { hp: 100.0 });
        let handle = world.spawn(go);

        // Verify component exists
        {
            let player = world.get_gameobject(handle).unwrap();
            assert!(player.get_component::<HealthComponent>().is_some());
            assert_eq!(player.get_component::<HealthComponent>().unwrap().hp, 100.0);
        }

        // Execute destroy
        let cmd = DestroyObjectCommand::new(&world, handle).unwrap();
        system.execute(Box::new(cmd), &mut world);

        assert!(!world.is_valid(handle));

        // Undo should restore with component
        system.undo(&mut world);

        let restored = world.find_gameobject("Player").unwrap();
        let player = world.get_gameobject(restored).unwrap();
        assert!(player.get_component::<HealthComponent>().is_some());
        assert_eq!(player.get_component::<HealthComponent>().unwrap().hp, 100.0);
    }

    #[test]
    fn test_redo_destroy_with_children() {
        let mut world = World::new();
        let mut system = UndoSystem::new(10);

        let parent = world.spawn(GameObject::new("Parent"));
        let child = world.spawn(GameObject::new("Child"));
        world.set_parent(child, Some(parent));

        let cmd = DestroyObjectCommand::new(&world, parent).unwrap();
        system.execute(Box::new(cmd), &mut world);

        // Undo then redo
        system.undo(&mut world);
        assert!(world.find_gameobject("Parent").is_some());
        assert!(world.find_gameobject("Child").is_some());

        system.redo(&mut world);
        assert!(!world.is_valid(parent));
        assert!(!world.is_valid(child));
    }
}
