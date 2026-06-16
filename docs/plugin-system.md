# Plugin System

RustEngine includes a plugin system for extending the engine with custom functionality.

## Overview

The plugin system supports two types of plugins:

1. **Static Plugins** — Compiled into the engine, registered at build time
2. **Dynamic Plugins** — Loaded at runtime from shared libraries (.dll/.so/.dylib)

## Static Plugins

Static plugins implement the `Plugin` trait:

```rust
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;

struct MyPlugin;

impl Plugin for MyPlugin {
    fn build(&self, app: &mut AppBuilder) {
        // Register systems, resources, event handlers
        app.add_system(my_system);
        app.insert_resource(MyResource::default());
    }
}

// Register the plugin
let mut app = AppBuilder::new();
app.add_plugin(MyPlugin);
```

### Plugin Lifecycle

1. **Registration** — `app.add_plugin(MyPlugin)` calls `MyPlugin::build(&self, &mut AppBuilder)`
2. **Build phase** — The plugin inserts resources, registers systems, and adds hooks
3. **Build order** — Plugins execute `build()` in the order they are registered
4. **Dependencies** — Handle plugin dependencies by registration order

### Built-in Plugins

RustEngine includes several built-in plugins:

```rust
use engine_core::plugins::CorePlugins;

// Core plugins (Time, ActionMap)
app.add_plugin(CorePlugins);

// With logging
app.add_plugin(CorePlugins::with_logging(LogLevel::Debug));

// Individual plugins
app.add_plugin(TimePlugin);
app.add_plugin(ActionPlugin);
app.add_plugin(LoggerPlugin::new(LogLevel::Info));
app.add_plugin(ProfilerPlugin::new(120));
app.add_plugin(MemoryTrackerPlugin);
```

## Dynamic Plugins

Dynamic plugins are loaded at runtime from shared libraries:

### Plugin Manifest

Each dynamic plugin has a `plugin.json` manifest:

```json
{
    "name": "my-plugin",
    "version": "1.0.0",
    "description": "A custom plugin for RustEngine",
    "author": "Your Name",
    "entry_point": "create_plugin",
    "engine_version": ">=0.1.0",
    "dependencies": {}
}
```

### Plugin Entry Point

The plugin must export an entry point function:

```rust
use engine_core::plugin::Plugin;
use engine_core::app::AppBuilder;

pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn build(&self, app: &mut AppBuilder) {
        // Plugin implementation
    }
}

#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn Plugin {
    Box::into_raw(Box::new(MyPlugin))
}
```

### Plugin Cargo.toml

The plugin must be a `cdylib` crate:

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
engine-core = { path = "../../crates/engine-core" }
log = "0.4"
```

### Loading Dynamic Plugins

Use the `PluginLoader` to manage dynamic plugins:

```rust
use engine_core::plugin_loader::PluginLoader;

// Create a plugin loader
let mut loader = PluginLoader::new("plugins/registry.json".into())?;

// Install a plugin
loader.install(plugin_dir, plugins_dir)?;

// Load all registered plugins
unsafe { loader.load_all()?; }

// Register plugins with the application
loader.register_all(&mut app);

// Uninstall a plugin
loader.uninstall("my-plugin")?;
```

### Plugin Registry

The plugin loader maintains a JSON registry:

```json
{
    "plugins": {
        "my-plugin": "plugins/my-plugin",
        "another-plugin": "plugins/another-plugin"
    }
}
```

## Plugin Examples

### Example: Logging Plugin

```rust
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;

struct LoggingPlugin {
    level: log::LevelFilter,
}

impl Plugin for LoggingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        env_logger::Builder::new()
            .filter_level(self.level)
            .init();
        log::info!("LoggingPlugin initialized with level: {:?}", self.level);
    }
}
```

### Example: Physics Plugin

```rust
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;

struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(PhysicsWorld::new());
        app.add_system(physics_step_system);
        app.add_system(collision_detection_system);
    }
}
```

### Example: Custom Component Plugin

```rust
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;

struct HealthPlugin;

#[derive(Debug, Clone)]
struct Health {
    current: f32,
    max: f32,
}

impl Plugin for HealthPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(health_regen_system);
        app.add_system(health_death_system);
    }
}

fn health_regen_system(world: &mut World) {
    let query = Query::<Health>::new();
    for health in query.iter_mut(world) {
        if health.current < health.max {
            health.current += 0.1;
        }
    }
}
```

## Best Practices

1. **Keep plugins focused** — Each plugin should do one thing well
2. **Document dependencies** — Use the `dependencies` field in the manifest
3. **Version compatibility** — Specify `engine_version` requirements
4. **Error handling** — Return `Result` from plugin initialization
5. **Logging** — Use `log` crate for debug output
6. **Testing** — Write unit tests for plugin functionality

## See Also

- [Quick Start](quick-start.md) — Get started with RustEngine
- [Architecture](architecture.md) — Engine architecture overview
- [Contributing](contributing.md) — How to contribute
