use crate::app::AppBuilder;
use crate::logger::Logger;
use crate::memory::MemoryTracker;
use crate::plugin::Plugin;
use crate::profiler::Profiler;
use crate::time::Time;
use engine_input::action::ActionMap;

/// Plugin that registers an [`ActionMap`](engine_input::action::ActionMap) resource.
///
/// Add this plugin to enable action-based input mapping (e.g. "jump", "fire")
/// decoupled from raw key codes.
pub struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(ActionMap::new());
    }
}

/// Plugin that registers a [`Time`] resource and updates it each frame.
///
/// Inserts a pre-update hook that calls [`Time::update`] every frame,
/// keeping `delta_seconds`, `elapsed`, and `frame_count` current.
pub struct TimePlugin;

impl Plugin for TimePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(Time::new());

        // Add a pre-update hook to update time each frame
        app.add_pre_update_hook(Box::new(|app| {
            if let Some(time) = app.world_mut().get_resource_mut::<Time>() {
                time.update();
            }
        }));
    }
}

/// Plugin that registers a [`Logger`] resource with a configurable verbosity level.
///
/// # Example
///
/// ```rust
/// use engine_core::plugins::LoggerPlugin;
/// use engine_core::logger::LogLevel;
///
/// let plugin = LoggerPlugin::new(LogLevel::Debug);
/// ```
pub struct LoggerPlugin {
    level: crate::logger::LogLevel,
}

impl LoggerPlugin {
    /// Create a new logger plugin that filters messages below `level`.
    pub fn new(level: crate::logger::LogLevel) -> Self {
        Self { level }
    }
}

impl Plugin for LoggerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(Logger::new(self.level));
    }
}

/// Convenience plugin bundle that registers the most common core plugins:
/// [`TimePlugin`] and [`ActionPlugin`].
///
/// For logging as well, use [`CorePlugins::with_logging`].
pub struct CorePlugins;

impl CorePlugins {
    /// Create a plugin bundle that includes [`TimePlugin`], [`ActionPlugin`],
    /// and [`LoggerPlugin`] with the given log level.
    pub fn with_logging(level: crate::logger::LogLevel) -> impl Plugin {
        struct ConfigurablePlugins {
            log_level: crate::logger::LogLevel,
        }

        impl Plugin for ConfigurablePlugins {
            fn build(&self, app: &mut AppBuilder) {
                app.add_plugin(TimePlugin);
                app.add_plugin(ActionPlugin);
                app.add_plugin(LoggerPlugin::new(self.log_level));
            }
        }

        ConfigurablePlugins { log_level: level }
    }
}

impl Plugin for CorePlugins {
    fn build(&self, app: &mut AppBuilder) {
        app.add_plugin(TimePlugin);
        app.add_plugin(ActionPlugin);
        // InputManager is already added by AppBuilder::new()
    }
}

/// Plugin that registers a [`Profiler`] resource and hooks into the frame
/// lifecycle to collect per-frame timing data.
///
/// Automatically calls `begin_frame` in the pre-update hook and `end_frame`
/// in the post-update hook.
///
/// # Example
///
/// ```rust
/// use engine_core::plugins::ProfilerPlugin;
///
/// let plugin = ProfilerPlugin::new(120); // track last 120 frames
/// ```
pub struct ProfilerPlugin {
    max_frames: usize,
}

impl ProfilerPlugin {
    /// Create a profiler plugin that keeps the last `max_frames` frame records.
    pub fn new(max_frames: usize) -> Self {
        Self { max_frames }
    }
}

impl Default for ProfilerPlugin {
    fn default() -> Self {
        Self::new(120)
    }
}

impl Plugin for ProfilerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(Profiler::new(self.max_frames));

        // Pre-update: begin frame profiling
        app.add_pre_update_hook(Box::new(|app| {
            if let Some(profiler) = app.world_mut().get_resource_mut::<Profiler>() {
                profiler.begin_frame();
            }
        }));

        // Post-update: end frame profiling
        app.add_post_update_hook(Box::new(|app| {
            if let Some(profiler) = app.world_mut().get_resource_mut::<Profiler>() {
                profiler.end_frame();
            }
        }));
    }
}

/// Plugin that hooks into the post-update phase to take per-frame memory
/// snapshots via [`MemoryTracker::take_frame_snapshot`].
///
/// Useful for detecting memory leaks during development.
pub struct MemoryTrackerPlugin;

impl Plugin for MemoryTrackerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_post_update_hook(Box::new(|_app| {
            MemoryTracker::take_frame_snapshot();
        }));
    }
}
