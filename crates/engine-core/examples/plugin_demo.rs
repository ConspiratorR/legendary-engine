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
        println!("Static plugin initialized.");
    }
}

fn main() {
    // Initialize logging
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .init();

    println!("=== Plugin System Demo ===");
    println!();

    // Create app
    let mut app = AppBuilder::new();
    app.add_plugin(CorePlugins);
    app.add_plugin(PluginDemoPlugin);

    // Load dynamic plugins from a directory (if it exists)
    let plugins_dir = std::path::Path::new("plugins");
    if plugins_dir.exists() {
        println!("Loading dynamic plugins from {:?}...", plugins_dir);
        match app.load_dynamic_plugins(plugins_dir) {
            Ok(_) => println!("Dynamic plugins loaded successfully."),
            Err(e) => println!("No dynamic plugins loaded: {e}"),
        }
    } else {
        println!("No 'plugins' directory found. Skipping dynamic plugin loading.");
        println!(
            "To load dynamic plugins, create a 'plugins' directory with plugin subdirectories."
        );
        println!("Each plugin directory should contain:");
        println!("  - plugin.json (manifest)");
        println!("  - <name>plugin.dll (Windows) or lib<name>plugin.so (Linux)");
    }

    // Build and run one frame
    let mut app = app.build();
    app.run();

    println!();
    println!("Demo complete!");
}
