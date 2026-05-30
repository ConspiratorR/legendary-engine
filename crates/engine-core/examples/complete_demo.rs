use engine_core::app::{App, AppBuilder};
use engine_core::plugin::Plugin;
use engine_core::plugins::CorePlugins;
use engine_core::time::Time;

/// A simple demo plugin that shows time and input
struct DemoPlugin;

impl Plugin for DemoPlugin {
    fn build(&self, _app: &mut AppBuilder) {
        // Demo plugin is simple for now
    }
}

pub fn main() {
    println!("=== RustEngine Complete Demo ===\n");
    println!("This demo shows the complete engine functionality:");
    println!("  - Time management");
    println!("  - Resource system");
    println!("  - Plugin system\n");

    // Create the app
    let mut app_builder = AppBuilder::new();
    app_builder.add_plugin(CorePlugins);
    app_builder.add_plugin(DemoPlugin);

    // Add a hook to show time info
    app_builder.add_post_update_hook(Box::new(|app: &mut App| {
        if let Some(time) = app.world.get_resource::<Time>() {
            let frame = time.frame_count();
            let fps = time.fps();
            let elapsed = time.elapsed_seconds();

            // Print every second
            if frame % 60 == 0 {
                println!(
                    "[Frame {}] Elapsed: {:.1}s | FPS: {:.1}",
                    frame, elapsed, fps
                );
            }
        }
    }));

    println!("Simulating 300 frames (about 5 seconds at 60fps)...\n");

    // Build and run a quick simulation
    let mut app = app_builder.build();
    for _ in 0..300 {
        app.run();
    }

    println!("\n=== Demo Complete ===");
    println!("Try:");
    println!("  - cargo run --example basic -p engine-core");
    println!("  - cargo run --example input_demo -p engine-core");
    println!("  - cargo test --all");
}
