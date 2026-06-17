//! Mod System Demo
//!
//! Demonstrates how to use the mod system to load and run WASM mods.
//!
//! Usage:
//! ```
//! cargo run --example mod_demo -p engine-core
//! ```

use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;
use engine_core::plugins::CorePlugins;

/// Plugin that demonstrates the mod system concept.
struct ModDemoPlugin;

impl Plugin for ModDemoPlugin {
    fn build(&self, _app: &mut AppBuilder) {
        println!("Mod system plugin initialized.");
        println!("To use the mod system, add engine-script as a dependency.");
        println!("See examples/test-mod/ for an example mod.");
    }
}

fn main() {
    // Initialize logging
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .init();

    println!("=== Mod System Demo ===");
    println!("This demo demonstrates the mod system concept.");
    println!();
    println!("The mod system allows loading WASM mods at runtime.");
    println!("Mods can register new components, systems, and assets.");
    println!();

    // Create app
    let mut app = AppBuilder::new();
    app.add_plugin(CorePlugins);
    app.add_plugin(ModDemoPlugin);

    // Build and run one frame
    let mut app = app.build();
    app.run();

    println!();
    println!("Demo complete!");
}
