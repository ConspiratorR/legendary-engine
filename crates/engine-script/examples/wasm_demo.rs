//! Example: Using WASM modules as ECS systems.
//!
//! Run with: `cargo run --example wasm_demo -p engine-script`
//!
//! This example demonstrates:
//! - Creating a WASM runtime with sandbox limits
//! - Registering components for WASM access
//! - Loading and executing a WASM module as an ECS system
//! - The WASM module reading and writing Position/Velocity components

use engine_ecs::system::System as _;
use engine_ecs::world::World;
use engine_script::prelude::*;
use std::sync::{Arc, RwLock};

/// A simple 3D position component (12 bytes: 3x f32).
#[derive(Debug, Clone)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

/// A velocity component (12 bytes: 3x f32).
#[derive(Debug, Clone)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}

fn position_to_bytes(pos: &Position) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(12);
    bytes.extend_from_slice(&pos.x.to_le_bytes());
    bytes.extend_from_slice(&pos.y.to_le_bytes());
    bytes.extend_from_slice(&pos.z.to_le_bytes());
    bytes
}

fn position_from_bytes(bytes: &[u8]) -> Position {
    Position {
        x: f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        y: f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        z: f32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
    }
}

fn velocity_to_bytes(vel: &Velocity) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(12);
    bytes.extend_from_slice(&vel.x.to_le_bytes());
    bytes.extend_from_slice(&vel.y.to_le_bytes());
    bytes.extend_from_slice(&vel.z.to_le_bytes());
    bytes
}

fn velocity_from_bytes(bytes: &[u8]) -> Velocity {
    Velocity {
        x: f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        y: f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        z: f32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
    }
}

fn main() -> anyhow::Result<()> {
    // 1. Create a WASM component bridge and register our types
    let mut bridge = WasmComponentBridge::new();

    // Register Position: 12 bytes (3x f32)
    bridge.register::<Position>("Position", 12, position_to_bytes, position_from_bytes);

    // Register Velocity: 12 bytes (3x f32)
    bridge.register::<Velocity>("Velocity", 12, velocity_to_bytes, velocity_from_bytes);

    let bridge = Arc::new(RwLock::new(bridge));

    // 2. Create a WASM runtime with sandbox configuration
    let sandbox = WasmSandbox {
        max_memory_bytes: 16 * 1024 * 1024, // 16 MiB
        max_fuel: 10_000_000,               // Generous fuel for demo
        ..Default::default()
    };
    let runtime = Arc::new(WasmRuntime::with_sandbox(sandbox)?);

    // 3. Set up the ECS world with some entities
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
            z: 0.5,
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
            z: 1.0,
        },
    );

    println!("=== Initial State ===");
    print_positions(&world, &[entity1.index(), entity2.index()]);

    // 4. Load and run the WASM movement system
    //    The .wat file is compiled to .wasm at build time or loaded directly.
    //    For this example, we use a minimal inline WASM module.

    // Compile the .wat file to WASM bytes
    // In production, you'd load pre-compiled .wasm files.
    let wasm_path = "examples/movement.wat";

    // Check if the .wat file exists; if not, use a minimal test module
    let wasm_bytes = if std::path::Path::new(wasm_path).exists() {
        // wasmtime can compile .wat directly via Module::from_file
        // but for the example we'll demonstrate the byte-based API
        match std::fs::read(wasm_path) {
            Ok(_bytes) => {
                // .wat files need to be parsed; wasmtime handles this internally
                // when using Module::from_file, but compile() expects .wasm bytes.
                // For the example, we'll use from_file directly.
                println!("Loading WASM module from: {}", wasm_path);
                println!("(In production, use pre-compiled .wasm files)");
                // Create the system using from_file
                let movement_system =
                    WasmSystem::from_file("Movement", wasm_path, runtime.clone(), bridge.clone())?;

                println!("=== Running WASM Movement System ===");
                for frame in 0..5 {
                    movement_system.run(&mut world);
                    println!("Frame {}:", frame + 1);
                    print_positions(&world, &[entity1.index(), entity2.index()]);
                }

                return Ok(());
            }
            Err(e) => {
                println!("Could not read {}: {}", wasm_path, e);
                println!("Using minimal inline WASM module instead.");
                minimal_wasm_module()
            }
        }
    } else {
        println!("{} not found, using minimal inline WASM module.", wasm_path);
        minimal_wasm_module()
    };

    let movement_system =
        WasmSystem::new("Movement", &wasm_bytes, runtime.clone(), bridge.clone())?;

    // 5. Run the system for a few frames
    println!("=== Running WASM Movement System ===");
    for frame in 0..5 {
        movement_system.run(&mut world);
        println!("Frame {}:", frame + 1);
        print_positions(&world, &[entity1.index(), entity2.index()]);
    }

    // 6. Demonstrate sandbox limits
    println!("\n=== Sandbox Configuration ===");
    println!("Max memory: {} bytes", runtime.sandbox().max_memory_bytes);
    println!("Max fuel: {} units", runtime.sandbox().max_fuel);
    println!(
        "Registered components: {:?}",
        bridge.read().unwrap().registered_names()
    );

    Ok(())
}

/// Create a minimal WASM module that just logs and returns.
/// This is used when the full movement.wat is not available.
fn minimal_wasm_module() -> Vec<u8> {
    // A minimal WASM module with:
    // - memory export
    // - env imports for log and delta_time
    // - an exported update(f32) function
    wat::parse_str(
        r#"
        (module
            (import "env" "log" (func $log (param i32 i32)))
            (import "env" "delta_time" (func $delta_time (result f32)))
            (import "env" "has_component" (func $has_component (param i32 i32 i32) (result i32)))
            (import "env" "get_component" (func $get_component (param i32 i32 i32 i32 i32) (result i32)))
            (import "env" "set_component" (func $set_component (param i32 i32 i32 i32 i32) (result i32)))

            (memory (export "memory") 1 16)

            (data (i32.const 0) "WASM update called")

            (func (export "update") (param $dt f32)
                (call $log (i32.const 0) (i32.const 18))
            )
        )
        "#,
    )
    .expect("Failed to parse WAT")
}

fn print_positions(world: &World, indices: &[u32]) {
    for &idx in indices {
        if let Some(pos) = world.get_by_index::<Position>(idx) {
            println!(
                "  Entity {}: ({:.2}, {:.2}, {:.2})",
                idx, pos.x, pos.y, pos.z
            );
        }
    }
}
