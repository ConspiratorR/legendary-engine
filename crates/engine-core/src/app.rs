use crate::plugin::Plugin;
use crate::resource::ResourceRegistry;
use engine_ecs::schedule::Schedule;
use engine_ecs::world::World;

pub struct AppBuilder {
    world: World,
    schedule: Schedule,
    resources: ResourceRegistry,
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AppBuilder {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            schedule: Schedule::new(),
            resources: ResourceRegistry::new(),
        }
    }

    pub fn add_plugin(&mut self, plugin: impl Plugin) -> &mut Self {
        plugin.build(self);
        self
    }

    pub fn add_system(
        &mut self,
        system: impl engine_ecs::system::IntoSystem + 'static,
    ) -> &mut Self {
        self.schedule.add_system(system.system());
        self
    }

    pub fn insert_resource<T: 'static>(&mut self, resource: T) -> &mut Self {
        self.resources.insert(resource);
        self
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    pub fn resources(&self) -> &ResourceRegistry {
        &self.resources
    }

    pub fn resources_mut(&mut self) -> &mut ResourceRegistry {
        &mut self.resources
    }
}

#[cfg(test)]
mod tests {
    use crate::app::AppBuilder;
    use crate::plugin::Plugin;

    struct CounterPlugin(u32);

    impl Plugin for CounterPlugin {
        fn build(&self, app: &mut AppBuilder) {
            let world = app.world_mut();
            let e = world.spawn();
            world.add_component(e, self.0);
        }
    }

    #[test]
    fn test_plugin_adds_data_to_world() {
        let mut app = AppBuilder::new();
        app.add_plugin(CounterPlugin(42));
        let world = app.world_mut();
        let entities = world.component_entities::<u32>();
        assert_eq!(entities.len(), 1);
        let val = world.get_by_index::<u32>(entities[0]).unwrap();
        assert_eq!(*val, 42);
    }
}
