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

#[allow(dead_code)]
impl EditorLayout {
    fn draw_menu_bar(&mut self, _gui: &mut Gui, _rect: Rect) {}
    fn draw_toolbar(&mut self, _gui: &mut Gui, _rect: Rect) {}
    fn draw_hierarchy(&mut self, _gui: &mut Gui, _rect: Rect) {}
    fn draw_viewport(&mut self, _gui: &mut Gui, _rect: Rect) {}
    fn draw_inspector(&mut self, _gui: &mut Gui, _rect: Rect) {}
    fn draw_bottom_panel(&mut self, _ui: &egui::Ui, _rect: Rect, _skin: &GuiSkin) {}
    fn draw_status_bar(&mut self, _gui: &mut Gui, _rect: Rect) {}
}
