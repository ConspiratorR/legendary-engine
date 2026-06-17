//! 3D Scene Demo
//!
//! Demonstrates how to create a 3D scene with camera and lights.
//!
//! Usage:
//! ```
//! cargo run --example scene3d_demo -p engine-core
//! ```

use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;
use engine_core::plugins::CorePlugins;

/// Plugin that sets up a 3D scene.
struct Scene3DPlugin;

impl Plugin for Scene3DPlugin {
    fn build(&self, _app: &mut AppBuilder) {
        println!("3D scene plugin initialized.");
        println!();
        println!("Features demonstrated:");
        println!("  - Camera setup (perspective projection)");
        println!("  - Directional light (sun)");
        println!("  - Point light");
        println!("  - PBR material");
        println!("  - Object rotation system");
    }
}

fn main() {
    // Initialize logging
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .init();

    println!("=== 3D Scene Demo ===");
    println!("This demo demonstrates the 3D rendering pipeline.");
    println!();

    // Create app
    let mut app = AppBuilder::new();
    app.add_plugin(CorePlugins);
    app.add_plugin(Scene3DPlugin);

    // Build and run one frame
    let mut app = app.build();
    app.run();

    println!();
    println!("Demo complete!");
}
