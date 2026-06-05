# Phase 2: Layer 5 Systems Crates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Goal:** Polish the Layer 5 systems crates (engine-framework, engine-physics, engine-network, engine-script, engine-ui, engine-terrain) with tests, docs, and API consistency.

**Architecture:** Process each crate sequentially. These crates all depend on engine-core and are isolated from each other (no inter-dependencies).

**Tech Stack:** Rust 2024 edition, thiserror, anyhow, cargo test, cargo clippy

---

### Task 1: Polish engine-framework

**Files:**
- Modify: crates/engine-framework/src/*.rs
- Create: crates/engine-framework/tests/framework_tests.rs

- [ ] **Step 1: Add module-level documentation**

Add to crates/engine-framework/src/lib.rs:

`ust
//! # engine-framework
//!
//! Game state management framework for the RustEngine.
//!
//! Provides a state stack with push/pop/replace operations,
//! state lifecycle callbacks (on_enter, on_exit, on_update),
//! and save/load functionality.
//!
//! ## Quick Start
//!
//! `ust
//! use engine_framework::{GameState, StateStack};
//!
//! struct MenuState;
//! impl GameState for MenuState {
//!     fn on_enter(&mut self) { /* show menu */ }
//!     fn on_update(&mut self, dt: f32) { /* handle input */ }
//!     fn on_exit(&mut self) { /* cleanup */ }
//! }
//!
//! let mut stack = StateStack::new();
//! stack.push(Box::new(MenuState));
//! `
`

- [ ] **Step 2: Add documentation to all public functions**

- [ ] **Step 3: Add state stack tests**

Create crates/engine-framework/tests/framework_tests.rs:

`ust
use engine_framework::{GameState, StateStack};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

struct TestState {
    id: i32,
    enter_count: Arc<AtomicI32>,
    exit_count: Arc<AtomicI32>,
    update_count: Arc<AtomicI32>,
}

impl GameState for TestState {
    fn on_enter(&mut self) { self.enter_count.fetch_add(1, Ordering::SeqCst); }
    fn on_exit(&mut self) { self.exit_count.fetch_add(1, Ordering::SeqCst); }
    fn on_update(&mut self, _dt: f32) { self.update_count.fetch_add(1, Ordering::SeqCst); }
}

#[test]
fn test_state_stack_push() {
    let mut stack = StateStack::new();
    let enter = Arc::new(AtomicI32::new(0));
    let exit = Arc::new(AtomicI32::new(0));
    let update = Arc::new(AtomicI32::new(0));

    stack.push(Box::new(TestState {
        id: 1,
        enter_count: enter.clone(),
        exit_count: exit.clone(),
        update_count: update.clone(),
    }));

    assert_eq!(enter.load(Ordering::SeqCst), 1);
    assert_eq!(stack.len(), 1);
}

#[test]
fn test_state_stack_pop() {
    let mut stack = StateStack::new();
    let enter = Arc::new(AtomicI32::new(0));
    let exit = Arc::new(AtomicI32::new(0));
    let update = Arc::new(AtomicI32::new(0));

    stack.push(Box::new(TestState {
        id: 1,
        enter_count: enter.clone(),
        exit_count: exit.clone(),
        update_count: update.clone(),
    }));
    stack.pop();

    assert_eq!(exit.load(Ordering::SeqCst), 1);
    assert_eq!(stack.len(), 0);
}

#[test]
fn test_state_stack_replace() {
    let mut stack = StateStack::new();
    let enter1 = Arc::new(AtomicI32::new(0));
    let exit1 = Arc::new(AtomicI32::new(0));
    let update1 = Arc::new(AtomicI32::new(0));

    stack.push(Box::new(TestState {
        id: 1,
        enter_count: enter1.clone(),
        exit_count: exit1.clone(),
        update_count: update1.clone(),
    }));

    let enter2 = Arc::new(AtomicI32::new(0));
    let exit2 = Arc::new(AtomicI32::new(0));
    let update2 = Arc::new(AtomicI32::new(0));

    stack.replace(Box::new(TestState {
        id: 2,
        enter_count: enter2.clone(),
        exit_count: exit2.clone(),
        update_count: update2.clone(),
    }));

    assert_eq!(exit1.load(Ordering::SeqCst), 1);
    assert_eq!(enter2.load(Ordering::SeqCst), 1);
    assert_eq!(stack.len(), 1);
}

#[test]
fn test_state_stack_update() {
    let mut stack = StateStack::new();
    let enter = Arc::new(AtomicI32::new(0));
    let exit = Arc::new(AtomicI32::new(0));
    let update = Arc::new(AtomicI32::new(0));

    stack.push(Box::new(TestState {
        id: 1,
        enter_count: enter.clone(),
        exit_count: exit.clone(),
        update_count: update.clone(),
    }));

    stack.update(0.016);
    stack.update(0.016);

    assert_eq!(update.load(Ordering::SeqCst), 2);
}

#[test]
fn test_state_stack_empty() {
    let mut stack = StateStack::new();
    assert!(stack.is_empty());
    assert_eq!(stack.len(), 0);
}
`

- [ ] **Step 4: Run tests**

Run: cargo test -p engine-framework
Expected: All tests PASS

- [ ] **Step 5: Run clippy**

Run: cargo clippy -p engine-framework
Expected: Zero warnings

- [ ] **Step 6: Commit**

`ash
git add crates/engine-framework/
git commit -m "feat(framework): add docs and state stack tests for engine-framework"
`

---

### Task 2: Polish engine-physics

**Files:**
- Modify: crates/engine-physics/src/*.rs
- Create: crates/engine-physics/tests/physics_tests.rs

- [ ] **Step 1: Add module-level documentation**

Add to crates/engine-physics/src/lib.rs:

`ust
//! # engine-physics
//!
//! Physics simulation for the RustEngine.
//!
//! Features:
//! - Rigid body dynamics (static, kinematic, dynamic)
//! - Collision detection (sphere, box, capsule, cylinder)
//! - Contact solving with warm starting
//! - Continuous collision detection (CCD)
//! - Joint system (hinge, ball-socket, spring)
//!
//! ## Quick Start
//!
//! `ust
//! use engine_physics::{RigidBody, Collider, PhysicsWorld};
//!
//! let mut world = PhysicsWorld::new();
//! let body = world.create_rigid_body(RigidBody::dynamic());
//! world.attach_collider(body, Collider::sphere(0.5));
//! world.step(1.0 / 60.0);
//! `
`

- [ ] **Step 2: Add documentation to all public functions**

- [ ] **Step 3: Add collision detection tests**

Create crates/engine-physics/tests/physics_tests.rs:

`ust
use engine_physics::{RigidBody, Collider, PhysicsWorld, BodyType};
use engine_math::Vec3;

#[test]
fn test_physics_world_creation() {
    let world = PhysicsWorld::new();
    assert_eq!(world.body_count(), 0);
}

#[test]
fn test_rigid_body_creation() {
    let mut world = PhysicsWorld::new();
    let body = world.create_rigid_body(RigidBody::dynamic());
    assert_eq!(world.body_count(), 1);
    assert!(world.is_alive(body));
}

#[test]
fn test_rigid_body_types() {
    let mut world = PhysicsWorld::new();

    let dynamic = world.create_rigid_body(RigidBody::dynamic());
    let kinematic = world.create_rigid_body(RigidBody::kinematic());
    let static_body = world.create_rigid_body(RigidBody::static_body());

    assert_eq!(world.body_type(dynamic), BodyType::Dynamic);
    assert_eq!(world.body_type(kinematic), BodyType::Kinematic);
    assert_eq!(world.body_type(static_body), BodyType::Static);
}

#[test]
fn test_collider_attachment() {
    let mut world = PhysicsWorld::new();
    let body = world.create_rigid_body(RigidBody::dynamic());
    world.attach_collider(body, Collider::sphere(0.5));

    assert!(world.has_collider(body));
}

#[test]
fn test_sphere_sphere_collision() {
    let mut world = PhysicsWorld::new();

    let a = world.create_rigid_body(RigidBody::dynamic());
    world.attach_collider(a, Collider::sphere(1.0));
    world.set_position(a, Vec3::new(0.0, 0.0, 0.0));

    let b = world.create_rigid_body(RigidBody::dynamic());
    world.attach_collider(b, Collider::sphere(1.0));
    world.set_position(b, Vec3::new(1.5, 0.0, 0.0));

    world.step(1.0 / 60.0);

    // Should detect collision and separate
    let pos_a = world.position(a);
    let pos_b = world.position(b);
    let distance = (pos_a - pos_b).length();
    assert!(distance >= 2.0); // Should be separated by sum of radii
}

#[test]
fn test_force_application() {
    let mut world = PhysicsWorld::new();
    let body = world.create_rigid_body(RigidBody::dynamic());
    world.set_mass(body, 1.0);

    world.apply_force(body, Vec3::new(10.0, 0.0, 0.0));
    world.step(1.0 / 60.0);

    let velocity = world.velocity(body);
    assert!(velocity.x > 0.0);
}

#[test]
fn test_gravity() {
    let mut world = PhysicsWorld::new();
    world.set_gravity(Vec3::new(0.0, -9.81, 0.0));

    let body = world.create_rigid_body(RigidBody::dynamic());
    world.step(1.0 / 60.0);

    let velocity = world.velocity(body);
    assert!(velocity.y < 0.0);
}
`

- [ ] **Step 4: Run tests**

Run: cargo test -p engine-physics
Expected: All tests PASS

- [ ] **Step 5: Run clippy**

Run: cargo clippy -p engine-physics
Expected: Zero warnings

- [ ] **Step 6: Commit**

`ash
git add crates/engine-physics/
git commit -m "feat(physics): add docs and collision tests for engine-physics"
`

---

### Task 3: Polish engine-network

**Files:**
- Modify: crates/engine-network/src/*.rs
- Create: crates/engine-network/tests/network_tests.rs

- [ ] **Step 1: Add module-level documentation**

Add to crates/engine-network/src/lib.rs:

`ust
//! # engine-network
//!
//! Networking system for the RustEngine.
//!
//! Features:
//! - UDP/TCP socket I/O
//! - Message serialization/deserialization
//! - Connection management with RTT tracking
//! - Server/client architecture
//! - Authoritative server mode
//!
//! ## Quick Start
//!
//! `ust
//! use engine_network::GameServer;
//!
//! let server = GameServer::new("0.0.0.0:7777")?;
//! server.start()?;
//! `
`

- [ ] **Step 2: Add documentation to all public functions**

- [ ] **Step 3: Add connection tests**

Create crates/engine-network/tests/network_tests.rs:

`ust
use engine_network::{GameServer, GameClient, ConnectionState};

#[test]
fn test_server_creation() {
    let server = GameServer::new("127.0.0.1:0"); // Port 0 = auto-assign
    assert!(server.is_ok());
}

#[test]
fn test_client_creation() {
    let client = GameClient::new();
    assert!(client.is_ok());
    assert_eq!(client.state(), ConnectionState::Disconnected);
}

#[test]
fn test_connection_state() {
    let client = GameClient::new().unwrap();
    assert_eq!(client.state(), ConnectionState::Disconnected);
}

#[test]
fn test_message_serialization() {
    use engine_network::messages::{ChatMessage, NetworkMessage};

    let msg = ChatMessage {
        sender: "Player1".to_string(),
        content: "Hello!".to_string(),
    };

    let bytes = msg.serialize().unwrap();
    let deserialized = ChatMessage::deserialize(&bytes).unwrap();

    assert_eq!(deserialized.sender, "Player1");
    assert_eq!(deserialized.content, "Hello!");
}
`

- [ ] **Step 4: Run tests**

Run: cargo test -p engine-network
Expected: All tests PASS

- [ ] **Step 5: Run clippy**

Run: cargo clippy -p engine-network
Expected: Zero warnings

- [ ] **Step 6: Commit**

`ash
git add crates/engine-network/
git commit -m "feat(network): add docs and connection tests for engine-network"
`

---

### Task 4: Polish engine-script

**Files:**
- Modify: crates/engine-script/src/*.rs
- Create: crates/engine-script/tests/script_tests.rs

- [ ] **Step 1: Add module-level documentation**

Add to crates/engine-script/src/lib.rs:

`ust
//! # engine-script
//!
//! Scripting system for the RustEngine.
//!
//! Features:
//! - Lua integration via mlua with ECS bridge
//! - WASM runtime via wasmtime with sandbox
//! - Hot-reload support
//! - Type registry and event bridge
//!
//! ## Quick Start
//!
//! `ust
//! use engine_script::ScriptEngine;
//!
//! let engine = ScriptEngine::new()?;
//! engine.load_script("game.lua")?;
//! engine.call_function("on_init", &[])?;
//! `
`

- [ ] **Step 2: Add documentation to all public functions**

- [ ] **Step 3: Add script integration tests**

Create crates/engine-script/tests/script_tests.rs:

`ust
use engine_script::ScriptEngine;

#[test]
fn test_script_engine_creation() {
    let engine = ScriptEngine::new();
    assert!(engine.is_ok());
}

#[test]
fn test_lua_basic_execution() {
    let engine = ScriptEngine::new().unwrap();
    let result: i32 = engine.eval("return 1 + 2").unwrap();
    assert_eq!(result, 3);
}

#[test]
fn test_lua_function_call() {
    let engine = ScriptEngine::new().unwrap();
    engine.load_script_inline(r#"
        function greet(name)
            return "Hello, " .. name .. "!"
        end
    "#).unwrap();

    let result: String = engine.call_function("greet", &["World"]).unwrap();
    assert_eq!(result, "Hello, World!");
}

#[test]
fn test_lua_ecs_bridge() {
    let engine = ScriptEngine::new().unwrap();

    // Register a component type
    engine.register_component::<Position>("Position").unwrap();

    // Create entity from Lua
    engine.load_script_inline(r#"
        local entity = create_entity()
        add_component(entity, "Position", { x = 10, y = 20 })
    "#).unwrap();
}

#[test]
fn test_wasm_basic_execution() {
    let engine = ScriptEngine::new().unwrap();

    // Minimal WASM module that exports an "add" function
    let wasm_bytes = create_test_wasm_module();

    engine.load_wasm(&wasm_bytes).unwrap();
    let result: i32 = engine.call_wasm_function("add", &[1, 2]).unwrap();
    assert_eq!(result, 3);
}

#[test]
fn test_sandbox_violation() {
    let engine = ScriptEngine::new().unwrap();

    // WASM module that tries to access memory outside sandbox
    let malicious_wasm = create_malicious_wasm_module();

    let result = engine.load_wasm(&malicious_wasm);
    assert!(result.is_err());
}
`

- [ ] **Step 4: Run tests**

Run: cargo test -p engine-script
Expected: All tests PASS

- [ ] **Step 5: Run clippy**

Run: cargo clippy -p engine-script
Expected: Zero warnings

- [ ] **Step 6: Commit**

`ash
git add crates/engine-script/
git commit -m "feat(script): add docs and integration tests for engine-script"
`

---

### Task 5: Polish engine-ui

**Files:**
- Modify: crates/engine-ui/src/*.rs
- Create: crates/engine-ui/tests/ui_tests.rs

- [ ] **Step 1: Add module-level documentation**

Add to crates/engine-ui/src/lib.rs:

`ust
//! # engine-ui
//!
//! UI framework for the RustEngine.
//!
//! Features:
//! - Retained-mode UI system
//! - Theme support
//! - Text rendering
//! - Animation system
//! - Widget library (buttons, labels, panels, etc.)
//!
//! ## Quick Start
//!
//! `ust
//! use engine_ui::{UiBuilder, Button, Label};
//!
//! let ui = UiBuilder::new()
//!     .add(Label::new("Hello, World!"))
//!     .add(Button::new("Click Me", || { /* handler */ }))
//!     .build();
//! `
`

- [ ] **Step 2: Add documentation to all public functions**

- [ ] **Step 3: Add UI component tests**

Create crates/engine-ui/tests/ui_tests.rs:

`ust
use engine_ui::{UiBuilder, Button, Label, Panel};

#[test]
fn test_label_creation() {
    let label = Label::new("Test");
    assert_eq!(label.text(), "Test");
}

#[test]
fn test_button_creation() {
    let clicked = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let clicked_clone = clicked.clone();

    let button = Button::new("Click", move || {
        clicked_clone.store(true, std::sync::atomic::Ordering::SeqCst);
    });

    assert_eq!(button.text(), "Click");
}

#[test]
fn test_panel_layout() {
    let panel = Panel::new()
        .with_child(Label::new("Item 1"))
        .with_child(Label::new("Item 2"));

    assert_eq!(panel.children().len(), 2);
}

#[test]
fn test_ui_builder() {
    let ui = UiBuilder::new()
        .add(Label::new("Title"))
        .add(Button::new("OK", || {}))
        .build();

    assert_eq!(ui.widgets().len(), 2);
}
`

- [ ] **Step 4: Run tests**

Run: cargo test -p engine-ui
Expected: All tests PASS

- [ ] **Step 5: Run clippy**

Run: cargo clippy -p engine-ui
Expected: Zero warnings

- [ ] **Step 6: Commit**

`ash
git add crates/engine-ui/
git commit -m "feat(ui): add docs and component tests for engine-ui"
`

---

### Task 6: Polish engine-terrain

**Files:**
- Modify: crates/engine-terrain/src/*.rs
- Create: crates/engine-terrain/tests/terrain_tests.rs

- [ ] **Step 1: Add module-level documentation**

Add to crates/engine-terrain/src/lib.rs:

`ust
//! # engine-terrain
//!
//! Terrain system for the RustEngine.
//!
//! Features:
//! - Heightmap-based terrain
//! - Paint layers (texture splatting)
//! - Terrain sculpting
//! - Editor integration
//!
//! ## Quick Start
//!
//! `ust
//! use engine_terrain::Terrain;
//!
//! let terrain = Terrain::new(256, 256);
//! terrain.set_height(128, 128, 10.0);
//! `
`

- [ ] **Step 2: Add documentation to all public functions**

- [ ] **Step 3: Add terrain generation tests**

Create crates/engine-terrain/tests/terrain_tests.rs:

`ust
use engine_terrain::Terrain;

#[test]
fn test_terrain_creation() {
    let terrain = Terrain::new(256, 256);
    assert_eq!(terrain.width(), 256);
    assert_eq!(terrain.height(), 256);
}

#[test]
fn test_terrain_height() {
    let mut terrain = Terrain::new(64, 64);
    terrain.set_height(32, 32, 10.0);
    assert_eq!(terrain.height_at(32, 32), 10.0);
}

#[test]
fn test_terrain_default_height() {
    let terrain = Terrain::new(64, 64);
    assert_eq!(terrain.height_at(0, 0), 0.0);
}

#[test]
fn test_terrain_paint_layer() {
    let mut terrain = Terrain::new(64, 64);
    terrain.add_layer("grass").unwrap();
    terrain.paint("grass", 32, 32, 1.0).unwrap();

    let weight = terrain.layer_weight("grass", 32, 32);
    assert_eq!(weight, 1.0);
}

#[test]
fn test_terrain_multiple_layers() {
    let mut terrain = Terrain::new(64, 64);
    terrain.add_layer("grass").unwrap();
    terrain.add_layer("rock").unwrap();

    terrain.paint("grass", 32, 32, 0.7).unwrap();
    terrain.paint("rock", 32, 32, 0.3).unwrap();

    assert_eq!(terrain.layer_weight("grass", 32, 32), 0.7);
    assert_eq!(terrain.layer_weight("rock", 32, 32), 0.3);
}

#[test]
fn test_terrain_sculpt() {
    let mut terrain = Terrain::new(64, 64);
    terrain.sculpt(32, 32, 5.0, 1.0); // radius=5, strength=1

    let height = terrain.height_at(32, 32);
    assert!(height > 0.0);
}
`

- [ ] **Step 4: Run tests**

Run: cargo test -p engine-terrain
Expected: All tests PASS

- [ ] **Step 5: Run clippy**

Run: cargo clippy -p engine-terrain
Expected: Zero warnings

- [ ] **Step 6: Commit**

`ash
git add crates/engine-terrain/
git commit -m "feat(terrain): add docs and generation tests for engine-terrain"
`

---

### Task 7: Final verification for Layer 5

**Files:**
- None (verification only)

- [ ] **Step 1: Run full test suite**

Run: cargo test -p engine-framework -p engine-physics -p engine-network -p engine-script -p engine-ui -p engine-terrain
Expected: All tests PASS

- [ ] **Step 2: Run clippy**

Run: cargo clippy -p engine-framework -p engine-physics -p engine-network -p engine-script -p engine-ui -p engine-terrain
Expected: Zero warnings

- [ ] **Step 3: Verify documentation**

Run: cargo doc -p engine-framework -p engine-physics -p engine-network -p engine-script -p engine-ui -p engine-terrain --no-deps
Expected: No warnings about missing docs
