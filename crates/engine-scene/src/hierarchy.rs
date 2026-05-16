use engine_ecs::entity::Entity;

#[derive(Debug, Clone)]
pub struct Parent(pub Entity);

#[derive(Debug, Clone)]
pub struct Children(pub Vec<Entity>);

impl Children {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

impl Default for Children {
    fn default() -> Self {
        Self::new()
    }
}
