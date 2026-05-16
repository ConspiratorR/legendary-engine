use crate::transform::Transform;
use engine_ecs::entity::Entity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneNode {
    entity: Entity,
}

impl SceneNode {
    pub fn new(entity: Entity) -> Self {
        Self { entity }
    }

    pub fn entity(&self) -> Entity {
        self.entity
    }
}

pub struct SceneBuilder {
    pub entity: Entity,
    pub transform: Option<Transform>,
}
