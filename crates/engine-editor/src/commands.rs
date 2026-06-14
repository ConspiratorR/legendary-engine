//! Undo/redo command system for the editor.
//!
//! Provides a [`Command`] trait and [`CommandManager`] that maintains an
//! undo/redo stack with configurable history depth. Each editor action that
//! should be reversible implements [`Command`] with paired `execute`/`undo`
//! methods.

use crate::state::EditorState;
use std::collections::VecDeque;

/// Trait for undoable editor commands.
pub trait Command: std::fmt::Debug + Send {
    /// Executes the command (first time or redo).
    fn execute(&mut self, state: &mut EditorState);
    /// Reverses the command.
    fn undo(&mut self, state: &mut EditorState);
    /// Re-applies the command after an undo.
    fn redo(&mut self, state: &mut EditorState);
    /// Human-readable description for the undo/redo menu.
    fn description(&self) -> String;
}

/// Manages an undo/redo stack of [`Command`] instances.
pub struct CommandManager {
    undo_stack: VecDeque<Box<dyn Command>>,
    redo_stack: VecDeque<Box<dyn Command>>,
    max_history: usize,
}

impl std::fmt::Debug for CommandManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandManager")
            .field("undo_count", &self.undo_stack.len())
            .field("redo_count", &self.redo_stack.len())
            .finish()
    }
}

impl CommandManager {
    /// Creates a new command manager with the given maximum undo history depth.
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: VecDeque::with_capacity(max_history),
            redo_stack: VecDeque::with_capacity(max_history),
            max_history,
        }
    }

    /// Executes a command, pushing it onto the undo stack and clearing the redo stack.
    pub fn execute(&mut self, mut command: Box<dyn Command>, state: &mut EditorState) {
        command.execute(state);
        self.undo_stack.push_back(command);
        self.redo_stack.clear();
        while self.undo_stack.len() > self.max_history {
            self.undo_stack.pop_front();
        }
    }

    /// Undoes the last command.
    pub fn undo(&mut self, state: &mut EditorState) -> Option<()> {
        let mut cmd = self.undo_stack.pop_back()?;
        cmd.undo(state);
        self.redo_stack.push_back(cmd);
        Some(())
    }

    /// Redoes the last undone command.
    pub fn redo(&mut self, state: &mut EditorState) -> Option<()> {
        let mut cmd = self.redo_stack.pop_back()?;
        cmd.redo(state);
        self.undo_stack.push_back(cmd);
        Some(())
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn undo_description(&self) -> Option<String> {
        self.undo_stack.back().map(|cmd| cmd.description())
    }

    pub fn redo_description(&self) -> Option<String> {
        self.redo_stack.back().map(|cmd| cmd.description())
    }
}

impl Default for CommandManager {
    fn default() -> Self {
        Self::new(100)
    }
}

// 具体命令实现
#[derive(Debug)]
pub struct MoveEntityCommand {
    entity_id: u64,
    from_parent: Option<u64>,
    to_parent: Option<u64>,
}

impl MoveEntityCommand {
    pub fn new(entity_id: u64, from: Option<u64>, to: Option<u64>) -> Self {
        Self {
            entity_id,
            from_parent: from,
            to_parent: to,
        }
    }
}

impl Command for MoveEntityCommand {
    fn execute(&mut self, state: &mut EditorState) {
        state.scene_tree.reparent(self.entity_id, self.to_parent);
    }

    fn undo(&mut self, state: &mut EditorState) {
        state.scene_tree.reparent(self.entity_id, self.from_parent);
    }

    fn redo(&mut self, state: &mut EditorState) {
        state.scene_tree.reparent(self.entity_id, self.to_parent);
    }

    fn description(&self) -> String {
        "Move Entity".to_string()
    }
}

#[derive(Debug)]
pub struct TransformEntityCommand {
    entity_id: u64,
    old_transform: [f32; 9],
    new_transform: [f32; 9],
}

impl TransformEntityCommand {
    pub fn new(entity_id: u64, old_transform: [f32; 9], new_transform: [f32; 9]) -> Self {
        Self {
            entity_id,
            old_transform,
            new_transform,
        }
    }
}

impl Command for TransformEntityCommand {
    fn execute(&mut self, state: &mut EditorState) {
        if let Some(t) = state.node_transforms.get_mut(&self.entity_id) {
            *t = self.new_transform;
        }
    }

    fn undo(&mut self, state: &mut EditorState) {
        if let Some(t) = state.node_transforms.get_mut(&self.entity_id) {
            *t = self.old_transform;
        }
    }

    fn redo(&mut self, state: &mut EditorState) {
        if let Some(t) = state.node_transforms.get_mut(&self.entity_id) {
            *t = self.new_transform;
        }
    }

    fn description(&self) -> String {
        "Transform Entity".to_string()
    }
}

#[derive(Debug)]
pub struct DeleteEntityCommand {
    entity_id: u64,
    entity_name: String,
    transform: Option<[f32; 9]>,
    parent: Option<u64>,
}

impl DeleteEntityCommand {
    pub fn new(state: &EditorState, entity_id: u64) -> Self {
        let node = state.scene_tree.nodes.iter().find(|n| n.id == entity_id);
        Self {
            entity_id,
            entity_name: node.map(|n| n.name.clone()).unwrap_or_default(),
            transform: state.node_transforms.get(&entity_id).copied(),
            parent: node.and_then(|n| n.parent),
        }
    }
}

impl Command for DeleteEntityCommand {
    fn execute(&mut self, state: &mut EditorState) {
        state.scene_tree.remove_node(self.entity_id);
        state.node_transforms.remove(&self.entity_id);
        state.node_materials.remove(&self.entity_id);
        state.node_lights.remove(&self.entity_id);
        state.selected_nodes.retain(|&id| id != self.entity_id);
    }

    fn undo(&mut self, state: &mut EditorState) {
        let new_id = state.scene_tree.add_node(&self.entity_name, self.parent);
        if let Some(t) = self.transform {
            state.node_transforms.insert(new_id, t);
        }
        // Update the entity_id to the new ID for future redo
        self.entity_id = new_id;
    }

    fn redo(&mut self, state: &mut EditorState) {
        state.scene_tree.remove_node(self.entity_id);
        state.node_transforms.remove(&self.entity_id);
        state.node_materials.remove(&self.entity_id);
        state.node_lights.remove(&self.entity_id);
        state.selected_nodes.retain(|&id| id != self.entity_id);
    }

    fn description(&self) -> String {
        format!("Delete {}", self.entity_name)
    }
}

/// Command for terrain sculpting operations.
#[derive(Debug)]
pub struct SculptCommand {
    pub entity_id: u64,
    pub affected_min: (u32, u32),
    pub affected_max: (u32, u32),
    pub resolution: u32,
    pub height_snapshot: Vec<f32>,
    pub description: String,
}

impl SculptCommand {
    pub fn new(
        entity_id: u64,
        terrain: &engine_terrain::components::Terrain,
        center: engine_math::Vec3,
        radius: f32,
    ) -> Self {
        let half_w = terrain.world_size.x * 0.5;
        let half_h = terrain.world_size.y * 0.5;
        let res = terrain.resolution;

        let min_i = ((center.x - radius + half_w) / terrain.world_size.x * res as f32)
            .floor()
            .max(0.0) as u32;
        let max_i = ((center.x + radius + half_w) / terrain.world_size.x * res as f32)
            .ceil()
            .min(res as f32) as u32;
        let min_j = ((center.z - radius + half_h) / terrain.world_size.y * res as f32)
            .floor()
            .max(0.0) as u32;
        let max_j = ((center.z + radius + half_h) / terrain.world_size.y * res as f32)
            .ceil()
            .min(res as f32) as u32;

        let mut snapshot = Vec::new();
        for j in min_j..=max_j {
            for i in min_i..=max_i {
                let idx = (j * (res + 1) + i) as usize;
                snapshot.push(terrain.heightmap[idx]);
            }
        }

        Self {
            entity_id,
            affected_min: (min_i, min_j),
            affected_max: (max_i, max_j),
            resolution: res,
            height_snapshot: snapshot,
            description: "Sculpt Terrain".to_string(),
        }
    }
}

impl Command for SculptCommand {
    fn execute(&mut self, _state: &mut EditorState) {
        // Brush already applied by the sculpt system.
    }

    fn undo(&mut self, _state: &mut EditorState) {
        // Restoration requires World access — handled by editor integration.
    }

    fn redo(&mut self, _state: &mut EditorState) {
        // Re-application requires World access — handled by editor integration.
    }

    fn description(&self) -> String {
        self.description.clone()
    }
}
