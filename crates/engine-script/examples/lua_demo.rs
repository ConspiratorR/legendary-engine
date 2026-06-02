//! Example: Using Lua scripts as ECS systems.
//!
//! Run with: `cargo run --example lua_demo -p engine-script`

use engine_ecs::system::System as _;
use engine_ecs::world::World;
use engine_script::prelude::*;
use mlua::prelude::*;
use std::sync::{Arc, RwLock};

/// A simple 3D position component.
#[derive(Debug, Clone)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

/// A velocity component.
#[derive(Debug, Clone)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}

fn main() -> LuaResult<()> {
    // 1. Create a component bridge and register our types
    let mut bridge = ComponentBridge::new();

    // Register Position: Lua sees { x, y, z } tables
    bridge.register_get::<Position>("Position", |lua, pos| {
        let table = lua.create_table()?;
        table.set("x", pos.x)?;
        table.set("y", pos.y)?;
        table.set("z", pos.z)?;
        Ok(LuaValue::Table(table))
    });
    bridge.register_set::<Position>("Position", |_lua, pos, val| {
        if let LuaValue::Table(t) = val {
            pos.x = t.get::<f32>("x").unwrap_or(pos.x);
            pos.y = t.get::<f32>("y").unwrap_or(pos.y);
            pos.z = t.get::<f32>("z").unwrap_or(pos.z);
        }
        Ok(())
    });
    bridge.register_add::<Position>("Position", |_lua, val| {
        if let LuaValue::Table(t) = val {
            Ok(Position {
                x: t.get::<f32>("x").unwrap_or(0.0),
                y: t.get::<f32>("y").unwrap_or(0.0),
                z: t.get::<f32>("z").unwrap_or(0.0),
            })
        } else {
            Ok(Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            })
        }
    });

    // Register Velocity
    bridge.register_get::<Velocity>("Velocity", |lua, vel| {
        let table = lua.create_table()?;
        table.set("x", vel.x)?;
        table.set("y", vel.y)?;
        table.set("z", vel.z)?;
        Ok(LuaValue::Table(table))
    });
    bridge.register_set::<Velocity>("Velocity", |_lua, vel, val| {
        if let LuaValue::Table(t) = val {
            vel.x = t.get::<f32>("x").unwrap_or(vel.x);
            vel.y = t.get::<f32>("y").unwrap_or(vel.y);
            vel.z = t.get::<f32>("z").unwrap_or(vel.z);
        }
        Ok(())
    });
    bridge.register_add::<Velocity>("Velocity", |_lua, val| {
        if let LuaValue::Table(t) = val {
            Ok(Velocity {
                x: t.get::<f32>("x").unwrap_or(0.0),
                y: t.get::<f32>("y").unwrap_or(0.0),
                z: t.get::<f32>("z").unwrap_or(0.0),
            })
        } else {
            Ok(Velocity {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            })
        }
    });

    let bridge = Arc::new(RwLock::new(bridge));

    // 2. Set up the ECS world with some entities
    let mut world = World::new();

    let entity1 = world.spawn();
    world.add_component(
        entity1,
        Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    );
    world.add_component(
        entity1,
        Velocity {
            x: 1.0,
            y: 2.0,
            z: 0.0,
        },
    );

    let entity2 = world.spawn();
    world.add_component(
        entity2,
        Position {
            x: 10.0,
            y: 10.0,
            z: 0.0,
        },
    );
    world.add_component(
        entity2,
        Velocity {
            x: -0.5,
            y: 0.5,
            z: 0.0,
        },
    );

    println!("=== Initial State ===");
    print_positions(&world, &[entity1.index(), entity2.index()]);

    // 3. Create a Lua movement system
    let movement_script = r#"
        function update(dt)
            local positions = world:entities("Position")
            for _, entity in ipairs(positions) do
                if world:has(entity, "Velocity") then
                    local pos = world:get(entity, "Position")
                    local vel = world:get(entity, "Velocity")
                    world:set(entity, "Position", {
                        x = pos.x + vel.x * dt,
                        y = pos.y + vel.y * dt,
                        z = pos.z + vel.z * dt,
                    })
                end
            end
        end
    "#;

    let movement_system = ScriptSystem::new("Movement", movement_script, bridge.clone())?;

    // 4. Run the system for a few frames
    println!("=== Running Lua Movement System ===");
    for frame in 0..5 {
        movement_system.run(&mut world);
        println!("Frame {}:", frame + 1);
        print_positions(&world, &[entity1.index(), entity2.index()]);
    }

    // 5. Demonstrate hot-reload
    println!("=== Hot-Reload Demo ===");
    let mut reloader = HotReloader::new(bridge.clone());
    reloader.watch("examples/movement.lua", "Movement")?;
    println!("Registered scripts: {:?}", reloader.script_names());
    println!("(In a real game loop, file changes would trigger automatic reloads)");

    Ok(())
}

fn print_positions(world: &World, indices: &[u32]) {
    for &idx in indices {
        if let Some(pos) = world.get_by_index::<Position>(idx) {
            println!("  Entity {}: ({}, {}, {})", idx, pos.x, pos.y, pos.z);
        }
    }
}
