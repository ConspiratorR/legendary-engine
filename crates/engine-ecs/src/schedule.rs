use crate::system::System;
use crate::world::World;

/// An ordered list of [`System`]s executed sequentially.
///
/// Systems are run in the order they were added via [`add_system`](Self::add_system).
pub struct Schedule {
    systems: Vec<Box<dyn System>>,
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

impl Schedule {
    /// Create an empty schedule.
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    /// Append a system to the end of the schedule.
    pub fn add_system(&mut self, system: impl System + 'static) -> &mut Self {
        self.systems.push(Box::new(system));
        self
    }

    /// Run all systems in order against the given `world`.
    pub fn run(&self, world: &mut World) {
        for system in &self.systems {
            system.run(world);
        }
    }
}
