use engine_core::plugin::Plugin;
use engine_core::app::AppBuilder;

/// A simple test plugin that demonstrates the dynamic plugin system.
pub struct TestPlugin;

impl Plugin for TestPlugin {
    fn build(&self, app: &mut AppBuilder) {
        log::info!("TestPlugin: build() called");
        // Register a test resource
        app.insert_resource(TestPluginData {
            message: "Hello from TestPlugin!".to_string(),
        });
    }
}

/// Test resource registered by the plugin.
#[derive(Debug, Clone)]
pub struct TestPluginData {
    pub message: String,
}

/// Entry point function that the plugin loader will call.
///
/// This function must be `extern "C"` and return a raw pointer to a Plugin.
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn Plugin {
    Box::into_raw(Box::new(TestPlugin))
}
