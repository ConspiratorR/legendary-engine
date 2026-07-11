//! Mod System Demo
//!
//! Demonstrates how to load and run WASM mods.
//!
//! Usage:
//! ```
//! cargo run --example mod_demo -p engine-core
//! ```

use engine_core::app::AppBuilder;
use engine_core::plugins::CorePlugins;
use engine_script::prelude::{ModPlugin, mod_update_system};

fn main() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .init();

    println!("=== Mod System Demo ===");
    println!();

    let mut app = AppBuilder::new();
    app.add_plugin(CorePlugins);

    // Load WASM mods from the mods directory
    let mods_dir = std::path::Path::new("mods");
    if mods_dir.exists() {
        println!("Loading WASM mods from {:?}...", mods_dir);
        app.add_plugin(ModPlugin::new(mods_dir));
        app.add_system(mod_update_system);
    } else {
        println!("No 'mods' directory found. Skipping mod loading.");
        println!("To load WASM mods, create a 'mods' directory with mod subdirectories.");
        println!("Each mod directory should contain:");
        println!("  - mod.json (manifest)");
        println!("  - <entry_point>.wasm (compiled WASM module)");
    }

    // Build and run a few frames
    let mut app = app.build();
    for _ in 0..3 {
        app.run();
    }

    println!();
    println!("Demo complete!");
}
