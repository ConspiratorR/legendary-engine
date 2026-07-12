use crate::context::Context;

/// Trait for game logic systems (like Unity's PlayerLoopSystem).
pub trait System: Send + Sync {
    /// Run the system.
    fn run(&self, context: &mut Context);

    /// Get the system name (for debugging).
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// Blanket implementation for closures.
impl<F: Fn(&mut Context) + Send + Sync> System for F {
    fn run(&self, context: &mut Context) {
        self(context);
    }
}

/// Wrapper for systems with a custom name.
pub struct NamedSystem {
    name: String,
    system: Box<dyn System>,
}

impl NamedSystem {
    /// Create a new named system.
    pub fn new(name: impl Into<String>, system: impl System + 'static) -> Self {
        Self {
            name: name.into(),
            system: Box::new(system),
        }
    }
}

impl System for NamedSystem {
    fn run(&self, context: &mut Context) {
        self.system.run(context);
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventBus;
    use crate::time::Time;
    use crate::world::World;

    #[test]
    fn test_closure_system() {
        let system = |ctx: &mut Context| {
            ctx.frame += 1;
        };
        let mut world = World::new();
        let mut events = EventBus::new();
        let mut ctx = Context::new(&mut world, Time::default(), 0, &mut events);
        system.run(&mut ctx);
        assert_eq!(ctx.frame, 1);
    }

    #[test]
    fn test_named_system() {
        let system = NamedSystem::new("increment", |ctx: &mut Context| {
            ctx.frame += 10;
        });
        let mut world = World::new();
        let mut events = EventBus::new();
        let mut ctx = Context::new(&mut world, Time::default(), 0, &mut events);
        system.run(&mut ctx);
        assert_eq!(ctx.frame, 10);
        assert_eq!(system.name(), "increment");
    }

    #[test]
    fn test_default_system_name() {
        let system = |_: &mut Context| {};
        // type_name is compiler-generated, just verify it's not empty
        assert!(!system.name().is_empty());
    }
}
