# Phase 2: Layer 2 Infrastructure Crates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Goal:** Polish the infrastructure crates (engine-scene, engine-input) with tests, docs, and API consistency.

**Architecture:** Process each crate sequentially, completing all quality items before moving to the next.

**Tech Stack:** Rust 2024 edition, thiserror, anyhow, cargo test, cargo clippy, Criterion

---

### Task 1: Polish engine-scene

**Files:**
- Modify: crates/engine-scene/src/*.rs
- Create: crates/engine-scene/tests/scene_tests.rs

- [ ] **Step 1: Add module-level documentation**

Add to crates/engine-scene/src/lib.rs:

`ust
//! # engine-scene
//!
//! Scene management for the RustEngine.
//!
//! Provides a scene graph with parent-child hierarchy,
//! Transform/GlobalTransform synchronization, and
//! serialization support.
//!
//! ## Quick Start
//!
//! `ust
//! use engine_ecs::World;
//! use engine_scene::{SceneNode, Transform};
//!
//! let mut world = World::new();
//! let root = world.spawn();
//! world.add_component(root, SceneNode::new("Root"));
//! world.add_component(root, Transform::default());
//!
//! let child = world.spawn();
//! world.add_component(child, SceneNode::new("Child"));
//! world.add_component(child, Transform::from_translation(Vec3::new(1.0, 0.0, 0.0)));
//! `
`

- [ ] **Step 2: Add documentation to all public functions**

Document all public types and functions with /// docs.

- [ ] **Step 3: Add hierarchy sync tests**

Create crates/engine-scene/tests/scene_tests.rs:

`ust
use engine_ecs::World;
use engine_scene::{SceneNode, Transform, GlobalTransform};
use engine_math::Vec3;

#[test]
fn test_scene_node_creation() {
    let node = SceneNode::new("Test");
    assert_eq!(node.name, "Test");
    assert!(node.parent.is_none());
    assert!(node.children.is_empty());
}

#[test]
fn test_parent_child_relationship() {
    let mut world = World::new();

    let parent = world.spawn();
    world.add_component(parent, SceneNode::new("Parent"));
    world.add_component(parent, Transform::default());

    let child = world.spawn();
    world.add_component(child, SceneNode::new("Child"));
    world.add_component(child, Transform::from_translation(Vec3::new(1.0, 0.0, 0.0)));

    // Set parent-child relationship
    world.get_component_mut::<SceneNode>(child).unwrap().parent = Some(parent);
    world.get_component_mut::<SceneNode>(parent).unwrap().children.push(child);

    // Verify
    let child_node = world.get_component::<SceneNode>(child).unwrap();
    assert_eq!(child_node.parent, Some(parent));

    let parent_node = world.get_component::<SceneNode>(parent).unwrap();
    assert!(parent_node.children.contains(&child));
}

#[test]
fn test_global_transform_hierarchy() {
    let mut world = World::new();

    let parent = world.spawn();
    world.add_component(parent, Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)));
    world.add_component(parent, GlobalTransform::default());

    let child = world.spawn();
    world.add_component(child, Transform::from_translation(Vec3::new(5.0, 0.0, 0.0)));
    world.add_component(child, GlobalTransform::default());

    // Set parent-child
    world.get_component_mut::<SceneNode>(child).unwrap().parent = Some(parent);
    world.get_component_mut::<SceneNode>(parent).unwrap().children.push(child);

    // Simulate transform sync
    engine_scene::sync_transforms(&mut world);

    // Child's global transform should be parent + child
    let child_global = world.get_component::<GlobalTransform>(child).unwrap();
    assert_eq!(child_global.translation(), Vec3::new(15.0, 0.0, 0.0));
}

#[test]
fn test_cascade_delete() {
    let mut world = World::new();

    let parent = world.spawn();
    world.add_component(parent, SceneNode::new("Parent"));

    let child = world.spawn();
    world.add_component(child, SceneNode::new("Child"));
    world.get_component_mut::<SceneNode>(child).unwrap().parent = Some(parent);
    world.get_component_mut::<SceneNode>(parent).unwrap().children.push(child);

    // Delete parent should cascade to children
    engine_scene::despawn_recursive(&mut world, parent);

    assert!(!world.is_alive(parent));
    assert!(!world.is_alive(child));
}
`

- [ ] **Step 4: Add serialization tests**

Add to crates/engine-scene/tests/scene_tests.rs:

`ust
#[test]
fn test_scene_serialization() {
    let node = SceneNode::new("Test");
    let json = serde_json::to_string(&node).unwrap();
    let deserialized: SceneNode = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, "Test");
}

#[test]
fn test_transform_serialization() {
    let transform = Transform::from_translation(Vec3::new(1.0, 2.0, 3.0));
    let json = serde_json::to_string(&transform).unwrap();
    let deserialized: Transform = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.translation, Vec3::new(1.0, 2.0, 3.0));
}
`

- [ ] **Step 5: Run tests**

Run: cargo test -p engine-scene
Expected: All tests PASS

- [ ] **Step 6: Run clippy**

Run: cargo clippy -p engine-scene
Expected: Zero warnings

- [ ] **Step 7: Commit**

`ash
git add crates/engine-scene/
git commit -m "feat(scene): add docs, hierarchy and serialization tests for engine-scene"
`

---

### Task 2: Polish engine-input

**Files:**
- Modify: crates/engine-input/src/*.rs
- Create: crates/engine-input/tests/input_tests.rs

- [ ] **Step 1: Add module-level documentation**

Add to crates/engine-input/src/lib.rs:

`ust
//! # engine-input
//!
//! Input management for the RustEngine.
//!
//! Provides keyboard/mouse state tracking, action mapping,
//! and input action detection for game controls.
//!
//! ## Quick Start
//!
//! `ust
//! use engine_input::{InputManager, Action};
//!
//! let mut input = InputManager::new();
//!
//! // Register an action
//! input.register_action("jump", Action::key(KeyCode::Space));
//!
//! // Check if action was just pressed
//! if input.just_pressed("jump") {
//!     // Player jumps!
//! }
//! `
`

- [ ] **Step 2: Add documentation to all public functions**

Document all public types and functions with /// docs.

- [ ] **Step 3: Add action mapping tests**

Create crates/engine-input/tests/input_tests.rs:

`ust
use engine_input::{InputManager, Action, KeyCode, MouseButton};

#[test]
fn test_input_manager_creation() {
    let input = InputManager::new();
    assert!(input.is_ok());
}

#[test]
fn test_register_action() {
    let mut input = InputManager::new().unwrap();
    input.register_action("jump", Action::key(KeyCode::Space));
    assert!(input.has_action("jump"));
}

#[test]
fn test_duplicate_action() {
    let mut input = InputManager::new().unwrap();
    input.register_action("jump", Action::key(KeyCode::Space));
    let result = input.register_action("jump", Action::key(KeyCode::KeyW));
    assert!(result.is_err());
}

#[test]
fn test_key_binding() {
    let mut input = InputManager::new().unwrap();
    input.register_action("move_forward", Action::key(KeyCode::KeyW));

    // Simulate key press
    input.on_key_press(KeyCode::KeyW);
    assert!(input.is_pressed("move_forward"));
    assert!(input.just_pressed("move_forward"));

    // Simulate key release
    input.on_key_release(KeyCode::KeyW);
    assert!(!input.is_pressed("move_forward"));
}

#[test]
fn test_mouse_binding() {
    let mut input = InputManager::new().unwrap();
    input.register_action("shoot", Action::mouse(MouseButton::Left));

    input.on_mouse_press(MouseButton::Left);
    assert!(input.just_pressed("shoot"));
}

#[test]
fn test_axis_input() {
    let mut input = InputManager::new().unwrap();
    input.register_axis("horizontal", Action::key(KeyCode::KeyD), Action::key(KeyCode::KeyA));

    input.on_key_press(KeyCode::KeyD);
    let value = input.axis_value("horizontal");
    assert!(value > 0.0);

    input.on_key_release(KeyCode::KeyD);
    input.on_key_press(KeyCode::KeyA);
    let value = input.axis_value("horizontal");
    assert!(value < 0.0);
}
`

- [ ] **Step 4: Run tests**

Run: cargo test -p engine-input
Expected: All tests PASS

- [ ] **Step 5: Run clippy**

Run: cargo clippy -p engine-input
Expected: Zero warnings

- [ ] **Step 6: Commit**

`ash
git add crates/engine-input/
git commit -m "feat(input): add docs and action mapping tests for engine-input"
`

---

### Task 3: Final verification for Layer 2

**Files:**
- None (verification only)

- [ ] **Step 1: Run full test suite**

Run: cargo test -p engine-scene -p engine-input
Expected: All tests PASS

- [ ] **Step 2: Run clippy**

Run: cargo clippy -p engine-scene -p engine-input
Expected: Zero warnings

- [ ] **Step 3: Verify documentation**

Run: cargo doc -p engine-scene -p engine-input --no-deps
Expected: No warnings about missing docs
