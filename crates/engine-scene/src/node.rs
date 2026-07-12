use engine_ecs::gameobject::GameObjectHandle;
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
