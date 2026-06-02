use crate::access::SystemAccess;
use crate::world::World;

/// Scheduling priority for a system.
///
/// Higher-priority systems are scheduled first within a stage.
/// The default priority is [`Normal`](Self::Normal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum JobPriority {
    /// Reserved for systems that must run before everything else
    /// (e.g. transform propagation, animation skinning).
    Critical = 3,
    /// High-priority systems that should run early (e.g. physics, input).
    High = 2,
    /// Default priority for most game logic.
    #[default]
    Normal = 1,
    /// Low-priority systems that can run last (e.g. debug overlays, stats).
    Low = 0,
}

/// A system that operates on a [`World`].
///
/// Systems are the primary way to express game logic. They receive
/// mutable access to the world and can read/write components and resources.
pub trait System: Send + Sync {
    /// Execute this system against the given `world`.
    fn run(&self, world: &mut World);

    /// Return the access descriptor for this system.
    ///
    /// The default implementation returns empty access (no declared dependencies).
    /// Override this to enable parallel scheduling.
    fn access(&self) -> SystemAccess {
        SystemAccess::new()
    }

    /// Human-readable name for debugging and profiling.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    /// Scheduling priority for this system.
    ///
    /// Higher-priority systems are scheduled first within a stage.
    /// Defaults to [`JobPriority::Normal`].
    fn priority(&self) -> JobPriority {
        JobPriority::Normal
    }
}

/// Conversion trait from closures/functions into [`System`] instances.
///
/// Any `Fn(&mut World) + Send + Sync` automatically implements this trait.
pub trait IntoSystem {
    /// The concrete `System` type produced.
    type System: System;
    /// Convert into a boxed system.
    fn system(self) -> Self::System;
}

impl<F> IntoSystem for F
where
    F: Fn(&mut World) + Send + Sync,
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
    F: Fn(&mut World) + Send + Sync,
{
    fn run(&self, world: &mut World) {
        (self.0)(world);
    }
}

/// A system with explicitly declared access.
///
/// Wraps any system and overrides its access descriptor with
/// user-provided read/write declarations.
pub struct AccessSystem<S: System> {
    inner: S,
    access: SystemAccess,
}

impl<S: System> AccessSystem<S> {
    /// Wrap a system with explicit access declarations.
    pub fn new(inner: S, access: SystemAccess) -> Self {
        Self { inner, access }
    }
}

impl<S: System> System for AccessSystem<S> {
    fn run(&self, world: &mut World) {
        self.inner.run(world);
    }

    fn access(&self) -> SystemAccess {
        self.access.clone()
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn priority(&self) -> JobPriority {
        self.inner.priority()
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
