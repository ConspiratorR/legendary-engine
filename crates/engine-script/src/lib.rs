//! Lua scripting integration for the RustEngine ECS.
//!
//! This crate provides:
//! - **[`ComponentBridge`]**: Register Rust component types for Lua access
//! - **[`ScriptSystem`]**: Execute Lua scripts as ECS systems
//! - **[`HotReloader`]**: Watch `.lua` files for automatic reloading
//!
//! # Quick Start
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
//!         -- access world through the 'world' global
//!     end
//! "#, bridge).unwrap();
//!
//! // 3. Add to your ECS schedule like any other system
//! ```

pub mod bridge;
pub mod hot_reload;
pub mod system;

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::bridge::ComponentBridge;
    pub use crate::hot_reload::HotReloader;
    pub use crate::system::ScriptSystem;
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
