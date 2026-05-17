# Editor Core Interaction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build core scene-editing interaction features (hierarchy CRUD, 3D camera, gizmo, inspector with component system) as a first-class `crates/engine-editor` crate.

**Architecture:** Editor is a separate crate (`engine-editor`) that implements `Plugin` and is added to `AppBuilder`. It maintains its own scene tree (not directly coupled to engine ECS). Each panel (hierarchy, viewport, gizmo, inspector) is a module in `src/` that draws into egui rects and mutates `EditorState` directly.

**Tech Stack:** Rust 2024, egui 0.30, glam (via engine-math), engine-core (Plugin trait), engine-ui (Gui, GuiSkin)

---

### Task 0: Scaffold `crates/engine-editor/`

**Files:**
- Create: `crates/engine-editor/Cargo.toml`
- Create: `crates/engine-editor/src/lib.rs` (minimal skeleton — re-export modules)

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "engine-editor"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
engine-core = { path = "../engine-core" }
engine-ui = { path = "../engine-ui" }
engine-math = { path = "../engine-math" }
egui = "=0.30.0"
```

- [ ] **Step 2: Create minimal lib.rs**

```rust
pub mod state;
pub mod hierarchy;
pub mod camera;
pub mod viewport;
pub mod gizmo;
pub mod inspector;

mod plugin;
pub use plugin::EditorPlugin;
```

- [ ] **Step 3: Create src/plugin.rs**

```rust
use engine_core::app::{App, AppBuilder};
use engine_core::plugin::Plugin;
use engine_ui::EguiState;
use engine_ui::GuiSkin;
use crate::state::EditorState;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(EditorState::new());
        app.add_post_update_hook(Box::new(|app: &mut App| {
            let skin = app.resources.get::<GuiSkin>().cloned().unwrap_or_default();
            let egui_state = app.resources.get_mut::<EguiState>().unwrap();
            let state = app.resources.get_mut::<EditorState>().unwrap();
            state.frame(egui_state.ctx(), &skin);
        }));
    }
}
```

- [ ] **Step 4: Register in workspace Cargo.toml**

Add `"crates/engine-editor"` to the `members` list in `Cargo.toml`.

- [ ] **Step 5: Verify cargo check**

Run: `cargo check -p engine-editor`
Expected: success (no tests yet, just compiles the empty crate)

- [ ] **Step 6: Commit**

```bash
git add crates/engine-editor/ Cargo.toml Cargo.lock
git commit -m "feat(editor): scaffold engine-editor crate"
```

---

### Task 1: EditorState + SceneTree + EditorCamera Core Types

**Files:**
- Create: `crates/engine-editor/src/state.rs`

- [ ] **Step 1: Define ToolType, GizmoInteraction**

```rust
use egui::Pos2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolType {
    Select,
    Translate,
    Rotate,
    Scale,
}

#[derive(Debug, Clone)]
pub struct GizmoInteraction {
    pub axis: u8,
    pub plane: Option<u8>,
    pub start_mouse: Pos2,
    pub start_value: f32,
}
```

- [ ] **Step 2: Define TreeNode and SceneTree**

```rust
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub id: u64,
    pub name: String,
    pub icon: String,
    pub expanded: bool,
    pub parent: Option<u64>,
    pub children: Vec<u64>,
}

#[derive(Debug, Clone)]
pub struct SceneTree {
    pub nodes: Vec<TreeNode>,
    pub root_ids: Vec<u64>,
    next_id: u64,
}

impl SceneTree {
    pub fn new() -> Self {
        let root_id = 1;
        Self {
            nodes: vec![TreeNode {
                id: root_id,
                name: "Root".into(),
                icon: "📁".into(),
                expanded: true,
                parent: None,
                children: vec![2, 3, 4, 5, 6],
            }],
            root_ids: vec![root_id],
            next_id: 7,
        }
    }

    pub fn add_node(&mut self, name: &str, parent: Option<u64>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let parent_id = parent.unwrap_or(self.root_ids[0]);
        self.nodes.push(TreeNode {
            id,
            name: name.to_string(),
            icon: "📦".into(),
            expanded: false,
            parent: Some(parent_id),
            children: Vec::new(),
        });
        if let Some(p) = self.nodes.iter_mut().find(|n| n.id == parent_id) {
            p.children.push(id);
        }
        id
    }

    pub fn remove_node(&mut self, id: u64) {
        let parent_id = self.nodes.iter().find(|n| n.id == id).and_then(|n| n.parent);
        // Remove from parent's children list
        if let Some(pid) = parent_id {
            if let Some(p) = self.nodes.iter_mut().find(|n| n.id == pid) {
                p.children.retain(|c| *c != id);
            }
        }
        // Collect all descendant ids (cascading removal)
        let mut to_remove = vec![id];
        let mut i = 0;
        while i < to_remove.len() {
            let cids: Vec<u64> = self.nodes.iter()
                .filter(|n| n.parent == Some(to_remove[i]))
                .map(|n| n.id)
                .collect();
            to_remove.extend(cids);
            i += 1;
        }
        self.nodes.retain(|n| !to_remove.contains(&n.id));
    }

    pub fn reparent(&mut self, id: u64, new_parent: Option<u64>) {
        let old_parent = self.nodes.iter().find(|n| n.id == id).and_then(|n| n.parent);
        if let Some(pid) = old_parent {
            if let Some(p) = self.nodes.iter_mut().find(|n| n.id == pid) {
                p.children.retain(|c| *c != id);
            }
        }
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == id) {
            node.parent = new_parent;
        }
        if let Some(npid) = new_parent {
            if let Some(p) = self.nodes.iter_mut().find(|n| n.id == npid) {
                p.children.push(id);
            }
        }
    }

    pub fn rename(&mut self, id: u64, name: &str) {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == id) {
            node.name = name.to_string();
        }
    }

    pub fn search(&self, query: &str) -> Vec<u64> {
        if query.is_empty() {
            return Vec::new();
        }
        let q = query.to_lowercase();
        self.nodes.iter()
            .filter(|n| n.name.to_lowercase().contains(&q))
            .map(|n| n.id)
            .collect()
    }
}
```

- [ ] **Step 3: Define EditorCamera**

```rust
use engine_math::{Mat4, Vec3};

#[derive(Debug, Clone)]
pub struct EditorCamera {
    pub target: Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f64,
    pub near: f64,
    pub far: f64,
}

impl EditorCamera {
    pub fn new() -> Self {
        Self {
            target: Vec3::ZERO,
            distance: 10.0,
            yaw: 0.0,
            pitch: 0.0,
            fov: 60.0_f64.to_radians(),
            near: 0.1,
            far: 1000.0,
        }
    }

    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw += delta_x * 0.005;
        self.pitch = (self.pitch + delta_y * 0.005).clamp(-1.55, 1.55);
    }

    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let right = self.right();
        let up = self.up();
        let speed = self.distance * 0.002;
        self.target -= right * delta_x * speed;
        self.target += up * delta_y * speed;
    }

    pub fn zoom(&mut self, delta: f32) {
        self.distance = (self.distance * (1.0 - delta * 0.1)).clamp(0.5, 500.0);
    }

    pub fn eye(&self) -> Vec3 {
        let dir = self.forward();
        self.target + dir * self.distance
    }

    fn forward(&self) -> Vec3 {
        let pitch_sin = self.pitch.sin();
        let pitch_cos = self.pitch.cos();
        Vec3::new(
            self.yaw.sin() * pitch_cos,
            pitch_sin,
            self.yaw.cos() * pitch_cos,
        )
    }

    fn right(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin())
    }

    fn up(&self) -> Vec3 {
        Vec3::new(0.0, 1.0, 0.0)
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_lh(self.eye(), self.target, self.up())
    }

    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov as f32, aspect, self.near as f32, self.far as f32)
    }
}
```

- [ ] **Step 4: Write tests for SceneTree and EditorCamera**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_tree_new_has_root() {
        let tree = SceneTree::new();
        assert_eq!(tree.nodes.len(), 6); // root + 5 children from current editor.rs
    }

    #[test]
    fn test_add_node_creates_child() {
        let mut tree = SceneTree::new();
        let root_id = tree.root_ids[0];
        let child = tree.add_node("NewNode", Some(root_id));
        assert!(tree.nodes.iter().any(|n| n.id == child));
        let root = tree.nodes.iter().find(|n| n.id == root_id).unwrap();
        assert!(root.children.contains(&child));
    }

    #[test]
    fn test_remove_node_cascading() {
        let mut tree = SceneTree::new();
        let root_id = tree.root_ids[0];
        let child = tree.add_node("Child", Some(root_id));
        let grandchild = tree.add_node("Grandchild", Some(child));
        let n_before = tree.nodes.len();
        tree.remove_node(child);
        assert_eq!(tree.nodes.len(), n_before - 2);
        assert!(!tree.nodes.iter().any(|n| n.id == grandchild));
    }

    #[test]
    fn test_reparent_moves_node() {
        let mut tree = SceneTree::new();
        let root_id = tree.root_ids[0];
        let a = tree.add_node("A", Some(root_id));
        let b = tree.add_node("B", Some(root_id));
        tree.reparent(a, Some(b));
        let node_b = tree.nodes.iter().find(|n| n.id == b).unwrap();
        assert!(node_b.children.contains(&a));
    }

    #[test]
    fn test_rename_changes_name() {
        let mut tree = SceneTree::new();
        let root_id = tree.root_ids[0];
        let node = tree.add_node("Old", Some(root_id));
        tree.rename(node, "New");
        let n = tree.nodes.iter().find(|n| n.id == node).unwrap();
        assert_eq!(n.name, "New");
    }

    #[test]
    fn test_search_finds_by_name() {
        let mut tree = SceneTree::new();
        let root_id = tree.root_ids[0];
        let node = tree.add_node("PlayerCharacter", Some(root_id));
        let results = tree.search("player");
        assert!(results.contains(&node));
    }

    #[test]
    fn test_search_empty_query_returns_empty() {
        let tree = SceneTree::new();
        assert!(tree.search("").is_empty());
    }

    #[test]
    fn test_camera_initial_state() {
        let cam = EditorCamera::new();
        assert_eq!(cam.target, Vec3::ZERO);
        assert!((cam.distance - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_camera_orbit_clamps_pitch() {
        let mut cam = EditorCamera::new();
        cam.orbit(0.0, 1000.0); // extreme drag
        assert!(cam.pitch < 1.56);
        cam.orbit(0.0, -1000.0);
        assert!(cam.pitch > -1.56);
    }

    #[test]
    fn test_camera_zoom_clamps() {
        let mut cam = EditorCamera::new();
        cam.zoom(100.0);
        assert!((cam.distance - 0.5).abs() < 1e-6);
        cam.zoom(-100.0);
        assert!((cam.distance - 500.0).abs() < 1e-6);
    }

    #[test]
    fn test_view_matrix_returns_identity_equivalent() {
        let cam = EditorCamera::new();
        let view = cam.view_matrix();
        // At default position (0,0,10) looking at origin, the 3rd row
        // translation should be [0, 0, 10] in the view matrix
        assert!((view.w_axis[2] - 10.0).abs() < 1e-4);
    }
}
```

- [ ] **Step 5: Create EditorState struct**

```rust
use crate::camera::EditorCamera;
use crate::hierarchy::SceneTree; // we'll define hierarchy module later, put SceneTree here for now
use crate::inspector::ComponentRegistry;

pub struct EditorState {
    pub selected_nodes: Vec<u64>,
    pub active_tool: ToolType,
    pub active_viewport_tab: usize,
    pub show_left_panel: bool,
    pub show_right_panel: bool,
    pub scene_tree: SceneTree,
    pub camera: EditorCamera,
    pub show_grid: bool,
    pub gizmo_interaction: Option<GizmoInteraction>,
    pub inspector_components: ComponentRegistry,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            selected_nodes: Vec::new(),
            active_tool: ToolType::Translate,
            active_viewport_tab: 0,
            show_left_panel: true,
            show_right_panel: true,
            scene_tree: SceneTree::new(),
            camera: EditorCamera::new(),
            show_grid: true,
            gizmo_interaction: None,
            inspector_components: ComponentRegistry::new(),
        }
    }

    pub fn frame(&mut self, ctx: &egui::Context, skin: &engine_ui::GuiSkin) {
        // TODO: copy layout from old editor.rs
    }
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p engine-editor`
Expected: all tests pass

- [ ] **Step 7: Commit**

```bash
git add crates/engine-editor/
git commit -m "feat(editor): add EditorState, SceneTree, EditorCamera core types"
```

---

### Task 2: Editor Layout Shell (menu bar, toolbar, status bar, bottom panel)

**Files:**
- Create: `crates/engine-editor/src/layout.rs`
- Modify: `crates/engine-editor/src/lib.rs` (add `mod layout`)

- [ ] **Step 1: Create layout.rs with the full editor frame**

Move the existing layout logic from `examples/basic/src/editor.rs` into the new crate, preserving:
- Screen-relative rect calculations (menu_h, toolbar_h, status_h, bottom_h, left_w, right_w)
- Menu bar drawing with hover/active states
- Toolbar drawing with tool buttons, view mode buttons, play controls, FPS
- Bottom panel with tabs (日志/性能/音频/网络) and content
- Status bar with ready indicator, object count, triangle count, view info

The key difference: instead of calling `self.draw_hierarchy()`, `self.draw_viewport()`, `self.draw_inspector()`, delegate to the new modules.

```rust
use egui::{Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use engine_ui::{Gui, GuiSkin};
use crate::state::EditorState;

pub fn frame(state: &mut EditorState, ctx: &egui::Context, skin: &GuiSkin) {
    let screen_rect = ctx.screen_rect();
    let h_scale = screen_rect.height() / 1080.0;
    let w_scale = screen_rect.width() / 1920.0;

    egui::Area::new(egui::Id::new("editor"))
        .interactable(true)
        .fixed_pos(Pos2::ZERO)
        .show(ctx, |ui| {
            let screen = ui.ctx().screen_rect();
            let menu_h = 32.0 * h_scale;
            let toolbar_h = 44.0 * h_scale;
            let status_h = 24.0 * h_scale;
            let bottom_h = (screen.height() * 180.0 / 1080.0).clamp(120.0, 400.0);

            let menu_rect = Rect::from_min_size(screen.left_top(), vec2(screen.width(), menu_h));
            let toolbar_rect = Rect::from_min_size(
                Pos2::new(screen.left(), menu_rect.bottom()),
                vec2(screen.width(), toolbar_h),
            );
            let status_rect = Rect::from_min_size(
                Pos2::new(screen.left(), screen.bottom() - status_h),
                vec2(screen.width(), status_h),
            );
            let bottom_rect = Rect::from_min_size(
                Pos2::new(screen.left(), status_rect.top() - bottom_h),
                vec2(screen.width(), bottom_h),
            );
            let main_rect = Rect::from_min_size(
                Pos2::new(screen.left(), toolbar_rect.bottom()),
                vec2(screen.width(), bottom_rect.top() - toolbar_rect.bottom()),
            );

            let left_w = (main_rect.width() * 260.0 / 1920.0).clamp(180.0, 400.0);
            let right_w = (main_rect.width() * 300.0 / 1920.0).clamp(200.0, 500.0);

            let hierarchy_rect = Rect::from_min_size(
                main_rect.left_top(),
                vec2(if state.show_left_panel { left_w } else { 0.0 }, main_rect.height()),
            );
            let inspector_rect = Rect::from_min_size(
                Pos2::new(main_rect.right() - (if state.show_right_panel { right_w } else { 0.0 }), main_rect.top()),
                vec2(if state.show_right_panel { right_w } else { 0.0 }, main_rect.height()),
            );
            let viewport_rect = Rect::from_min_size(
                Pos2::new(hierarchy_rect.right(), main_rect.top()),
                vec2(inspector_rect.left() - hierarchy_rect.right(), main_rect.height()),
            );

            let mut gui = Gui::new(ui, skin);
            draw_menu_bar(state, &mut gui, menu_rect, w_scale, h_scale);
            draw_toolbar(state, &mut gui, toolbar_rect, w_scale, h_scale);
            if state.show_left_panel {
                crate::hierarchy::draw(state, &mut gui, hierarchy_rect);
            }
            crate::viewport::draw(state, &mut gui, viewport_rect);
            if state.show_right_panel {
                crate::inspector::draw(state, &mut gui, inspector_rect);
            }
            draw_bottom_panel(state, ui, bottom_rect, skin);
            draw_status_bar(state, &mut gui, status_rect);
        });
}
```

Then copy `draw_menu_bar`, `draw_toolbar`, `draw_bottom_panel`, `draw_status_bar` from the old `editor.rs`:

- [ ] **Step 2: Copy draw_menu_bar**

Ported from `editor.rs:134-176`. Takes `state`, `gui`, `rect`, `w_scale`, `h_scale`. Use `state.active_menu: Option<usize>` instead of local field. Same rendering: dark rect, hover highlight, menu items text, right-aligned "MyGame" label.

```rust
fn draw_menu_bar(state: &mut EditorState, gui: &mut Gui, rect: Rect, w_scale: f32, h_scale: f32) {
    let painter = gui.ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));
    painter.add(Shape::line(
        vec![Pos2::new(rect.left(), rect.bottom() - 1.0), Pos2::new(rect.right(), rect.bottom() - 1.0)],
        Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
    ));

    let items = &["文件", "编辑", "视图", "场景", "资源", "构建", "窗口", "帮助"];
    let font_sz = 13.0 * h_scale;
    let char_w = 8.0 * w_scale;
    let item_pad = 12.0 * w_scale;
    let rounding = 4.0 * h_scale;
    let mut x = rect.left() + 8.0 * w_scale;
    for (i, item) in items.iter().enumerate() {
        let text_w = item.len() as f32 * char_w;
        let item_rect = Rect::from_min_size(Pos2::new(x, rect.top()), vec2(text_w + item_pad * 2.0, rect.height()));
        let id = egui::Id::new("mm").with(i as u64);
        let response = gui.ui.interact(item_rect, id, egui::Sense::click());
        if response.hovered() || state.active_menu == Some(i) {
            painter.add(Shape::rect_filled(item_rect, Rounding::same(rounding), Color32::from_rgb(30, 30, 34)));
        }
        painter.text(
            egui::pos2(x + item_pad, rect.center().y),
            egui::Align2::LEFT_CENTER,
            *item,
            egui::FontId::proportional(font_sz),
            if response.hovered() { Color32::from_rgb(232, 232, 236) } else { Color32::from_gray(152) },
        );
        if response.clicked() {
            state.active_menu = Some(i);
        }
        x += text_w + item_pad * 2.0 + 4.0 * w_scale;
    }
}
```

- [ ] **Step 3: Copy draw_toolbar and draw_separator**

Ported from `editor.rs:179-248`. Uses `state.active_tool`, `state.show_left_panel`, `state.show_right_panel`, `state.active_viewport_tab`.

- [ ] **Step 4: Copy draw_bottom_panel and draw_status_bar**

`draw_bottom_panel` from `editor.rs:626-706` — uses `state.active_bottom_tab` (add this field to EditorState).
`draw_status_bar` from `editor.rs:708-752`.

- [ ] **Step 5: Add active_bottom_tab and active_menu to EditorState**

```rust
pub active_menu: Option<usize>,
pub active_bottom_tab: usize,
pub fps: u32,
```

Initialize in `EditorState::new()`:
```rust
active_menu: None,
active_bottom_tab: 0,
fps: 60,
```

- [ ] **Step 6: Wire frame() in layout.rs**

Update `EditorState::frame()` in `state.rs` to call `crate::layout::frame(self, ctx, skin)`.

```rust
pub fn frame(&mut self, ctx: &egui::Context, skin: &engine_ui::GuiSkin) {
    crate::layout::frame(self, ctx, skin);
}
```

- [ ] **Step 7: Register layout module in lib.rs**

Add `pub mod layout;` to `lib.rs`.

- [ ] **Step 8: Run tests + cargo check**

Run: `cargo check -p engine-editor`
Expected: compiles (sparse — hierarchy/viewport/inspector modules exist but draw() are stubs)

- [ ] **Step 9: Commit**

```bash
git add crates/engine-editor/src/layout.rs crates/engine-editor/src/lib.rs crates/engine-editor/src/state.rs
git commit -m "feat(editor): port editor layout shell into engine-editor crate"
```

---

### Task 3: Hierarchy Panel

**Files:**
- Create: `crates/engine-editor/src/hierarchy.rs`

- [ ] **Step 1: Write draw() function**

Receives `&mut EditorState`, `&mut Gui`, `Rect`. Draws:

1. Panel header with "层级" title, "+" and "🔍" action buttons
2. Search text input (draw rect, track typing via egui input events, store in `editor_state`)
3. Scene tree rendering via recursive helper

```rust
use egui::{Color32, Pos2, Rect, Rounding, Shape, Vec2};
use engine_ui::{Gui, GuiSkin};
use crate::state::EditorState;

pub fn draw(state: &mut EditorState, gui: &mut Gui, rect: Rect) {
    let painter = gui.ui.painter_at(rect);
    let h_scale = gui.ui.ctx().screen_rect().height() / 1080.0;
    let w_scale = gui.ui.ctx().screen_rect().width() / 1920.0;

    // Background
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));
    painter.add(Shape::line(
        vec![Pos2::new(rect.right() - 1.0, rect.top()), Pos2::new(rect.right() - 1.0, rect.bottom())],
        Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
    ));

    // Header
    let header_h = 36.0 * h_scale;
    let header_rect = Rect::from_min_size(rect.left_top(), vec2(rect.width(), header_h));
    gui.panel_header(header_rect, "层级");
    // ... (reuse existing header drawing from old editor.rs lines 252-288)

    // Search bar
    let search_h = 28.0 * h_scale;
    let search_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 8.0 * w_scale, header_rect.bottom() + 4.0 * h_scale),
        vec2(rect.width() - 16.0 * w_scale, search_h),
    );
    // Draw search rect, handle text input

    // Tree content
    let content_top = search_rect.bottom() + 4.0 * h_scale;
    let content_rect = Rect::from_min_size(
        Pos2::new(rect.left(), content_top),
        vec2(rect.width(), rect.bottom() - content_top),
    );

    // Recursive tree drawing
    let mut y = content_rect.top() + 4.0 * h_scale;
    for &root_id in &state.scene_tree.root_ids {
        y = draw_node(state, gui, root_id, 0, &mut y, rect.left(), rect.right(), h_scale, w_scale);
    }
}
```

- [ ] **Step 2: Implement draw_node recursive helper**

Each node renders:
- Background highlight if selected (`state.selected_nodes.contains(&id)`)
- Expand/collapse arrow if has children (▸/▾)
- Icon + name text
- Click → toggle selection (Ctrl+click for multi-select later)
- Drag → compute drop position → call `state.scene_tree.reparent()`

```rust
fn draw_node(state: &mut EditorState, gui: &mut Gui, node_id: u64, depth: u32,
             y: &mut f32, left: f32, right: f32, h_scale: f32, w_scale: f32) -> f32 {
    let node = match state.scene_tree.nodes.iter().find(|n| n.id == node_id) {
        Some(n) => n,
        None => return *y,
    };

    let indent_step = 16.0 * h_scale;
    let item_h = 28.0 * h_scale;
    let indent = left + 8.0 * w_scale + depth as f32 * indent_step;
    let node_rect = Rect::from_min_size(Pos2::new(left, *y), vec2(right - left, item_h));
    let id_rect = Rect::from_min_size(Pos2::new(indent, *y), vec2(right - indent, item_h));

    let painter = gui.ui.painter_at(node_rect);
    let id = egui::Id::new("h_node").with(node_id);
    let response = gui.ui.interact(id_rect, id, egui::Sense::click_and_drag());

    let is_selected = state.selected_nodes.contains(&node_id);
    if is_selected {
        painter.add(Shape::rect_filled(id_rect, Rounding::same(4.0 * h_scale),
            Color32::from_rgba_premultiplied(0, 212, 170, 30)));
    } else if response.hovered() {
        painter.add(Shape::rect_filled(id_rect, Rounding::same(4.0 * h_scale),
            Color32::from_rgb(30, 30, 34)));
    }

    // Arrow
    let arrow_sz = 16.0 * h_scale;
    let arrow_rect = Rect::from_min_size(Pos2::new(indent, *y + (item_h - arrow_sz) / 2.0), vec2(arrow_sz, arrow_sz));
    if !node.children.is_empty() {
        painter.text(arrow_rect.center(), egui::Align2::CENTER_CENTER,
            if node.expanded { "▾" } else { "▸" },
            egui::FontId::proportional(10.0 * h_scale), Color32::from_gray(90));
        // Click on arrow to toggle expand
    }

    // Icon
    painter.text(egui::pos2(indent + 20.0 * h_scale, *y + item_h / 2.0),
        egui::Align2::LEFT_CENTER, &node.icon,
        egui::FontId::proportional(14.0 * h_scale), Color32::from_gray(200));

    // Name
    painter.text(egui::pos2(indent + 42.0 * h_scale, *y + item_h / 2.0),
        egui::Align2::LEFT_CENTER, &node.name,
        egui::FontId::proportional(13.0 * h_scale), Color32::from_rgb(232, 232, 236));

    // Click handling
    if response.clicked() {
        state.selected_nodes.clear();
        state.selected_nodes.push(node_id);
    }

    // Drag handling — basic reorder
    if response.dragged() && response.drag_delta().y.abs() > 4.0 {
        // Simple reorder: if dragged node has parent, swap position in parent's children list
    }

    *y += item_h;

    let mut last_y = *y;
    if node.expanded {
        for &child_id in &node.children {
            last_y = draw_node(state, gui, child_id, depth + 1, &mut last_y, left, right, h_scale, w_scale);
        }
    }
    last_y
}
```

- [ ] **Step 3: Add hierarchy search field to EditorState**

```rust
pub hierarchy_search: String,
```

- [ ] **Step 4: Write test for hierarchy draw doesn't panic**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::EditorState;
    use engine_ui::{Gui, GuiSkin};

    #[test]
    fn test_draw_hierarchy_no_panic() {
        let ctx = egui::Context::default();
        let skin = GuiSkin::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::Area::new(egui::Id::new("h_test")).show(ctx, |ui| {
                let mut state = EditorState::new();
                let mut gui = Gui::new(ui, &skin);
                let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(260.0, 600.0));
                draw(&mut state, &mut gui, rect);
            });
        });
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p engine-editor`
Expected: all pass

- [ ] **Step 6: Commit**

```bash
git add crates/engine-editor/src/hierarchy.rs
git commit -m "feat(editor): implement hierarchy panel with CRUD and search"
```

---

### Task 4: Viewport Camera Controls

**Files:**
- `crates/engine-editor/src/camera.rs` (already created in Task 1 — Camera math)
- Create: `crates/engine-editor/src/viewport.rs` (draw + mouse handling)

- [ ] **Step 1: Create viewport.rs — draw() function**

Draws the 3D viewport with:
1. Header (scene/game/physics tabs + gizmo/grid/camera tools — ported from old editor.rs lines 379-429)
2. Canvas area with gradient background + grid
3. Axis labels (X red, Y green, Z blue)
4. Scene object rects drawn at projected positions (simple placeholder)
5. Camera transform info overlay
6. Mouse event handling for camera control and gizmo

```rust
use egui::{Color32, CursorIcon, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use engine_ui::{Gui, GuiSkin};
use crate::state::{EditorState, ToolType};

pub fn draw(state: &mut EditorState, gui: &mut Gui, rect: Rect) {
    let painter = gui.ui.painter_at(rect);
    let h_scale = gui.ui.ctx().screen_rect().height() / 1080.0;
    let w_scale = gui.ui.ctx().screen_rect().width() / 1920.0;

    // ── Header tabs ──
    let header_h = 32.0 * h_scale;
    let header_rect = Rect::from_min_size(rect.left_top(), vec2(rect.width(), header_h));
    // ... draw viewport tabs (旧editor.rs lines 382-429)

    // ── Canvas ──
    let canvas_rect = Rect::from_min_size(
        Pos2::new(rect.left(), header_rect.bottom()),
        vec2(rect.width(), rect.bottom() - header_rect.bottom()),
    );

    // Background gradient
    draw_viewport_background(&painter, canvas_rect);

    // Grid
    if state.show_grid {
        draw_grid(&painter, canvas_rect, w_scale, h_scale);
    }

    // Axis labels
    draw_axis_labels(&painter, canvas_rect, h_scale);

    // Draw scene objects (projected positions, placeholder)
    draw_scene_objects(state, gui, canvas_rect, h_scale, w_scale);

    // Gizmo (if a node is selected)
    if !state.selected_nodes.is_empty() {
        crate::gizmo::draw(state, gui, canvas_rect, &painter, h_scale, w_scale);
    }

    // Transform overlay
    draw_transform_overlay(state, &painter, canvas_rect, h_scale, w_scale);

    // ── Camera mouse controls ──
    handle_camera_input(state, gui, canvas_rect);
}
```

- [ ] **Step 2: Implement camera mouse handling**

```rust
fn handle_camera_input(state: &mut EditorState, gui: &mut Gui, canvas_rect: Rect) {
    let ctx = gui.ui.ctx();
    let canvas_id = egui::Id::new("viewport_canvas");
    let canvas_response = gui.ui.interact(canvas_rect, canvas_id, egui::Sense::click_and_drag());

    // Right-click drag → orbit
    if canvas_response.dragged_by(egui::PointerButton::Secondary) {
        let delta = canvas_response.drag_delta();
        state.camera.orbit(delta.x, -delta.y);
    }

    // Middle-click drag → pan
    if canvas_response.dragged_by(egui::PointerButton::Middle) {
        let delta = canvas_response.drag_delta();
        state.camera.pan(delta.x, delta.y);
    }

    // Scroll → zoom
    let scroll_delta = ctx.input(|i| i.scroll_delta);
    if scroll_delta.y != 0.0 && canvas_rect.contains(ctx.pointer_interact_pos().unwrap_or(Pos2::ZERO)) {
        state.camera.zoom(scroll_delta.y / 120.0);
    }
}
```

- [ ] **Step 3: Implement draw_viewport_background, draw_grid, draw_axis_labels**

```rust
fn draw_viewport_background(painter: &egui::Painter, rect: Rect) {
    let gradient_steps = 20;
    let step_h = rect.height() / gradient_steps as f32;
    for i in 0..gradient_steps {
        let t = i as f32 / (gradient_steps - 1) as f32;
        let r = (10.0 + t * 10.0) as u8;
        let g = (10.0 + t * 10.0) as u8;
        let b = (12.0 + t * 16.0) as u8;
        let strip = Rect::from_min_size(
            Pos2::new(rect.left(), rect.top() + i as f32 * step_h),
            vec2(rect.width(), step_h + 1.0),
        );
        painter.add(Shape::rect_filled(strip, Rounding::ZERO, Color32::from_rgb(r, g, b)));
    }
}

fn draw_grid(painter: &egui::Painter, rect: Rect, w_scale: f32, h_scale: f32) {
    let grid_size = 50.0 * w_scale;
    let grid_color = Color32::from_rgba_premultiplied(37, 37, 48, 128);
    let mut x = rect.left();
    while x <= rect.right() {
        painter.add(Shape::line(
            vec![Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(1.0, grid_color),
        ));
        x += grid_size;
    }
    let mut y = rect.top();
    while y <= rect.bottom() {
        painter.add(Shape::line(
            vec![Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(1.0, grid_color),
        ));
        y += grid_size;
    }
}

fn draw_axis_labels(painter: &egui::Painter, rect: Rect, h_scale: f32) {
    let axes = [
        ("X", Color32::from_rgb(255, 107, 107)),
        ("Y", Color32::from_rgb(46, 213, 115)),
        ("Z", Color32::from_rgb(77, 171, 247)),
    ];
    for (i, (label, color)) in axes.iter().enumerate() {
        painter.text(
            egui::pos2(rect.left() + 20.0, rect.top() + 20.0 + i as f32 * 14.0 * h_scale),
            egui::Align2::LEFT_CENTER,
            *label,
            egui::FontId::proportional(10.0 * h_scale),
            *color,
        );
    }
}
```

- [ ] **Step 4: Implement draw_scene_objects (placeholder)**

Draw colored rects at transformed positions to represent scene objects in the viewport. Use camera view-projection to compute screen positions from world positions.

```rust
fn draw_scene_objects(state: &EditorState, gui: &mut Gui, canvas_rect: Rect, h_scale: f32, w_scale: f32) {
    let painter = gui.ui.painter_at(canvas_rect);
    let aspect = canvas_rect.width() / canvas_rect.height().max(1.0);
    let view_proj = state.camera.projection_matrix(aspect) * state.camera.view_matrix();

    // Simple perspective projection: map world x,z to screen x,y
    // Placeholder — real 3D rendering will come later
    for node in &state.scene_tree.nodes {
        if node.id == 1 { continue; } // skip root
        let world_pos = engine_math::Vec3::new(
            node.id as f32 * 2.0 - 5.0,
            0.0,
            node.id as f32 * 0.5 - 2.0,
        );
        // Simple projection to screen coords
        let clip = view_proj * world_pos.extend_with_w(1.0);
        if clip.w <= 0.0 { continue; }
        let ndc = clip.truncate() / clip.w;
        let screen_x = canvas_rect.center().x + ndc.x * canvas_rect.width() * 0.5;
        let screen_y = canvas_rect.center().y - ndc.y * canvas_rect.height() * 0.5;

        let size = 40.0 * h_scale;
        let obj_rect = Rect::from_center_size(Pos2::new(screen_x, screen_y), Vec2::new(size, size));

        let is_selected = state.selected_nodes.contains(&node.id);
        let border_color = if is_selected {
            Color32::from_rgb(255, 107, 53)
        } else {
            Color32::from_rgb(0, 212, 170)
        };

        painter.add(Shape::rect_filled(obj_rect, Rounding::same(4.0 * h_scale), Color32::from_rgb(42, 42, 53)));
        painter.rect_stroke(obj_rect, Rounding::same(4.0 * h_scale), Stroke::new(2.0, border_color));
        painter.text(obj_rect.center(), egui::Align2::CENTER_CENTER, &node.icon,
            egui::FontId::proportional(18.0 * h_scale), Color32::WHITE);
    }
}
```

- [ ] **Step 5: Implement draw_transform_overlay**

```rust
fn draw_transform_overlay(state: &EditorState, painter: &egui::Painter, canvas_rect: Rect, h_scale: f32, w_scale: f32) {
    let transform_bar_h = 28.0 * h_scale;
    let transform_w = 200.0 * w_scale;
    let transform_rect = Rect::from_min_size(
        Pos2::new(canvas_rect.left() + 20.0 * w_scale, canvas_rect.bottom() - 44.0 * h_scale),
        vec2(transform_w, transform_bar_h),
    );
    painter.add(Shape::rect_filled(transform_rect, Rounding::same(6.0 * h_scale),
        Color32::from_rgba_premultiplied(22, 22, 25, 230)));

    // Default position overlay for now
    let pos = (0.0, 0.0, 0.0);
    let transform_axes = [
        ("X", pos.0, Color32::from_rgb(255, 107, 107)),
        ("Y", pos.1, Color32::from_rgb(46, 213, 115)),
        ("Z", pos.2, Color32::from_rgb(77, 171, 247)),
    ];
    for (i, (label, val, color)) in transform_axes.iter().enumerate() {
        painter.text(
            egui::pos2(transform_rect.left() + 12.0 * w_scale + i as f32 * 60.0 * w_scale, transform_rect.center().y),
            egui::Align2::LEFT_CENTER,
            format!("{} {}", label, val),
            egui::FontId::proportional(11.0 * h_scale),
            *color,
        );
    }
}
```

- [ ] **Step 6: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::EditorState;
    use engine_ui::{Gui, GuiSkin};

    #[test]
    fn test_viewport_draw_no_panic() {
        let ctx = egui::Context::default();
        let skin = GuiSkin::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::Area::new(egui::Id::new("vp_test")).show(ctx, |ui| {
                let mut state = EditorState::new();
                let mut gui = Gui::new(ui, &skin);
                let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(800.0, 600.0));
                draw(&mut state, &mut gui, rect);
            });
        });
    }
}
```

- [ ] **Step 7: Run tests**

Run: `cargo test -p engine-editor`
Expected: all pass

- [ ] **Step 8: Commit**

```bash
git add crates/engine-editor/src/viewport.rs
git commit -m "feat(editor): add 3D viewport with orbit camera controls"
```

---

### Task 5: Transform Gizmo

**Files:**
- Create: `crates/engine-editor/src/gizmo.rs`

- [ ] **Step 1: Define GizmoConfig**

```rust
pub struct GizmoConfig {
    pub axis_length: f32,
    pub axis_thickness: f32,
    pub ring_radius: f32,
}
```

- [ ] **Step 2: Write draw() function**

Draws gizmo at the selected node's position (or a fixed origin for now). Uses different visuals based on `state.active_tool`.

```rust
use egui::{Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use engine_ui::Gui;
use crate::state::{EditorState, GizmoInteraction, ToolType};

/// Colors for X/Y/Z axes
const AXIS_COLORS: [Color32; 3] = [
    Color32::from_rgb(255, 107, 107),  // X - red
    Color32::from_rgb(46, 213, 115),   // Y - green
    Color32::from_rgb(77, 171, 247),   // Z - blue
];

pub fn draw(state: &EditorState, gui: &mut Gui, canvas_rect: Rect, painter: &egui::Painter, h_scale: f32, w_scale: f32) {
    let gizmo_center = canvas_rect.center() + Vec2::new(-20.0 * w_scale, 20.0 * h_scale);
    let gizmo_size = 60.0 * h_scale;

    match state.active_tool {
        ToolType::Translate => draw_translate_gizmo(painter, gizmo_center, gizmo_size, state),
        ToolType::Rotate => draw_rotate_gizmo(painter, gizmo_center, gizmo_size, state),
        ToolType::Scale => draw_scale_gizmo(painter, gizmo_center, gizmo_size, state),
        ToolType::Select => {} // no gizmo in select mode
    }
}

fn draw_translate_gizmo(painter: &egui::Painter, center: Pos2, size: f32, state: &EditorState) {
    let dirs = [
        Vec2::new(1.0, 0.0),   // X → right
        Vec2::new(0.0, -1.0),  // Y → up
        Vec2::new(-0.7, 0.7),  // Z → diagonal
    ];
    for (i, &dir) in dirs.iter().enumerate() {
        let tip = Pos2::new(center.x + dir.x * size, center.y + dir.y * size);
        let color = AXIS_COLORS[i];
        let active = state.gizmo_interaction.as_ref().map_or(false, |g| g.axis == (1 << i as u8));
        let draw_color = if active {
            color
        } else {
            Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 128)
        };
        // Shaft
        painter.add(Shape::line(vec![center, tip], Stroke::new(3.0, draw_color)));
        // Arrow head (triangle)
        let arrow_base = Pos2::new(center.x + dir.x * (size - 8.0), center.y + dir.y * (size - 8.0));
        let perp = Vec2::new(-dir.y, dir.x);
        painter.add(Shape::convex_polygon(
            vec![tip, arrow_base + perp * 4.0, arrow_base - perp * 4.0],
            draw_color,
        ));
    }
}

fn draw_rotate_gizmo(painter: &egui::Painter, center: Pos2, size: f32, state: &EditorState) {
    for (i, &start_angle) in [0.0, 90.0_f32.to_radians(), 180.0_f32.to_radians()].iter().enumerate() {
        let color = AXIS_COLORS[i];
        let mut points = Vec::with_capacity(32);
        for a in 0..=30 {
            let angle = start_angle + a as f32 * 120.0_f32.to_radians() / 30.0;
            let p = Pos2::new(center.x + angle.cos() * size, center.y + angle.sin() * size);
            points.push(p);
        }
        painter.add(Shape::line(points, Stroke::new(2.0, color)));
    }
}

fn draw_scale_gizmo(painter: &egui::Painter, center: Pos2, size: f32, state: &EditorState) {
    let dirs = [
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, -1.0),
        Vec2::new(-0.7, 0.7),
    ];
    for (i, &dir) in dirs.iter().enumerate() {
        let tip = Pos2::new(center.x + dir.x * size, center.y + dir.y * size);
        let color = AXIS_COLORS[i];
        // Shaft (lighter)
        painter.add(Shape::line(vec![center, tip], Stroke::new(2.0,
            Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 100))));
        // End cube
        let cube_half = 5.0;
        let cube_rect = Rect::from_center_size(tip, Vec2::new(cube_half * 2.0, cube_half * 2.0));
        painter.add(Shape::rect_filled(cube_rect, Rounding::ZERO, color));
    }
    // Center cube for uniform scale
    let center_cube = Rect::from_center_size(center, Vec2::new(10.0, 10.0));
    painter.add(Shape::rect_filled(center_cube, Rounding::same(2.0), Color32::WHITE));
}
```

- [ ] **Step 3: Add gizmo size to EditorState**

```rust
pub gizmo_size: f32,
```

Initialize to `60.0`.

- [ ] **Step 4: Write test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::EditorState;

    #[test]
    fn test_gizmo_draw_no_panic_translate() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = egui::Painter::new(ctx.clone(), egui::LayerId::new(egui::Order::Foreground, egui::Id::new("gizmo")), ctx.screen_rect());
            let state = EditorState::new();
            let rect = ctx.screen_rect();
            draw_translate_gizmo(&painter, rect.center(), 60.0, &state);
        });
    }

    // (add rotate/scale variants)
}
```

- [ ] **Step 5: Register in lib.rs**

Add `pub mod gizmo;` to `lib.rs`.

- [ ] **Step 6: Run tests**

Run: `cargo test -p engine-editor`
Expected: all pass

- [ ] **Step 7: Commit**

```bash
git add crates/engine-editor/src/gizmo.rs
git commit -m "feat(editor): add transform gizmo (translate, rotate, scale)"
```

---

### Task 6: Inspector + Component System

**Files:**
- Create: `crates/engine-editor/src/inspector.rs`

- [ ] **Step 1: Define ComponentEditor trait and ComponentRegistry**

```rust
use egui::{Color32, Pos2, Rect, Rounding, Shape, Vec2};
use engine_ui::Gui;
use crate::state::EditorState;

pub trait ComponentEditor: Send + Sync {
    fn name(&self) -> &'static str;
    fn draw(&mut self, gui: &mut Gui, rect: &mut Rect, state: &mut EditorState);
    fn clone_box(&self) -> Box<dyn ComponentEditor>;
}

pub struct ComponentRegistry {
    pub editors: Vec<Box<dyn ComponentEditor>>,       // per-entity editors
    pub available: Vec<(&'static str, Box<dyn ComponentEditor>)>,  // factory defaults
}

impl ComponentRegistry {
    pub fn new() -> Self {
        let mut reg = Self { editors: Vec::new(), available: Vec::new() };
        reg.register::<TransformEditor>();
        reg.register::<RenderEditor>();
        reg.register::<PhysicsEditor>();
        reg
    }

    pub fn register<T: ComponentEditor + Default + 'static>(&mut self) {
        let default = T::default();
        self.available.push((default.name(), Box::new(default)));
    }

    pub fn add_to_entity(&mut self, component_name: &str) {
        if let Some((_, default)) = self.available.iter().find(|(n, _)| *n == component_name) {
            self.editors.push(default.clone_box());
        }
    }

    pub fn remove_from_entity(&mut self, index: usize) {
        if index < self.editors.len() {
            self.editors.remove(index);
        }
    }

    pub fn draw_for_entity(&mut self, gui: &mut Gui, mut rect: Rect, state: &mut EditorState) {
        let mut remove_idx: Option<usize> = None;
        for (i, editor) in self.editors.iter_mut().enumerate() {
            rect = Rect::from_min_size(
                Pos2::new(rect.left(), rect.top()),
                Vec2::new(rect.width(), 20.0),
            );
            // Section header
            editor.draw(gui, &mut rect, state);
            rect.set_top(rect.bottom() + 8.0);
        }
    }
}
```

- [ ] **Step 2: Implement TransformEditor**

```rust
#[derive(Default, Clone)]
pub struct TransformEditor {
    pub translation: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
}

impl ComponentEditor for TransformEditor {
    fn name(&self) -> &'static str { "变换" }

    fn draw(&mut self, gui: &mut Gui, rect: &mut Rect, _state: &mut EditorState) {
        let painter = gui.ui.painter_at(*rect);
        let label_font = egui::FontId::proportional(11.0);
        let row_h = 26.0;
        let label_w = 80.0;

        // Section header
        painter.text(egui::pos2(rect.left(), rect.top()), egui::Align2::LEFT_CENTER, "变换",
            label_font, Color32::from_gray(90));
        let sep_y = rect.top() + 18.0;
        painter.add(Shape::line(
            vec![Pos2::new(rect.left() + 40.0, sep_y), Pos2::new(rect.right(), sep_y)],
            Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
        ));

        let mut y = sep_y + 12.0;

        // Position
        let pos_rect = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), row_h));
        let mut px = self.translation[0];
        let mut py = self.translation[1];
        let mut pz = self.translation[2];
        gui.vec3_input(pos_rect, "位置", &mut px, &mut py, &mut pz);
        self.translation = [px, py, pz];
        y += row_h + 6.0;

        // Rotation
        let rot_rect = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), row_h));
        let mut rx = self.rotation[0];
        let mut ry = self.rotation[1];
        let mut rz = self.rotation[2];
        gui.vec3_input(rot_rect, "旋转", &mut rx, &mut ry, &mut rz);
        self.rotation = [rx, ry, rz];
        y += row_h + 6.0;

        // Scale
        let scale_rect = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), row_h));
        let mut sx = self.scale[0];
        let mut sy = self.scale[1];
        let mut sz = self.scale[2];
        gui.vec3_input(scale_rect, "缩放", &mut sx, &mut sy, &mut sz);
        self.scale = [sx, sy, sz];

        rect.set_top(y + 16.0);
    }

    fn clone_box(&self) -> Box<dyn ComponentEditor> {
        Box::new(self.clone())
    }
}
```

- [ ] **Step 3: Implement RenderEditor and PhysicsEditor (simple display)**

```rust
#[derive(Default, Clone)]
pub struct RenderEditor {
    pub material: String,
    pub mesh: String,
    pub cast_shadow: bool,
}

impl ComponentEditor for RenderEditor {
    fn name(&self) -> &'static str { "渲染" }
    fn draw(&mut self, gui: &mut Gui, rect: &mut Rect, _state: &mut EditorState) {
        let painter = gui.ui.painter_at(*rect);
        let row_h = 26.0;
        // Header
        painter.text(egui::pos2(rect.left(), rect.top()), egui::Align2::LEFT_CENTER, "渲染",
            egui::FontId::proportional(11.0), Color32::from_gray(90));
        let sep_y = rect.top() + 18.0;
        painter.add(Shape::line(
            vec![Pos2::new(rect.left() + 30.0, sep_y), Pos2::new(rect.right(), sep_y)],
            Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
        ));
        let mut y = sep_y + 12.0;

        let mat_rect = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), row_h));
        gui.input_labeled(mat_rect, "材质", &self.material);
        y += row_h + 6.0;

        let mesh_rect = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), row_h));
        gui.input_labeled(mesh_rect, "网格", &self.mesh);
        y += row_h + 6.0;

        let shadow_rect = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), row_h));
        gui.checkbox(shadow_rect, "投射阴影", &mut self.cast_shadow);
        y += 16.0;

        rect.set_top(y);
    }
    fn clone_box(&self) -> Box<dyn ComponentEditor> { Box::new(self.clone()) }
}

#[derive(Default, Clone)]
pub struct PhysicsEditor {
    pub body_type: String,
    pub collision_shape: String,
}

impl ComponentEditor for PhysicsEditor {
    fn name(&self) -> &'static str { "物理" }
    fn draw(&mut self, gui: &mut Gui, rect: &mut Rect, _state: &mut EditorState) {
        let painter = gui.ui.painter_at(*rect);
        let row_h = 26.0;
        painter.text(egui::pos2(rect.left(), rect.top()), egui::Align2::LEFT_CENTER, "物理",
            egui::FontId::proportional(11.0), Color32::from_gray(90));
        let sep_y = rect.top() + 18.0;
        painter.add(Shape::line(
            vec![Pos2::new(rect.left() + 30.0, sep_y), Pos2::new(rect.right(), sep_y)],
            Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
        ));
        let mut y = sep_y + 12.0;

        let body_rect = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), row_h));
        gui.input_labeled(body_rect, "刚体", &self.body_type);
        y += row_h + 6.0;

        let col_rect = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), row_h));
        gui.input_labeled(col_rect, "碰撞", &self.collision_shape);

        rect.set_top(y + 16.0);
    }
    fn clone_box(&self) -> Box<dyn ComponentEditor> { Box::new(self.clone()) }
}
```

- [ ] **Step 4: Write draw() for inspector panel**

The `draw()` function is called from the layout. It renders the search bar, then draws components for the selected entity.

```rust
pub fn draw(state: &mut EditorState, gui: &mut Gui, rect: Rect) {
    let painter = gui.ui.painter_at(rect);
    let h_scale = gui.ui.ctx().screen_rect().height() / 1080.0;
    let w_scale = gui.ui.ctx().screen_rect().width() / 1920.0;

    // Background
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));
    painter.add(Shape::line(
        vec![Pos2::new(rect.left(), rect.top()), Pos2::new(rect.left(), rect.bottom())],
        Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
    ));

    // Search bar
    let search_h = 36.0 * h_scale;
    let search_round = 6.0 * h_scale;
    let pad8 = 8.0 * w_scale;
    let search_font = egui::FontId::proportional(12.0 * h_scale);
    painter.add(Shape::rect_filled(
        Rect::from_min_size(Pos2::new(rect.left() + pad8, rect.top() + 8.0 * h_scale),
            vec2(rect.width() - pad8 * 2.0, search_h)),
        Rounding::same(search_round),
        Color32::from_rgb(30, 30, 34),
    ));
    painter.text(
        egui::pos2(rect.left() + 20.0 * w_scale, rect.top() + (8.0 * h_scale + search_h / 2.0)),
        egui::Align2::LEFT_CENTER,
        "🔍 搜索属性...",
        search_font,
        Color32::from_gray(90),
    );

    // Component editors
    let content_top = rect.top() + (8.0 * h_scale + search_h + 8.0 * h_scale);
    let content_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 12.0 * w_scale, content_top),
        vec2(rect.width() - 24.0 * w_scale, rect.bottom() - content_top),
    );

    state.inspector_components.draw_for_entity(gui, content_rect, state);
}
```

- [ ] **Step 5: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::EditorState;
    use engine_ui::{Gui, GuiSkin};

    #[test]
    fn test_component_registry_new_has_defaults() {
        let reg = ComponentRegistry::new();
        assert!(!reg.available.is_empty());
        assert!(reg.available.iter().any(|(n, _)| *n == "变换"));
        assert!(reg.available.iter().any(|(n, _)| *n == "渲染"));
        assert!(reg.available.iter().any(|(n, _)| *n == "物理"));
    }

    #[test]
    fn test_component_registry_add_remove() {
        let mut reg = ComponentRegistry::new();
        reg.add_to_entity("变换");
        reg.add_to_entity("渲染");
        assert_eq!(reg.editors.len(), 2);
        reg.remove_from_entity(0);
        assert_eq!(reg.editors.len(), 1);
        assert_eq!(reg.editors[0].name(), "渲染");
    }
}
```

- [ ] **Step 6: Register in lib.rs**

Add `pub mod inspector;` to `lib.rs`.

- [ ] **Step 7: Run tests**

Run: `cargo test -p engine-editor`
Expected: all pass

- [ ] **Step 8: Commit**

```bash
git add crates/engine-editor/src/inspector.rs
git commit -m "feat(editor): add inspector with extensible component editors"
```

---

### Task 7: Wire into examples/basic

**Files:**
- Modify: `examples/basic/src/main.rs`
- Remove: `examples/basic/src/editor.rs`
- Modify: `examples/basic/Cargo.toml`

- [ ] **Step 1: Update examples/basic/Cargo.toml**

Add `engine-editor = { path = "../../crates/engine-editor" }` to dependencies.

- [ ] **Step 2: Update main.rs**

Remove the inline `GamePlugin` editor hooks. Add `EditorPlugin`.

```rust
use engine_core::app::{App, AppBuilder};
use engine_core::debug::DebugPlugin;
use engine_core::engine::run_default;
use engine_core::plugin::Plugin;
use engine_framework::{FrameworkPlugin, GameState, StateCtx, StateStack};
use engine_ui::{EguiPlugin, EguiState, GuiSkin, ImGuiPlugin};
use engine_editor::EditorPlugin;

struct MenuState;
impl GameState for MenuState {
    fn on_enter(&mut self, _: &mut StateCtx) { println!("Menu entered"); }
    fn on_exit(&mut self, _: &mut StateCtx) { println!("Menu exited"); }
    fn update(&mut self, _: &mut StateCtx, _dt: f32) {}
}

struct GamePlugin;
impl Plugin for GamePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_pre_update_hook(Box::new(|app: &mut App| {
            static PUSHED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            if !PUSHED.swap(true, std::sync::atomic::Ordering::Relaxed)
                && let Some(stack) = app.resources.get_mut::<StateStack>()
            {
                stack.push(Box::new(MenuState));
            }
        }));
    }
}

fn main() {
    let mut builder = AppBuilder::new();
    builder
        .add_plugin(FrameworkPlugin)
        .add_plugin(DebugPlugin)
        .add_plugin(EguiPlugin)
        .add_plugin(ImGuiPlugin)
        .add_plugin(EditorPlugin)
        .add_plugin(GamePlugin);
    run_default(builder);
}
```

- [ ] **Step 3: Remove old editor.rs**

Delete `examples/basic/src/editor.rs`.

- [ ] **Step 4: Update lib.rs to remove `mod editor`**

No `mod editor;` in main.rs — already removed in Step 2.

- [ ] **Step 5: Build and run**

Run: `cargo build -p basic`
Expected: success

- [ ] **Step 6: Commit**

```bash
git add examples/basic/src/main.rs examples/basic/Cargo.toml
git rm examples/basic/src/editor.rs
git commit -m "feat(editor): wire engine-editor crate into basic example"
```

---

### Task 8: Integration Verification

- [ ] **Step 1: Run full cargo test**

Run: `cargo test --workspace`
Expected: all tests pass (engine-editor, engine-ui, engine-scene, etc.)

- [ ] **Step 2: Run cargo clippy**

Run: `cargo clippy --workspace`
Expected: no warnings
Fix any clippy issues found.

- [ ] **Step 3: Run cargo fmt --check**

Run: `cargo fmt --check --workspace`
Expected: no formatting issues

- [ ] **Step 4: Final commit if fixes needed**

```bash
git add -A
git commit -m "chore: fix clippy and formatting after editor refactor"
```
