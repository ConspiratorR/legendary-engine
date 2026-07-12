use crate::event::EventBus;
use crate::time::Time;
use crate::world::World;

/// Context passed to systems during execution.
pub struct Context<'a> {
    /// The ECS world.
    pub world: &'a mut World,
    /// Time information.
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
        assert_eq!(ctx.frame, 42);
    }
}
