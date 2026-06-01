use crate::world::World;

/// A system that operates on a [`World`].
///
/// Systems are the primary way to express game logic. They receive
/// mutable access to the world and can read/write components and resources.
pub trait System {
    /// Execute this system against the given `world`.
    fn run(&self, world: &mut World);
}

/// Conversion trait from closures/functions into [`System`] instances.
///
/// Any `Fn(&mut World)` automatically implements this trait.
pub trait IntoSystem {
    /// The concrete `System` type produced.
    type System: System;
    /// Convert into a boxed system.
    fn system(self) -> Self::System;
}

impl<F> IntoSystem for F
where
    F: Fn(&mut World),
{
    type System = FnSystem<F>;

    fn system(self) -> Self::System {
        FnSystem(self)
    }
}

/// A [`System`] implementation that wraps a closure.
pub struct FnSystem<F>(F);

impl<F> System for FnSystem<F>
where
    F: Fn(&mut World),
{
    fn run(&self, world: &mut World) {
        (self.0)(world);
    }
}

#[cfg(test)]
mod tests {
    use crate::query::Query;
    use crate::schedule::Schedule;
    use crate::system::IntoSystem;
    use crate::world::World;

    struct Counter(u32);

    fn increment(world: &mut World) {
        let query = Query::<Counter>::new();
        for counter in query.iter_mut(world) {
            counter.0 += 1;
        }
    }

    #[test]
    fn test_schedule_run_once() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Counter(0));

        let mut schedule = Schedule::new();
        schedule.add_system(increment.system());

        schedule.run(&mut world);

        let counter = world.get::<Counter>(e).unwrap();
        assert_eq!(counter.0, 1);
    }

    #[test]
    fn test_schedule_run_twice() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Counter(0));

        let mut schedule = Schedule::new();
        schedule.add_system(increment.system());

        schedule.run(&mut world);
        schedule.run(&mut world);

        let counter = world.get::<Counter>(e).unwrap();
        assert_eq!(counter.0, 2);
    }
}
