# Quick Start Guide

This guide walks you through creating your first RustEngine project.

## Prerequisites

- Rust 1.95.0+ (2024 edition)
- A GPU with wgpu support

## Creating a Project

```bash
cargo new my_game
cd my_game
```

Add RustEngine crates to your `Cargo.toml`:

```toml
[dependencies]
engine-core = { path = "../legendary-engine/crates/engine-core" }
engine-ecs = { path = "../legendary-engine/crates/engine-ecs" }
engine-window = { path = "../legendary-engine/crates/engine-window" }
engine-input = { path = "../legendary-engine/crates/engine-input" }
engine-render = { path = "../legendary-engine/crates/engine-render" }
engine-math = { path = "../legendary-engine/crates/engine-math" }
```

## Minimal Example

```rust
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;
use engine_ecs::world::World;

struct MyPlugin;

impl Plugin for MyPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(my_system);
    }
}

fn my_system(world: &mut World) {
    // Your game logic here
}

fn main() {
    let mut app = AppBuilder::new();
    app.add_plugin(MyPlugin);
    let mut app = app.build();

    // Game loop
    loop {
        app.run();
    }
}
```

## Next Steps

- [ECS Usage Tutorial](ecs-tutorial.md) — Learn about entities, components, and systems
- [Rendering Pipeline](rendering-pipeline.md) — Set up rendering
- [Physics System](physics-system.md) — Add physics simulation
- [Audio System](audio-system.md) — Play sounds and music
