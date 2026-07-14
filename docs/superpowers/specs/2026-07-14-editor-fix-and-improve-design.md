# Editor Fix and Improvement Design

## Overview

Comprehensive fix and improvement of the RustEngine editor, referencing Unity documentation patterns. Phased implementation to prioritize critical bugs first, then core features, then polish.

## Phase 1: Fix Critical Bugs

### 1.1 Gizmo Interactive Handles

**Problem:** Gizmo renders visually but has no mouse interaction — users cannot drag to move/rotate/scale objects.

**Design:**
- Add screen-space ray casting in `gizmo.rs` to detect mouse hover over gizmo axes
- Implement drag state machine: Idle → Hover → Dragging → Idle
- Calculate transform delta from mouse delta projected onto gizmo axis direction
- Route mouse events from `viewport.rs` through gizmo handler
- Reference: Unity's `HandleUtility` and `Handles.TransformHandle`

**Implementation:**
- `gizmo.rs`: Add `GizmoState` (idle/hover/dragging), `hover_axis`, `drag_start_screen`, `drag_start_world`
- `gizmo.rs`: Implement `handle_translate_drag()`, `handle_rotate_drag()`, `handle_scale_drag()`
- `viewport.rs`: On `CursorMoved`, call `gizmo.update_hover()` then `gizmo.handle_drag()`
- On drag end, push `TransformEntityCommand` to undo stack

### 1.2 Viewport Depth Buffer

**Problem:** `viewport_renderer.rs` render pass has `depth_stencil_attachment: None` — 3D objects render without depth testing, causing incorrect occlusion.

**Design:**
- Create depth texture alongside color texture in `ViewportRenderer`
- Attach depth to render pass in `clear_target()`
- Resize depth texture when viewport resizes

**Implementation:**
- `viewport_renderer.rs`: Add `depth_view: wgpu::TextureView`, `depth_texture: wgpu::Texture`
- In `new()`: create depth texture with `TextureFormat::Depth32Float`
- In `clear_target()`: add `depth_stencil_attachment` to `RenderPassDescriptor`
- In `resize()`: recreate depth texture

### 1.3 Undo/Redo Completeness

**Problem:** `SculptCommand::undo()` and `redo()` are empty stubs. `DeleteEntityCommand::undo()` creates new entity IDs breaking references.

**Design:**
- `SculptCommand`: Store heightmap snapshot before sculpt, restore on undo
- `DeleteEntityCommand`: Store full entity data (transform, components, hierarchy), restore with original ID on undo

**Implementation:**
- `commands.rs`:
  - `SculptCommand` add `heightmap_snapshot: Vec<f32>`, `affected_vertices: Vec<(u32, u32)>`
  - `SculptCommand::undo()`: restore snapshot values to heightmap
  - `DeleteEntityCommand`: store `entity_data: EntitySnapshot` with full component data
  - `DeleteEntityCommand::undo()`: recreate entity with stored data (not new ID)

### 1.4 Scene Bridge Hierarchy Preservation

**Problem:** `scene_bridge.rs` `import_world()` creates flat entity list — parent-child relationships lost.

**Design:**
- Add `parent_index: Option<usize>` to `SceneEntity`
- During import, track index mapping, set parent-child after all entities created

**Implementation:**
- `scene_bridge.rs`:
  - `SceneEntity` add `parent_index: Option<usize>`
  - `import_world()`: build index map `entity_index -> world_entity`, then loop setting parents
  - `export_world()`: record parent index in `SceneEntity`

## Phase 2: Core Feature Completion

### 2.1 Resource Browser Enhancement

**Current:** Basic file listing with path navigation, no search/preview/drag.

**Design:**
- Add search filter (text input, filter by name/extension)
- Add file type icons (folder, image, mesh, script, material)
- Add drag-and-drop from browser to viewport (asset instantiation)
- Add file context menu (rename, delete, show in explorer)

**Implementation:**
- `resource_browser.rs`:
  - Add `search_query: String`, `filtered_files: Vec<FileInfo>`
  - Add `file_type_icon()` returning colored text/icon per extension
  - Implement `draw_file_item()` with icon + name + drag source
  - On drop to viewport: detect asset type, create appropriate entity

### 2.2 Hot Reload Implementation

**Current:** File watcher detects changes but doesn't reload assets.

**Design:**
- On file change: detect asset type, reload into asset store
- For textures: re-upload to GPU
- For materials: recompile shader
- For scenes: prompt user to reload
- Add visual notification bar

**Implementation:**
- `hot_reload.rs`:
  - Add `reload_texture(path)`, `reload_material(path)`, `reload_scene(path)`
  - Add `ReloadNotification` struct with message, timestamp, status
  - In `process_reload()`: call appropriate reload based on extension
- `state.rs`: integrate `HotReloadSystem` into main loop, poll every frame

### 2.3 Inspector Panel Fix

**Current:** `InspectorPanel::new()` called every frame — search state lost.

**Design:**
- Store `InspectorPanel` in `EditorState` as persistent field
- Only recreate on explicit reset

**Implementation:**
- `state.rs`: Add `inspector_panel: InspectorPanel` field
- `inspector.rs`: Remove per-frame `InspectorPanel::new()`, use `state.inspector_panel` directly
- Add `reset_inspector()` method for explicit reset

## Phase 3: Polish and Missing Features

### 3.1 Script Editor Basics

**Current:** No cursor, no selection, no undo.

**Design:**
- Add cursor position tracking (line, column)
- Add text selection (shift+arrow, shift+click)
- Add clipboard operations (Ctrl+C/V/X)
- Add undo/redo for text edits

**Implementation:**
- `script_editor/`:
  - Add `Cursor { line, col }`, `Selection { start, end }`
  - Handle arrow keys, shift, home/end
  - Implement `TextBuffer` with edit history

### 3.2 Performance Profiler Connection

**Current:** UI exists but shows zeros.

**Design:**
- Add `ProfilerData` resource fed by render pipeline
- Record per-frame: draw calls, triangle count, GPU time
- Display in profiler panel

**Implementation:**
- `performance_profiler.rs`: Add `record_frame(data: ProfilerData)`
- `viewport_renderer.rs`: After render, record stats to `ProfilerData`

### 3.3 Editor Preferences

**Current:** No persistent settings.

**Design:**
- `EditorPreferences` struct: shortcut bindings, layout, theme, recent files
- Save to `~/.rustengine/editor.json` or project `.editorconfig`
- Load on editor startup

**Implementation:**
- New file `preferences.rs`:
  - `EditorPreferences` with serde
  - `load_preferences()`, `save_preferences()`
  - Integrate into `EditorState::new()`

## Files Modified

| Phase | File | Changes |
|-------|------|---------|
| 1 | `gizmo.rs` | Add interaction state machine, drag handlers (~300 lines) |
| 1 | `viewport.rs` | Route mouse events to gizmo, integrate undo |
| 1 | `viewport_renderer.rs` | Add depth texture, depth stencil attachment |
| 1 | `commands.rs` | Fix SculptCommand undo/redo, fix DeleteEntityCommand ID |
| 1 | `scene_bridge.rs` | Add parent_index, preserve hierarchy |
| 2 | `resource_browser.rs` | Add search, icons, drag-and-drop (~200 lines) |
| 2 | `hot_reload.rs` | Add actual reload logic, notification bar |
| 2 | `inspector.rs` | Remove per-frame reconstruction |
| 2 | `state.rs` | Add InspectorPanel field, integrate hot reload |
| 3 | `script_editor/mod.rs` | Add cursor, selection, clipboard, undo |
| 3 | `performance_profiler.rs` | Connect to render pipeline data |
| 3 | `preferences.rs` | New file for persistent settings |

## Testing Strategy

- Each phase: verify with `cargo clippy && cargo fmt --check && cargo test`
- Gizmo: manual test — click and drag on gizmo axes in viewport
- Depth buffer: verify 3D objects occlude correctly
- Undo/redo: test sculpt undo, delete+undo preserves references
- Resource browser: test search, drag to scene
- Hot reload: modify texture file, verify editor updates

## Success Criteria

1. Phase 1: Gizmo interaction works, 3D rendering correct, undo/redo reliable
2. Phase 2: Resource browser usable, hot reload functional, inspector stable
3. Phase 3: Script editor has basic editing, profiler shows real data, preferences persist
