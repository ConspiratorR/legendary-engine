use crate::app::AppBuilder;
use crate::logger::Logger;
use crate::plugin::Plugin;
use crate::time::Time;
use engine_input::action::ActionMap;

/// Action plugin, adds an ActionMap resource.
pub struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(ActionMap::new());
    }
}

/// Time plugin, adds a Time resource and updates it each frame.
pub struct TimePlugin;

impl Plugin for TimePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(Time::new());
        
        // Add a pre-update hook to update time each frame
        app.add_pre_update_hook(Box::new(|app| {
            if let Some(time) = app.resources_mut().get_mut::<Time>() {
                time.update();
            }
        }));
    }
}

/// Logger plugin, adds a Logger resource for logging.
pub struct LoggerPlugin {
    level: crate::logger::LogLevel,
}

impl LoggerPlugin {
    pub fn new(level: crate::logger::LogLevel) -> Self {
        Self { level }
    }
}

impl Plugin for LoggerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(Logger::new(self.level));
    }
}

/// A collection of core plugins that are typically needed.
pub struct CorePlugins;

impl CorePlugins {
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
