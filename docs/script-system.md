# Script System

The `engine-script` crate provides dual-runtime scripting integration for the RustEngine ECS, supporting both **Lua** and **WebAssembly (WASM)** as scripting languages. Both runtimes share the same ECS world and component bridge infrastructure, enabling gameplay logic to be written in interpreted or compiled languages while retaining full ECS access.

## Table of Contents

- [Overview](#overview)
- [Dual-Runtime Architecture](#dual-runtime-architecture)
- [Lua Scripting](#lua-scripting)
- [WASM Scripting](#wasm-scripting)
- [Component Bridge](#component-bridge)
- [Type Registry](#type-registry)
- [Callback System](#callback-system)
- [Hot-Reload Support](#hot-reload-support)
- [Mod System](#mod-system)
- [Sandbox Safety Model](#sandbox-safety-model)
- [Error Handling](#error-handling)
- [API Reference](#api-reference)

---

## Overview

The scripting layer is designed around three principles:

1. **Type-safe bridging**: Rust components are registered by type, with closures that handle conversion to/from Lua values (tables) or WASM bytes (little-endian binary).
2. **Sandbox isolation**: WASM modules run inside a `WasmSandbox` with configurable memory, fuel, and table limits. Lua scripts run in a fresh Lua 5.4 state.
3. **Hot-reload support**: Lua scripts can be watched on disk and reloaded at runtime via `HotReloader`.

Both runtimes implement the `System` trait, so they integrate directly into the ECS schedule like any other Rust system.

---

## Dual-Runtime Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      ECS World                          │
├─────────────────────────────────────────────────────────┤
│  ComponentBridge (Lua)  │  WasmComponentBridge (WASM)  │
│  ┌──────────────────┐   │  ┌────────────────────────┐  │
│  │ get/set/add Fn   │   │  │ get/set/add Fn         │  │
│  │ LuaValue ↔ T     │   │  │ bytes ↔ T              │  │
│  └──────────────────┘   │  └────────────────────────┘  │
├─────────────────────────────────────────────────────────┤
│  ScriptSystem (Lua)     │  WasmSystem (WASM)           │
│  ┌──────────────────┐   │  ┌────────────────────────┐  │
│  │ mlua::Lua state  │   │  │ wasmtime Store         │  │
│  │ world table API  │   │  │ host function imports  │  │
│  │ update(dt)       │   │  │ update(dt) export      │  │
│  └──────────────────┘   │  └────────────────────────┘  │
├─────────────────────────────────────────────────────────┤
│  TypeRegistry — unified type mapping for both runtimes  │
├─────────────────────────────────────────────────────────┤
│  HotReloader (Lua)      │  ModPlugin/ModLoader (WASM)  │
└─────────────────────────────────────────────────────────┘
```

### When to Use Each Runtime

| Feature | Lua | WASM |
|---------|-----|------|
| Iteration speed | Fast (no compile step) | Slower (requires compile) |
| Execution performance | Moderate (interpreted) | High (AOT compiled) |
| Hot-reload | Yes (file watcher) | No (must recompile) |
| Sandbox safety | Basic (Lua state isolation) | Strong (memory/fuel/table limits) |
| Untrusted code | Not recommended | Recommended |
| Dependencies | `mlua` (Lua 5.4) | `wasmtime` |
| Binary size | N/A (interpreted) | ~1-100 KB per module |

---

## Lua Scripting

### Quick Start

```rust
use engine_script::prelude::*;
use std::sync::{Arc, RwLock};

// 1. Create a component bridge and register your types
let mut bridge = ComponentBridge::new();
bridge.register_get::<f32>("Health", |_lua, &v| Ok(mlua::Value::Number(v as f64)));
bridge.register_set::<f32>("Health", |_lua, val, lua_val| {
    if let mlua::Value::Number(n) = lua_val { *val = *n as f32; }
    Ok(())
});

// 2. Create a script system
let bridge = Arc::new(RwLock::new(bridge));
let system = ScriptSystem::new("health_monitor", r#"
    function update(dt)
        local hp = world:get(player_entity, "Health")
        if hp and hp < 20 then
            print("Warning: Low health!")
        end
    end
"#, bridge).unwrap();

// 3. Add to your ECS schedule
```

### Creating a ScriptSystem

```rust
// From inline source
let system = ScriptSystem::new("my_script", source_code, bridge)?;

// From a file
let system = ScriptSystem::from_file("my_script", "scripts/movement.lua", bridge)?;
```

### Lua World API

The `world` table is injected into the Lua global scope on each tick:

| Method | Signature | Description |
|--------|-----------|-------------|
| `world:spawn()` | `→ u32` | Spawn a new entity, return its index |
| `world:despawn(idx)` | `(u32) → nil` | Despawn an entity by index |
| `world:get(idx, name)` | `(u32, string) → value\|nil` | Read a component by name |
| `world:set(idx, name, val)` | `(u32, string, value) → nil` | Write a component value |
| `world:add(idx, name, val)` | `(u32, string, value) → nil` | Add a component to an entity |
| `world:has(idx, name)` | `(u32, string) → bool` | Check if entity has a component |
| `world:entities(name)` | `(string) → {u32, ...}` | Get all entities with a component |
| `world:delta_time()` | `→ number` | Get current frame delta time |

### Lua Script Example

```lua
-- scripts/enemy_ai.lua

local speed = 5.0
local health = 100.0

function update(dt)
    -- Spawn a new enemy entity on first tick
    if not self_entity then
        self_entity = world:spawn()
        world:add(self_entity, "Health", health)
        world:add(self_entity, "Position", {x = 0, y = 0, z = 0})
        world:add(self_entity, "Velocity", {x = speed, y = 0, z = 0})
    end

    -- Read and update position
    local pos = world:get(self_entity, "Position")
    local vel = world:get(self_entity, "Velocity")

    if pos and vel then
        pos.x = pos.x + vel.x * dt
        pos.y = pos.y + vel.y * dt
        world:set(self_entity, "Position", pos)
    end

    -- Check health
    local hp = world:get(self_entity, "Health")
    if hp and hp <= 0 then
        world:despawn(self_entity)
        self_entity = nil
    end
end
```

### Reloading Lua Scripts

```rust
// Reload from new source
system.reload(new_source)?;

// Reload from file
system.reload_from_file("scripts/movement.lua")?;
```

---

## WASM Scripting

### Quick Start

```rust
use engine_script::prelude::*;
use std::sync::{Arc, RwLock};

// 1. Create a WASM runtime (shared across all WASM systems)
let runtime = Arc::new(WasmRuntime::new()?);

// 2. Create a WASM component bridge
let mut bridge = WasmComponentBridge::new();
bridge.register::<Position>("Position", 12, position_to_bytes, position_from_bytes);
let bridge = Arc::new(RwLock::new(bridge));

// 3. Create a WASM system
let system = WasmSystem::new(
    "enemy_ai",
    &wasm_bytes,      // compiled WASM binary
    runtime.clone(),
    bridge.clone(),
)?;

// 4. Add to ECS schedule
```

### WASM Host Functions

WASM modules import the following host functions from the `"env"` namespace:

| Function | Signature | Description |
|----------|-----------|-------------|
| `spawn()` | `→ i32` | Spawn entity, return index |
| `despawn(entity)` | `(i32)` | Despawn entity |
| `get_component(entity, name_ptr, name_len, result_ptr, result_cap)` | `→ i32` | Read component; returns bytes written |
| `set_component(entity, name_ptr, name_len, value_ptr, value_len)` | `→ i32` | Write component; returns 1 on success |
| `add_component(entity, name_ptr, name_len, value_ptr, value_len)` | `→ i32` | Add component; returns 1 on success |
| `has_component(entity, name_ptr, name_len)` | `→ i32` | Check component; returns 1 if exists |
| `component_size(name_ptr, name_len)` | `→ i32` | Get component byte size |
| `log(ptr, len)` | `(i32, i32)` | Print string to host console |
| `delta_time()` | `→ f32` | Get current delta time |

### WASM Module Export

The WASM module must export an `update` function:

```wat
(module
    (import "env" "spawn" (func $spawn (result i32)))
    (import "env" "add_component" (func $add (param i32 i32 i32 i32 i32) (result i32)))
    (import "env" "log" (func $log (param i32 i32)))
    (import "env" "delta_time" (func $dt (result f32)))

    (memory (export "memory") 1 16)

    ;; "Position" string at offset 0
    (data (i32.const 0) "Position")

    ;; Position data: x=1.0, y=2.0, z=3.0 (little-endian f32)
    (data (i32.const 256) "\00\00\80\3F\00\00\00\40\00\00\40\40")

    (func (export "update") (param $dt f32)
        (local $entity i32)
        ;; Spawn entity
        (local.set $entity (call $spawn))
        ;; Add Position component
        (drop (call $add
            (local.get $entity)
            (i32.const 0)    ;; "Position" name ptr
            (i32.const 8)    ;; "Position" name len
            (i32.const 256)  ;; value ptr
            (i32.const 12)   ;; value len (3 × f32)
        ))
    )
)
```

### Compiling WASM Modules

```rust
let runtime = WasmRuntime::new()?;

// From bytes
let module = runtime.compile(&wasm_bytes)?;

// From file
let module = runtime.compile_file("target/wasm32-unknown-unknown/release/my_mod.wasm")?;
```

### Custom WasmSandbox

```rust
// Default sandbox: 16 MiB memory, 1M fuel
let runtime = WasmRuntime::new()?;

// Strict sandbox for untrusted code
let sandbox = WasmSandbox::strict();  // 4 MiB memory, 100K fuel
let runtime = WasmRuntime::with_sandbox(sandbox)?;

// Relaxed sandbox for trusted code
let sandbox = WasmSandbox::relaxed(); // 64 MiB memory, 100M fuel
let runtime = WasmRuntime::with_sandbox(sandbox)?;
```

---

## Component Bridge

### Lua ComponentBridge

The `ComponentBridge` maps human-readable component names to Lua conversion closures:

```rust
use engine_script::prelude::*;
use std::sync::{Arc, RwLock};

let mut bridge = ComponentBridge::new();

// Register a read-only component
bridge.register_get::<f32>("Health", |_lua, &v| {
    Ok(mlua::Value::Number(v as f64))
});

// Register a writable component
bridge.register_set::<f32>("Health", |_lua, val, lua_val| {
    if let mlua::Value::Number(n) = lua_val {
        *val = *n as f32;
    }
    Ok(())
});

// Register a factory for creating components from Lua
bridge.register_add::<Position>("Position", |_lua, lua_val| {
    if let mlua::Value::Table(t) = lua_val {
        Ok(Position {
            x: t.get("x")?,
            y: t.get("y")?,
            z: t.get("z")?,
        })
    } else {
        Err(mlua::Error::runtime("expected table for Position"))
    }
});

let bridge = Arc::new(RwLock::new(bridge));
```

### WASM WasmComponentBridge

The `WasmComponentBridge` maps component names to binary serialization closures:

```rust
use engine_script::prelude::*;
use std::sync::{Arc, RwLock};

let mut bridge = WasmComponentBridge::new();

bridge.register::<Position>(
    "Position",              // name
    12,                      // byte size (3 × f32)
    |pos| {                  // to_bytes: &T → Vec<u8>
        let mut bytes = Vec::with_capacity(12);
        bytes.extend_from_slice(&pos.x.to_le_bytes());
        bytes.extend_from_slice(&pos.y.to_le_bytes());
        bytes.extend_from_slice(&pos.z.to_le_bytes());
        bytes
    },
    |bytes| {                // from_bytes: &[u8] → T
        Position {
            x: f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            y: f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            z: f32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        }
    },
);

let bridge = Arc::new(RwLock::new(bridge));
```

---

## Type Registry

The `TypeRegistry` provides a unified mapping layer for Rust ↔ Lua/WASM conversion. Register engine types once, then use the same registry for both Lua `ScriptSystem` and WASM `WasmSystem` execution.

### Built-in Type Mappings

| Rust Type | Lua Representation | WASM Binary |
|-----------|-------------------|-------------|
| `Vec2` | `{x, y}` | 8 bytes (2×f32 LE) |
| `Vec3` | `{x, y, z}` | 12 bytes (3×f32 LE) |
| `Vec4` | `{x, y, z, w}` | 16 bytes (4×f32 LE) |
| `Quat` | `{x, y, z, w}` | 16 bytes (4×f32 LE) |
| `Color` | `{r, g, b, a}` | 16 bytes (4×f32 LE) |
| `Transform` | `{position, rotation, scale}` | 36 bytes (9×f32 LE) |
| `bool` | boolean | 1 byte |
| `i32` | integer | 4 bytes (i32 LE) |
| `u32` | integer | 4 bytes (u32 LE) |
| `f32` | number | 4 bytes (f32 LE) |
| `f64` | number | 8 bytes (f64 LE) |

### Usage

```rust
use engine_script::type_registry::TypeRegistry;

// Create a registry with all engine types pre-registered
let registry = TypeRegistry::default();

// Or register manually
let mut registry = TypeRegistry::new();
registry.register_all_engine_types();

// Query registered types
assert!(registry.has("Vec3"));
assert_eq!(registry.wasm_size("Vec3"), Some(12));

// Use for Lua
let val = registry.lua_get(&lua, &world, "Vec3", entity_idx)?;

// Use for WASM
let mut buf = [0u8; 12];
let written = registry.wasm_get(&world, "Vec3", entity_idx, &mut buf);
```

### Custom Type Registration

To register a custom type, use the `ComponentBridge` directly:

```rust
// Lua bridge
bridge.register_get::<MyComponent>("MyComponent", |_lua, comp| {
    let t = lua.create_table()?;
    t.set("field_a", comp.field_a)?;
    t.set("field_b", comp.field_b)?;
    Ok(mlua::Value::Table(t))
});

bridge.register_set::<MyComponent>("MyComponent", |_lua, comp, lua_val| {
    if let mlua::Value::Table(t) = lua_val {
        comp.field_a = t.get("field_a")?;
        comp.field_b = t.get("field_b")?;
    }
    Ok(())
});

// WASM bridge
bridge.register::<MyComponent>(
    "MyComponent",
    8, // byte size
    |c| {
        let mut bytes = Vec::with_capacity(8);
        bytes.extend_from_slice(&c.field_a.to_le_bytes());
        bytes.extend_from_slice(&c.field_b.to_le_bytes());
        bytes
    },
    |bytes| {
        MyComponent {
            field_a: f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            field_b: f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        }
    },
);
```

---

## Callback System

The `CallbackRegistry` allows Rust code to register named callbacks that scripts can invoke by name.

### Registering Callbacks

```rust
use engine_script::callback::{CallbackRegistry, CallbackArg, CallbackResult};

let mut registry = CallbackRegistry::new();

registry.register("on_damage", |args| {
    let entity = args[0].as_u32().unwrap_or(0);
    let amount = args[1].as_f32().unwrap_or(0.0);
    println!("Entity {} took {} damage", entity, amount);
    Ok(CallbackResult::None)
});

registry.register("get_player_name", |_args| {
    Ok(CallbackResult::String("Hero".to_string()))
});

registry.register("calculate_damage", |args| {
    let base = args[0].as_f32().unwrap_or(0.0);
    let multiplier = args[1].as_f32().unwrap_or(1.0);
    Ok(CallbackResult::F32(base * multiplier))
});
```

### Invoking from Lua

```rust
use std::sync::{Arc, RwLock};

let registry = Arc::new(RwLock::new(registry));
let lua_fn = CallbackRegistry::create_lua_function(registry);

// Inject into Lua state
lua.globals().set("callback", lua.create_function(lua_fn)?)?;
```

```lua
-- Lua script
local result = callback("calculate_damage", 100.0, 2.5)
print("Damage: " .. result)  -- 250.0

callback("on_damage", player_entity, 50.0)
local name = callback("get_player_name")
```

### Invoking from Rust

```rust
let result = registry.invoke("calculate_damage", &[
    CallbackArg::F32(100.0),
    CallbackArg::F32(2.5),
])?;
```

---

## Hot-Reload Support

The `HotReloader` watches `.lua` file changes and automatically reloads the corresponding `ScriptSystem` instances.

### Setup

```rust
use engine_script::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

let bridge = Arc::new(RwLock::new(ComponentBridge::new()));
let mut reloader = HotReloader::new(bridge.clone());

// Register scripts to watch
reloader.watch("scripts/player.lua", "player")?;
reloader.watch("scripts/enemy.lua", "enemy")?;

// Start watching a directory
reloader.start_watching("scripts/")?;
```

### Frame Loop Integration

```rust
// In your main loop, check for reloads each frame
let reloads = reloader.check_reloads();
for name in reloads {
    println!("Hot-reloading script: {}", name);
    reloader.reload_script(&mut script_systems, name)?;
}
```

### How It Works

1. `start_watching` sets up a filesystem watcher with a 500ms debounce
2. When a `.lua` file is modified, it's added to a pending reload queue
3. `check_reloads` drains the queue and returns script names that need reloading
4. `reload_script` reads the file and calls `ScriptSystem::reload`, which creates a fresh Lua state

---

## Mod System

The mod system allows loading WASM modules as gameplay mods from a directory structure.

### Mod Directory Structure

```
mods/
├── my_mod/
│   ├── mod.json           # mod manifest
│   └── my_mod.wasm        # compiled WASM entry point
├── another_mod/
│   ├── mod.json
│   └── main.wasm
```

### Mod Manifest (`mod.json`)

```json
{
    "name": "my_mod",
    "version": "1.0.0",
    "description": "A gameplay mod",
    "author": "ModAuthor",
    "entry_point": "my_mod.wasm",
    "engine_version": ">=0.1.0",
    "dependencies": {
        "base_framework": ">=1.0.0"
    },
    "assets": ["textures/my_texture.png"],
    "components": [
        {
            "name": "CustomHealth",
            "component_type": "f32",
            "size": 4
        }
    ],
    "systems": [
        {
            "name": "enemy_ai",
            "order": 100,
            "dependencies": ["physics_system"]
        }
    ]
}
```

### Loading Mods

```rust
use engine_script::prelude::*;

// Option 1: Using ModPlugin
let mut app = AppBuilder::new();
app.add_plugin(ModPlugin::new("mods"));

// Option 2: Manual ModLoader
let mut loader = ModLoader::new()?;
loader.add_mod_dir("mods/".into());
loader.load_all()?;

// Access loaded mods
for mod in loader.mods() {
    println!("Loaded: {} v{}", mod.name(), mod.version());
}
```

### Running Mods

```rust
use engine_script::mod_plugin::mod_update_system;

// Add as a system to your ECS schedule
// This runs all loaded WASM mod systems each frame
```

### Dependency Resolution

Mods are loaded in topological order based on their declared dependencies. The loader:

1. Scans all mod directories for `mod.json` files
2. Validates that all dependencies exist
3. Detects circular dependencies
4. Loads mods in dependency order (dependencies first)

### Mod Error Handling

```rust
use engine_script::mod_system::ModLoadError;

match loader.load_all() {
    Ok(()) => {}
    Err(ModLoadError::MissingDependency(mod, dep)) => {
        eprintln!("{} requires {}", mod, dep);
    }
    Err(ModLoadError::CircularDependency(msg)) => {
        eprintln!("Circular dependency: {}", msg);
    }
    Err(ModLoadError::WasmNotFound(path)) => {
        eprintln!("WASM file missing: {}", path.display());
    }
    _ => {}
}
```

---

## Sandbox Safety Model

### WASM Sandbox

WASM modules are executed inside a `WasmSandbox` that enforces hard resource limits:

| Resource | Default | Strict | Relaxed |
|----------|---------|--------|---------|
| Memory | 16 MiB | 4 MiB | 64 MiB |
| Fuel | 1M units | 100K units | 100M units |
| Table entries | 10,000 | 1,000 | 100,000 |
| Instances | 100 | 10 | 1,000 |
| Memories | 1 | 1 | 4 |
| Stack | 1 MiB | 1 MiB | 1 MiB |

**Memory limits** prevent WASM modules from allocating unbounded memory. The `SandboxLimiter` rejects `memory.grow` requests that would exceed the limit.

**Fuel limits** terminate infinite loops. Each WASM instruction consumes fuel; when fuel is exhausted, execution is halted with an error.

**Table limits** bound the size of indirect call tables, preventing abuse of function pointer tables.

### Lua Sandboxing

Lua scripts run in a fresh Lua 5.4 state with:

- `print` redirected to the host console (`[Lua]` prefix)
- `io` and `os` modules available by default (consider removing for production)
- No filesystem access (unless explicitly registered)
- No network access

For production use, restrict the Lua environment:

```rust
let lua = Lua::new();
lua.globals().set("io", mlua::Value::Nil)?;
lua.globals().set("os", mlua::Value::Nil)?;
```

### Choosing Sandbox Level

- **Strict** (`WasmSandbox::strict()`): For untrusted third-party mods. Low memory, low fuel. Suitable for user-contributed content.
- **Default** (`WasmSandbox::default()`): For known mods. Balanced limits.
- **Relaxed** (`WasmSandbox::relaxed()`): For first-party or fully trusted modules. High limits for complex gameplay logic.

---

## Error Handling

The crate provides a unified error hierarchy:

### BridgeError

```rust
pub enum BridgeError {
    Lua(mlua::Error),
    Wasm(anyhow::Error),
    TypeMismatch { expected, actual },
    TypeNotRegistered(String),
    CallbackNotFound(String),
    EventChannelNotFound(String),
    EntityNotFound(u32),
    ComponentNotFound { entity, component },
    BufferOverflow { needed, available },
}
```

### ScriptError

```rust
pub enum ScriptError {
    CompilationFailed(String),
    RuntimeError(String),
    LuaError(String),
    WasmError(String),
    HotReloadFailed(String),
    SandboxViolation(String),
}
```

### ModLoadError

```rust
pub enum ModLoadError {
    IoError(PathBuf, std::io::Error),
    InvalidManifest(serde_json::Error),
    WasmNotFound(PathBuf),
    WasmError(anyhow::Error),
    CircularDependency(String),
    MissingDependency(String, String),
}
```

---

## API Reference

### Prelude

All public types are re-exported via `engine_script::prelude`:

```rust
use engine_script::prelude::*;
```

| Type | Module | Description |
|------|--------|-------------|
| `ComponentBridge` | `bridge` | Lua component bridge registry |
| `ScriptSystem` | `system` | Lua script ECS system |
| `WasmRuntime` | `wasm` | WASM engine wrapper |
| `WasmSandbox` | `wasm` | WASM resource limits |
| `WasmComponentBridge` | `wasm` | WASM component bridge |
| `WasmSystem` | `wasm` | WASM script ECS system |
| `HotReloader` | `hot_reload` | Lua file watcher |
| `TypeRegistry` | `type_registry` | Unified type mapping |
| `CallbackRegistry` | `callback` | Rust callback registry |
| `CallbackArg` | `callback` | Callback argument wrapper |
| `CallbackResult` | `callback` | Callback return value |
| `ScriptEventBus` | `event_bridge` | Event bridge for scripts |
| `ModPlugin` | `mod_plugin` | WASM mod plugin |
| `mod_update_system` | `mod_plugin` | Mod update system fn |
| `ScriptError` | `error` | Script error types |
| `BridgeError` | `error` | Bridge error types |

### TypeRegistry Helper Functions

```rust
// Lua conversion
vec2_to_lua(lua, v) → LuaTable
lua_to_vec2(table) → Vec2
vec3_to_lua(lua, v) → LuaTable
lua_to_vec3(table) → Vec3
vec4_to_lua(lua, v) → LuaTable
lua_to_vec4(table) → Vec4
quat_to_lua(lua, q) → LuaTable
lua_to_quat(table) → Quat
color_to_lua(lua, c) → LuaTable
lua_to_color(table) → Color
transform_to_lua(lua, tr) → LuaTable
lua_to_transform(table) → Transform

// WASM byte conversion
vec2_to_bytes(v) → [u8; 8]
bytes_to_vec2(buf) → Vec2
vec3_to_bytes(v) → [u8; 12]
bytes_to_vec3(buf) → Vec3
vec4_to_bytes(v) → [u8; 16]
bytes_to_vec4(buf) → Vec4
quat_to_bytes(q) → [u8; 16]
bytes_to_quat(buf) → Quat
color_to_bytes(c) → [u8; 16]
bytes_to_color(buf) → Color
transform_to_bytes(tr) → [u8; 36]
bytes_to_transform(buf) → Transform
```

---

## Dependencies

The `engine-script` crate depends on:

- `mlua` — Lua 5.4 bindings (feature: `lua54`)
- `wasmtime` — WASM runtime
- `notify` / `notify-debouncer-mini` — Filesystem watching for hot-reload
- `serde` / `serde_json` — Mod manifest parsing
- `thiserror` — Error derivation
- `engine-ecs` — ECS world and system traits
- `engine-core` — Engine types (Transform, Color)
- `engine-math` — Math types (Vec2, Vec3, Vec4, Quat)
