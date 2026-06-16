//! Lua and WASM scripting integration for the RustEngine ECS.
//!
//! This crate bridges the engine's Entity Component System with script
//! runtimes, enabling gameplay logic to be written in Lua or compiled
//! WASM modules while retaining full ECS access.
//!
//! # Dual-Runtime Architecture
//!
//! The scripting layer supports two independent runtimes that share the
//! same ECS world and component bridge infrastructure:
//!
//! - **Lua** (via [`mlua`](https://docs.rs/mlua)): Interpreted scripts with
//!   hot-reload support. Best for rapid iteration and gameplay scripting.
//!   Scripts are compiled and executed inside a sandboxed Lua 5.4 state.
//!
//! - **WASM** (via [`wasmtime`](https://docs.rs/wasmtime)): Ahead-of-time
//!   compiled modules with deterministic execution. Best for performance-
//!   critical or untrusted code. Modules run inside a resource-bounded
//!   [`WasmSandbox`](wasm::WasmSandbox).
//!
//! Both runtimes interact with the ECS through the same
//! [`ComponentBridge`](bridge::ComponentBridge) / [`WasmComponentBridge`](wasm::WasmComponentBridge)
//! pattern: Rust component types are registered with closures that handle
//! conversion to/from script values (Lua tables or WASM linear memory bytes).
//!
//! # Sandbox Safety Model
//!
//! ## WASM Sandbox
//!
//! WASM modules are executed inside a [`WasmSandbox`](wasm::WasmSandbox) that
//! enforces hard resource limits:
//!
//! - **Memory**: Linear memory is capped (default 16 MiB). Growth beyond the
//!   limit is rejected by [`SandboxLimiter`](wasm::SandboxLimiter).
//! - **Fuel**: Each execution receives a finite fuel budget (default 1M units).
//!   Infinite loops are terminated when fuel is exhausted.
//! - **Table**: Indirect call table size is bounded (default 10K entries).
//! - **Stack**: WASM call stack is limited to 1 MiB.
//!
//! Use [`WasmSandbox::strict()`](wasm::WasmSandbox::strict) for untrusted code
//! (4 MiB memory, 100K fuel) and [`WasmSandbox::relaxed()`](wasm::WasmSandbox::relaxed)
//! for trusted modules (64 MiB memory, 100M fuel).
//!
//! ## Lua Sandboxing
//!
//! Lua scripts run in a fresh Lua 5.4 state with the standard `print` function
//! redirected to the host console. The `io` and `os` modules are available by
//! default in Lua 5.4 — for production use, consider removing them via
//! `lua.globals().set("io", LuaValue::Nil)`.
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
pub mod mod_system;
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
    fn test_component_bridge_round_trip() {
        let bridge = make_bridge_with_f32();
        let lua = Lua::new();
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, 42.0f32);

        let bridge = bridge.read().unwrap();
        // Get the value
        let result = bridge.get(&lua, &world, "Health", e.index()).unwrap();
        assert!(result.is_some());
        if let Some(LuaValue::Number(v)) = result {
            assert!((v - 42.0).abs() < 0.01);
        } else {
            panic!("Expected Number");
        }

        // Set a new value
        bridge
            .set(
                &lua,
                &mut world,
                "Health",
                e.index(),
                &LuaValue::Number(99.0),
            )
            .unwrap();

        // Verify the set
        let result = bridge.get(&lua, &world, "Health", e.index()).unwrap();
        if let Some(LuaValue::Number(v)) = result {
            assert!((v - 99.0).abs() < 0.01);
        } else {
            panic!("Expected Number after set");
        }
    }

    #[test]
    fn test_component_bridge_get_unregistered() {
        let bridge = make_bridge_with_f32();
        let lua = Lua::new();
        let world = World::new();

        let bridge = bridge.read().unwrap();
        let result = bridge.get(&lua, &world, "NonExistent", 0).unwrap();
        assert!(result.is_none());
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
