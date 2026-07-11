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
engine-core = { path = "../RustEngine/crates/engine-core" }
engine-ecs = { path = "../RustEngine/crates/engine-ecs" }
engine-math = { path = "../RustEngine/crates/engine-math" }
```

## Minimal Example

```rust
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;
use engine_ecs::query::QueryPair;
use engine_ecs::system::IntoSystem;
use engine_ecs::world::World;
use engine_math::Vec3;

struct Position(Vec3);
struct Velocity(Vec3);

struct MyPlugin;

impl Plugin for MyPlugin {
    fn build(&self, app: &mut AppBuilder) {
        // Create entities with components
        let world = app.world_mut();
        let player = world.spawn();
        world.add_component(player, Position(Vec3::new(0.0, 0.0, 0.0)));
        world.add_component(player, Velocity(Vec3::new(1.0, 2.0, 0.0)));

        // Register systems
        app.add_system(movement_system());
    }
}

fn movement_system() -> impl IntoSystem {
    |world: &mut World| {
        let query = QueryPair::<Position, Velocity>::new();
        for (pos, vel) in query.iter_mut(world) {
            pos.0 += vel.0 * 0.016;
        }
    }
}

fn main() {
    let mut app = AppBuilder::new();
    app.add_plugin(MyPlugin);
    let mut app = app.build();

    // Run 60 frames of simulation
    for _ in 0..60 {
        app.run();
    }
}
```

## Running with Full Window + Rendering

For a complete windowed application with rendering:

```rust
use engine_core::app::AppBuilder;
use engine_core::engine::Engine;
use engine_core::plugins::CorePlugins;

fn main() {
    let mut app = Engine::new();
    app.add_plugin(CorePlugins);
    // Add your game plugins here
    // engine_core::engine::run_default(app).unwrap();
}
```

## Running the Editor

The built-in editor provides scene authoring, debugging, and asset management:

```bash
cargo run -p engine-editor
```

## Running Examples

```bash
# Basic ECS example
cargo run --example basic -p engine-core

# Input handling
cargo run --example input_demo -p engine-core

# Complete demo with rendering
cargo run --example complete_demo -p engine-core

# Russian Tetris game
cargo run --example tetris -p engine-core
```

## Building for Web/WASM

RustEngine supports building for WebAssembly:

```bash
# Install WASM target
rustup target add wasm32-unknown-unknown

# Build the renderer
cargo build -p engine-render --target wasm32-unknown-unknown

# Build and run Web Demo
cd examples/web-demo
wasm-pack build --target web --release
python -m http.server 8080
# Browser: http://localhost:8080
```

See [WASM_STATUS.md](../WASM_STATUS.md) for details on WASM support.

## Building for Android

```bash
# Install Android target
rustup target add aarch64-linux-android

# Build for Android
cargo build --target aarch64-linux-android
```

See [android-setup.md](android-setup.md) for detailed Android setup instructions.

## Next Steps

- [ECS Usage Tutorial](ecs-tutorial.md) — Learn about entities, components, and systems
- [Rendering Pipeline](rendering-pipeline.md) — Set up rendering
- [Physics System](physics-system.md) — Add physics simulation
- [Audio System](audio-system.md) — Play sounds and music
- [Plugin System](plugin-system.md) — Extend the engine with plugins
- [Editor Guide](editor-guide.md) — Use the built-in editor
