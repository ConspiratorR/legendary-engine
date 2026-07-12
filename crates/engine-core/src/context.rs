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
}

impl<'a> Context<'a> {
    /// Create a new context.
    pub fn new(world: &'a mut World, time: Time, frame: u64) -> Self {
        Self { world, time, frame }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let mut world = World::new();
        let time = Time::default();
        let ctx = Context::new(&mut world, time, 42);
        assert_eq!(ctx.frame, 42);
    }
}
