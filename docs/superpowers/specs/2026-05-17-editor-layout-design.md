# Editor Layout Design

## Goal
Implement the IMGUI editor layout matching `design/editor.html` using the existing engine-ui Gui/GuiLayout system, with all necessary new controls added to `gui.rs`.

## Architecture

```
gui.rs (new primitives)       editor.rs (example new file)
┌─────────────────────┐       ┌──────────────────────────┐
│ menu_bar             │       │  EditorLayout  struct    │
│ tool_button          │◄──────│  ├─ frame(ui, skin)     │
│ tab                  │       │  │  ├─ draw_menu_bar    │
│ tree_node            │       │  │  ├─ draw_toolbar     │
│ panel_header         │       │  │  ├─ draw_hierarchy   │
│ colored_label        │       │  │  ├─ draw_viewport    │
│ vec3_input           │       │  │  ├─ draw_inspector   │
│ checkbox             │       │  │  ├─ draw_bottom      │
│ input_labeled        │       │  │  └─ draw_status_bar  │
│ status_item          │       │  └─ per-frame call      │
└─────────────────────┘       └──────────────────────────┘
```

## Layout Structure

```
┌─ Menu Bar (32px) ────────────────────────────────────┐
├─ Toolbar (44px) ─────────────────────────────────────┤
├─ Hierarchy(260px) ┼─ Viewport ───────────┼─ Inspector(300px) │
├─ Bottom Panel (180px) ───────────────────────────────┤
├─ Status Bar (24px) ──────────────────────────────────┤
```

## New Controls (gui.rs)

11 new methods on `Gui`:

| Control | Signature | Behavior |
|---------|-----------|----------|
| `menu_bar` | `fn(&mut self, Rect, &[&str]) -> Option<usize>` | Dark bar + horizontal items; returns clicked index |
| `tool_button` | `fn(&mut self, Rect, &str, bool) -> bool` | Icon button; active state fills accent color |
| `tab` | `fn(&mut self, Rect, &str, bool) -> bool` | Active: bottom border accent; Inactive: muted text |
| `tree_node` | `fn(&mut self, Rect, &str, &str, bool, u32) -> bool` | Indent by depth; selected bg highlight |
| `panel_header` | `fn(&mut self, Rect, &str) -> Rect` | Title + underline; returns content area |
| `colored_label` | `fn(&mut self, Rect, &str, Color32)` | Text in given color |
| `vec3_input` | `fn(&mut self, Rect, &str, &mut f32, &mut f32, &mut f32)` | Label + X(red) Y(green) Z(blue) drag inputs |
| `checkbox` | `fn(&mut self, Rect, &str, &mut bool)` | □/☑ toggle |
| `input_labeled` | `fn(&mut self, Rect, &str, &mut String)` | Label + read-only text field |
| `status_item` | `fn(&mut self, Rect, &str, Color32)` | Dot + label (display only) |
| `separator_h` | `fn(&mut self, Rect)` | Horizontal line |

Each follows the existing pattern: `draw_background` for fills, `ui.painter().rect_filled/text` for rendering, `ui.rect_intersects_pointer` for hit testing.

Notes:
- `menu_bar` items are clickable headers only (no dropdown sub-menus in v1)
- `vec3_input`, `input_labeled`, `checkbox` are click-to-modify for scalar/boolean values;
  text fields (`input_labeled`) are read-only matching editor.html's `<input readonly>`
- Gui struct is borrowed for a single `Ui`; the editor creates one `Gui` per frame and calls
  all draw methods sequentially

## EditorLayout (examples/basic/src/editor.rs)

### State
```rust
pub struct EditorLayout {
    // Control state
    active_menu: Option<usize>,
    active_tool: usize,          // 0=select,1=move,2=rotate,3=scale
    active_viewport: usize,      // 0=scene,1=game,2=physics
    active_bottom_tab: usize,    // 0=log,1=perf,2=audio,3=network
    show_left_panel: bool,
    show_right_panel: bool,
    show_grid: bool,

    // Scene tree
    selected_node: usize,
    scene_tree: Vec<SceneNode>,

    // Inspector values
    pos: [f32; 3],
    rot: [f32; 3],
    scale: [f32; 3],
    material_name: String,
    mesh_name: String,
    cast_shadow: bool,

    // Viewport info
    fps: u32,
}
```

### Data Flow

1. `EditorLayout::new()` — initializes default values
2. Every frame: `editor.frame(ctx, &skin)` called from post_update_hook
3. `frame()` creates `egui::Area("editor")` with full-screen rect
4. Inside Area: creates `Gui`, splits rects, calls draw methods
5. Each draw method renders + updates state based on click results

## Files Changed

| File | Change |
|------|--------|
| `crates/engine-ui/src/gui.rs` | Add 11 new control methods + tests |
| `examples/basic/src/editor.rs` | NEW: EditorLayout struct + draw logic |
| `examples/basic/src/main.rs` | Use EditorLayout instead of inline demo |

## Testing

- Each new gui.rs control: 1 unit test (draws without panic + correct return value)
- No automated tests for editor.rs (visual composition, verified by running example)
