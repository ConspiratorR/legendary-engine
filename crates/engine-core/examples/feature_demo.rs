use engine_core::app::{App, AppBuilder};
use engine_core::config::Config;
use engine_core::plugin::Plugin;
use engine_core::plugins::CorePlugins;
use engine_core::profiler::Profiler;
use engine_core::time::Time;

struct SetupPlugin;

impl Plugin for SetupPlugin {
    fn build(&self, app: &mut AppBuilder) {
        // Insert some config
        let mut config = Config::new();
        config.set("game.title".to_string(), "My Awesome Game".to_string());
        config.set("game.version".to_string(), "1.0.0".to_string());
        config.set("player.health".to_string(), "100".to_string());
        config.set("player.speed".to_string(), "5.0".to_string());
        app.insert_resource(config);
        
        // Insert profiler
        app.insert_resource(Profiler::new(60));
    }
}

pub fn main() {
    println!("=== RustEngine Feature Demo ===\n");
    println!("This demo showcases the full feature set:");
    println!("  - Configuration system");
    println!("  - Performance profiling");
    println!("  - Time management");
    println!("  - Resource management\n");
    
    let mut app_builder = AppBuilder::new();
    app_builder.add_plugin(CorePlugins);
    app_builder.add_plugin(SetupPlugin);
    
    // Add profiling hooks
    app_builder.add_pre_update_hook(Box::new(|app| {
        if let Some(profiler) = app.resources.get_mut::<Profiler>() {
            profiler.start("update");
        }
        
        if let Some(time) = app.resources.get_mut::<Time>() {
            time.update();
        }
    }));
    
    app_builder.add_post_update_hook(Box::new(|app| {
        if let Some(profiler) = app.resources.get_mut::<Profiler>() {
            profiler.end("update");
            profiler.record_frame();
        }
    }));
    
    // Add stats display hook
    app_builder.add_post_update_hook(Box::new(|app: &mut App| {
        if let Some(time) = app.resources.get::<Time>() {
            if time.frame_count() % 60 == 0 {
                if let Some(config) = app.resources.get::<Config>() {
                    println!("\n--- Configuration ---");
                    println!("Game Title: {}", config.get("game.title").map_or("Unknown", |v| v));
                    println!("Version: {}", config.get("game.version").map_or("0.0.0", |v| v));
                    println!("Player Health: {}", config.get("player.health").map_or("100", |v| v));
                    println!("Player Speed: {}", config.get("player.speed").map_or("1.0", |v| v));
                }
                
                if let Some(profiler) = app.resources.get::<Profiler>() {
                    profiler.print_stats();
                }
            }
        }
    }));
    
    println!("Running 300 frames (5 seconds at 60fps)...\n");
    
    let mut app = app_builder.build();
    for _ in 0..300 {
        app.run();
    }
    
    println!("\n=== Demo Complete ===");
}
