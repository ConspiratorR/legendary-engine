//! MonoBehaviour lifecycle runner.
//!
//! Dispatches lifecycle callbacks on MonoBehaviours in the correct order.
//!
//! # Unity Execution Order
//! <https://docs.unity3d.com/Manual/ExecutionOrder.html>
//!
//! ## Lifecycle Order
//! 1. FixedUpdate (0+ times per frame)
//! 2. Update
//! 3. LateUpdate
//! 4. Sync transforms
//! 5. Flush destroy

use crate::context::Context;
use crate::gameobject::GameObjectHandle;
use crate::world::World;

/// Dispatches lifecycle callbacks on MonoBehaviours.
///
/// # Usage
/// Call the appropriate method at the correct point in the game loop:
///
/// ```ignore
/// // At startup (once per object)
/// MonoBehaviourRunner::run_awake(&mut world, handle, &mut context);
/// MonoBehaviourRunner::run_start(&mut world, &mut context);
///
/// // Each frame (in order)
/// for _ in 0..fixed_update_count {
///     MonoBehaviourRunner::run_fixed_update(&mut world, &mut context);
/// }
/// MonoBehaviourRunner::run_update(&mut world, &mut context);
/// MonoBehaviourRunner::run_late_update(&mut world, &mut context);
///
/// // After all updates
/// world.sync_transforms();
/// world.flush_destroy();
/// ```
pub struct MonoBehaviourRunner;

impl MonoBehaviourRunner {
    /// Run Awake on all MonoBehaviours for a specific GameObject.
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.Awake.html>
    ///
    /// Called when the script instance is being loaded. Always called before
    /// any Start. Called even if the GameObject is inactive.
    pub fn run_awake(world: &mut World, handle: GameObjectHandle, context: &mut Context) {
        world.run_awake(handle, context);
    }

    /// Run Start on all MonoBehaviours that haven't started yet.
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.Start.html>
    ///
    /// Called once before the first Update. All Start calls happen before
    /// any Update calls.
    pub fn run_start(world: &mut World, context: &mut Context) {
        world.run_start(context);
    }

    /// Run Update on all enabled MonoBehaviours.
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.Update.html>
    ///
    /// Called once per frame. The most common callback for game logic.
    pub fn run_update(world: &mut World, context: &mut Context) {
        world.run_update(context);
    }

    /// Run FixedUpdate on all enabled MonoBehaviours.
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.FixedUpdate.html>
    ///
    /// Called at fixed time steps (default 0.02 seconds). All physics
    /// calculations should happen here. Called 0+ times per frame.
    pub fn run_fixed_update(world: &mut World, context: &mut Context) {
        world.run_fixed_update(context);
    }

    /// Run LateUpdate on all enabled MonoBehaviours.
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.LateUpdate.html>
    ///
    /// Called once per frame after all Update calls. Commonly used for
    /// camera follow logic.
    pub fn run_late_update(world: &mut World, context: &mut Context) {
        world.run_late_update(context);
    }

    /// Run OnDestroy on all MonoBehaviours for a specific GameObject.
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnDestroy.html>
    ///
    /// Called on the last frame before the object is removed from the scene.
    pub fn run_on_destroy(world: &mut World, handle: GameObjectHandle, context: &mut Context) {
        // On actual destroy, this would be called
        // For now, the lifecycle is handled by World::destroy_internal
    }

    /// Run OnEnable on all MonoBehaviours for a specific GameObject.
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnEnable.html>
    pub fn run_on_enable(world: &mut World, handle: GameObjectHandle, context: &mut Context) {
        // Will be implemented when SetActive is fully wired
    }

    /// Run OnDisable on all MonoBehaviours for a specific GameObject.
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnDisable.html>
    pub fn run_on_disable(world: &mut World, handle: GameObjectHandle, context: &mut Context) {
        // Will be implemented when SetActive is fully wired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runner_instantiation() {
        let _runner = MonoBehaviourRunner;
    }

    #[test]
    fn test_runner_struct_is_unit() {
        // MonoBehaviourRunner is a unit struct — verify it can be created
        // multiple times without issue.
        let _r1 = MonoBehaviourRunner;
        let _r2 = MonoBehaviourRunner;
    }

    #[test]
    fn test_runner_method_signatures_compile() {
        // Verify the method signatures exist and are callable.
        // We can't actually call them with a Context because Context
        // borrows World mutably, creating an aliasing issue.
        // Actual lifecycle behavior is tested via integration tests.
        let _f_awake: fn(&mut World, GameObjectHandle, &mut Context) =
            MonoBehaviourRunner::run_awake;
        let _f_start: fn(&mut World, &mut Context) = MonoBehaviourRunner::run_start;
        let _f_update: fn(&mut World, &mut Context) = MonoBehaviourRunner::run_update;
        let _f_fixed: fn(&mut World, &mut Context) = MonoBehaviourRunner::run_fixed_update;
        let _f_late: fn(&mut World, &mut Context) = MonoBehaviourRunner::run_late_update;
        let _f_destroy: fn(&mut World, GameObjectHandle, &mut Context) =
            MonoBehaviourRunner::run_on_destroy;
        let _f_enable: fn(&mut World, GameObjectHandle, &mut Context) =
            MonoBehaviourRunner::run_on_enable;
        let _f_disable: fn(&mut World, GameObjectHandle, &mut Context) =
            MonoBehaviourRunner::run_on_disable;
    }
}
