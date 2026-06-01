use crate::plugin::Plugin;
use crate::resource::ResourceRegistry;
use engine_ecs::schedule::Schedule;
use engine_ecs::world::World;
use engine_input::input_manager::InputManager;

type Hook = Box<dyn FnMut(&mut App)>;

/// Builder for constructing an [`App`] with plugins, systems, and resources.
///
/// # Example
///
/// ```
/// use engine_core::app::AppBuilder;
///
/// let mut app = AppBuilder::new();
/// // app.add_plugin(MyPlugin);
/// // app.add_system(my_system.system());
/// let app = app.build();
/// ```
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
    /// Create a new builder with default resources (including [`InputManager`]).
    pub fn new() -> Self {
        let mut world = World::new();
        world.insert_resource(InputManager::new());
        Self {
            world,
            schedule: Schedule::new(),
            resources: ResourceRegistry::new(),
            pre_update_hooks: Vec::new(),
            post_update_hooks: Vec::new(),
            post_render_hooks: Vec::new(),
        }
    }

    /// Register a plugin.
    pub fn add_plugin(&mut self, plugin: impl Plugin) -> &mut Self {
        plugin.build(self);
        self
    }

    /// Add a system to the update schedule.
    pub fn add_system(
        &mut self,
        system: impl engine_ecs::system::IntoSystem + 'static,
    ) -> &mut Self {
        self.schedule.add_system(system.system());
        self
    }

    /// Insert a global resource into the ECS world.
    pub fn insert_resource<T: 'static>(&mut self, resource: T) -> &mut Self {
        self.world.insert_resource(resource);
        self
    }

    /// Get mutable access to the ECS world.
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Get shared access to the system schedule.
    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// Get shared access to the resource registry.
    pub fn resources(&self) -> &ResourceRegistry {
        &self.resources
    }

    /// Get mutable access to the resource registry.
    pub fn resources_mut(&mut self) -> &mut ResourceRegistry {
        &mut self.resources
    }

    /// Register a hook that runs before the update phase.
    pub fn add_pre_update_hook(&mut self, hook: Hook) -> &mut Self {
        self.pre_update_hooks.push(hook);
        self
    }

    /// Register a hook that runs after the update phase.
    pub fn add_post_update_hook(&mut self, hook: Hook) -> &mut Self {
        self.post_update_hooks.push(hook);
        self
    }

    /// Register a hook that runs after the render phase.
    pub fn add_post_render_hook(&mut self, hook: Hook) -> &mut Self {
        self.post_render_hooks.push(hook);
        self
    }

    /// Finalize the builder and produce an [`App`].
    pub fn build(self) -> App {
        App::from(self)
    }
}

/// The main application, holding the ECS world, schedule, and renderer.
///
/// Call [`run`](Self::run) each frame (or in a loop) to execute
/// pre-update hooks → input frame advance → systems → post-update hooks.
pub struct App {
    /// The ECS world.
    pub world: World,
    /// The system schedule.
    pub schedule: Schedule,
    /// The resource registry.
    pub resources: ResourceRegistry,
    renderer: Option<engine_render::renderer::Renderer>,
    /// Hooks executed before the update phase.
    pub pre_update_hooks: Vec<Hook>,
    /// Hooks executed after the update phase.
    pub post_update_hooks: Vec<Hook>,
    /// Hooks executed after the render phase.
    pub post_render_hooks: Vec<Hook>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Create a new app with default resources.
    pub fn new() -> Self {
        let mut world = World::new();
        world.insert_resource(InputManager::new());
        Self {
            world,
            schedule: Schedule::new(),
            resources: ResourceRegistry::new(),
            renderer: None,
            pre_update_hooks: Vec::new(),
            post_update_hooks: Vec::new(),
            post_render_hooks: Vec::new(),
        }
    }

    /// Execute one frame: pre-hooks → input update → systems → post-hooks.
    pub fn run(&mut self) {
        let mut pre_hooks = std::mem::take(&mut self.pre_update_hooks);
        for hook in &mut pre_hooks {
            hook(self);
        }
        self.pre_update_hooks = pre_hooks;
        if let Some(input) = self.world.get_resource_mut::<InputManager>() {
            input.update_frame();
        }
        self.schedule.run(&mut self.world);
        let mut post_hooks = std::mem::take(&mut self.post_update_hooks);
        for hook in &mut post_hooks {
            hook(self);
        }
        self.post_update_hooks = post_hooks;
    }

    /// Run a single frame (alias for [`run`](Self::run)).
    pub fn run_with_resources(&mut self) {
        self.run();
    }

    /// Run only the systems (no hooks, no input update).
    pub fn run_old(&mut self) {
        self.schedule.run(&mut self.world);
    }

    /// Get mutable access to the [`InputManager`] resource.
    pub fn input_mut(&mut self) -> &mut InputManager {
        self.world.get_resource_mut::<InputManager>().unwrap()
    }

    /// Get a shared reference to the renderer, if set.
    pub fn renderer(&self) -> Option<&engine_render::renderer::Renderer> {
        self.renderer.as_ref()
    }

    /// Get a mutable reference to the renderer, if set.
    pub fn renderer_mut(&mut self) -> Option<&mut engine_render::renderer::Renderer> {
        self.renderer.as_mut()
    }

    /// Set the renderer, inserting its device and queue as resources.
    pub fn set_renderer(&mut self, renderer: engine_render::renderer::Renderer) {
        self.world.insert_resource(renderer.device.clone());
        self.world.insert_resource(renderer.queue.clone());
        self.renderer = Some(renderer);
    }

    /// Get mutable access to the ECS world.
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Get mutable access to the resource registry.
    pub fn resources_mut(&mut self) -> &mut ResourceRegistry {
        &mut self.resources
    }

    /// Get shared access to the system schedule.
    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// Borrow the renderer and resources mutably at the same time.
    pub fn split_renderer_mut(
        &mut self,
    ) -> (
        Option<&mut engine_render::renderer::Renderer>,
        &mut ResourceRegistry,
    ) {
        (self.renderer.as_mut(), &mut self.resources)
    }

    /// Borrow the renderer immutably and resources mutably at the same time.
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
