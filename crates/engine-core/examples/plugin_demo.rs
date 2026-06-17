//! Plugin System Demo
//!
//! Demonstrates how to use the plugin system to load and run dynamic plugins.
//!
//! Usage:
//! ```
//! cargo run --example plugin_demo -p engine-core
//! ```

use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;
use engine_core::plugins::CorePlugins;

/// Plugin that demonstrates the plugin system concept.
struct PluginDemoPlugin;

impl Plugin for PluginDemoPlugin {
    fn build(&self, _app: &mut AppBuilder) {
        println!("Plugin system plugin initialized.");
        println!("To use the plugin system, create a cdylib crate with a plugin.");
        println!("See examples/test-plugin/ for an example plugin.");
    }
}

fn main() {
    // Initialize logging
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .init();

    println!("=== Plugin System Demo ===");
    println!("This demo demonstrates the plugin system concept.");
    println!();
    println!("The plugin system allows loading dynamic plugins at runtime.");
    println!("Plugins are shared libraries (.dll/.so/.dylib) with a manifest.");
    println!();

    // Create app
    let mut app = AppBuilder::new();
    app.add_plugin(CorePlugins);
    app.add_plugin(PluginDemoPlugin);

    // Build and run one frame
    let mut app = app.build();
    app.run();

    println!();
    println!("Demo complete!");
}
