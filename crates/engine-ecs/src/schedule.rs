use crate::system::System;
use crate::world::World;

pub struct Schedule {
    systems: Vec<Box<dyn System>>,
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

impl Schedule {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    pub fn add_system(&mut self, system: impl System + 'static) -> &mut Self {
        self.systems.push(Box::new(system));
        self
    }

    pub fn run(&self, world: &mut World) {
        for system in &self.systems {
            system.run(world);
        }
    }
}
