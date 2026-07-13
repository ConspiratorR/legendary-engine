//! Undo/redo command system for the engine core.
//!
//! Provides a [`UndoCommand`] trait and [`UndoSystem`] that maintains an
//! undo/redo stack with configurable history depth.

use crate::gameobject::{GameObject, GameObjectHandle};
use crate::world::World;
use std::collections::VecDeque;

/// Trait for undoable engine commands.
pub trait UndoCommand: std::fmt::Debug + Send {
    fn Execute(&mut self, world: &mut World) -> GameObjectHandle;
    fn Undo(&mut self, world: &mut World);
    fn Redo(&mut self, world: &mut World);
    fn Description(&self) -> String;
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
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: VecDeque::with_capacity(max_history),
            redo_stack: VecDeque::with_capacity(max_history),
            max_history,
        }
    }

    pub fn Execute(
        &mut self,
        mut command: Box<dyn UndoCommand>,
        world: &mut World,
    ) -> GameObjectHandle {
        let handle = command.Execute(world);
        self.undo_stack.push_back(command);
        self.redo_stack.clear();
        while self.undo_stack.len() > self.max_history {
            self.undo_stack.pop_front();
        }
        handle
    }

    pub fn Undo(&mut self, world: &mut World) -> Option<()> {
        let mut cmd = self.undo_stack.pop_back()?;
        cmd.Undo(world);
        self.redo_stack.push_back(cmd);
        Some(())
    }

    pub fn Redo(&mut self, world: &mut World) -> Option<()> {
        let mut cmd = self.redo_stack.pop_back()?;
        cmd.Redo(world);
        self.undo_stack.push_back(cmd);
        Some(())
    }

    pub fn CanUndo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn CanRedo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn Clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn UndoDescription(&self) -> Option<String> {
        self.undo_stack.back().map(|cmd| cmd.Description())
    }

    pub fn RedoDescription(&self) -> Option<String> {
        self.redo_stack.back().map(|cmd| cmd.Description())
    }

    // ============================================================
    // Backward-compatible snake_case aliases
    // ============================================================

    /// Execute a command (snake_case alias for Execute).
    pub fn execute(
        &mut self,
        command: Box<dyn UndoCommand>,
        world: &mut World,
    ) -> GameObjectHandle {
        self.Execute(command, world)
    }

    /// Undo last command (snake_case alias for Undo).
    pub fn undo(&mut self, world: &mut World) -> Option<()> {
        self.Undo(world)
    }

    /// Redo last undone command (snake_case alias for Redo).
    pub fn redo(&mut self, world: &mut World) -> Option<()> {
        self.Redo(world)
    }

    /// Can undo (snake_case alias for CanUndo).
    pub fn can_undo(&self) -> bool {
        self.CanUndo()
    }

    /// Can redo (snake_case alias for CanRedo).
    pub fn can_redo(&self) -> bool {
        self.CanRedo()
    }

    /// Clear history (snake_case alias for Clear).
    pub fn clear(&mut self) {
        self.Clear();
    }

    /// Get undo description (snake_case alias for UndoDescription).
    pub fn undo_description(&self) -> Option<String> {
        self.UndoDescription()
    }

    /// Get redo description (snake_case alias for RedoDescription).
    pub fn redo_description(&self) -> Option<String> {
        self.RedoDescription()
    }
}

impl Default for UndoSystem {
    fn default() -> Self {
        Self::new(100)
    }
}

/// Command to create a new GameObject.
#[derive(Debug)]
pub struct CreateObjectCommand {
    name: String,
    tag: String,
    layer: i32,
    active: bool,
    parent: Option<GameObjectHandle>,
    created_handle: Option<GameObjectHandle>,
}

impl CreateObjectCommand {
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

    pub fn WithTag(mut self, tag: &str) -> Self {
        self.tag = tag.to_string();
        self
    }

    pub fn WithLayer(mut self, layer: i32) -> Self {
        self.layer = layer;
        self
    }

    pub fn WithActive(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn WithParent(mut self, parent: GameObjectHandle) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn CreatedHandle(&self) -> Option<GameObjectHandle> {
        self.created_handle
    }

    // ============================================================
    // Backward-compatible snake_case aliases
    // ============================================================

    /// Set tag (snake_case alias for WithTag).
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.WithTag(tag)
    }

    /// Set layer (snake_case alias for WithLayer).
    pub fn with_layer(mut self, layer: i32) -> Self {
        self.WithLayer(layer)
    }

    /// Set active (snake_case alias for WithActive).
    pub fn with_active(mut self, active: bool) -> Self {
        self.WithActive(active)
    }

    /// Set parent (snake_case alias for WithParent).
    pub fn with_parent(mut self, parent: GameObjectHandle) -> Self {
        self.WithParent(parent)
    }

    /// Get created handle (snake_case alias for CreatedHandle).
    pub fn created_handle(&self) -> Option<GameObjectHandle> {
        self.CreatedHandle()
    }
}

impl UndoCommand for CreateObjectCommand {
    fn Execute(&mut self, world: &mut World) -> GameObjectHandle {
        let handle = world.CreateGameObject(&self.name);
        world.SetTag(handle, &self.tag);
        world.SetLayer(handle, self.layer);
        world.SetActive(handle, self.active);

        if let Some(parent) = self.parent {
            world.SetParent(handle, Some(parent));
        }

        self.created_handle = Some(handle);
        handle
    }

    fn Undo(&mut self, world: &mut World) {
        if let Some(handle) = self.created_handle {
            world.DestroyImmediate(handle);
            self.created_handle = None;
        }
    }

    fn Redo(&mut self, world: &mut World) {
        self.Execute(world);
    }

    fn Description(&self) -> String {
        format!("Create '{}'", self.name)
    }
}

/// Command to destroy a GameObject.
#[derive(Debug)]
pub struct DestroyObjectCommand {
    handle: GameObjectHandle,
    name: String,
    tag: String,
    layer: i32,
    active: bool,
    parent: Option<GameObjectHandle>,
    children: Vec<GameObjectHandle>,
    /// Captured child data for restoration
    child_data: Vec<ChildData>,
    recreated_handle: Option<GameObjectHandle>,
}

/// Captured data for a child GameObject.
#[derive(Debug, Clone)]
struct ChildData {
    name: String,
    tag: String,
    layer: i32,
    active: bool,
}

impl DestroyObjectCommand {
    pub fn new(world: &World, handle: GameObjectHandle) -> Option<Self> {
        let name = world.GetName(handle).to_string();
        let tag = world.GetTag(handle).to_string();
        let layer = world.GetLayer(handle);
        let active = world.IsActive(handle);
        let parent = world.GetParent(handle);
        let children = world.GetChildren(handle);

        // Capture child data
        let child_data: Vec<ChildData> = children
            .iter()
            .map(|&child| ChildData {
                name: world.GetName(child).to_string(),
                tag: world.GetTag(child).to_string(),
                layer: world.GetLayer(child),
                active: world.IsActive(child),
            })
            .collect();

        Some(Self {
            handle,
            name,
            tag,
            layer,
            active,
            parent,
            children,
            child_data,
            recreated_handle: None,
        })
    }

    pub fn RecreatedHandle(&self) -> Option<GameObjectHandle> {
        self.recreated_handle
    }

    // ============================================================
    // Backward-compatible snake_case aliases
    // ============================================================

    /// Get recreated handle (snake_case alias for RecreatedHandle).
    pub fn recreated_handle(&self) -> Option<GameObjectHandle> {
        self.RecreatedHandle()
    }
}

impl UndoCommand for DestroyObjectCommand {
    fn Execute(&mut self, world: &mut World) -> GameObjectHandle {
        // Destroy children first
        for child in self.children.iter().rev() {
            world.DestroyImmediate(*child);
        }

        // Destroy the parent
        world.DestroyImmediate(self.handle);
        self.handle
    }

    fn Undo(&mut self, world: &mut World) {
        // Recreate the parent
        let new_handle = world.CreateGameObject(&self.name);
        world.SetTag(new_handle, &self.tag);
        world.SetLayer(new_handle, self.layer);
        world.SetActive(new_handle, self.active);

        // Recreate children with their captured data
        for child_data in &self.child_data {
            let child_handle = world.CreateGameObject(&child_data.name);
            world.SetTag(child_handle, &child_data.tag);
            world.SetLayer(child_handle, child_data.layer);
            world.SetActive(child_handle, child_data.active);
            world.SetParent(child_handle, Some(new_handle));
        }

        // Reattach to original parent
        if let Some(parent) = self.parent {
            world.SetParent(new_handle, Some(parent));
        }

        self.recreated_handle = Some(new_handle);
        self.handle = new_handle;
    }

    fn Redo(&mut self, world: &mut World) {
        self.Execute(world);
    }

    fn Description(&self) -> String {
        format!("Destroy '{}'", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_system_creation() {
        let system = UndoSystem::new(50);
        assert!(!system.CanUndo());
        assert!(!system.CanRedo());
    }

    #[test]
    fn test_create_object_command() {
        let mut world = World::new();
        let mut system = UndoSystem::new(10);

        let cmd = CreateObjectCommand::new("TestObject");
        let handle = system.Execute(Box::new(cmd), &mut world);

        assert_eq!(world.GetName(handle), "TestObject");
        assert!(system.CanUndo());
        assert!(!system.CanRedo());
    }

    #[test]
    fn test_undo_create_object() {
        let mut world = World::new();
        let mut system = UndoSystem::new(10);

        let cmd = CreateObjectCommand::new("TestObject");
        let handle = system.Execute(Box::new(cmd), &mut world);

        system.Undo(&mut world);

        assert!(!system.CanUndo());
        assert!(system.CanRedo());
    }

    #[test]
    fn test_redo_create_object() {
        let mut world = World::new();
        let mut system = UndoSystem::new(10);

        let cmd = CreateObjectCommand::new("TestObject");
        let _handle = system.Execute(Box::new(cmd), &mut world);

        system.Undo(&mut world);
        system.Redo(&mut world);

        assert!(system.CanUndo());
        assert!(!system.CanRedo());
    }

    #[test]
    fn test_destroy_object_command() {
        let mut world = World::new();
        let mut system = UndoSystem::new(10);

        let handle = world.CreateGameObject("TestObject");
        world.SetTag(handle, "enemy");
        world.SetLayer(handle, 1);

        let cmd = DestroyObjectCommand::new(&world, handle).unwrap();
        system.Execute(Box::new(cmd), &mut world);
    }

    #[test]
    fn test_clear_history() {
        let mut world = World::new();
        let mut system = UndoSystem::new(10);

        let cmd = CreateObjectCommand::new("TestObject");
        system.Execute(Box::new(cmd), &mut world);

        assert!(system.CanUndo());

        system.Clear();

        assert!(!system.CanUndo());
        assert!(!system.CanRedo());
    }

    #[test]
    fn test_max_history() {
        let mut world = World::new();
        let mut system = UndoSystem::new(2);

        system.Execute(Box::new(CreateObjectCommand::new("Obj1")), &mut world);
        system.Execute(Box::new(CreateObjectCommand::new("Obj2")), &mut world);
        system.Execute(Box::new(CreateObjectCommand::new("Obj3")), &mut world);

        assert!(system.Undo(&mut world).is_some());
        assert!(system.Undo(&mut world).is_some());
        assert!(system.Undo(&mut world).is_none());
    }

    #[test]
    fn test_description() {
        let cmd = CreateObjectCommand::new("Player");
        assert_eq!(cmd.Description(), "Create 'Player'");
    }
}
