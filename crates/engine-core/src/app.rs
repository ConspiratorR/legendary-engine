use crate::plugin::Plugin;
use crate::resource::ResourceRegistry;
use engine_ecs::schedule::Schedule;
use engine_ecs::world::World;
use engine_input::input_manager::InputManager;

type Hook = Box<dyn FnMut(&mut App)>;

pub struct AppBuilder {
    world: World,
    schedule: Schedule,
    resources: ResourceRegistry,
    pre_update_hooks: Vec<Hook>,
    post_update_hooks: Vec<Hook>,
    post_render_hooks: Vec<Hook>,
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AppBuilder {
    pub fn new() -> Self {
        let mut resources = ResourceRegistry::new();
        resources.insert(InputManager::new());
        Self {
            world: World::new(),
            schedule: Schedule::new(),
            resources,
            pre_update_hooks: Vec::new(),
            post_update_hooks: Vec::new(),
            post_render_hooks: Vec::new(),
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

    pub fn add_pre_update_hook(&mut self, hook: Hook) -> &mut Self {
        self.pre_update_hooks.push(hook);
        self
    }

    pub fn add_post_update_hook(&mut self, hook: Hook) -> &mut Self {
        self.post_update_hooks.push(hook);
        self
    }

    pub fn add_post_render_hook(&mut self, hook: Hook) -> &mut Self {
        self.post_render_hooks.push(hook);
        self
    }

    pub fn build(self) -> App {
        App::from(self)
    }
}

pub struct App {
    pub world: World,
    pub schedule: Schedule,
    pub resources: ResourceRegistry,
    renderer: Option<engine_render::renderer::Renderer>,
    pub pre_update_hooks: Vec<Hook>,
    pub post_update_hooks: Vec<Hook>,
    pub post_render_hooks: Vec<Hook>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let mut resources = ResourceRegistry::new();
        resources.insert(InputManager::new());
        Self {
            world: World::new(),
            schedule: Schedule::new(),
            resources,
            renderer: None,
            pre_update_hooks: Vec::new(),
            post_update_hooks: Vec::new(),
            post_render_hooks: Vec::new(),
        }
    }

    pub fn run(&mut self) {
        let mut pre_hooks = std::mem::take(&mut self.pre_update_hooks);
        for hook in &mut pre_hooks {
            hook(self);
        }
        self.pre_update_hooks = pre_hooks;
        if let Some(input) = self.resources.get_mut::<InputManager>() {
            input.update_frame();
        }
        self.schedule.run(&mut self.world);
        let mut post_hooks = std::mem::take(&mut self.post_update_hooks);
        for hook in &mut post_hooks {
            hook(self);
        }
        self.post_update_hooks = post_hooks;
    }

    pub fn run_old(&mut self) {
        self.schedule.run(&mut self.world);
    }

    pub fn input_mut(&mut self) -> &mut InputManager {
        self.resources.get_mut::<InputManager>().unwrap()
    }

    pub fn renderer(&self) -> Option<&engine_render::renderer::Renderer> {
        self.renderer.as_ref()
    }

    pub fn renderer_mut(&mut self) -> Option<&mut engine_render::renderer::Renderer> {
        self.renderer.as_mut()
    }

    pub fn set_renderer(&mut self, renderer: engine_render::renderer::Renderer) {
        self.resources
            .insert(renderer.device.clone());
        self.resources
            .insert(renderer.queue.clone());
        self.renderer = Some(renderer);
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn resources_mut(&mut self) -> &mut ResourceRegistry {
        &mut self.resources
    }

    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    pub fn split_renderer_mut(
        &mut self,
    ) -> (
        Option<&mut engine_render::renderer::Renderer>,
        &mut ResourceRegistry,
    ) {
        (self.renderer.as_mut(), &mut self.resources)
    }

    pub fn split_renderer_ref(
        &mut self,
    ) -> (
        Option<&engine_render::renderer::Renderer>,
        &mut ResourceRegistry,
    ) {
        (self.renderer.as_ref(), &mut self.resources)
    }
}

impl From<AppBuilder> for App {
    fn from(b: AppBuilder) -> Self {
        Self {
            world: b.world,
            schedule: b.schedule,
            resources: b.resources,
            renderer: None,
            pre_update_hooks: b.pre_update_hooks,
            post_update_hooks: b.post_update_hooks,
            post_render_hooks: b.post_render_hooks,
        }
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
