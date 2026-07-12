use crate::plugin::Plugin;
use crate::resource::ResourceRegistry;
use engine_ecs::schedule::{ParallelSchedule, Schedule};
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
    parallel_schedule: Option<ParallelSchedule>,
    resources: ResourceRegistry,
    pre_update_hooks: Vec<Hook>,
    post_update_hooks: Vec<Hook>,
    post_render_hooks: Vec<Hook>,
    player_loop: crate::player_loop::PlayerLoop,
    time: crate::time::Time,
    events: crate::event::EventBus,
    plugins: Vec<Box<dyn Plugin>>,
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
            parallel_schedule: None,
            resources: ResourceRegistry::new(),
            pre_update_hooks: Vec::new(),
            post_update_hooks: Vec::new(),
            post_render_hooks: Vec::new(),
            player_loop: crate::player_loop::PlayerLoop::new(),
            time: crate::time::Time::new(),
            events: crate::event::EventBus::new(),
            plugins: Vec::new(),
        }
    }

    /// Enable parallel scheduling with the given number of threads.
    ///
    /// When enabled, `add_system` adds to the parallel schedule instead
    /// of the sequential schedule. Systems with non-conflicting access
    /// descriptors are grouped into parallel stages.
    pub fn with_parallel_schedule(&mut self, threads: usize) -> &mut Self {
        self.parallel_schedule = Some(ParallelSchedule::new(threads));
        self
    }

    /// Register a plugin.
    pub fn add_plugin(&mut self, plugin: impl Plugin) -> &mut Self {
        plugin.build(self);
        self
    }

    /// Register a plugin for deferred execution.
    ///
    /// Unlike [`add_plugin`](Self::add_plugin), which calls `build()` immediately,
    /// this method stores the plugin and runs it during [`build`](Self::build).
    pub fn add_plugin_stored(&mut self, plugin: impl Plugin + 'static) -> &mut Self {
        self.plugins.push(Box::new(plugin));
        self
    }

    /// Load all dynamic plugins from a directory.
    ///
    /// Each plugin directory must contain a `plugin.json` manifest and
    /// a shared library (`.dll`, `.so`, or `.dylib`).
    ///
    /// # Safety
    /// This function loads shared libraries and calls their entry points.
    /// Plugins must be compiled for the correct target platform.
    pub fn load_dynamic_plugins(
        &mut self,
        plugins_dir: &std::path::Path,
    ) -> Result<&mut Self, Box<dyn std::error::Error>> {
        use crate::plugin_loader::PluginLoader;

        let mut loader = PluginLoader::new(plugins_dir.join("registry.json"))
            .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
        unsafe { loader.load_all() }.map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
        loader.register_all(self);
        Ok(self)
    }

    /// Add a system to the update schedule (parallel if enabled, sequential otherwise).
    pub fn add_system(
        &mut self,
        system: impl engine_ecs::system::IntoSystem + 'static,
    ) -> &mut Self {
        if let Some(ref mut ps) = self.parallel_schedule {
            ps.add_system(system.system());
        } else {
            self.schedule.add_system(system.system());
        }
        self
    }

    /// Insert a global resource into the ECS world.
    pub fn insert_resource<T: Send + Sync + 'static>(&mut self, resource: T) -> &mut Self {
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

    /// Get shared access to the event bus.
    pub fn events(&self) -> &crate::event::EventBus {
        &self.events
    }

    /// Get mutable access to the event bus.
    pub fn events_mut(&mut self) -> &mut crate::event::EventBus {
        &mut self.events
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

    /// Add a system to a specific phase (Unity-like PlayerLoop system).
    pub fn add_system_to_phase(
        &mut self,
        phase: crate::player_loop::Phase,
        system: impl crate::system::System + 'static,
    ) -> &mut Self {
        self.player_loop.add_system(phase, system);
        self
    }

    /// Add a startup system (runs once before first frame).
    pub fn add_startup_system(
        &mut self,
        system: impl crate::system::System + 'static,
    ) -> &mut Self {
        self.player_loop.add_startup_system(system);
        self
    }

    /// Add a system to the FixedUpdate phase.
    pub fn add_fixed_update_system(
        &mut self,
        system: impl crate::system::System + 'static,
    ) -> &mut Self {
        self.player_loop
            .add_system(crate::player_loop::Phase::FixedUpdate, system);
        self
    }

    /// Add a system to the LateUpdate phase.
    pub fn add_late_update_system(
        &mut self,
        system: impl crate::system::System + 'static,
    ) -> &mut Self {
        self.player_loop
            .add_system(crate::player_loop::Phase::LateUpdate, system);
        self
    }

    /// Get mutable access to the player loop.
    pub fn player_loop_mut(&mut self) -> &mut crate::player_loop::PlayerLoop {
        &mut self.player_loop
    }

    /// Get shared access to the time.
    pub fn time(&self) -> &crate::time::Time {
        &self.time
    }

    /// Finalize the builder and produce an [`App`].
    ///
    /// Runs any stored plugins (via [`add_plugin_stored`](Self::add_plugin_stored))
    /// before producing the final App.
    pub fn build(mut self) -> App {
        // Run stored plugins
        let plugins: Vec<Box<dyn Plugin>> = std::mem::take(&mut self.plugins);
        for plugin in plugins {
            plugin.build(&mut self);
        }
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
    /// The system schedule (used when parallel schedule is not set).
    pub schedule: Schedule,
    /// The parallel schedule (if enabled).
    pub parallel_schedule: Option<ParallelSchedule>,
    /// The resource registry.
    pub resources: ResourceRegistry,
    renderer: Option<engine_render::renderer::Renderer>,
    /// Asset registry for texture loading (lazily initialized in render_phase).
    /// Registry is not Send+Sync but is only accessed from the main thread.
    asset_registry: Option<engine_asset::registry::Registry>,
    /// Hooks executed before the update phase.
    pub pre_update_hooks: Vec<Hook>,
    /// Hooks executed after the update phase.
    pub post_update_hooks: Vec<Hook>,
    /// Hooks executed after the render phase.
    pub post_render_hooks: Vec<Hook>,
    player_loop: crate::player_loop::PlayerLoop,
    time: crate::time::Time,
    events: crate::event::EventBus,
    running: bool,
    frame: u64,
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
            parallel_schedule: None,
            resources: ResourceRegistry::new(),
            renderer: None,
            asset_registry: None,
            pre_update_hooks: Vec::new(),
            post_update_hooks: Vec::new(),
            post_render_hooks: Vec::new(),
            player_loop: crate::player_loop::PlayerLoop::new(),
            time: crate::time::Time::new(),
            events: crate::event::EventBus::new(),
            running: false,
            frame: 0,
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
        if let Some(ref mut ps) = self.parallel_schedule {
            ps.run(&mut self.world);
        } else {
            self.schedule.run(&mut self.world);
        }
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
        if let Some(ref mut ps) = self.parallel_schedule {
            ps.run(&mut self.world);
        } else {
            self.schedule.run(&mut self.world);
        }
    }

    /// Get mutable access to the [`InputManager`] resource.
    pub fn input_mut(&mut self) -> &mut InputManager {
        self.world
            .get_resource_mut::<InputManager>()
            .expect("InputManager resource must be inserted before use")
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

    /// Get shared access to the event bus.
    pub fn events(&self) -> &crate::event::EventBus {
        &self.events
    }

    /// Get mutable access to the event bus.
    pub fn events_mut(&mut self) -> &mut crate::event::EventBus {
        &mut self.events
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

    /// Execute the render phase: collect cameras and sprites from ECS, then render.
    ///
    /// This is called automatically by `run_default` in the event loop.
    /// If no renderer is set or no cameras exist, this is a no-op.
    pub fn render_phase(&mut self) {
        use engine_render::camera::Camera;
        use engine_render::sprite::Sprite;
        use engine_render::texture_bridge::TextureBridge;

        let has_renderer = self.renderer.is_some();
        if !has_renderer {
            return;
        }

        // Collect cameras (clone to avoid borrow conflict)
        let cameras: Vec<Camera> = {
            let entities = self.world.component_entities::<Camera>();
            entities
                .iter()
                .filter_map(|&idx| self.world.get_by_index::<Camera>(idx).cloned())
                .collect()
        };

        if cameras.is_empty() {
            return;
        }

        // Collect sprites (clone to avoid borrow conflict)
        let sprites: Vec<Sprite> = {
            let entities = self.world.component_entities::<Sprite>();
            entities
                .iter()
                .filter_map(|&idx| self.world.get_by_index::<Sprite>(idx).cloned())
                .collect()
        };

        // Get TextureBridge from world
        let Some(bridge) = self.world.get_resource_mut::<TextureBridge>() else {
            return;
        };

        // Lazily initialize asset registry (persists across frames for texture loading)
        if self.asset_registry.is_none() {
            self.asset_registry = Some(engine_asset::registry::Registry::new());
        }
        let registry = self.asset_registry.as_ref().unwrap();

        // Render (renderer is a separate field, no borrow conflict with world)
        let camera_refs: Vec<&Camera> = cameras.iter().collect();
        let renderer = self.renderer.as_mut().unwrap();
        let _ = renderer.render_frame(&camera_refs, &sprites, bridge, registry);
    }

    /// Run one frame with delta time (Unity-like Update).
    ///
    /// Updates the time and increments the frame counter.
    /// Note: PlayerLoop integration requires the local World type;
    /// use the existing [`run`](Self::run) method for engine_ecs systems.
    pub fn update(&mut self, delta: f32) {
        self.time.update(delta);
        self.frame += 1;
    }

    /// Run fixed update step (Unity-like FixedUpdate).
    ///
    /// Updates the fixed time step.
    /// Note: PlayerLoop integration requires the local World type;
    /// use the existing [`run`](Self::run) method for engine_ecs systems.
    pub fn fixed_update(&mut self) {
        self.time.update_fixed();
    }

    /// Check if the app is running.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Set the running state.
    pub fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    /// Get the current frame number.
    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// Quit the application (Unity-like Application.Quit).
    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Get a reference to the time.
    pub fn time_ref(&self) -> &crate::time::Time {
        &self.time
    }

    /// Get a mutable reference to the time.
    pub fn time_mut(&mut self) -> &mut crate::time::Time {
        &mut self.time
    }

    /// Get a reference to the player loop.
    pub fn player_loop(&self) -> &crate::player_loop::PlayerLoop {
        &self.player_loop
    }

    /// Get a mutable reference to the player loop.
    pub fn player_loop_mut(&mut self) -> &mut crate::player_loop::PlayerLoop {
        &mut self.player_loop
    }
}

impl From<AppBuilder> for App {
    fn from(b: AppBuilder) -> Self {
        Self {
            world: b.world,
            schedule: b.schedule,
            parallel_schedule: b.parallel_schedule,
            resources: b.resources,
            renderer: None,
            asset_registry: None,
            pre_update_hooks: b.pre_update_hooks,
            post_update_hooks: b.post_update_hooks,
            post_render_hooks: b.post_render_hooks,
            player_loop: b.player_loop,
            time: b.time,
            events: b.events,
            running: false,
            frame: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::AppBuilder;
    use crate::plugin::Plugin;
    use engine_ecs::world::World;

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

    #[test]
    fn test_parallel_schedule_builder() {
        let mut app = AppBuilder::new();
        app.with_parallel_schedule(4);
        app.add_system(|_world: &mut World| {});
        assert!(app.build().parallel_schedule.is_some());
    }

    #[test]
    fn test_load_dynamic_plugins_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let mut builder = AppBuilder::new();
        let result = builder.load_dynamic_plugins(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_app_builder_phase_systems() {
        let builder = AppBuilder::new();
        let app = builder.build();
        assert_eq!(app.player_loop().system_count(), 0);
    }

    #[test]
    fn test_app_builder_phase_systems_with_mutable() {
        use crate::player_loop::Phase;

        let mut builder = AppBuilder::new();
        builder.add_system_to_phase(Phase::Update, |_: &mut crate::context::Context| {});
        builder.add_late_update_system(|_: &mut crate::context::Context| {});

        let app = builder.build();
        assert_eq!(app.player_loop().system_count(), 2);
    }

    #[test]
    fn test_app_builder_startup_system() {
        let mut builder = AppBuilder::new();
        builder.add_startup_system(|_: &mut crate::context::Context| {});
        builder.add_system_to_phase(
            crate::player_loop::Phase::Update,
            |_: &mut crate::context::Context| {},
        );

        let app = builder.build();
        assert_eq!(app.player_loop().system_count(), 1);
        assert_eq!(app.player_loop().startup_system_count(), 1);
    }

    #[test]
    fn test_app_fixed_update() {
        let mut builder = AppBuilder::new();
        builder.add_fixed_update_system(|_: &mut crate::context::Context| {});

        let mut app = builder.build();
        // fixed_update updates time
        let before = app.time_ref().time();
        app.fixed_update();
        assert!(app.time_ref().in_fixed_time_step());
        // Time should not advance in fixed_update (only sets the flag)
        assert_eq!(app.time_ref().time(), before);
    }

    #[test]
    fn test_app_quit_and_running() {
        let builder = AppBuilder::new();
        let mut app = builder.build();

        assert!(!app.is_running());
        app.set_running(true);
        assert!(app.is_running());

        app.quit();
        assert!(!app.is_running());
    }

    #[test]
    fn test_app_frame_count() {
        let builder = AppBuilder::new();
        let mut app = builder.build();

        assert_eq!(app.frame(), 0);
        app.update(0.016);
        assert_eq!(app.frame(), 1);
        app.update(0.016);
        assert_eq!(app.frame(), 2);
    }

    #[test]
    fn test_plugin_name() {
        struct MyPlugin;
        impl crate::plugin::Plugin for MyPlugin {
            fn build(&self, _app: &mut AppBuilder) {}
        }
        assert!(MyPlugin.name().contains("MyPlugin"));
    }

    #[test]
    fn test_closure_plugin() {
        let mut builder = AppBuilder::new();
        builder.add_plugin(|_app: &mut AppBuilder| {});
        // Should compile and not panic
        let _app = builder.build();
    }
}
