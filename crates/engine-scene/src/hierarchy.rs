use engine_ecs::entity::Entity;

/// Component linking a child entity to its parent in the scene graph.
#[derive(Debug, Clone)]
pub struct Parent(pub Entity);

/// Component listing the child entities of a scene node.
#[derive(Debug, Clone)]
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
