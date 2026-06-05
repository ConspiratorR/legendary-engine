//! Physics plugins for engine (3D and 2D).
use crate::physics_2d::PhysicsWorld2D;
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

fn physics_2d_step_system(world: &mut engine_ecs::world::World) {
    let mut pw = match world.remove_resource::<PhysicsWorld2D>() {
        Some(pw) => pw,
        None => return,
    };
    let dt = world
        .get_resource::<engine_core::time::Time>()
        .map(|t| t.delta_seconds())
        .unwrap_or(1.0 / 60.0);
    pw.step(world, dt);
    world.insert_resource(pw);
}

/// Plugin that adds 2D physics simulation capabilities.
pub struct Physics2DPlugin;

impl Plugin for Physics2DPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(PhysicsWorld2D::default());
        app.add_system(physics_2d_step_system);
    }
}
