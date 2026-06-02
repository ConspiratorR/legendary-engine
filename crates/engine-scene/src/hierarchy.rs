use engine_ecs::entity::Entity;
use serde::{Deserialize, Serialize};

/// Component linking a child entity to its parent in the scene graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parent(pub Entity);

/// Component listing the child entities of a scene node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Children(pub Vec<Entity>);

impl Children {
    /// Create an empty children list.
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

impl Default for Children {
    fn default() -> Self {
        Self::new()
    }
}
