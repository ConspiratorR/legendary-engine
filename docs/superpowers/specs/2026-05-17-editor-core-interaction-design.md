# Editor Core Interaction — Design Spec

**Date:** 2026-05-17
**Project:** RustEngine
**Author:** ConspiratorR
**Status:** Draft

## Overview

Phase 1 of the RustEngine editor: implement core scene-editing interaction
features on top of the existing editor layout shell.

### Scope

1.  Hierarchy panel — full CRUD + drag reorder + search filter
2.  3D viewport camera — orbit, pan, zoom with mouse
3.  Full transform Gizmo — translate / rotate / scale with axis handles
4.  Inspector — live editing of Transform + extensible component system
5.  Wiring via `crates/engine-editor/EditorPlugin`

### Non-Goals (Phase 1)

- Asset browser / content drawer
- Material editor graph
- Animation timeline
- Build dialog / project wizard / settings
- Direct ECS integration (editor keeps its own scene tree)

---

## Architecture

```
                         EditorPlugin
  (registered on AppBuilder, driven in post_update hook)
                              │
                              ▼
                         EditorState
         ┌──────────┬──────────┬──────────┬──────────┐
         │Hierarchy │ Viewport │  Gizmo   │Inspector │
         │  Panel   │  Camera  │  Tools   │+Registry │
         └──────────┴──────────┴──────────┴──────────┘
                              │
                              ▼
                        engine-ui
                    (Gui + GuiSkin)
```

`EditorPlugin` is added to `AppBuilder`. On each frame, the post-update hook
reads the shared `GuiSkin` and `EguiState` from app resources, retrieves or
creates `EditorState`, and calls `state.frame(ctx, &skin)`.

### Module Layout

```
crates/engine-editor/
├── Cargo.toml
└── src/
    ├── lib.rs            # EditorPlugin + public API re-exports
    ├── state.rs          # EditorState, ToolType, GizmoInteraction
    ├── hierarchy.rs      # HierarchyPanel — draw + event handling
    ├── viewport.rs       # Viewport — camera controls + grid drawing
    ├── camera.rs         # EditorCamera — orbit/pan/zoom math
    ├── gizmo.rs          # TransformGizmo — axis picking + dragging
    └── inspector.rs      # Inspector — component registry + editors
```

### Data Flow

```
egui events
     │
     ▼
EditorState.frame(ctx, skin)
     │
     ├── hierarchy.draw(&mut state, gui)    → mutates state.scene_tree
     ├── viewport.draw(&mut state, gui)     → mutates state.camera
     ├── gizmo.draw(&mut state, gui)        → mutates selection transform
     └── inspector.draw(&mut state, gui)    → mutates selection components
```

Each module's `draw()` receives `&mut EditorState` and writes changes
directly — no message bus or event queue needed at this scale.

---

## Core Types (`state.rs`)

### EditorState

```rust
pub enum ToolType { Select, Translate, Rotate, Scale }

pub struct GizmoInteraction {
    pub axis: u8,                    // bitmask: X=1, Y=2, Z=4
    pub plane: Option<u8>,           // XY=3, XZ=5, YZ=6
    pub start_mouse: Pos2,
    pub start_value: f32,            // for single-axis delta
}

pub struct EditorState {
    // Selection
    pub selected_nodes: Vec<u64>,

    // Tools
    pub active_tool: ToolType,
    pub active_viewport_tab: usize,  // 0=scene, 1=game, 2=physics

    // Panels
    pub show_left_panel: bool,
    pub show_right_panel: bool,

    // Scene tree (editor-local, not ECS)
    pub scene_tree: SceneTree,

    // Viewport camera
    pub camera: EditorCamera,
    pub show_grid: bool,

    // Gizmo
    pub show_gizmo: bool,
    pub gizmo_interaction: Option<GizmoInteraction>,

    // Inspector
    pub inspector_search: String,
    pub component_registry: ComponentRegistry,
}

impl EditorState {
    /// Called once per frame from EditorPlugin's post-update hook.
    /// Dispatches draw calls to each panel module in layout order.
    pub fn frame(&mut self, ctx: &egui::Context, skin: &GuiSkin) {
        // 1. Layout rects (unchanged from current editor.rs)
        // 2. hierarchy.draw(self, gui)
        // 3. viewport.draw(self, gui)
        // 4. gizmo.draw(self, gui)
        // 5. inspector.draw(self, gui)
    }
}
```

---

## Scene Tree (`hierarchy.rs`)

### Data Types

```rust
pub struct SceneTree {
    pub nodes: Vec<TreeNode>,
    pub root_ids: Vec<u64>,
}

pub struct TreeNode {
    pub id: u64,
    pub name: String,
    pub icon: String,
    pub expanded: bool,
    pub parent: Option<u64>,
    pub children: Vec<u64>,
    pub components: Vec<Box<dyn ComponentEditor>>,
}

impl SceneTree {
    pub fn add_node(&mut self, name: &str, parent: Option<u64>) -> u64;
    pub fn remove_node(&mut self, id: u64);          // cascading
    pub fn reparent(&mut self, id: u64, new_parent: Option<u64>);
    pub fn rename(&mut self, id: u64, name: &str);
    pub fn search(&self, query: &str) -> Vec<u64>;   // matching ids
}
```

### Hierarchy Panel

```
┌──────────────────────┐
│ 层 级            + 🔍│  header
├──────────────────────┤
│ 搜索...              │  filter field
├──────────────────────┤
│ 📁 Root              │
│  ├─ 🎮 Player        │  ← click = select
│  ├─ 🏔 Terrain       │     drag = reorder
│  └─ 📦 Props ▸       │     arrow = expand/collapse
│     ├─ 📦 Cube(1)   │
│     └─ 📦 Cube(2)   │
└──────────────────────┘
```

**Interactions:**
- **Click** → select single node (Ctrl+click = toggle multi-select)
- **Drag** → `egui::Sense::click_and_drag()` → compute drop index → `reparent()`
- **Right-click** → context menu: Rename, Delete, Create Empty, Duplicate
- **Toolbar "+"** → creates empty child under root
- **Search** → filters visible nodes; matching branches auto-expand

---

## Viewport Camera (`camera.rs` + `viewport.rs`)

### EditorCamera

```rust
pub struct EditorCamera {
    pub target: Vec3,       // orbit pivot point
    pub distance: f32,      // distance from target
    pub yaw: f32,           // horizontal angle (radians)
    pub pitch: f32,         // vertical angle (radians, clamped ±89°)
    pub fov: f64,           // vertical FOV in radians
    pub near: f64,
    pub far: f64,
}

impl EditorCamera {
    /// Apply mouse deltas from viewport interaction
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32);
    pub fn pan(&mut self, delta_x: f32, delta_y: f32);
    pub fn zoom(&mut self, delta: f32);

    /// Compute view matrix (LH coordinate system)
    pub fn view_matrix(&self) -> Mat4;

    /// Compute projection matrix from viewport aspect ratio
    pub fn projection_matrix(&self, aspect: f32) -> Mat4;

    /// Eye position derived from target + distance + yaw/pitch
    pub fn eye(&self) -> Vec3;
}
```

### Mouse Mapping

| Button | Action |
|--------|--------|
| Right-click drag | Orbit |
| Middle-click drag | Pan |
| Scroll wheel | Zoom |

All deltas scaled by `sensitivity` multiplier (default 0.5° per pixel for
orbit, 0.01 × distance for pan).

### Viewport Drawing

1. Fill `Rect` with dark gradient (matching existing style)
2. Draw grid using camera's view-projection matrix (lines on XZ plane)
3. Draw axis labels (X red, Y green, Z blue) in screen-space corner
4. Draw scene nodes as simple colored rects (placeholder until 3D renderer)
5. Draw transform info overlay (position values in corner)

---

## Transform Gizmo (`gizmo.rs`)

### Modes

| Tool | Visual | Interaction |
|------|--------|-------------|
| Translate | 3 arrows (conical) | Drag → move along axis |
| Rotate | 3 rings (toroidal) | Drag → rotate around axis |
| Scale | 3 cubes + center cube | Drag → scale along axis / uniform |

### Axis Colors

| Axis | Color |
|------|-------|
| X | `#ff6b6b` (red) |
| Y | `#2ed573` (green) |
| Z | `#4dabf7` (blue) |

Hovered axis → 50% brighter. Active (being dragged) → 100% brighter + glow.

### Picking

Each axis handle is represented as a screen-space bounding region computed
from the 3D world-space positions of the gizmo projected into the viewport.

1. On mouse press in viewport: test each axis region for overlap; if hit,
   record `GizmoInteraction { axis, start_mouse, start_value }`.
2. On mouse move while `gizmo_interaction.is_some()`: compute delta in
   screen-space and convert to world-space displacement along the active
   axis. Update the selected entity's Transform.
3. On mouse release: clear `gizmo_interaction`.

### Position

Gizmo is drawn at world-space position of the **last selected node**'s
transform translation. If no node selected, gizmo is hidden.

---

## Inspector (`inspector.rs`)

### Component Registry

```rust
pub trait ComponentEditor: Send + Sync {
    fn name(&self) -> &'static str;
    fn draw(&mut self, ui: &mut Gui, rect: Rect, state: &mut EditorState);
    fn clone_box(&self) -> Box<dyn ComponentEditor>;
}

pub struct ComponentRegistry {
    editors: Vec<Box<dyn ComponentEditor>>,
    available: Vec<&'static str>,  // names of registerable component types
}

impl ComponentRegistry {
    /// Register a component editor type. A default instance is stored
    /// and cloned via `clone_box()` each time it is added to an entity.
    pub fn register<T: ComponentEditor + Default + 'static>(&mut self);
    /// Clone a default editor instance by name and push into entity's list.
    pub fn add_to_entity(&mut self, entity_id: u64, component_name: &str);
    /// Remove component at `index` from entity's list.
    pub fn remove_from_entity(&mut self, entity_id: u64, index: usize);
    /// Draw all components of the selected entity in the inspector rect.
    pub fn draw_for_entity(&mut self, gui: &mut Gui, rect: Rect, entity_id: u64, state: &mut EditorState);
}
```

### Built-in Editors

1. **TransformComponent** — Position / Rotation / Scale (Vec3 inputs)
2. **RenderComponent** — Material (text), Mesh (text), Cast Shadow (checkbox)
3. **PhysicsComponent** — Body Type (dropdown: Static/Dynamic/Kinematic),
   Collision Shape (dropdown: Box/Sphere/Capsule)

### Inspector Layout

```
┌──────────────────────┐
│ 🔍 搜索属性...       │  filter
├──────────────────────┤
│ 变换                 │
│ ─────────────────── │
│ 位置  X [300] Y[200] │  vec3_input (editable)
│ 旋转  X [0]   Y[0]   │
│ 缩放  X [1.0] Y[1.0] │
│                      │
│ 渲染                 │
│ ─────────────────── │
│ 材质  Default        │  input_labeled
│ 网格  Cube           │
│ ☑ 投射阴影           │  checkbox
│                      │
│ 物理                 │
│ ─────────────────── │
│ 刚体  Static         │  dropdown
│ 碰撞  Box            │  dropdown
│                      │
│ [+ 添加组件]         │  button → show available list
└──────────────────────┘
```

Each component section has a `···` menu (top-right) with "Remove" action.
The "+" button at the bottom opens a popup list of registered component
types not yet on this entity.

---

## Wiring (`lib.rs`)

```rust
pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_post_update_hook(Box::new(|app: &mut App| {
            let skin = app.resources
                .get::<GuiSkin>().cloned().unwrap_or_default();
            let egui_state = app.resources.get_mut::<EguiState>().unwrap();
            let state = app.resources
                .get_or_insert_with(|| EditorState::new());

            state.frame(egui_state.ctx(), &skin);
        }));
    }
}
```

The `examples/basic/src/main.rs` changes from:

```rust
.add_plugin(GamePlugin)   // <- old: editor baked into GamePlugin
```

to:

```rust
.add_plugin(EditorPlugin)
.add_plugin(GamePlugin)
```

`examples/basic/src/editor.rs` is removed. Its visual definition moves into
`engine-editor`.

---

## Integration Points

| Crate | Dependency |
|-------|-----------|
| `engine-editor` | `engine-ui` (Gui, GuiSkin) |
| | `engine-math` (Vec3, Mat4, Quat) |
| | `engine-core` (Plugin, AppBuilder) |
| | `engine-framework` (StateStack — for future game-mode toggle) |

No dependency on `engine-scene` or `engine-ecs` — editor maintains its own
scene tree for Phase 1.

---

## Testing Strategy

- **Unit tests** for `SceneTree` CRUD operations (add, remove, reparent)
- **Unit tests** for `EditorCamera` math (orbit limits, zoom clamping,
  view_matrix sanity)
- **Unit tests** for `ComponentRegistry` (register, add/remove from entity)
- **Integration** via `cargo test` in workspace — existing engine-ui tests
  remain passing

No visual/UI regression tests in Phase 1.

---

## Future Considerations (Not Implemented)

- Wire editor scene tree ↔ engine SceneManager via sync bridge
- Replace placeholder viewport rects with actual 3D rendering
- Gizmo screen-space → world-space ray casting for precise picking
- Undo/Redo system
- Keyboard shortcuts
