use engine_ecs::entity::Entity;
use engine_ecs::gameobject::GameObjectHandle;
use serde::{Deserialize, Serialize};

/// A lightweight handle for a node in the scene graph.
/// Wraps both a GameObjectHandle and the underlying ECS Entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SceneNode {
    gameobject: GameObjectHandle,
    entity: Entity,
}

impl SceneNode {
    /// Create a scene node from a GameObject handle and Entity.
    pub fn new(gameobject: GameObjectHandle, entity: Entity) -> Self {
        Self { gameobject, entity }
    }

    /// Return the underlying GameObject handle.
    pub fn gameobject(&self) -> GameObjectHandle {
        self.gameobject
    }

    /// Return the underlying Entity (for backward compatibility).
    #[deprecated(note = "Use gameobject() instead")]
    pub fn entity(&self) -> Entity {
        self.entity
    }
}

impl From<SceneNode> for GameObjectHandle {
    fn from(node: SceneNode) -> Self {
        node.gameobject()
    }
}
