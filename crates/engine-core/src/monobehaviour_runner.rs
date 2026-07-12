use crate::context::Context;
use crate::gameobject::GameObjectHandle;
use crate::world::World;

/// Runs lifecycle callbacks on MonoBehaviours.
pub struct MonoBehaviourRunner;

impl MonoBehaviourRunner {
    /// Run awake on all MonoBehaviours (called when GameObject is spawned).
    pub fn run_awake(_world: &mut World, _handle: GameObjectHandle) {
        // This would be called by World::spawn() if we had access to MonoBehaviourHolder
        // For now, this is a placeholder for the lifecycle system
    }

    /// Run start on all MonoBehaviours (called once before first update).
    pub fn run_start(_world: &mut World, _context: &mut Context) {
        // Placeholder for start lifecycle
    }

    /// Run update on all MonoBehaviours.
    pub fn run_update(world: &mut World, _context: &mut Context) {
        let handles: Vec<GameObjectHandle> = world.all_gameobjects();

        for _handle in handles {
            // In production, we'd iterate over MonoBehaviourHolder components
            // For now, this is a placeholder
        }
    }

    /// Run fixed_update on all MonoBehaviours.
    pub fn run_fixed_update(_world: &mut World, _context: &mut Context) {
        // Placeholder for fixed_update lifecycle
    }

    /// Run late_update on all MonoBehaviours.
    pub fn run_late_update(_world: &mut World, _context: &mut Context) {
        // Placeholder for late_update lifecycle
    }

    /// Run on_destroy on all MonoBehaviours (called when GameObject is despawned).
    pub fn run_on_destroy(_world: &mut World, _handle: GameObjectHandle) {
        // Placeholder for on_destroy lifecycle
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameobject::GameObject;

    #[test]
    fn test_monobehaviour_runner_can_be_instantiated() {
        let _runner = MonoBehaviourRunner;
    }

    #[test]
    fn test_monobehaviour_runner_run_awake() {
        let mut world = World::new();
        let handle = world.spawn(GameObject::new("Test"));
        // Should not panic
        MonoBehaviourRunner::run_awake(&mut world, handle);
    }

    #[test]
    fn test_monobehaviour_runner_run_start() {
        let mut world = World::new();
        let _handle = world.spawn(GameObject::new("Test"));
        // Placeholder - will be expanded when MonoBehaviourHolder is integrated
    }

    #[test]
    fn test_monobehaviour_runner_run_update() {
        let mut world = World::new();
        let _handle = world.spawn(GameObject::new("Test"));
        // Placeholder - will be expanded when MonoBehaviourHolder is integrated
    }

    #[test]
    fn test_monobehaviour_runner_run_fixed_update() {
        let mut world = World::new();
        let _handle = world.spawn(GameObject::new("Test"));
        // Placeholder - will be expanded when MonoBehaviourHolder is integrated
    }

    #[test]
    fn test_monobehaviour_runner_run_late_update() {
        let mut world = World::new();
        let _handle = world.spawn(GameObject::new("Test"));
        // Placeholder - will be expanded when MonoBehaviourHolder is integrated
    }

    #[test]
    fn test_monobehaviour_runner_run_on_destroy() {
        let mut world = World::new();
        let handle = world.spawn(GameObject::new("Test"));
        // Should not panic
        MonoBehaviourRunner::run_on_destroy(&mut world, handle);
    }
}
