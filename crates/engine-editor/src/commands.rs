use std::collections::VecDeque;

pub trait Command: std::fmt::Debug {
    fn execute(&mut self);
    fn undo(&mut self);
    fn redo(&mut self);
    fn description(&self) -> String;
}

pub struct CommandManager {
    undo_stack: VecDeque<Box<dyn Command>>,
    redo_stack: VecDeque<Box<dyn Command>>,
    max_history: usize,
}

impl CommandManager {
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: VecDeque::with_capacity(max_history),
            redo_stack: VecDeque::with_capacity(max_history),
            max_history,
        }
    }

    pub fn execute(&mut self, command: Box<dyn Command>) {
        let mut cmd = command;
        cmd.execute();

        self.undo_stack.push_back(cmd);
        self.redo_stack.clear();

        while self.undo_stack.len() > self.max_history {
            self.undo_stack.pop_front();
        }
    }

    pub fn undo(&mut self) -> Option<()> {
        let mut cmd = self.undo_stack.pop_back()?;
        cmd.undo();
        self.redo_stack.push_back(cmd);
        Some(())
    }

    pub fn redo(&mut self) -> Option<()> {
        let mut cmd = self.redo_stack.pop_back()?;
        cmd.redo();
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

// 具体命令实现示例
#[derive(Debug)]
pub struct CreateEntityCommand {
    entity_id: u64,
    entity_name: String,
    #[allow(dead_code)]
    parent_id: Option<u64>,
}

impl CreateEntityCommand {
    pub fn new(entity_id: u64, entity_name: String, parent_id: Option<u64>) -> Self {
        Self {
            entity_id,
            entity_name,
            parent_id,
        }
    }
}

impl Command for CreateEntityCommand {
    fn execute(&mut self) {
        println!(
            "Creating entity: {} (ID: {})",
            self.entity_name, self.entity_id
        );
    }

    fn undo(&mut self) {
        println!("Undo: Delete entity {}", self.entity_id);
    }

    fn redo(&mut self) {
        println!("Redo: Recreate entity {}", self.entity_id);
    }

    fn description(&self) -> String {
        format!("Create {}", self.entity_name)
    }
}

#[derive(Debug)]
pub struct DeleteEntityCommand {
    entity_id: u64,
    entity_name: String,
}

impl DeleteEntityCommand {
    pub fn new(entity_id: u64, entity_name: String) -> Self {
        Self {
            entity_id,
            entity_name,
        }
    }
}

impl Command for DeleteEntityCommand {
    fn execute(&mut self) {
        println!(
            "Deleting entity: {} (ID: {})",
            self.entity_name, self.entity_id
        );
    }

    fn undo(&mut self) {
        println!("Undo: Restore entity {}", self.entity_id);
    }

    fn redo(&mut self) {
        println!("Redo: Delete entity again {}", self.entity_id);
    }

    fn description(&self) -> String {
        format!("Delete {}", self.entity_name)
    }
}

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
    fn execute(&mut self) {
        println!(
            "Moving entity {} to parent {:?}",
            self.entity_id, self.to_parent
        );
    }

    fn undo(&mut self) {
        println!(
            "Undo: Move entity {} back to parent {:?}",
            self.entity_id, self.from_parent
        );
    }

    fn redo(&mut self) {
        println!(
            "Redo: Move entity {} to parent {:?}",
            self.entity_id, self.to_parent
        );
    }

    fn description(&self) -> String {
        "Move Entity".to_string()
    }
}

#[derive(Debug)]
pub struct RenameEntityCommand {
    entity_id: u64,
    old_name: String,
    new_name: String,
}

impl RenameEntityCommand {
    pub fn new(entity_id: u64, old_name: String, new_name: String) -> Self {
        Self {
            entity_id,
            old_name,
            new_name,
        }
    }
}

impl Command for RenameEntityCommand {
    fn execute(&mut self) {
        println!("Renaming entity {} to '{}'", self.entity_id, self.new_name);
    }

    fn undo(&mut self) {
        println!(
            "Undo: Rename entity {} back to '{}'",
            self.entity_id, self.old_name
        );
    }

    fn redo(&mut self) {
        println!(
            "Redo: Rename entity {} to '{}'",
            self.entity_id, self.new_name
        );
    }

    fn description(&self) -> String {
        format!("Rename to '{}'", self.new_name)
    }
}

#[derive(Debug)]
pub struct TransformEntityCommand {
    entity_id: u64,
    old_position: (f32, f32, f32),
    new_position: (f32, f32, f32),
    old_rotation: (f32, f32, f32),
    new_rotation: (f32, f32, f32),
    old_scale: (f32, f32, f32),
    new_scale: (f32, f32, f32),
}

impl TransformEntityCommand {
    pub fn new(
        entity_id: u64,
        old_pos: (f32, f32, f32),
        new_pos: (f32, f32, f32),
        old_rot: (f32, f32, f32),
        new_rot: (f32, f32, f32),
        old_scale: (f32, f32, f32),
        new_scale: (f32, f32, f32),
    ) -> Self {
        Self {
            entity_id,
            old_position: old_pos,
            new_position: new_pos,
            old_rotation: old_rot,
            new_rotation: new_rot,
            old_scale,
            new_scale,
        }
    }
}

impl Command for TransformEntityCommand {
    fn execute(&mut self) {
        println!(
            "Transform entity {}: pos {:?} -> {:?}, rot {:?} -> {:?}, scale {:?} -> {:?}",
            self.entity_id,
            self.old_position,
            self.new_position,
            self.old_rotation,
            self.new_rotation,
            self.old_scale,
            self.new_scale
        );
    }

    fn undo(&mut self) {
        println!(
            "Undo: Restore entity {} transform to pos {:?}, rot {:?}, scale {:?}",
            self.entity_id, self.old_position, self.old_rotation, self.old_scale
        );
    }

    fn redo(&mut self) {
        println!(
            "Redo: Apply entity {} transform to pos {:?}, rot {:?}, scale {:?}",
            self.entity_id, self.new_position, self.new_rotation, self.new_scale
        );
    }

    fn description(&self) -> String {
        "Transform Entity".to_string()
    }
}

/// Command for terrain sculpting operations.
///
/// Captures a heightmap snapshot of the affected region before a sculpt
/// brush is applied, enabling undo/redo of the modification.
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
    fn execute(&mut self) {
        // Brush already applied by the sculpt system — nothing to do here.
    }

    fn undo(&mut self) {
        // Restoration requires World access — handled by editor integration.
    }

    fn redo(&mut self) {
        // Re-application requires World access — handled by editor integration.
    }

    fn description(&self) -> String {
        self.description.clone()
    }
}
