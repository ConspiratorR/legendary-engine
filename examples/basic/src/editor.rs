use egui::{vec2, Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use engine_ui::{Gui, GuiSkin};

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

impl EditorLayout {
    fn draw_menu_bar(&mut self, gui: &mut Gui, rect: Rect) {
        let items = &["文件", "编辑", "视图", "场景", "资源", "构建", "窗口", "帮助"];
        let clicked = gui.menu_bar(rect, items);
        if let Some(i) = clicked {
            self.active_menu = Some(i);
        }
    }

    fn draw_toolbar(&mut self, gui: &mut Gui, rect: Rect) {
        let painter = gui.ui.painter_at(rect);
        painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));

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

        let sep_x = group_x + 4.0 * (btn_size + 4.0) + 8.0;
        let sep_rect = Rect::from_min_size(Pos2::new(sep_x, rect.top()), vec2(1.0, rect.height()));
        gui.separator_v(sep_rect);

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

        let sep2_x = btn2_x + 2.0 * (btn_size + 4.0) + 8.0;
        let sep2_rect = Rect::from_min_size(Pos2::new(sep2_x, rect.top()), vec2(1.0, rect.height()));
        gui.separator_v(sep2_rect);

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

        let sep3_x = view_btn_x + 4.0 * (btn_size + 4.0) + 8.0;

        let play_x = sep3_x + 12.0;
        let play_btn = Rect::from_min_size(Pos2::new(play_x, rect.top() + (rect.height() - btn_size) / 2.0), vec2(btn_size, btn_size));
        gui.tool_button(play_btn, "▶", false);

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

    fn draw_hierarchy(&mut self, gui: &mut Gui, rect: Rect) {
        let painter = gui.ui.painter_at(rect);
        painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));

        let header_rect = Rect::from_min_size(rect.left_top(), vec2(rect.width(), 36.0));
        gui.panel_header(header_rect, "层级");

        let content_rect = Rect::from_min_size(
            Pos2::new(rect.left(), header_rect.bottom()),
            vec2(rect.width(), rect.bottom() - header_rect.bottom()),
        );

        let mut node_counter = 0usize;
        self.draw_tree(gui, &self.scene_tree, 0, &mut (content_rect.top() + 4.0), 24.0, content_rect.right(), &mut node_counter);
    }

    fn draw_tree(&mut self, gui: &mut Gui, nodes: &[SceneNode], depth: u32, y: &mut f32, item_h: f32, right: f32, counter: &mut usize) {
        for node in nodes.iter() {
            let node_rect = Rect::from_min_size(
                Pos2::new(0.0, *y),
                vec2(right, item_h),
            );
            let idx = *counter;
            *counter += 1;
            let selected = self.selected_node == idx;
            if gui.tree_node(node_rect, &node.name, &node.icon, selected, depth) {
                self.selected_node = idx;
            }
            *y += item_h;

            if node.expanded {
                self.draw_tree(gui, &node.children, depth + 1, y, item_h, right, counter);
            }
        }
    }

    fn draw_viewport(&mut self, gui: &mut Gui, rect: Rect) {
        let painter = gui.ui.painter_at(rect);

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

        let canvas_rect = Rect::from_min_size(
            Pos2::new(rect.left(), header_rect.bottom()),
            vec2(rect.width(), rect.bottom() - header_rect.bottom()),
        );

        painter.add(Shape::rect_filled(canvas_rect, Rounding::ZERO, Color32::from_rgb(10, 10, 12)));

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

    fn draw_bottom_panel(&mut self, ui: &egui::Ui, rect: Rect, skin: &GuiSkin) {
        let painter = ui.painter_at(rect);
        painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));

        // Tab bar
        let tab_bar_rect = Rect::from_min_size(rect.left_top(), vec2(rect.width(), 32.0));
        let tab_labels = &["日志", "性能", "音频", "网络"];
        let tab_w = 60.0;

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
                        level_color,
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
}
