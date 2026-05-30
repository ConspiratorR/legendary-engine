//! Physics plugin for engine.
use crate::world::PhysicsWorld;
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;

fn physics_step_system(world: &mut engine_ecs::world::World) {
    if let Some(mut pw) = world.get_resource_mut::<PhysicsWorld>().cloned() {
        pw.step(world);
        // Store updated counts back
        if let Some(res) = world.get_resource_mut::<PhysicsWorld>() {
            res.body_count = pw.body_count;
            res.collider_count = pw.collider_count;
            res.collisions = pw.collisions;
        }
    }
}

/// Plugin that adds physics simulation capabilities.
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(PhysicsWorld::default());
        app.add_system(physics_step_system);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_physics_plugin_creation() {
        let _plugin = PhysicsPlugin;
    }

    #[test]
    fn test_physics_plugin_registers_system() {
        let mut app = AppBuilder::new();
        app.add_plugin(PhysicsPlugin);
        // PhysicsWorld resource should exist
        let pw = app.world_mut().get_resource::<PhysicsWorld>();
        assert!(pw.is_some());
    }
}
