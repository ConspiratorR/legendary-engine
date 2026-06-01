use engine_core::resource::ResourceRegistry;
use engine_ecs::world::World;

/// Context passed to [`GameState`](crate::GameState) lifecycle methods.
///
/// Provides mutable access to the ECS world, the resource registry,
/// and the current frame's delta time.
pub struct StateCtx<'a> {
    /// The ECS world.
    pub world: &'a mut World,
    /// The resource registry.
    pub resources: &'a mut ResourceRegistry,
    /// Frame delta time in seconds.
    pub delta: f32,
}
