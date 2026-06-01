use engine_ecs::entity::Entity;

/// A lightweight handle for a node in the scene graph.
///
/// Wraps an ECS [`Entity`] and is used with [`SceneManager`](super::scene_manager::SceneManager)
/// to manage hierarchy and transforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneNode {
    entity: Entity,
}

impl SceneNode {
    /// Create a scene node from an existing entity.
    pub fn new(entity: Entity) -> Self {
        Self { entity }
    }

    /// Return the underlying entity handle.
    pub fn entity(&self) -> Entity {
        self.entity
    }
}
