# Phase 2: Layer 6 Application Crate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Goal:** Polish engine-editor (the top-level application crate) with tests, docs, and workflow documentation.

**Architecture:** engine-editor depends on 11 other engine crates. Focus on testing UI components, documenting the editor workflow, and ensuring all editor features work correctly.

**Tech Stack:** Rust 2024 edition, egui, thiserror, anyhow, cargo test, cargo clippy

---

### Task 1: Polish engine-editor

**Files:**
- Modify: crates/engine-editor/src/*.rs
- Create: crates/engine-editor/tests/editor_tests.rs

- [ ] **Step 1: Add module-level documentation**

Add to crates/engine-editor/src/lib.rs:

`ust
//! # engine-editor
//!
//! Visual editor for the RustEngine.
//!
//! Features:
//! - Scene hierarchy panel
//! - Inspector panel
//! - Resource browser
//! - Viewport with gizmo controls
//! - Menu bar and toolbar
//! - Undo/redo system
//! - Scene serialization
//! - Animation editor
//! - Material editor
//! - Node graph editor
//! - Terrain editor
//! - Script editor
//!
//! ## Architecture
//!
//! The editor is built as a plugin for engine-core:
//!
//! `	ext
//! EditorPlugin -> EditorState -> Panels (Hierarchy, Inspector, Browser, Viewport)
//! `
//!
//! Each panel is a separate module that communicates via the editor state.
//!
//! ## Quick Start
//!
//! `ust
//! use engine_editor::Editor;
//!
//! let editor = Editor::new()?;
//! editor.run();
//! `
`

- [ ] **Step 2: Add documentation to all public functions**

- [ ] **Step 3: Add UI component tests**

Create crates/engine-editor/tests/editor_tests.rs:

`ust
use engine_editor::Editor;
use engine_editor::panels::{Hierarchy, Inspector, ResourceBrowser};

#[test]
fn test_editor_creation() {
    let editor = Editor::new();
    assert!(editor.is_ok());
}

#[test]
fn test_hierarchy_panel() {
    let mut hierarchy = Hierarchy::new();
    assert!(hierarchy.is_empty());

    hierarchy.add_node("Root");
    assert_eq!(hierarchy.node_count(), 1);

    let child = hierarchy.add_node("Child");
    hierarchy.set_parent(child, hierarchy.root());
    assert_eq!(hierarchy.children(hierarchy.root()).len(), 1);
}

#[test]
fn test_hierarchy_search() {
    let mut hierarchy = Hierarchy::new();
    hierarchy.add_node("Player");
    hierarchy.add_node("Enemy");
    hierarchy.add_node("Camera");

    let results = hierarchy.search("Player");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name(), "Player");
}

#[test]
fn test_inspector_panel() {
    let mut inspector = Inspector::new();
    assert!(inspector.selected_entity().is_none());

    inspector.select_entity(Some(42));
    assert_eq!(inspector.selected_entity(), Some(42));
}

#[test]
fn test_resource_browser() {
    let browser = ResourceBrowser::new("assets/");
    assert!(browser.is_ok());

    let browser = browser.unwrap();
    let files = browser.list_files(".");
    assert!(files.is_ok());
}

#[test]
fn test_undo_redo() {
    let mut editor = Editor::new().unwrap();

    // Perform an action
    editor.execute_command(CreateNodeCommand::new("Test"));
    assert_eq!(editor.scene().node_count(), 1);

    // Undo
    editor.undo();
    assert_eq!(editor.scene().node_count(), 0);

    // Redo
    editor.redo();
    assert_eq!(editor.scene().node_count(), 1);
}
`

- [ ] **Step 4: Add scene serialization tests**

Add to crates/engine-editor/tests/editor_tests.rs:

`ust
use engine_editor::SceneSerializer;

#[test]
fn test_scene_save_load() {
    let mut editor = Editor::new().unwrap();
    editor.execute_command(CreateNodeCommand::new("Node1"));
    editor.execute_command(CreateNodeCommand::new("Node2"));

    // Save scene
    let json = editor.save_scene_to_json().unwrap();
    assert!(!json.is_empty());

    // Load scene
    let mut new_editor = Editor::new().unwrap();
    new_editor.load_scene_from_json(&json).unwrap();
    assert_eq!(new_editor.scene().node_count(), 2);
}
`

- [ ] **Step 5: Run tests**

Run: cargo test -p engine-editor
Expected: All tests PASS

- [ ] **Step 6: Add editor workflow documentation**

Create docs/editor-workflow.md:

`markdown
# Editor Workflow Guide

## Getting Started

1. Run the editor: cargo run -p engine-editor
2. Create a new scene: File > New Scene
3. Add entities: Right-click in hierarchy > Create Node
4. Edit properties: Select entity, modify in Inspector
5. Save: Ctrl+S

## Panels

### Hierarchy
- Tree view of all entities
- Drag-and-drop to reparent
- Right-click context menu

### Inspector
- Edit selected entity properties
- Transform, Material, Physics, etc.
- Component add/remove

### Resource Browser
- Browse project files
- Drag assets into scene
- Preview textures and models

### Viewport
- 3D/2D scene view
- Camera controls (orbit, pan, zoom)
- Gizmo controls (translate, rotate, scale)

## Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+Z | Undo |
| Ctrl+Y | Redo |
| Ctrl+S | Save |
| Delete | Delete selected |
| F | Focus on selected |
| W | Translate gizmo |
| E | Rotate gizmo |
| R | Scale gizmo |
`

- [ ] **Step 7: Run clippy**

Run: cargo clippy -p engine-editor
Expected: Zero warnings

- [ ] **Step 8: Commit**

`ash
git add crates/engine-editor/
git add docs/editor-workflow.md
git commit -m "feat(editor): add docs, tests, workflow guide for engine-editor"
`

---

### Task 2: Final verification for Layer 6

**Files:**
- None (verification only)

- [ ] **Step 1: Run full test suite**

Run: cargo test -p engine-editor
Expected: All tests PASS

- [ ] **Step 2: Run clippy**

Run: cargo clippy -p engine-editor
Expected: Zero warnings

- [ ] **Step 3: Verify documentation**

Run: cargo doc -p engine-editor --no-deps
Expected: No warnings about missing docs
