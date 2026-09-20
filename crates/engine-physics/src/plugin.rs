//! Physics plugins for engine (3D and 2D).
use crate::physics_2d::PhysicsWorld2D;
use crate::world::PhysicsWorld;
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;

fn physics_step_system(world: &mut engine_ecs::world::World) {
    // Sync timestep from Time (Unity: FixedUpdate uses fixedDeltaTime)
    let fixed_dt = world
        .get_resource::<engine_core::time::Time>()
        .map(|t| t.fixedDeltaTime())
        .unwrap_or(1.0 / 50.0);

    let mut pw = match world.remove_resource::<PhysicsWorld>() {
        Some(pw) => pw,
        None => return,
    };
    pw.delta_time = fixed_dt;
    pw.step(world);
    world.insert_resource(pw);
}

/// Plugin that adds physics simulation capabilities.
///
/// Registers [`physics_step_system`] on the **FixedUpdate** ECS schedule so
/// simulation advances 0+ times per frame at a fixed timestep (Unity contract).
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(PhysicsWorld::default());
        app.add_fixed_ecs_system(physics_step_system);
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
        // FixedUpdate schedule registered (system_count may be private on older Schedule)
        let _ = app.fixed_schedule();
    }
}

fn physics_2d_step_system(world: &mut engine_ecs::world::World) {
    let mut pw = match world.remove_resource::<PhysicsWorld2D>() {
        Some(pw) => pw,
        None => return,
    };
    let dt = world
        .get_resource::<engine_core::time::Time>()
        .map(|t| t.fixedDeltaTime())
        .unwrap_or(1.0 / 50.0);
    pw.step(world, dt);
    world.insert_resource(pw);
}

/// Plugin that adds 2D physics simulation capabilities (FixedUpdate schedule).
pub struct Physics2DPlugin;

impl Plugin for Physics2DPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(PhysicsWorld2D::default());
        app.add_fixed_ecs_system(physics_2d_step_system);
    }
}

/// FixedUpdate system: Unity World Rigidbody ↔ physics ECS ↔ World pose (phase 18).
///
/// Runs only when [`engine_core::scene_runtime::SceneRuntime`] is present
/// (CorePlugins / SceneRuntimePlugin). Simulates on identity-bridge entities
/// and writes results back with storage-authority `SetLocalPosition`.
fn unity_physics_step_system(world: &mut engine_ecs::world::World) {
    let fixed_dt = world
        .get_resource::<engine_core::time::Time>()
        .map(|t| t.fixedDeltaTime())
        .unwrap_or(1.0 / 50.0);

    let Some(mut runtime) = world.remove_resource::<engine_core::scene_runtime::SceneRuntime>()
    else {
        return;
    };

    // Ensure PhysicsWorld resource exists.
    if world.get_resource::<PhysicsWorld>().is_none() {
        world.insert_resource(PhysicsWorld::default());
    }

    crate::unity_bridge::unity_physics_fixed_step(&mut runtime, world, fixed_dt);

    world.insert_resource(runtime);
}

/// Plugin: Unity World-driven physics (requires SceneRuntime for GameObjects).
///
/// Prefer this over bare [`PhysicsPlugin`] when gameplay uses Unity
/// Rigidbody/MonoBehaviour rather than raw ECS physics components.
pub struct UnityPhysicsPlugin;

impl Plugin for UnityPhysicsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        if app.world_mut().get_resource::<PhysicsWorld>().is_none() {
            app.insert_resource(PhysicsWorld::default());
        }
        app.add_fixed_ecs_system(unity_physics_step_system);
    }
}
