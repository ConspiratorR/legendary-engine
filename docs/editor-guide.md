# Editor Usage Guide

RustEngine includes a built-in editor for scene authoring, debugging, and asset management.

## Launching the Editor

The editor is launched by running the engine with the `EditorPlugin`:

```rust
use engine_core::app::AppBuilder;
use engine_editor::EditorPlugin;
use engine_ui::EguiPlugin;

let mut app = AppBuilder::new();
app.add_plugin(EguiPlugin);
app.add_plugin(EditorPlugin);
```

## Editor Panels

### Viewport

The 3D viewport shows your scene with an orbit camera:

- **Middle mouse** — Pan
- **Right mouse** — Orbit
- **Scroll wheel** — Zoom

### Scene Hierarchy

The hierarchy panel shows all scene nodes in a tree structure:

- Click to select
- Drag to reparent
- Right-click for context menu

### Inspector

The inspector shows properties of the selected node:

- Transform (position, rotation, scale)
- Components attached to the entity
- Material properties
- Light settings

### Resource Browser

Browse and manage project assets:

- Navigate the file tree
- Drag assets into the viewport
- Preview textures and meshes

## Editor State

The `EditorState` resource tracks editor-level data:

```rust
use engine_editor::state::EditorState;

let state = app.resources.get::<EditorState>();
```

Key fields:

- `scene_tree` — The scene hierarchy
- `selected_node` — Currently selected node
- `editor_camera` — The viewport camera
- `lights` — Light definitions in the scene
- `materials` — Material definitions in the scene

## Scene Serialization

Save and load scenes:

```rust
use engine_editor::scene_serializer;

// Save current scene
scene_serializer::save_scene(&state, "scenes/level1.json")?;

// Load a scene
scene_serializer::load_scene(&mut state, "scenes/level1.json")?;
```

## Gizmos

Transform gizmos for moving, rotating, and scaling objects:

- **W** — Move tool
- **E** — Rotate tool
- **R** — Scale tool

## Shortcuts

Common keyboard shortcuts:

| Key | Action |
|-----|--------|
| `W` | Move tool |
| `E` | Rotate tool |
| `R` | Scale tool |
| `F` | Focus selected |
| `Delete` | Delete selected |
| `Ctrl+S` | Save scene |
| `Ctrl+Z` | Undo |
| `Ctrl+Y` | Redo |
| `Ctrl+N` | New scene |
| `Ctrl+D` | Duplicate selected |

## Animation Editor

Edit animation clips and state machines:

- Create animation clips
- Add keyframes for position, rotation, scale
- Set up state machine transitions
- Preview animations in the viewport

## Prefab System

Create reusable scene templates:

- Select nodes in the hierarchy
- Create prefab from selection (right-click menu)
- Instantiate prefabs into the scene
- Save/load prefabs as JSON files

## Visual Scripting (Blueprints)

Create game logic without code:

- Node graph editor for visual scripting
- Execution flow nodes (Sequence, Branch, Loop, Delay, Event)
- Data nodes (Math, Transform, Input, Physics Raycast, Spawn Entity)
- Blueprints run during Play mode (begin_play + tick)

## Asset Management

Manage project assets with .meta files:

- Each asset has a companion `.meta` file with GUID and import settings
- Import settings: Texture (max size, mipmaps, compression), Mesh (LOD, scale), Audio (sample rate, streaming)
- Asset browser for file navigation
- Drag assets into the viewport

## Play Mode

Test your game in the editor:

- **Play** — Start the game runtime (ECS, physics, audio, scripts)
- **Pause** — Freeze the runtime
- **Stop** — Return to editor mode
- Game viewport shows runtime camera view
- Debug visualization shows colliders and bounding boxes
