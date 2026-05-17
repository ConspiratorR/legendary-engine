# Editor Layout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the full IMGUI editor layout matching `design/editor.html` — menu bar, toolbar, hierarchy panel, viewport, inspector, bottom panel, status bar.

**Architecture:** 11 new control methods on `Gui` in `gui.rs`; `EditorLayout` struct + draw methods in new `examples/basic/src/editor.rs`; wire into `main.rs`.

**Tech Stack:** Rust, egui 0.30, wgpu 23, our engine-ui crate

---

### Task 1: Add 5 display/static controls to Gui

**Files:**
- Modify: `crates/engine-ui/src/gui.rs`
- Test: `crates/engine-ui/src/gui.rs` (existing test module)

**New methods on `Gui`:**

- [ ] **Step 1: Add `separator_h`**

```rust
pub fn separator_h(&mut self, rect: Rect) {
    let painter = self.ui.painter_at(rect);
    let y = rect.center().y;
    painter.add(Shape::line(
        vec![Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
        Stroke::new(1.0, Color32::from_gray(60)),
    ));
}
```

- [ ] **Step 2: Add `colored_label`**

```rust
pub fn colored_label(&mut self, rect: Rect, text: &str, color: Color32) {
    let painter = self.ui.painter_at(rect);
    painter.text(
        egui::pos2(rect.left(), rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        self.skin.font.clone(),
        color,
    );
}
```

- [ ] **Step 3: Add `status_item`**

```rust
pub fn status_item(&mut self, rect: Rect, text: &str, dot_color: Color32) {
    let painter = self.ui.painter_at(rect);
    let dot_r = 4.0;
    let dot_center = egui::pos2(rect.left() + dot_r + 2.0, rect.center().y);
    painter.add(Shape::circle_filled(dot_center, dot_r, dot_color));
    painter.text(
        egui::pos2(dot_center.x + dot_r + 6.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        self.skin.font.clone(),
        self.skin.label.normal.text,
    );
}
```

- [ ] **Step 4: Add `panel_header`**

```rust
pub fn panel_header(&mut self, rect: Rect, title: &str) -> Rect {
    let painter = self.ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));
    painter.text(
        egui::pos2(rect.left() + 12.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(12.0),
        Color32::from_gray(90),
    );
    let line_y = rect.bottom() - 1.0;
    painter.add(Shape::line(
        vec![Pos2::new(rect.left(), line_y), Pos2::new(rect.right(), line_y)],
        Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
    ));
    Rect::from_min_size(
        Pos2::new(rect.left(), rect.bottom()),
        egui::vec2(rect.width(), 0.0),
    )
}
```

Note: `panel_header` returns a "marker" rect starting at the bottom. The caller uses it to calculate the content area below.

- [ ] **Step 5: Add `checkbox`** (visual square checkbox, different from the circular `toggle`)

```rust
pub fn checkbox(&mut self, rect: Rect, label: &str, checked: &mut bool) {
    let id = egui::Id::new("gui_chk").with(rect.min.x as u64).with(rect.min.y as u64);
    let response = self.ui.interact(rect, id, egui::Sense::click());

    let box_size = rect.height() - 4.0;
    let box_rect = Rect::from_min_size(
        egui::pos2(rect.left() + 2.0, rect.top() + 2.0),
        egui::vec2(box_size, box_size),
    );

    let painter = self.ui.painter_at(rect);
    let bg = if *checked { Color32::from_rgb(0, 212, 170) } else { Color32::from_gray(40) };
    painter.add(Shape::rect_filled(box_rect, Rounding::same(3.0), bg));
    painter.add(Shape::rect_stroke(box_rect, Rounding::same(3.0), Stroke::new(1.0, Color32::from_gray(100))));

    if *checked {
        painter.text(box_rect.center(), egui::Align2::CENTER_CENTER, "✓", self.skin.font.clone(), Color32::from_rgb(13, 13, 15));
    }

    painter.text(
        egui::pos2(box_rect.right() + 6.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        self.skin.font.clone(),
        self.skin.label.normal.text,
    );

    if response.clicked() {
        *checked = !*checked;
    }
}
```

- [ ] **Step 6: Add tests for new display controls**

```rust
#[test]
fn test_separator_h_draws_without_panic() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(100.0, 4.0));
        gui.separator_h(rect);
    });
}

#[test]
fn test_colored_label_draws_without_panic() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 20.0), egui::vec2(100.0, 20.0));
        gui.colored_label(rect, "Hello", Color32::RED);
    });
}

#[test]
fn test_checkbox_default_not_checked() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 30.0), egui::vec2(150.0, 22.0));
        let mut checked = false;
        gui.checkbox(rect, "Shadow", &mut checked);
        assert!(!checked, "checkbox should remain false without click");
    });
}

#[test]
fn test_checkbox_checked_state() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 60.0), egui::vec2(150.0, 22.0));
        let mut checked = true;
        gui.checkbox(rect, "Shadow", &mut checked);
        assert!(checked, "checkbox should remain true when initialized true");
    });
}

#[test]
fn test_panel_header_returns_content_rect() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 90.0), egui::vec2(200.0, 36.0));
        let content_rect = gui.panel_header(rect, "层级");
        assert!(content_rect.left() >= rect.left());
        assert!(content_rect.top() >= rect.bottom());
    });
}
```

- [ ] **Step 7: Build and run tests**

Run: `cargo test -p engine-ui`
Expected: existing 23 tests + 5 new = 28 tests pass

- [ ] **Step 8: Commit**

```bash
git add crates/engine-ui/src/gui.rs
git commit -m "feat(gui): add separator_h, colored_label, status_item, panel_header, checkbox"
```

---

### Task 2: Add 3 interactive controls to Gui

**Files:**
- Modify: `crates/engine-ui/src/gui.rs`

- [ ] **Step 1: Add `menu_bar`**

```rust
pub fn menu_bar(&mut self, rect: Rect, items: &[&str]) -> Option<usize> {
    let painter = self.ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));

    let n = items.len() as f32;
    let item_w = rect.width() / n;

    for (i, item) in items.iter().enumerate() {
        let item_rect = Rect::from_min_size(
            egui::pos2(rect.left() + i as f32 * item_w, rect.top()),
            egui::vec2(item_w, rect.height()),
        );

        let id = egui::Id::new("gui_menu").with(i as u64);
        let response = self.ui.interact(item_rect, id, egui::Sense::click());

        if response.hovered() {
            painter.add(Shape::rect_filled(item_rect, Rounding::ZERO, Color32::from_rgb(30, 30, 34)));
        }

        let text_color = if response.hovered() {
            Color32::from_rgb(232, 232, 236)
        } else {
            Color32::from_gray(152)
        };
        painter.text(
            item_rect.center(),
            egui::Align2::CENTER_CENTER,
            *item,
            egui::FontId::proportional(13.0),
            text_color,
        );

        if response.clicked() {
            return Some(i);
        }
    }
    None
}
```

- [ ] **Step 2: Add `tool_button`**

```rust
pub fn tool_button(&mut self, rect: Rect, label: &str, active: bool) -> bool {
    let id = egui::Id::new("gui_tbtn").with(rect.min.x as u64).with(rect.min.y as u64);
    let response = self.ui.interact(rect, id, egui::Sense::click());

    let painter = self.ui.painter_at(rect);

    if active {
        painter.add(Shape::rect_filled(rect, Rounding::same(6.0), Color32::from_rgb(0, 212, 170)));
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, self.skin.font.clone(), Color32::from_rgb(13, 13, 15));
    } else if response.hovered() {
        painter.add(Shape::rect_filled(rect, Rounding::same(6.0), Color32::from_rgb(30, 30, 34)));
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, self.skin.font.clone(), Color32::from_gray(152));
    } else {
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, self.skin.font.clone(), Color32::from_gray(152));
    }

    response.clicked()
}
```

- [ ] **Step 3: Add `tab`**

```rust
pub fn tab(&mut self, rect: Rect, label: &str, active: bool) -> bool {
    let id = egui::Id::new("gui_tab").with(rect.min.x as u64).with(rect.min.y as u64);
    let response = self.ui.interact(rect, id, egui::Sense::click());

    let painter = self.ui.painter_at(rect);

    if active {
        painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));
        let line_rect = Rect::from_min_size(
            egui::pos2(rect.left(), rect.bottom() - 2.0),
            egui::vec2(rect.width(), 2.0),
        );
        painter.add(Shape::rect_filled(line_rect, Rounding::ZERO, Color32::from_rgb(0, 212, 170)));
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(12.0), Color32::from_rgb(0, 212, 170));
    } else {
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(12.0), Color32::from_gray(90));
    }

    response.clicked()
}
```

- [ ] **Step 4: Add tests**

```rust
#[test]
fn test_menu_bar_returns_none_without_click() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 120.0), egui::vec2(400.0, 32.0));
        let result = gui.menu_bar(rect, &["文件", "编辑", "视图"]);
        assert!(result.is_none(), "menu_bar should return None without click");
    });
}

#[test]
fn test_tool_button_returns_false_without_click() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 160.0), egui::vec2(32.0, 32.0));
        let clicked = gui.tool_button(rect, "↖", false);
        assert!(!clicked);
    });
}

#[test]
fn test_tool_button_active_state() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(50.0, 160.0), egui::vec2(32.0, 32.0));
        let clicked = gui.tool_button(rect, "↔", true);
        assert!(!clicked, "active tool_button should not auto-click");
    });
}

#[test]
fn test_tab_returns_false_without_click() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 200.0), egui::vec2(60.0, 32.0));
        let clicked = gui.tab(rect, "场景", false);
        assert!(!clicked);
    });
}

#[test]
fn test_tab_active_does_not_auto_click() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(80.0, 200.0), egui::vec2(60.0, 32.0));
        let clicked = gui.tab(rect, "游戏", true);
        assert!(!clicked);
    });
}
```

- [ ] **Step 5: Build and run tests**

Run: `cargo test -p engine-ui`
Expected: 34 tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/engine-ui/src/gui.rs
git commit -m "feat(gui): add menu_bar, tool_button, tab controls"
```

---

### Task 3: Add 3 input/display controls to Gui

**Files:**
- Modify: `crates/engine-ui/src/gui.rs`

- [ ] **Step 1: Add `tree_node`**

```rust
pub fn tree_node(&mut self, rect: Rect, label: &str, icon: &str, selected: bool, depth: u32) -> bool {
    let id = egui::Id::new("gui_tree").with(rect.min.x as u64).with(rect.min.y as u64);
    let response = self.ui.interact(rect, id, egui::Sense::click());

    let painter = self.ui.painter_at(rect);

    if selected {
        painter.add(Shape::rect_filled(rect, Rounding::same(4.0), Color32::from_rgba_premultiplied(0, 212, 170, 40)));
    } else if response.hovered() {
        painter.add(Shape::rect_filled(rect, Rounding::same(4.0), Color32::from_rgb(30, 30, 34)));
    }

    let indent = 8.0 + depth as f32 * 16.0;
    painter.text(
        egui::pos2(rect.left() + indent, rect.center().y),
        egui::Align2::LEFT_CENTER,
        icon,
        egui::FontId::proportional(14.0),
        Color32::from_gray(200),
    );
    painter.text(
        egui::pos2(rect.left() + indent + 20.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        self.skin.font.clone(),
        if selected { Color32::from_rgb(0, 212, 170) } else { Color32::from_rgb(232, 232, 236) },
    );

    response.clicked()
}
```

- [ ] **Step 2: Add `vec3_input`**

```rust
pub fn vec3_input(&mut self, rect: Rect, label: &str, x: &mut f32, y: &mut f32, z: &mut f32) {
    let painter = self.ui.painter_at(rect);

    // Label
    painter.text(
        egui::pos2(rect.left(), rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(12.0),
        Color32::from_gray(152),
    );

    let input_w = (rect.width() - 80.0) / 3.0;
    let inputs = [
        ("X", x, Color32::from_rgb(255, 107, 107)),
        ("Y", y, Color32::from_rgb(46, 213, 115)),
        ("Z", z, Color32::from_rgb(77, 171, 247)),
    ];

    for (j, (axis_label, val, axis_color)) in inputs.iter().enumerate() {
        let field_x = rect.left() + 80.0 + j as f32 * input_w;
        let field_rect = Rect::from_min_size(
            egui::pos2(field_x, rect.top()),
            egui::vec2(input_w - 2.0, rect.height()),
        );

        // Colored axis label
        painter.text(
            egui::pos2(field_rect.left() + 4.0, field_rect.center().y),
            egui::Align2::LEFT_CENTER,
            *axis_label,
            egui::FontId::proportional(10.0),
            *axis_color,
        );

        // Value background
        let val_rect = Rect::from_min_size(
            egui::pos2(field_rect.left() + 14.0, field_rect.top()),
            egui::vec2(field_rect.width() - 14.0, field_rect.height()),
        );
        painter.add(Shape::rect_filled(val_rect, Rounding::same(4.0), Color32::from_rgb(30, 30, 34)));

        // Value text
        painter.text(
            val_rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{:.1}", **val),
            egui::FontId::proportional(11.0),
            Color32::from_rgb(232, 232, 236),
        );
    }
}
```

- [ ] **Step 3: Add `input_labeled`** (read-only display, matching editor.html `<input readonly>`)

```rust
pub fn input_labeled(&mut self, rect: Rect, label: &str, value: &str) {
    let painter = self.ui.painter_at(rect);

    painter.text(
        egui::pos2(rect.left(), rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(12.0),
        Color32::from_gray(152),
    );

    let input_rect = Rect::from_min_size(
        egui::pos2(rect.left() + 80.0, rect.top()),
        egui::vec2(rect.width() - 80.0, rect.height()),
    );
    painter.add(Shape::rect_filled(input_rect, Rounding::same(4.0), Color32::from_rgb(30, 30, 34)));

    painter.text(
        egui::pos2(input_rect.left() + 6.0, input_rect.center().y),
        egui::Align2::LEFT_CENTER,
        value,
        self.skin.font.clone(),
        Color32::from_rgb(232, 232, 236),
    );
}
```

- [ ] **Step 4: Add tests**

```rust
#[test]
fn test_tree_node_draws_without_panic() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 240.0), egui::vec2(200.0, 24.0));
        let clicked = gui.tree_node(rect, "Player", "🎮", false, 0);
        assert!(!clicked);
    });
}

#[test]
fn test_vec3_input_draws_without_panic() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 270.0), egui::vec2(300.0, 22.0));
        let mut x = 1.0; let mut y = 2.0; let mut z = 3.0;
        gui.vec3_input(rect, "位置", &mut x, &mut y, &mut z);
    });
}

#[test]
fn test_input_labeled_draws_without_panic() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 300.0), egui::vec2(200.0, 22.0));
        gui.input_labeled(rect, "材质", "Default");
    });
}
```

- [ ] **Step 5: Build and run tests**

Run: `cargo test -p engine-ui`
Expected: 37 tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/engine-ui/src/gui.rs
git commit -m "feat(gui): add tree_node, vec3_input, input_labeled controls"
```

---

### Task 4: Create EditorLayout struct

**Files:**
- Create: `examples/basic/src/editor.rs`

- [ ] **Step 1: Write EditorLayout + SceneNode structs**

```rust
use engine_ui::{Gui, GuiSkin};
use egui::{Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};

pub struct SceneNode {
    pub name: String,
    pub icon: String,
    pub children: Vec<SceneNode>,
    pub expanded: bool,
}

pub struct EditorLayout {
    // Control state
    pub active_menu: Option<usize>,
    pub active_tool: usize,
    pub active_viewport: usize,
    pub active_bottom_tab: usize,
    pub show_left_panel: bool,
    pub show_right_panel: bool,
    pub show_grid: bool,
    pub fps: u32,

    // Scene tree
    pub selected_node: usize,
    pub scene_tree: Vec<SceneNode>,

    // Inspector values
    pub pos: [f32; 3],
    pub rot: [f32; 3],
    pub scale: [f32; 3],
    pub material_name: String,
    pub mesh_name: String,
    pub cast_shadow: bool,
}
```

- [ ] **Step 2: Write `new()`**

```rust
impl EditorLayout {
    pub fn new() -> Self {
        Self {
            active_menu: None,
            active_tool: 0,
            active_viewport: 0,
            active_bottom_tab: 0,
            show_left_panel: true,
            show_right_panel: true,
            show_grid: true,
            fps: 60,
            selected_node: 0,
            scene_tree: vec![
                SceneNode {
                    name: "Root".into(),
                    icon: "📁".into(),
                    expanded: true,
                    children: vec![
                        SceneNode { name: "Player".into(), icon: "🎮".into(), children: vec![], expanded: false },
                        SceneNode { name: "Terrain".into(), icon: "🏔️".into(), children: vec![], expanded: false },
                        SceneNode { name: "Cube".into(), icon: "📦".into(), children: vec![], expanded: false },
                        SceneNode { name: "Sphere".into(), icon: "🔮".into(), children: vec![], expanded: false },
                        SceneNode { name: "Lights".into(), icon: "💡".into(), children: vec![], expanded: false },
                    ],
                },
            ],
            pos: [300.0, 200.0, 0.0],
            rot: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            material_name: "Default".into(),
            mesh_name: "Cube".into(),
            cast_shadow: true,
        }
    }
}
```

- [ ] **Step 3: Write `frame()` with rect calculation**

```rust
impl EditorLayout {
    pub fn frame(&mut self, ctx: &egui::Context, skin: &GuiSkin) {
        egui::Area::new(egui::Id::new("editor"))
            .interactable(true)
            .fixed_pos(Pos2::ZERO)
            .show(ctx, |ui| {
                let screen = ui.ctx().screen_rect();
                let menu_h = 32.0;
                let toolbar_h = 44.0;
                let status_h = 24.0;
                let bottom_h = 180.0;

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

                let left_w = 260.0;
                let right_w = 300.0;

                let hierarchy_rect = Rect::from_min_size(
                    main_rect.left_top(),
                    vec2(if self.show_left_panel { left_w } else { 0.0 }, main_rect.height()),
                );
                let inspector_rect = Rect::from_min_size(
                    Pos2::new(main_rect.right() - (if self.show_right_panel { right_w } else { 0.0 }), main_rect.top()),
                    vec2(if self.show_right_panel { right_w } else { 0.0 }, main_rect.height()),
                );
                let viewport_rect = Rect::from_min_size(
                    Pos2::new(hierarchy_rect.right(), main_rect.top()),
                    vec2(inspector_rect.left() - hierarchy_rect.right(), main_rect.height()),
                );

                let mut gui = Gui::new(ui, skin);
                self.draw_menu_bar(&mut gui, menu_rect);
                self.draw_toolbar(&mut gui, toolbar_rect);
                if self.show_left_panel {
                    self.draw_hierarchy(&mut gui, hierarchy_rect);
                }
                self.draw_viewport(&mut gui, viewport_rect);
                if self.show_right_panel {
                    self.draw_inspector(&mut gui, inspector_rect);
                }
                self.draw_bottom_panel(ui, bottom_rect, skin);
                self.draw_status_bar(&mut gui, status_rect);
            });
    }
}
```

- [ ] **Step 4: Build to verify compilation**

Run: `cargo build -p basic`
Expected: Build succeeds (editor.rs is not yet included in main.rs)

- [ ] **Step 5: Commit**

```bash
git add examples/basic/src/editor.rs
git commit -m "feat(example): create EditorLayout struct"
```

---

### Task 5: Draw menu bar, toolbar, hierarchy, viewport

**Files:**
- Modify: `examples/basic/src/editor.rs`

- [ ] **Step 1: `draw_menu_bar`**

```rust
fn draw_menu_bar(&mut self, gui: &mut Gui, rect: Rect) {
    let items = &["文件", "编辑", "视图", "场景", "资源", "构建", "窗口", "帮助"];
    let clicked = gui.menu_bar(rect, items);
    if let Some(i) = clicked {
        self.active_menu = Some(i);
    }
}
```

- [ ] **Step 2: `draw_toolbar`**

```rust
fn draw_toolbar(&mut self, gui: &mut Gui, rect: Rect) {
    let painter = gui.ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));

    // Tool group 1: select/move/rotate/scale
    let tools = &["↖", "↔", "⟳", "⤢"];
    let btn_size = 32.0;
    let group_x = rect.left() + 12.0;

    for (i, tool) in tools.iter().enumerate() {
        let btn_rect = Rect::from_min_size(
            Pos2::new(group_x + i as f32 * (btn_size + 4.0), rect.top() + (rect.height() - btn_size) / 2.0),
            vec2(btn_size, btn_size),
        );
        if gui.tool_button(btn_rect, tool, self.active_tool == i) {
            self.active_tool = i;
        }
    }

    // Separator
    let sep_x = group_x + 4.0 * (btn_size + 4.0) + 8.0;
    let sep_rect = Rect::from_min_size(Pos2::new(sep_x, rect.top()), vec2(1.0, rect.height()));
    gui.separator_h(sep_rect);

    // Tool group 2: panel toggles
    let btn2_x = sep_x + 12.0;
    let panel_btn_rect = |i: usize| -> Rect {
        Rect::from_min_size(
            Pos2::new(btn2_x + i as f32 * (btn_size + 4.0), rect.top() + (rect.height() - btn_size) / 2.0),
            vec2(btn_size, btn_size),
        )
    };
    if gui.tool_button(panel_btn_rect(0), "📁", false) {
        self.show_left_panel = !self.show_left_panel;
    }
    if gui.tool_button(panel_btn_rect(1), "🔍", false) {
        self.show_right_panel = !self.show_right_panel;
    }

    // Separator
    let sep2_x = btn2_x + 2.0 * (btn_size + 4.0) + 8.0;
    let sep2_rect = Rect::from_min_size(Pos2::new(sep2_x, rect.top()), vec2(1.0, rect.height()));
    gui.separator_h(sep2_rect);

    // Tool group 3: view mode
    let view_btn_x = sep2_x + 12.0;
    let modes = &["3D", "T", "F", "R"];
    for (i, mode) in modes.iter().enumerate() {
        let btn_rect = Rect::from_min_size(
            Pos2::new(view_btn_x + i as f32 * (btn_size + 4.0), rect.top() + (rect.height() - btn_size) / 2.0),
            vec2(btn_size, btn_size),
        );
        gui.tool_button(btn_rect, mode, self.active_viewport == i);
    }

    // Separator
    let sep3_x = view_btn_x + 4.0 * (btn_size + 4.0) + 8.0;

    // Play controls
    let play_x = sep3_x + 12.0;
    let play_btn = Rect::from_min_size(Pos2::new(play_x, rect.top() + (rect.height() - btn_size) / 2.0), vec2(btn_size, btn_size));
    gui.tool_button(play_btn, "▶", false);

    // FPS display
    let fps_text = format!("FPS: {}", self.fps);
    let painter = gui.ui.painter_at(rect);
    painter.text(
        egui::pos2(rect.right() - 12.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        &fps_text,
        egui::FontId::proportional(12.0),
        Color32::from_gray(90),
    );
}
```

- [ ] **Step 3: `draw_hierarchy`**

```rust
fn draw_hierarchy(&mut self, gui: &mut Gui, rect: Rect) {
    let painter = gui.ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));

    // Panel header
    let header_rect = Rect::from_min_size(rect.left_top(), vec2(rect.width(), 36.0));
    gui.panel_header(header_rect, "层级");

    // Content area
    let content_rect = Rect::from_min_size(
        Pos2::new(rect.left(), header_rect.bottom()),
        vec2(rect.width(), rect.bottom() - header_rect.bottom()),
    );

    let mut y = content_rect.top() + 4.0;
    let item_h = 24.0;

    for (i, node) in self.scene_tree.iter().enumerate() {
        self.draw_tree_node(gui, node, 0, i, &mut y, item_h, content_rect.right());
    }

    fn draw_tree_node(&mut self, gui: &mut Gui, node: &SceneNode, depth: u32, idx: usize, y: &mut f32, item_h: f32, right: f32) {
        let node_rect = Rect::from_min_size(
            Pos2::new(0.0, *y),
            vec2(right, item_h),
        );
        let selected = self.selected_node == idx;
        if gui.tree_node(node_rect, &node.name, &node.icon, selected, depth) {
            self.selected_node = idx;
        }
        *y += item_h;

        if node.expanded {
            for (ci, child) in node.children.iter().enumerate() {
                draw_tree_node(self, gui, child, depth + 1, idx * 100 + ci + 1, y, item_h, right);
            }
        }
    }
}
```

- [ ] **Step 4: `draw_viewport`**

```rust
fn draw_viewport(&mut self, gui: &mut Gui, rect: Rect) {
    let painter = gui.ui.painter_at(rect);

    // Viewport header tabs
    let header_rect = Rect::from_min_size(rect.left_top(), vec2(rect.width(), 32.0));
    painter.add(Shape::rect_filled(header_rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));

    let tab_w = 60.0;
    let tabs = &["场景", "游戏", "物理"];
    for (i, tab_label) in tabs.iter().enumerate() {
        let tab_rect = Rect::from_min_size(
            Pos2::new(rect.left() + i as f32 * tab_w, rect.top()),
            vec2(tab_w, 32.0),
        );
        if gui.tab(tab_rect, tab_label, self.active_viewport == i) {
            self.active_viewport = i;
        }
    }

    // Canvas (scene rendering area)
    let canvas_rect = Rect::from_min_size(
        Pos2::new(rect.left(), header_rect.bottom()),
        vec2(rect.width(), rect.bottom() - header_rect.bottom()),
    );

    // Gradient background
    painter.add(Shape::rect_filled(canvas_rect, Rounding::ZERO, Color32::from_rgb(10, 10, 12)));

    // Grid
    if self.show_grid {
        let grid_size = 50.0;
        let mut x = canvas_rect.left();
        while x <= canvas_rect.right() {
            painter.add(Shape::line(
                vec![Pos2::new(x, canvas_rect.top()), Pos2::new(x, canvas_rect.bottom())],
                Stroke::new(1.0, Color32::from_rgba_premultiplied(37, 37, 48, 128)),
            ));
            x += grid_size;
        }
        let mut y = canvas_rect.top();
        while y <= canvas_rect.bottom() {
            painter.add(Shape::line(
                vec![Pos2::new(canvas_rect.left(), y), Pos2::new(canvas_rect.right(), y)],
                Stroke::new(1.0, Color32::from_rgba_premultiplied(37, 37, 48, 128)),
            ));
            y += grid_size;
        }
    }

    // Axis labels
    let axes = [("X", Color32::from_rgb(255, 107, 107)),
                ("Y", Color32::from_rgb(46, 213, 115)),
                ("Z", Color32::from_rgb(77, 171, 247))];
    for (i, (label, color)) in axes.iter().enumerate() {
        painter.text(
            egui::pos2(canvas_rect.left() + 20.0, canvas_rect.top() + 20.0 + i as f32 * 18.0),
            egui::Align2::LEFT_CENTER,
            *label,
            egui::FontId::proportional(10.0),
            *color,
        );
    }

    // Dummy cube objects
    let objects = [
        ("📦", Vec2::new(200.0, 150.0)),
        ("🎯", Vec2::new(350.0, 230.0)),
        ("🔮", Vec2::new(450.0, 130.0)),
    ];
    for (icon, pos) in &objects {
        let obj_rect = Rect::from_center_size(
            Pos2::new(canvas_rect.left() + pos.x, canvas_rect.top() + pos.y),
            Vec2::new(48.0, 48.0),
        );
        painter.add(Shape::rect_filled(
            obj_rect,
            Rounding::same(4.0),
            Color32::from_rgb(42, 42, 53),
        ));
        painter.rect_stroke(
            obj_rect,
            Rounding::same(4.0),
            Stroke::new(2.0, Color32::from_rgb(0, 212, 170)),
        );
        painter.text(
            obj_rect.center(),
            egui::Align2::CENTER_CENTER,
            *icon,
            egui::FontId::proportional(24.0),
            Color32::WHITE,
        );
    }
}
```

- [ ] **Step 5: Build**

Run: `cargo build -p basic`
Expected: Builds (unused code warnings OK)

- [ ] **Step 6: Commit**

```bash
git add examples/basic/src/editor.rs
git commit -m "feat(example): draw menu_bar, toolbar, hierarchy, viewport"
```

---

### Task 6: Draw inspector, bottom panel, status bar

**Files:**
- Modify: `examples/basic/src/editor.rs`

- [ ] **Step 1: `draw_inspector`**

```rust
fn draw_inspector(&mut self, gui: &mut Gui, rect: Rect) {
    let painter = gui.ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));

    // Search bar
    let search_rect = Rect::from_min_size(
        Pos2::new(rect.left(), rect.top() + 8.0),
        vec2(rect.width() - 16.0, 28.0),
    );
    painter.add(Shape::rect_filled(
        Rect::from_min_size(Pos2::new(rect.left() + 8.0, search_rect.top()), vec2(rect.width() - 16.0, 28.0)),
        Rounding::same(6.0),
        Color32::from_rgb(30, 30, 34),
    ));
    painter.text(
        egui::pos2(rect.left() + 16.0, search_rect.center().y),
        egui::Align2::LEFT_CENTER,
        "🔍 搜索属性...",
        egui::FontId::proportional(12.0),
        Color32::from_gray(90),
    );

    // Content
    let content_top = search_rect.bottom() + 16.0;
    let row_h = 22.0;
    let label_w = 65.0;
    let left = rect.left() + 12.0;
    let content_w = rect.width() - 24.0;

    // Transform section
    painter.text(
        egui::pos2(left, content_top),
        egui::Align2::LEFT_CENTER,
        "变换",
        egui::FontId::proportional(11.0),
        Color32::from_gray(90),
    );
    let sep_y = content_top + 18.0;
    painter.add(Shape::line(
        vec![Pos2::new(left, sep_y), Pos2::new(rect.right() - 12.0, sep_y)],
        Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
    ));

    let mut y = sep_y + 12.0;

    // Position
    let pos_rect = Rect::from_min_size(Pos2::new(left, y), vec2(content_w, row_h));
    gui.vec3_input(pos_rect, "位置", &mut self.pos[0], &mut self.pos[1], &mut self.pos[2]);
    y += row_h + 6.0;

    // Rotation
    let rot_rect = Rect::from_min_size(Pos2::new(left, y), vec2(content_w, row_h));
    gui.vec3_input(rot_rect, "旋转", &mut self.rot[0], &mut self.rot[1], &mut self.rot[2]);
    y += row_h + 6.0;

    // Scale
    let scale_rect = Rect::from_min_size(Pos2::new(left, y), vec2(content_w, row_h));
    gui.vec3_input(scale_rect, "缩放", &mut self.scale[0], &mut self.scale[1], &mut self.scale[2]);
    y += row_h + 12.0;

    // Render section
    painter.text(
        egui::pos2(left, y),
        egui::Align2::LEFT_CENTER,
        "渲染",
        egui::FontId::proportional(11.0),
        Color32::from_gray(90),
    );
    let sep2_y = y + 18.0;
    painter.add(Shape::line(
        vec![Pos2::new(left, sep2_y), Pos2::new(rect.right() - 12.0, sep2_y)],
        Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
    ));
    y = sep2_y + 12.0;

    // Material
    let mat_rect = Rect::from_min_size(Pos2::new(left, y), vec2(content_w, row_h));
    gui.input_labeled(mat_rect, "材质", &self.material_name);
    y += row_h + 6.0;

    // Mesh
    let mesh_rect = Rect::from_min_size(Pos2::new(left, y), vec2(content_w, row_h));
    gui.input_labeled(mesh_rect, "网格", &self.mesh_name);
    y += row_h + 6.0;

    // Cast shadow checkbox
    let shadow_rect = Rect::from_min_size(Pos2::new(left, y), vec2(content_w, row_h));
    gui.checkbox(shadow_rect, "投射阴影", &mut self.cast_shadow);
}
```

- [ ] **Step 2: `draw_bottom_panel`**

```rust
fn draw_bottom_panel(&mut self, ui: &egui::Ui, rect: Rect, skin: &GuiSkin) {
    let painter = ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));

    // Tab bar
    let tab_bar_rect = Rect::from_min_size(rect.left_top(), vec2(rect.width(), 32.0));
    let tab_labels = &["日志", "性能", "音频", "网络"];
    let tab_w = 60.0;

    for (i, tab_label) in tab_labels.iter().enumerate() {
        let tab_rect = Rect::from_min_size(
            Pos2::new(rect.left() + 8.0 + i as f32 * tab_w, rect.top()),
            vec2(tab_w, 32.0),
        );
        if let Some(gui) = Some(&mut Gui::new(ui, skin)) {
            let mut gui_bar = Gui::new(ui, skin);
            gui_bar.tab(tab_rect, tab_label, self.active_bottom_tab == i);
        }
    }

    // Content area
    let content_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 12.0, tab_bar_rect.bottom() + 8.0),
        vec2(rect.width() - 24.0, rect.bottom() - tab_bar_rect.bottom() - 16.0),
    );

    match self.active_bottom_tab {
        0 => {
            // Log content
            let logs = [
                ("10:23:15", "info", "编辑器已启动"),
                ("10:23:16", "info", "项目已加载: MyGame"),
                ("10:23:18", "info", "着色器编译完成 (12个)"),
                ("10:23:20", "warn", "缺少法线贴图: Materials/Wood"),
                ("10:23:22", "info", "场景保存成功"),
            ];
            let mut y = content_rect.top();
            for (time, level, msg) in &logs {
                let level_color = match *level {
                    "info" => Color32::from_gray(152),
                    "warn" => Color32::from_rgb(255, 184, 0),
                    _ => Color32::from_rgb(255, 71, 87),
                };
                painter.text(
                    egui::pos2(content_rect.left(), y + 10.0),
                    egui::Align2::LEFT_CENTER,
                    &format!("{}  {}  {}", time, level, msg),
                    egui::FontId::proportional(11.0),
                    Color32::from_rgb(232, 232, 236),
                );
                y += 18.0;
            }
        }
        1 => {
            // Performance
            let perf_data = [
                "Draw Calls: 128",
                "Triangles: 45.2K",
                "Vertices: 22.8K",
                "GPU: 32ms",
                "Memory: 256MB / 2GB",
            ];
            let mut y = content_rect.top();
            for line in &perf_data {
                painter.text(
                    egui::pos2(content_rect.left(), y + 10.0),
                    egui::Align2::LEFT_CENTER,
                    *line,
                    egui::FontId::proportional(11.0),
                    Color32::from_rgb(232, 232, 236),
                );
                y += 18.0;
            }
        }
        _ => {
            painter.text(
                content_rect.center(),
                egui::Align2::CENTER_CENTER,
                "-- 面板内容 --",
                egui::FontId::proportional(11.0),
                Color32::from_gray(90),
            );
        }
    }
}
```

Wait — `draw_bottom_panel` creates a `Gui` inside to call `tab()`. But the `tab()` method needs to interact with the same `ui`. Since `draw_bottom_panel` takes `&egui::Ui` directly, it creates a temporary `Gui`. But the issue is: the `if let Some(gui)` is pointless since `Some(&mut Gui::new(...))` is always Some. And then it creates TWO Gui instances. Let me fix this:

Replace the tab drawing section with:
```rust
    {
        let mut gui = Gui::new(ui, skin);
        for (i, tab_label) in tab_labels.iter().enumerate() {
            let tab_rect = Rect::from_min_size(
                Pos2::new(rect.left() + 8.0 + i as f32 * tab_w, rect.top()),
                vec2(tab_w, 32.0),
            );
            if gui.tab(tab_rect, tab_label, self.active_bottom_tab == i) {
                self.active_bottom_tab = i;
            }
        }
    }
```

- [ ] **Step 3: `draw_status_bar`**

```rust
fn draw_status_bar(&mut self, gui: &mut Gui, rect: Rect) {
    let painter = gui.ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(30, 30, 34)));

    gui.status_item(
        Rect::from_min_size(Pos2::new(rect.left() + 12.0, rect.top()), vec2(60.0, rect.height())),
        "就绪",
        Color32::from_rgb(46, 213, 115),
    );

    painter.text(
        egui::pos2(rect.left() + 80.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        &format!("对象: {}", self.scene_tree.iter().map(|n| 1 + n.children.len()).sum::<usize>()),
        egui::FontId::proportional(11.0),
        Color32::from_gray(90),
    );

    painter.text(
        egui::pos2(rect.left() + 150.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        "三角形: 45K",
        egui::FontId::proportional(11.0),
        Color32::from_gray(90),
    );

    painter.text(
        egui::pos2(rect.right() - 80.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        "perspective",
        egui::FontId::proportional(11.0),
        Color32::from_gray(90),
    );
}
```

- [ ] **Step 4: Build**

Run: `cargo build -p basic`
Expected: Builds

- [ ] **Step 5: Commit**

```bash
git add examples/basic/src/editor.rs
git commit -m "feat(example): draw inspector, bottom panel, status bar"
```

---

### Task 7: Wire EditorLayout into main.rs

**Files:**
- Modify: `examples/basic/src/main.rs`

- [ ] **Step 1: Replace inline demo with EditorLayout**

Current `main.rs` post_update_hook:
```rust
app.add_post_update_hook(Box::new(move |app: &mut App| {
    let skin = app.resources.get::<GuiSkin>().cloned().unwrap_or_default();
    let egui_state = app.resources.get_mut::<EguiState>().unwrap();
    let ctx = egui_state.ctx();

    egui::Area::new(egui::Id::new("gui_root"))
        .fixed_pos(Pos2::ZERO)
        .show(ctx, |ui| {
            let screen = ui.ctx().screen_rect();
            let mut gui = Gui::new(ui, &skin);
            gui.box_(Rect::from_min_size(screen.left_top(), Vec2::new(screen.width(), 30.0)), "RustEngine IMGUI Demo");
        });

    GuiLayout::new(ctx, &skin).window("Inspector", &mut panel_rect, |v| {
        v.label("Position:");
        v.horizontal(|h| {
            h.label("X:");
            h.text_field("0.0", 60.0);
        });
        v.separator();
        v.label("Visible:");
        v.toggle(&mut visible, "Show Grid");
        v.separator();
        v.label("Opacity:");
        v.slider(&mut opacity, 0.0, 1.0, 200.0);
        v.separator();
        if v.button("Apply") {
            println!("Apply clicked!");
        }
    });
}));
```

Replace with:
```rust
use editor::EditorLayout;
mod editor;

// In main():
// Remove: let mut visible = true; let mut opacity = 1.0f32; let mut panel_rect = ...
// Replace post_update_hook with:

let mut editor = EditorLayout::new();
app.add_post_update_hook(Box::new(move |app: &mut App| {
    let skin = app.resources.get::<GuiSkin>().cloned().unwrap_or_default();
    let egui_state = app.resources.get_mut::<EguiState>().unwrap();
    let ctx = egui_state.ctx();
    editor.frame(ctx, &skin);
}));
```

But wait — `editor` should be a `RefCell` or created inside the closure, because the closure is `move` and `editor` needs to be mutable across frames. Actually, the closure is already `move` (as seen in the existing code with `visible`, `opacity`, `panel_rect`). And `EditorLayout` has mutable state (click state, checked values, etc.).

The `move` closure captures `editor` by value. Since we declare `let mut editor = ...` and then `move |app: &mut App| { ... editor.frame(...) ... }`, the `editor` is moved into the closure. Inside the closure, we can mutate it because `Box<dyn FnMut>` allows mutable access.

Wait, but the closure is `Box::new(move |app: &mut App| { ... })`. With `move`, variables are moved into the closure. Since `editor` is `let mut`, it's moved as mutable. And since the closure is `FnMut` (because it captures `&mut`), this should work.

Actually, let me verify. The `Hook` type is:
```rust
type Hook = Box<dyn FnMut(&mut App)>;
```

And the hook is `Box::new(move |app: &mut App| { ... })`. With a `move` closure that captures `editor` (which is `let mut editor`), the closure will own `editor` and can mutate it. This works because `FnMut` allows mutation. 

But wait — the `move` keyword captures by value. For `let mut editor = ...`, the `move` closure takes ownership. And since the closure is `FnMut` (mutable callable), the captured variables can be modified. This is fine.

So the replacement is straightforward:

```rust
use engine_ui::{EguiPlugin, EguiState, GuiSkin, ImGuiPlugin, Gui, GuiLayout};
```
Changes to:
```rust
use engine_ui::{EguiPlugin, EguiState, GuiSkin, ImGuiPlugin};
```

And add:
```rust
mod editor;
use editor::EditorLayout;
```

Then replace the post_update_hook content.

Let me write the exact changes:

Old main.rs:
```rust
use engine_core::app::{App, AppBuilder};
use engine_core::debug::DebugPlugin;
use engine_core::engine::run_default;
use engine_core::plugin::Plugin;
use engine_framework::{FrameworkPlugin, GameState, StateCtx, StateStack};
use engine_ui::{EguiPlugin, EguiState, GuiSkin, ImGuiPlugin, Gui, GuiLayout};
use egui::{Rect, Pos2, Vec2};

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
            if !PUSHED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                if let Some(stack) = app.resources.get_mut::<StateStack>() {
                    stack.push(Box::new(MenuState));
                }
            }
        }));

        let mut visible = true;
        let mut opacity = 1.0f32;
        let mut panel_rect = Rect::from_min_size(Pos2::new(10.0, 40.0), Vec2::new(250.0, 300.0));

        app.add_post_update_hook(Box::new(move |app: &mut App| {
            // ... demo code ...
        }));
    }
}

fn main() { ... }
```

New:
```rust
use engine_core::app::{App, AppBuilder};
use engine_core::debug::DebugPlugin;
use engine_core::engine::run_default;
use engine_core::plugin::Plugin;
use engine_framework::{FrameworkPlugin, GameState, StateCtx, StateStack};
use engine_ui::{EguiPlugin, EguiState, GuiSkin, ImGuiPlugin};

mod editor;
use editor::EditorLayout;

// ... MenuState same ...

impl Plugin for GamePlugin {
    fn build(&self, app: &mut AppBuilder) {
        // ... pre_update_hook same ...

        let mut editor = EditorLayout::new();

        app.add_post_update_hook(Box::new(move |app: &mut App| {
            let skin = app.resources.get::<GuiSkin>().cloned().unwrap_or_default();
            let egui_state = app.resources.get_mut::<EguiState>().unwrap();
            let ctx = egui_state.ctx();
            editor.frame(ctx, &skin);
        }));
    }
}

fn main() { ... }
```

- [ ] **Step 2: Build**

Run: `cargo build --release -p basic`
Expected: Build succeeds

- [ ] **Step 3: Commit**

```bash
git add examples/basic/src/main.rs
git commit -m "feat(example): wire EditorLayout into main"
```

---

### Task 8: Full build, verify, push

- [ ] **Step 1: Run all tests**

Run: `cargo test -p engine-ui -p engine-core`
Expected: 38+ tests pass (engine-ui: 37 + 1 engine-core)

- [ ] **Step 2: Clippy**

Run: `cargo clippy -p engine-ui -p engine-core`
Expected: No warnings

- [ ] **Step 3: Build release**

Run: `cargo build --release`
Expected: Success

- [ ] **Step 4: Verify binary exists**

Run: `Get-Item "target/release/basic.exe"`
Expected: File exists, ~8MB+

- [ ] **Step 6 (sic): Commit + push**

```bash
git add -A
git status
git commit -m "feat(editor): full editor IMGUI layout with all panels"
git push
```
