use engine_core::resource::ResourceRegistry;
use engine_ecs::world::World;

pub struct StateCtx<'a> {
    pub world: &'a mut World,
    pub resources: &'a mut ResourceRegistry,
    pub delta: f32,
}
