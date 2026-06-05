# Phase 3: Integration Verification & Documentation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Goal:** Verify cross-crate integration, update examples, write architecture documentation, and optimize development workflow.

**Architecture:** This is the final phase. Focus on ensuring all crates work together correctly, examples compile and run, and documentation is complete.

**Tech Stack:** Rust 2024 edition, cargo test, cargo clippy, just

---

### Task 1: Cross-crate integration tests

**Files:**
- Create: tests/integration_tests.rs (workspace root)

- [ ] **Step 1: Create integration test file**

Create tests/integration_tests.rs at workspace root:

```rust
use engine_ecs::World;
use engine_scene::{SceneNode, Transform, GlobalTransform};
use engine_math::Vec3;

#[test]
fn test_ecs_scene_integration() {
    let mut world = World::new();
    let entity = world.spawn();
    world.add_component(entity, SceneNode::new("TestNode"));
    world.add_component(entity, Transform::from_translation(Vec3::new(1.0, 2.0, 3.0)));
    world.add_component(entity, GlobalTransform::default());
    assert!(world.has_component::<SceneNode>(entity));
    assert!(world.has_component::<Transform>(entity));
}

#[test]
fn test_scene_hierarchy_sync() {
    let mut world = World::new();
    let parent = world.spawn();
    world.add_component(parent, SceneNode::new("Parent"));
    world.add_component(parent, Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)));
    world.add_component(parent, GlobalTransform::default());

    let child = world.spawn();
    world.add_component(child, SceneNode::new("Child"));
    world.add_component(child, Transform::from_translation(Vec3::new(5.0, 0.0, 0.0)));
    world.add_component(child, GlobalTransform::default());

    world.get_component_mut::<SceneNode>(child).unwrap().parent = Some(parent);
    world.get_component_mut::<SceneNode>(parent).unwrap().children.push(child);

    engine_scene::sync_transforms(&mut world);
    let child_global = world.get_component::<GlobalTransform>(child).unwrap();
    assert_eq!(child_global.translation(), Vec3::new(15.0, 0.0, 0.0));
}
```

- [ ] **Step 2: Run integration tests**

Run: cargo test --test integration_tests
Expected: All tests PASS

- [ ] **Step 3: Commit**

```bash
git add tests/integration_tests.rs
git commit -m "test: add cross-crate integration tests"
```

---

### Task 2: Update examples

**Files:**
- Modify: crates/engine-core/examples/*.rs

- [ ] **Step 1: Verify each example compiles**

Run: cargo build --examples
Expected: All examples compile without errors

- [ ] **Step 2: Fix any broken examples**

For each example that fails to compile:
- Update imports to match current API
- Fix type mismatches
- Update deprecated function calls

- [ ] **Step 3: Commit**

```bash
git add crates/engine-core/examples/
git commit -m "docs: update examples for current API"
```

---

### Task 3: Architecture documentation

**Files:**
- Modify: docs/architecture.md
- Create: docs/contributing.md

- [ ] **Step 1: Update architecture.md**

Update docs/architecture.md with crate dependency graph and data flow.

- [ ] **Step 2: Create contributing.md**

Create docs/contributing.md with code style, error handling, testing, and git workflow guidelines.

- [ ] **Step 3: Commit**

```bash
git add docs/architecture.md docs/contributing.md
git commit -m "docs: update architecture and contributing guides"
```

---

### Task 4: Development workflow optimization

**Files:**
- Modify: justfile

- [ ] **Step 1: Add check command**

Add just check command for quick local validation.

- [ ] **Step 2: Verify just ci works**

Run: just ci
Expected: All checks pass

- [ ] **Step 3: Commit**

```bash
git add justfile
git commit -m "chore: add just check command"
```

---

### Task 5: Final verification

**Files:**
- None (verification only)

- [ ] **Step 1: Run full CI suite**

Run: just ci
Expected: All checks PASS

- [ ] **Step 2: Run all tests**

Run: cargo test --all
Expected: All tests PASS, 0 failures

- [ ] **Step 3: Run all clippy**

Run: cargo clippy --all
Expected: Zero warnings

- [ ] **Step 4: Build all examples**

Run: cargo build --examples
Expected: All examples compile
