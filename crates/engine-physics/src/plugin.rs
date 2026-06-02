//! Physics plugin for engine.
use crate::world::PhysicsWorld;
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;

fn physics_step_system(world: &mut engine_ecs::world::World) {
    // Remove PhysicsWorld from resources so we can call step() with &mut World
    let mut pw = match world.remove_resource::<PhysicsWorld>() {
        Some(pw) => pw,
        None => return,
    };
    pw.step(world);
    // Put it back
    world.insert_resource(pw);
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
        let pw = app.world_mut().get_resource::<PhysicsWorld>();
        assert!(pw.is_some());
    }
}
