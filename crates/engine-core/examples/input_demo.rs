use engine_core::app::{App, AppBuilder};
use engine_core::plugin::Plugin;
use std::collections::HashMap;

struct SetupPlugin;

impl Plugin for SetupPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let mut pressed_keys = HashMap::new();
        pressed_keys.insert("w".to_string(), true);
        pressed_keys.insert("d".to_string(), true);
        app.insert_resource(pressed_keys);
    }
}

pub fn main() {
    println!("=== RustEngine Input Demo Example ===");
    println!("This example demonstrates the input handling system architecture.\n");
    println!("Running 3 frames of simulation...\n");

    let mut app_builder = AppBuilder::new();
    app_builder.add_plugin(SetupPlugin);

    // 使用 pre_update_hook 来访问资源
    app_builder.add_pre_update_hook(Box::new(|app: &mut App| {
        if let Some(keys) = app.world.get_resource::<HashMap<String, bool>>() {
            println!("Keys pressed in this frame:");
            for (key, &pressed) in keys {
                if pressed {
                    println!("  - {} key is pressed", key.to_uppercase());
                }
            }
        }
    }));

    let mut app = app_builder.build();

    for frame in 1..=3 {
        println!("--- Frame {} ---", frame);
        app.run();
    }

    println!("\n=== Example Complete ===");
}
