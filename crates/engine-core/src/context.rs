//! System execution context.
//!
//! Passed to systems and MonoBehaviour callbacks during execution.

use crate::event::EventBus;
use crate::time::Time;
use crate::world::World;

/// Context passed to systems during execution.
///
/// # Unity Documentation
/// In Unity, MonoBehaviour callbacks receive no context parameter —
/// they access the world via `this.gameObject`, `this.transform`, etc.
///
/// In Rust, we pass a Context that provides access to:
/// - The World (for querying/creating/destroying GameObjects)
/// - Time information (delta time, elapsed time, etc.)
/// - Event bus (for sending/receiving events)
/// - Frame number
pub struct Context<'a> {
    /// The World containing all GameObjects.
    pub world: &'a mut World,

    /// Time information (matches Unity's `Time` class).
    pub time: Time,

    /// Current frame number.
    pub frame: u64,

    /// Event bus for sending/receiving events.
    pub events: &'a mut EventBus,
}

impl<'a> Context<'a> {
    /// Create a new context.
    pub fn new(world: &'a mut World, time: Time, frame: u64, events: &'a mut EventBus) -> Self {
        Self {
            world,
            time,
            frame,
            events,
        }
    }

    /// Get a reference to the World.
    pub fn World(&self) -> &World {
        self.world
    }

    /// Get a mutable reference to the World.
    pub fn WorldMut(&mut self) -> &mut World {
        self.world
    }

    /// Get delta time (matches `Time.deltaTime`).
    pub fn DeltaTime(&self) -> f32 {
        self.time.delta_seconds()
    }

    /// Get fixed delta time (matches `Time.fixedDeltaTime`).
    pub fn FixedDeltaTime(&self) -> f32 {
        self.time.delta_seconds() // Use delta_seconds as fallback
    }

    /// Get elapsed time (matches `Time.time`).
    pub fn Time(&self) -> f32 {
        self.time.elapsed_seconds()
    }

    /// Get the current frame (matches `Time.frameCount`).
    pub fn Frame(&self) -> u64 {
        self.frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let mut world = World::new();
        let time = Time::default();
        let mut events = EventBus::new();
        let ctx = Context::new(&mut world, time, 42, &mut events);
        assert_eq!(ctx.Frame(), 42);
    }

    #[test]
    fn test_context_world_access() {
        let mut world = World::new();
        let time = Time::default();
        let mut events = EventBus::new();
        let ctx = Context::new(&mut world, time, 0, &mut events);

        // Can access world
        assert_eq!(ctx.World().GetRootGameObjects().len(), 0);
    }

    #[test]
    fn test_context_time() {
        let mut world = World::new();
        let time = Time::default();
        let mut events = EventBus::new();
        let ctx = Context::new(&mut world, time, 0, &mut events);

        assert_eq!(ctx.DeltaTime(), 0.0);
        assert_eq!(ctx.Time(), 0.0);
    }
}
