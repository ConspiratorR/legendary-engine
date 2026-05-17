use egui::{vec2, Color32, FontId, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use engine_ui::{Gui, GuiSkin};

pub struct SceneNode {
    pub name: String,
    pub icon: String,
    pub children: Vec<SceneNode>,
    pub expanded: bool,
}

pub struct EditorLayout {
    pub active_menu: Option<usize>,
    pub active_tool: usize,
    pub active_viewport: usize,
    pub active_bottom_tab: usize,
    pub show_left_panel: bool,
    pub show_right_panel: bool,
    pub show_grid: bool,
    pub fps: u32,
    pub selected_node: usize,
    pub scene_tree: Vec<SceneNode>,
    pub pos: [f32; 3],
    pub rot: [f32; 3],
    pub scale: [f32; 3],
    pub material_name: String,
    pub mesh_name: String,
    pub cast_shadow: bool,
}

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

    // ── layout ──────────────────────────────────────────────────────
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

    // ── menu bar ────────────────────────────────────────────────────
    fn draw_menu_bar(&mut self, gui: &mut Gui, rect: Rect) {
        let painter = gui.ui.painter_at(rect);
        painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));
        painter.add(Shape::line(
            vec![Pos2::new(rect.left(), rect.bottom() - 1.0), Pos2::new(rect.right(), rect.bottom() - 1.0)],
            Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
        ));

        let items = &["文件", "编辑", "视图", "场景", "资源", "构建", "窗口", "帮助"];
        let mut x = rect.left() + 8.0;
        let item_pad = 12.0;
        for (i, item) in items.iter().enumerate() {
            let text_w = item.len() as f32 * 8.0;
            let item_rect = Rect::from_min_size(Pos2::new(x, rect.top()), vec2(text_w + item_pad * 2.0, rect.height()));
            let id = egui::Id::new("mm").with(i as u64);
            let response = gui.ui.interact(item_rect, id, egui::Sense::click());
            if response.hovered() || self.active_menu == Some(i) {
                painter.add(Shape::rect_filled(item_rect, Rounding::same(4.0), Color32::from_rgb(30, 30, 34)));
            }
            painter.text(
                egui::pos2(x + item_pad, rect.center().y),
                egui::Align2::LEFT_CENTER,
                *item,
                FontId::proportional(13.0),
                if response.hovered() { Color32::from_rgb(232, 232, 236) } else { Color32::from_gray(152) },
            );
            if response.clicked() {
                self.active_menu = Some(i);
            }
            x += text_w + item_pad * 2.0 + 4.0;
        }

        // Project name at right
        painter.text(
            egui::pos2(rect.right() - 12.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            "MyGame",
            FontId::proportional(13.0),
            Color32::from_gray(152),
        );
    }

    // ── toolbar ─────────────────────────────────────────────────────
    fn draw_separator(&self, painter: &egui::Painter, pos: f32, top: f32, bottom: f32) {
        painter.add(Shape::line(
            vec![Pos2::new(pos, top + 8.0), Pos2::new(pos, bottom - 8.0)],
            Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
        ));
    }

    fn draw_toolbar(&mut self, gui: &mut Gui, rect: Rect) {
        let painter = gui.ui.painter_at(rect);
        painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));
        painter.add(Shape::line(
            vec![Pos2::new(rect.left(), rect.bottom() - 1.0), Pos2::new(rect.right(), rect.bottom() - 1.0)],
            Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
        ));

        let mut x = rect.left() + 12.0;
        let btn_size = 32.0;
        let gap = 4.0;
        let cy = rect.top() + (rect.height() - btn_size) / 2.0;

        // Tool group 1: select/move/rotate/scale
        let tools = &["↖", "↔", "⟳", "⤢"];
        for (i, tool) in tools.iter().enumerate() {
            let btn_rect = Rect::from_min_size(Pos2::new(x + i as f32 * (btn_size + gap), cy), vec2(btn_size, btn_size));
            if gui.tool_button(btn_rect, tool, self.active_tool == i) {
                self.active_tool = i;
            }
        }
        x += 4.0 * (btn_size + gap) + 12.0;
        self.draw_separator(&painter, x, rect.top(), rect.bottom());
        x += 12.0;

        // Tool group 2: panel toggles
        for (i, icon) in ["📁", "🔍"].iter().enumerate() {
            let btn_rect = Rect::from_min_size(Pos2::new(x + i as f32 * (btn_size + gap), cy), vec2(btn_size, btn_size));
            if gui.tool_button(btn_rect, icon, false) {
                if i == 0 { self.show_left_panel = !self.show_left_panel; }
                if i == 1 { self.show_right_panel = !self.show_right_panel; }
            }
        }
        x += 2.0 * (btn_size + gap) + 12.0;
        self.draw_separator(&painter, x, rect.top(), rect.bottom());
        x += 12.0;

        // Tool group 3: view mode
        let modes = &["3D", "T", "F", "R"];
        for (i, mode) in modes.iter().enumerate() {
            let btn_rect = Rect::from_min_size(Pos2::new(x + i as f32 * (btn_size + gap), cy), vec2(btn_size, btn_size));
            gui.tool_button(btn_rect, mode, self.active_viewport == i);
        }
        x += 4.0 * (btn_size + gap) + 12.0;
        self.draw_separator(&painter, x, rect.top(), rect.bottom());
        x += 12.0;

        // Tool group 4: play controls (▶ ⏸ ⏹)
        for icon in ["▶", "⏸", "⏹"].iter() {
            let btn_rect = Rect::from_min_size(Pos2::new(x, cy), vec2(btn_size, btn_size));
            gui.tool_button(btn_rect, icon, false);
            x += btn_size + gap;
        }
        x += 8.0;
        self.draw_separator(&painter, x, rect.top(), rect.bottom());
        x += 12.0;

        // FPS label
        painter.text(
            egui::pos2(x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            format!("FPS: {}", self.fps),
            FontId::proportional(12.0),
            Color32::from_gray(90),
        );
    }

    // ── hierarchy panel ────────────────────────────────────────────
    fn draw_hierarchy(&mut self, gui: &mut Gui, rect: Rect) {
        let painter = gui.ui.painter_at(rect);
        painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));
        painter.add(Shape::line(
            vec![Pos2::new(rect.right() - 1.0, rect.top()), Pos2::new(rect.right() - 1.0, rect.bottom())],
            Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
        ));

        // Panel header with action buttons
        let header_rect = Rect::from_min_size(rect.left_top(), vec2(rect.width(), 36.0));
        painter.add(Shape::rect_filled(header_rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));
        painter.text(
            egui::pos2(rect.left() + 12.0, header_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "层级",
            FontId::proportional(12.0),
            Color32::from_gray(90),
        );
        // Action buttons + and 🔍
        for (i, icon) in ["+", "🔍"].iter().enumerate() {
            let btn_rect = Rect::from_min_size(
                Pos2::new(rect.right() - 28.0 - i as f32 * 28.0, header_rect.top() + (36.0 - 24.0) / 2.0),
                vec2(24.0, 24.0),
            );
            let id = egui::Id::new("hdr_act").with(i as u64);
            let response = gui.ui.interact(btn_rect, id, egui::Sense::click());
            if response.hovered() {
                painter.add(Shape::rect_filled(btn_rect, Rounding::same(4.0), Color32::from_rgb(30, 30, 34)));
            }
            painter.text(btn_rect.center(), egui::Align2::CENTER_CENTER, *icon, FontId::proportional(14.0), Color32::from_gray(90));
        }
        let line_y = header_rect.bottom() - 1.0;
        painter.add(Shape::line(
            vec![Pos2::new(rect.left(), line_y), Pos2::new(rect.right(), line_y)],
            Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
        ));

        // Content with 8px padding
        let content_rect = Rect::from_min_size(
            Pos2::new(rect.left(), header_rect.bottom()),
            vec2(rect.width(), rect.bottom() - header_rect.bottom()),
        );
        let left_pad = rect.left() + 8.0;
        let content_right = rect.right() - 8.0;
        let mut y = content_rect.top() + 8.0;
        let item_h = 28.0;

        let mut counter = 0usize;
        EditorLayout::draw_tree(gui, &self.scene_tree, 0, &mut y, &mut counter, left_pad, content_right, item_h, &mut self.selected_node);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_tree(
        gui: &mut Gui, nodes: &[SceneNode], depth: u32,
        y: &mut f32, counter: &mut usize,
        left: f32, right: f32, item_h: f32,
        selected_node: &mut usize,
    ) {
        for node in nodes.iter() {
            let indent = left + depth as f32 * 16.0;
            let node_rect = Rect::from_min_size(Pos2::new(left, *y), vec2(right - left, item_h));

            let idx = *counter;
            *counter += 1;

            let painter = gui.ui.painter_at(node_rect);
            let id_rect = Rect::from_min_size(Pos2::new(indent, *y), vec2(right - indent, item_h));
            let id = egui::Id::new("tree").with(idx as u64);
            let response = gui.ui.interact(id_rect, id, egui::Sense::click());

            if idx == *selected_node {
                painter.add(Shape::rect_filled(id_rect, Rounding::same(4.0), Color32::from_rgba_premultiplied(0, 212, 170, 30)));
            } else if response.hovered() {
                painter.add(Shape::rect_filled(id_rect, Rounding::same(4.0), Color32::from_rgb(30, 30, 34)));
            }

            let arrow_rect = Rect::from_min_size(Pos2::new(indent, *y + (item_h - 16.0) / 2.0), vec2(16.0, 16.0));
            if !node.children.is_empty() {
                painter.text(
                    arrow_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    if node.expanded { "▾" } else { "▸" },
                    FontId::proportional(10.0),
                    Color32::from_gray(90),
                );
            }

            painter.text(
                egui::pos2(indent + 20.0, *y + item_h / 2.0),
                egui::Align2::LEFT_CENTER,
                &node.icon,
                FontId::proportional(14.0),
                Color32::from_gray(200),
            );

            painter.text(
                egui::pos2(indent + 42.0, *y + item_h / 2.0),
                egui::Align2::LEFT_CENTER,
                &node.name,
                FontId::proportional(13.0),
                Color32::from_rgb(232, 232, 236),
            );

            if response.clicked() {
                *selected_node = idx;
            }

            *y += item_h;

            if node.expanded {
                Self::draw_tree(gui, &node.children, depth + 1, y, counter, left, right, item_h, selected_node);
            }
        }
    }

    // ── viewport ────────────────────────────────────────────────────
    fn draw_viewport(&mut self, gui: &mut Gui, rect: Rect) {
        let painter = gui.ui.painter_at(rect);

        // Header
        let header_rect = Rect::from_min_size(rect.left_top(), vec2(rect.width(), 32.0));
        painter.add(Shape::rect_filled(header_rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));
        painter.add(Shape::line(
            vec![Pos2::new(rect.left(), header_rect.bottom() - 1.0), Pos2::new(rect.right(), header_rect.bottom() - 1.0)],
            Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
        ));

        // Viewport tabs with gap 16px between them
        let tabs = &["场景", "游戏", "物理"];
        let mut tx = rect.left() + 12.0;
        let tab_pad = 12.0;
        for (i, label) in tabs.iter().enumerate() {
            let text_w = label.len() as f32 * 8.0;
            let tab_rect = Rect::from_min_size(Pos2::new(tx, rect.top()), vec2(text_w + tab_pad * 2.0, 32.0));
            let id = egui::Id::new("vp_tab").with(i as u64);
            let response = gui.ui.interact(tab_rect, id, egui::Sense::click());
            if self.active_viewport == i {
                // Bottom accent line
                let line_rect = Rect::from_min_size(Pos2::new(tab_rect.left(), tab_rect.bottom() - 2.0), vec2(tab_rect.width(), 2.0));
                painter.add(Shape::rect_filled(line_rect, Rounding::ZERO, Color32::from_rgb(0, 212, 170)));
                painter.text(tab_rect.center(), egui::Align2::CENTER_CENTER, *label, FontId::proportional(12.0), Color32::from_rgb(0, 212, 170));
            } else {
                painter.text(tab_rect.center(), egui::Align2::CENTER_CENTER, *label, FontId::proportional(12.0), Color32::from_gray(90));
            }
            if response.clicked() {
                self.active_viewport = i;
            }
            tx += text_w + tab_pad * 2.0 + 16.0;
        }

        // Viewport tools on the right (📐 # ⌖)
        let tool_icons = &["📐", "#", "⌖"];
        let mut tool_x = rect.right() - 12.0 - tool_icons.len() as f32 * (24.0 + 4.0);
        for icon in tool_icons {
            let tool_rect = Rect::from_min_size(Pos2::new(tool_x, rect.top() + (32.0 - 24.0) / 2.0), vec2(24.0, 24.0));
            let id = egui::Id::new("vp_tool").with(tool_x as u64);
            let response = gui.ui.interact(tool_rect, id, egui::Sense::click());
            if response.hovered() {
                painter.add(Shape::rect_filled(tool_rect, Rounding::same(4.0), Color32::from_rgb(30, 30, 34)));
            }
            painter.text(tool_rect.center(), egui::Align2::CENTER_CENTER, *icon, FontId::proportional(12.0), Color32::from_gray(90));
            tool_x += 28.0;
        }

        // Canvas with gradient background
        let canvas_rect = Rect::from_min_size(
            Pos2::new(rect.left(), header_rect.bottom()),
            vec2(rect.width(), rect.bottom() - header_rect.bottom()),
        );
        // Approximate gradient with layered rects: 0a0a0c → 141418
        let gradient_steps = 20;
        let step_h = canvas_rect.height() / gradient_steps as f32;
        for i in 0..gradient_steps {
            let t = i as f32 / (gradient_steps - 1) as f32;
            let r = (10.0 + t * 10.0) as u8;
            let g = (10.0 + t * 10.0) as u8;
            let b = (12.0 + t * 16.0) as u8;
            let strip = Rect::from_min_size(
                Pos2::new(canvas_rect.left(), canvas_rect.top() + i as f32 * step_h),
                vec2(canvas_rect.width(), step_h + 1.0),
            );
            painter.add(Shape::rect_filled(strip, Rounding::ZERO, Color32::from_rgb(r, g, b)));
        }

        // Grid
        if self.show_grid {
            let grid_size = 50.0;
            let grid_color = Color32::from_rgba_premultiplied(37, 37, 48, 128);
            let mut x = canvas_rect.left();
            while x <= canvas_rect.right() {
                painter.add(Shape::line(
                    vec![Pos2::new(x, canvas_rect.top()), Pos2::new(x, canvas_rect.bottom())],
                    Stroke::new(1.0, grid_color),
                ));
                x += grid_size;
            }
            let mut y = canvas_rect.top();
            while y <= canvas_rect.bottom() {
                painter.add(Shape::line(
                    vec![Pos2::new(canvas_rect.left(), y), Pos2::new(canvas_rect.right(), y)],
                    Stroke::new(1.0, grid_color),
                ));
                y += grid_size;
            }
        }

        // Axis labels: left 20, top 20, gap 4 between lines, font-size 10, font-weight 600
        let axes = [
            ("X", Color32::from_rgb(255, 107, 107)),
            ("Y", Color32::from_rgb(46, 213, 115)),
            ("Z", Color32::from_rgb(77, 171, 247)),
        ];
        for (i, (label, color)) in axes.iter().enumerate() {
            painter.text(
                egui::pos2(canvas_rect.left() + 20.0, canvas_rect.top() + 20.0 + i as f32 * 14.0),
                egui::Align2::LEFT_CENTER,
                *label,
                FontId::proportional(10.0),
                *color,
            );
        }

        // Floating objects 60×60 with gradient + glow
        let objects = [
            ("📦", Vec2::new(200.0, 150.0)),
            ("🎯", Vec2::new(350.0, 230.0)),
            ("🔮", Vec2::new(450.0, 130.0)),
        ];
        let obj_size = 60.0;
        for (icon, pos) in &objects {
            let obj_rect = Rect::from_center_size(
                Pos2::new(canvas_rect.left() + pos.x, canvas_rect.top() + pos.y),
                Vec2::new(obj_size, obj_size),
            );
            // Glow shadow
            let glow_rect = obj_rect.expand(8.0);
            painter.add(Shape::rect_filled(glow_rect, Rounding::same(8.0), Color32::from_rgba_premultiplied(0, 212, 170, 20)));
            // Gradient-like fill (two layers)
            painter.add(Shape::rect_filled(obj_rect, Rounding::same(4.0), Color32::from_rgb(42, 42, 53)));
            let inner_grad = Rect::from_min_size(obj_rect.left_top(), vec2(obj_rect.width(), obj_rect.height() / 2.0));
            painter.add(Shape::rect_filled(inner_grad, Rounding::same(4.0), Color32::from_rgba_premultiplied(255, 255, 255, 8)));
            // Border
            painter.rect_stroke(obj_rect, Rounding::same(4.0), Stroke::new(2.0, Color32::from_rgb(0, 212, 170)));
            painter.text(obj_rect.center(), egui::Align2::CENTER_CENTER, *icon, FontId::proportional(28.0), Color32::WHITE);
        }

        // Transform info bar at bottom-left
        let transform_rect = Rect::from_min_size(
            Pos2::new(canvas_rect.left() + 20.0, canvas_rect.bottom() - 44.0),
            vec2(200.0, 28.0),
        );
        painter.add(Shape::rect_filled(transform_rect, Rounding::same(6.0), Color32::from_rgba_premultiplied(22, 22, 25, 230)));
        let transform_axes = [
            ("X", self.pos[0], Color32::from_rgb(255, 107, 107)),
            ("Y", self.pos[1], Color32::from_rgb(46, 213, 115)),
            ("Z", self.pos[2], Color32::from_rgb(77, 171, 247)),
        ];
        for (i, (label, val, color)) in transform_axes.iter().enumerate() {
            painter.text(
                egui::pos2(transform_rect.left() + 12.0 + i as f32 * 60.0, transform_rect.center().y),
                egui::Align2::LEFT_CENTER,
                format!("{} {}", label, val.round() as i32),
                FontId::proportional(11.0),
                *color,
            );
        }
    }

    // ── inspector panel ────────────────────────────────────────────
    fn draw_inspector(&mut self, gui: &mut Gui, rect: Rect) {
        let painter = gui.ui.painter_at(rect);
        painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));
        painter.add(Shape::line(
            vec![Pos2::new(rect.left(), rect.top()), Pos2::new(rect.left(), rect.bottom())],
            Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
        ));

        // Search input
        painter.add(Shape::rect_filled(
            Rect::from_min_size(Pos2::new(rect.left() + 8.0, rect.top() + 8.0), vec2(rect.width() - 16.0, 36.0)),
            Rounding::same(6.0),
            Color32::from_rgb(30, 30, 34),
        ));
        painter.text(
            egui::pos2(rect.left() + 20.0, rect.top() + 26.0),
            egui::Align2::LEFT_CENTER,
            "🔍 搜索属性...",
            FontId::proportional(12.0),
            Color32::from_gray(90),
        );

        let content_top = rect.top() + 52.0;
        let left = rect.left() + 12.0;
        let content_w = rect.width() - 24.0;
        let row_h = 26.0;
        let mut y = content_top;

        // Helper: section header
        let section_header = |painter: &egui::Painter, title: &str, y: &mut f32| {
            painter.text(
                egui::pos2(left, *y),
                egui::Align2::LEFT_CENTER,
                title,
                FontId::proportional(11.0),
                Color32::from_gray(90),
            );
            let sep_y = *y + 18.0;
            painter.add(Shape::line(
                vec![Pos2::new(left + 30.0 + title.len() as f32 * 6.5, sep_y), Pos2::new(rect.right() - 12.0, sep_y)],
                Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
            ));
            *y = sep_y + 12.0;
        };

        // ── Transform section ──
        section_header(&painter, "变换", &mut y);

        // Position
        let pos_rect = Rect::from_min_size(Pos2::new(left, y), vec2(content_w, row_h));
        let (mut px, mut py, mut pz) = (self.pos[0], self.pos[1], self.pos[2]);
        gui.vec3_input(pos_rect, "位置", &mut px, &mut py, &mut pz);
        self.pos = [px, py, pz];
        y += row_h + 6.0;

        // Rotation
        let rot_rect = Rect::from_min_size(Pos2::new(left, y), vec2(content_w, row_h));
        let (mut rx, mut ry, mut rz) = (self.rot[0], self.rot[1], self.rot[2]);
        gui.vec3_input(rot_rect, "旋转", &mut rx, &mut ry, &mut rz);
        self.rot = [rx, ry, rz];
        y += row_h + 6.0;

        // Scale
        let scale_rect = Rect::from_min_size(Pos2::new(left, y), vec2(content_w, row_h));
        let (mut sx, mut sy, mut sz) = (self.scale[0], self.scale[1], self.scale[2]);
        gui.vec3_input(scale_rect, "缩放", &mut sx, &mut sy, &mut sz);
        self.scale = [sx, sy, sz];
        y += 16.0;

        // ── Render section ──
        section_header(&painter, "渲染", &mut y);

        let mat_rect = Rect::from_min_size(Pos2::new(left, y), vec2(content_w, row_h));
        gui.input_labeled(mat_rect, "材质", &self.material_name);
        y += row_h + 6.0;

        let mesh_rect = Rect::from_min_size(Pos2::new(left, y), vec2(content_w, row_h));
        gui.input_labeled(mesh_rect, "网格", &self.mesh_name);
        y += row_h + 6.0;

        let shadow_rect = Rect::from_min_size(Pos2::new(left, y), vec2(content_w, row_h));
        gui.checkbox(shadow_rect, "投射阴影", &mut self.cast_shadow);
        y += 16.0;

        // ── Physics section ──
        section_header(&painter, "物理", &mut y);

        let rigid_rect = Rect::from_min_size(Pos2::new(left, y), vec2(content_w, row_h));
        gui.input_labeled(rigid_rect, "刚体", "Static");
        y += row_h + 6.0;

        let collide_rect = Rect::from_min_size(Pos2::new(left, y), vec2(content_w, row_h));
        gui.input_labeled(collide_rect, "碰撞", "Box");
    }

    // ── bottom panel ───────────────────────────────────────────────
    fn draw_bottom_panel(&mut self, ui: &egui::Ui, rect: Rect, _skin: &GuiSkin) {
        let painter = ui.painter_at(rect);
        painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));
        painter.add(Shape::line(
            vec![Pos2::new(rect.left(), rect.top()), Pos2::new(rect.right(), rect.top())],
            Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
        ));

        // Tab bar with proper styling
        let tab_bar_rect = Rect::from_min_size(rect.left_top(), vec2(rect.width(), 32.0));
        let tabs = &["日志", "性能", "音频", "网络"];
        let mut tx = rect.left() + 8.0;
        for (i, label) in tabs.iter().enumerate() {
            let text_w = label.len() as f32 * 8.0;
            let tab_rect = Rect::from_min_size(Pos2::new(tx, rect.top()), vec2(text_w + 28.0, 32.0));
            let id = egui::Id::new("btm_tab").with(i as u64);
            let response = ui.interact(tab_rect, id, egui::Sense::click());
            if self.active_bottom_tab == i {
                let line_rect = Rect::from_min_size(Pos2::new(tab_rect.left(), tab_rect.bottom() - 2.0), vec2(tab_rect.width(), 2.0));
                painter.add(Shape::rect_filled(line_rect, Rounding::ZERO, Color32::from_rgb(0, 212, 170)));
                painter.text(tab_rect.center(), egui::Align2::CENTER_CENTER, *label, FontId::proportional(12.0), Color32::from_rgb(0, 212, 170));
            } else {
                painter.text(tab_rect.center(), egui::Align2::CENTER_CENTER, *label, FontId::proportional(12.0), Color32::from_gray(90));
            }
            if response.clicked() {
                self.active_bottom_tab = i;
            }
            tx += text_w + 28.0;
        }

        // Content area
        let content_rect = Rect::from_min_size(
            Pos2::new(rect.left() + 12.0, tab_bar_rect.bottom()),
            vec2(rect.width() - 24.0, rect.bottom() - tab_bar_rect.bottom()),
        );

        match self.active_bottom_tab {
            0 => {
                let logs = [
                    ("10:23:15", "info", "编辑器已启动"),
                    ("10:23:16", "info", "项目已加载: MyGame"),
                    ("10:23:18", "info", "着色器编译完成 (12个)"),
                    ("10:23:20", "warn", "缺少法线贴图: Materials/Wood"),
                    ("10:23:22", "info", "场景保存成功"),
                ];
                let mut y = content_rect.top() + 8.0;
                for (time, level, msg) in &logs {
                    let time_color = Color32::from_gray(90);
                    let level_color = match *level {
                        "info" => Color32::from_gray(152),
                        "warn" => Color32::from_rgb(255, 184, 0),
                        _ => Color32::from_rgb(255, 71, 87),
                    };
                    // Time
                    painter.text(egui::pos2(content_rect.left(), y), egui::Align2::LEFT_CENTER, *time, FontId::proportional(11.0), time_color);
                    // Level
                    painter.text(egui::pos2(content_rect.left() + 60.0, y), egui::Align2::LEFT_CENTER, *level, FontId::proportional(11.0), level_color);
                    // Message
                    painter.text(egui::pos2(content_rect.left() + 110.0, y), egui::Align2::LEFT_CENTER, *msg, FontId::proportional(11.0), Color32::from_rgb(232, 232, 236));
                    y += 18.0;
                }
            }
            1 => {
                let perf_data = [
                    "Draw Calls: 128",
                    "Triangles: 45.2K",
                    "Vertices: 22.8K",
                    "GPU: 32ms",
                    "Memory: 256MB / 2GB",
                ];
                let mut y = content_rect.top() + 8.0;
                for line in &perf_data {
                    painter.text(egui::pos2(content_rect.left(), y), egui::Align2::LEFT_CENTER, *line, FontId::proportional(11.0), Color32::from_rgb(232, 232, 236));
                    y += 18.0;
                }
            }
            _ => {
                painter.text(content_rect.center(), egui::Align2::CENTER_CENTER, "-- 面板内容 --", FontId::proportional(11.0), Color32::from_gray(90));
            }
        }
    }

    // ── status bar ──────────────────────────────────────────────────
    fn draw_status_bar(&mut self, gui: &mut Gui, rect: Rect) {
        let painter = gui.ui.painter_at(rect);
        painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(30, 30, 34)));
        painter.add(Shape::line(
            vec![Pos2::new(rect.left(), rect.top()), Pos2::new(rect.right(), rect.top())],
            Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
        ));

        // Status dot + "就绪"
        gui.status_item(
            Rect::from_min_size(Pos2::new(rect.left() + 12.0, rect.top()), vec2(60.0, rect.height())),
            "就绪",
            Color32::from_rgb(46, 213, 115),
        );

        // Object count
        painter.text(
            egui::pos2(rect.left() + 80.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            format!("对象: {}", self.scene_tree.iter().map(|n| 1 + n.children.len()).sum::<usize>()),
            FontId::proportional(11.0),
            Color32::from_gray(90),
        );

        // Triangle count
        painter.text(
            egui::pos2(rect.left() + 160.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            "三角形: 45K",
            FontId::proportional(11.0),
            Color32::from_gray(90),
        );

        // Right side: view mode + view name
        let view_modes = ["场景", "游戏", "物理"];
        let view_names = ["perspective", "top", "front", "right"];
        let view_mode = view_modes.get(self.active_viewport).unwrap_or(&"场景");
        let view_name = view_names.first().unwrap_or(&"perspective");
        painter.text(
            egui::pos2(rect.right() - 12.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            format!("{} 视图  |  {}", view_mode, view_name),
            FontId::proportional(11.0),
            Color32::from_gray(90),
        );
    }
}
