//! Lua and WASM scripting integration for the RustEngine ECS.
//!
//! This crate bridges the engine's Entity Component System with script
//! runtimes, enabling gameplay logic to be written in Lua or compiled
//! WASM modules while retaining full ECS access.
//!
//! # Features
//!
//! - **[`ComponentBridge`](bridge::ComponentBridge)**: Register Rust component types for Lua access
//! - **[`ScriptSystem`](system::ScriptSystem)**: Execute Lua scripts as ECS systems
//! - **[`WasmSystem`](wasm::WasmSystem)**: Execute WASM modules as ECS systems
//! - **[`HotReloader`](hot_reload::HotReloader)**: Watch `.lua` files for automatic reloading
//! - **[`TypeRegistry`](type_registry::TypeRegistry)**: Unified Rust ↔ Lua/WASM type mapping for engine math types
//! - **[`CallbackRegistry`](callback::CallbackRegistry)**: Register Rust callbacks callable from scripts
//! - **[`ScriptEventBus`](event_bridge::ScriptEventBus)**: Bridge engine events to/from scripts
//!
//! # Quick Start — Lua
//!
//! ```rust,no_run
//! use engine_script::prelude::*;
//! use std::sync::{Arc, RwLock};
//!
//! // 1. Create a component bridge and register your types
//! let mut bridge = ComponentBridge::new();
//! bridge.register_get::<f32>("Health", |_lua, &v| Ok(v as f64).map(mlua::Value::Number));
//! bridge.register_set::<f32>("Health", |_lua, val, lua_val| {
//!     if let mlua::Value::Number(n) = lua_val { *val = *n as f32; }
//!     Ok(())
//! });
//!
//! // 2. Create a script system
//! let bridge = Arc::new(RwLock::new(bridge));
//! let system = ScriptSystem::new("movement", r#"
//!     function update(dt)
//!         -- access entity components through the 'world' global
//!         -- world:spawn(), world:get(e, "Health"), world:set(e, "Health", 50)
//!     end
//! "#, bridge).unwrap();
//!
//! // 3. Add to your ECS schedule like any other system
//! ```
//!
//! # Quick Start — WASM
//!
//! ```rust,no_run
//! use engine_script::prelude::*;
//! use std::sync::{Arc, RwLock};
//!
//! let runtime = Arc::new(WasmRuntime::new().unwrap());
//! let mut bridge = WasmComponentBridge::new();
//! // bridge.register::<Position>("Position", 12, to_bytes, from_bytes);
//! let bridge = Arc::new(RwLock::new(bridge));
//! // let system = WasmSystem::new("ai", &wasm_bytes, runtime, bridge).unwrap();
//! ```
//!
//! # Architecture
//!
//! The scripting layer is designed around three principles:
//!
//! 1. **Type-safe bridging**: Rust components are registered by type, with
//!    closures that handle conversion to/from Lua values or WASM bytes.
//! 2. **Sandbox isolation**: WASM modules run inside a [`WasmSandbox`](wasm::WasmSandbox)
//!    with configurable memory, fuel, and table limits.
//! 3. **Hot-reload support**: Lua scripts can be watched on disk and reloaded
//!    at runtime via [`HotReloader`](hot_reload::HotReloader).

pub use error::ScriptError;

pub mod bridge;
pub mod callback;
pub mod error;
pub mod event_bridge;
pub mod hot_reload;
pub mod system;
pub mod type_registry;
pub mod wasm;

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::bridge::ComponentBridge;
    pub use crate::callback::{CallbackArg, CallbackRegistry, CallbackResult};
    pub use crate::error::{BridgeError, BridgeResult, ScriptError};
    pub use crate::event_bridge::{EventData, ScriptEventBus};
    pub use crate::hot_reload::HotReloader;
    pub use crate::system::ScriptSystem;
    pub use crate::type_registry::TypeRegistry;
    pub use crate::wasm::{WasmComponentBridge, WasmRuntime, WasmSandbox, WasmSystem};
}

#[cfg(test)]
mod tests {
    use super::prelude::*;
    use engine_ecs::world::World;
    use mlua::prelude::*;
    use std::sync::{Arc, RwLock};

    fn make_bridge_with_f32() -> Arc<RwLock<ComponentBridge>> {
        let mut bridge = ComponentBridge::new();
        bridge.register_get::<f32>("Health", |_lua, &v| Ok(LuaValue::Number(v as f64)));
        bridge.register_set::<f32>("Health", |_lua, val, lua_val| {
            if let LuaValue::Number(n) = lua_val {
                *val = *n as f32;
            }
            Ok(())
        });
        Arc::new(RwLock::new(bridge))
    }

    #[test]
    fn test_component_bridge_get_set() {
        let bridge = make_bridge_with_f32();
        let lua = Lua::new();
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, 100.0f32);

        let bridge = bridge.read().unwrap();
        let result = bridge.get(&lua, &world, "Health", e.index()).unwrap();
        assert!(result.is_some());
        if let Some(LuaValue::Number(v)) = result {
            assert!((v - 100.0).abs() < 0.01);
        } else {
            panic!("Expected Number");
        }
    }

    #[test]
    fn test_script_system_runs_update() {
        let bridge = make_bridge_with_f32();
        let source = r#"
            function update(dt)
                print("dt = " .. dt)
            end
        "#;
        let system = ScriptSystem::new("test", source, bridge).unwrap();
        let mut world = World::new();
        // Should not panic
        engine_ecs::system::System::run(&system, &mut world);
    }

    #[test]
    fn test_script_system_spawn_entity() {
        let bridge = make_bridge_with_f32();
        let source = r#"
            spawned_entity = nil
            function update(dt)
                spawned_entity = world:spawn()
            end
        "#;
        let system = ScriptSystem::new("spawn_test", source, bridge).unwrap();
        let mut world = World::new();
        engine_ecs::system::System::run(&system, &mut world);
        // The entity was spawned inside Lua (but we can't easily verify from here
        // since the Lua state is private to the system).
        // At minimum, it should not panic.
    }

    #[test]
    fn test_script_system_get_set_component() {
        let bridge = make_bridge_with_f32();
        let source = r#"
            function update(dt)
                local e = world:spawn()
                world:add(e, "Health", 100.0)
                local hp = world:get(e, "Health")
                print("Health = " .. tostring(hp))
                world:set(e, "Health", 50.0)
            end
        "#;
        let system = ScriptSystem::new("get_set_test", source, bridge).unwrap();
        let mut world = World::new();
        engine_ecs::system::System::run(&system, &mut world);
    }

    #[test]
    fn test_hot_reloader_creation() {
        let bridge = make_bridge_with_f32();
        let reloader = HotReloader::new(bridge);
        assert!(reloader.script_names().is_empty());
    }

    #[test]
    fn test_hot_reloader_register_watch() {
        let bridge = make_bridge_with_f32();
        let mut reloader = HotReloader::new(bridge);
        reloader.watch("/tmp/test.lua", "test_script").unwrap();
        assert_eq!(reloader.script_names(), vec!["test_script"]);
        assert!(reloader.script_path("test_script").is_some());
    }
}
