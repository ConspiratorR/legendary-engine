use crate::world::World;

pub trait System {
    fn run(&self, world: &mut World);
}

pub trait IntoSystem {
    type System: System;
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
