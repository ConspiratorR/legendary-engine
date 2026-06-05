use engine_script::prelude::*;
use engine_ecs::system::System;
use engine_ecs::world::World;
use std::sync::{Arc, RwLock};

fn make_bridge() -> Arc<RwLock<ComponentBridge>> {
    let mut bridge = ComponentBridge::new();
    bridge.register_get::<f32>("Health", |_lua, &v| Ok(mlua::Value::Number(v as f64)));
    bridge.register_set::<f32>("Health", |_lua, val, lua_val| {
        if let mlua::Value::Number(n) = lua_val {
            *val = *n as f32;
        }
        Ok(())
    });
    Arc::new(RwLock::new(bridge))
}

#[test]
fn script_engine_creation() {
    let bridge = make_bridge();
    let system = ScriptSystem::new("test", "function update(dt) end", bridge);
    assert!(system.is_ok(), "ScriptSystem should be created successfully");

    let system = system.unwrap();
    assert_eq!(system.script_name(), "test");
}

#[test]
fn lua_basic_execution() {
    let bridge = make_bridge();
    let source = r#"
        result = nil
        function update(dt)
            result = dt * 2.0
        end
    "#;
    let system = ScriptSystem::new("basic", source, bridge).unwrap();
    let mut world = World::new();

    // Run the system — should not panic
    system.run(&mut world);
}

#[test]
fn lua_function_call() {
    let bridge = make_bridge();
    let source = r#"
        function update(dt)
            local e = world:spawn()
            world:add(e, "Health", 100.0)
            local hp = world:get(e, "Health")
            assert(hp == 100.0, "expected 100, got " .. tostring(hp))
            world:set(e, "Health", 50.0)
            hp = world:get(e, "Health")
            assert(hp == 50.0, "expected 50 after set, got " .. tostring(hp))
        end
    "#;
    let system = ScriptSystem::new("fn_call", source, bridge).unwrap();
    let mut world = World::new();

    // Run the system — exercises spawn, add, get, set through Lua
    system.run(&mut world);
}
