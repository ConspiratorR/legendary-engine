//! Physics plugin for engine.
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;
use crate::world::PhysicsWorld;

/// Plugin that adds physics simulation capabilities.
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        // Add physics world as a resource
        let world = app.world_mut();
        world.insert_resource(PhysicsWorld::default());

        // Register physics components with ECS
        // This would typically add systems to the schedule
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_physics_plugin_creation() {
        let _plugin = PhysicsPlugin;
        // Plugin can be created
    }
}
