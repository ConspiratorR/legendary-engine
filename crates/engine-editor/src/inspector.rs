use egui::{Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use engine_ui::Gui;
use crate::state::EditorState;

pub trait ComponentEditor: Send + Sync {
    fn name(&self) -> &'static str;
    fn draw(&mut self, gui: &mut Gui, rect: Rect, state: &mut EditorState) -> f32;
    fn clone_box(&self) -> Box<dyn ComponentEditor>;
}

pub struct ComponentRegistry {
    pub editors: Vec<Box<dyn ComponentEditor>>,
    pub available: Vec<(&'static str, Box<dyn ComponentEditor>)>,
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

    pub fn draw_for_entity(&mut self, gui: &mut Gui, rect: Rect, state: &mut EditorState) {
        let mut y = rect.top();
        for editor in &mut self.editors {
            let section_rect = Rect::from_min_size(
                Pos2::new(rect.left(), y),
                Vec2::new(rect.width(), 200.0),
            );
            let h = editor.draw(gui, section_rect, state);
            y = h + 8.0;
        }
    }
}

#[derive(Default, Clone)]
pub struct TransformEditor {
    pub translation: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
}

impl ComponentEditor for TransformEditor {
    fn name(&self) -> &'static str { "变换" }

    fn draw(&mut self, gui: &mut Gui, rect: Rect, _state: &mut EditorState) -> f32 {
        let painter = gui.ui.painter_at(rect);
        let label_font = egui::FontId::proportional(11.0);
        let row_h = 26.0;

        painter.text(egui::pos2(rect.left(), rect.top()), egui::Align2::LEFT_CENTER,
            "变换", label_font, Color32::from_gray(90));
        let sep_y = rect.top() + 18.0;
        painter.add(Shape::line(
            vec![Pos2::new(rect.left() + 30.0, sep_y), Pos2::new(rect.right(), sep_y)],
            Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
        ));
        let mut y = sep_y + 12.0;

        let pos_rect = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), row_h));
        let (mut px, mut py, mut pz) = (self.translation[0], self.translation[1], self.translation[2]);
        gui.vec3_input(pos_rect, "位置", &mut px, &mut py, &mut pz);
        self.translation = [px, py, pz];
        y += row_h + 6.0;

        let rot_rect = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), row_h));
        let (mut rx, mut ry, mut rz) = (self.rotation[0], self.rotation[1], self.rotation[2]);
        gui.vec3_input(rot_rect, "旋转", &mut rx, &mut ry, &mut rz);
        self.rotation = [rx, ry, rz];
        y += row_h + 6.0;

        let scale_rect = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), row_h));
        let (mut sx, mut sy, mut sz) = (self.scale[0], self.scale[1], self.scale[2]);
        gui.vec3_input(scale_rect, "缩放", &mut sx, &mut sy, &mut sz);
        self.scale = [sx, sy, sz];

        y + 16.0
    }

    fn clone_box(&self) -> Box<dyn ComponentEditor> {
        Box::new(self.clone())
    }
}

#[derive(Default, Clone)]
pub struct RenderEditor {
    pub material: String,
    pub mesh: String,
    pub cast_shadow: bool,
}

impl ComponentEditor for RenderEditor {
    fn name(&self) -> &'static str { "渲染" }

    fn draw(&mut self, gui: &mut Gui, rect: Rect, _state: &mut EditorState) -> f32 {
        let painter = gui.ui.painter_at(rect);
        let label_font = egui::FontId::proportional(11.0);
        let row_h = 26.0;

        painter.text(egui::pos2(rect.left(), rect.top()), egui::Align2::LEFT_CENTER,
            "渲染", label_font, Color32::from_gray(90));
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

        y + 16.0
    }

    fn clone_box(&self) -> Box<dyn ComponentEditor> {
        Box::new(self.clone())
    }
}

#[derive(Default, Clone)]
pub struct PhysicsEditor {
    pub body_type: String,
    pub collision_shape: String,
}

impl ComponentEditor for PhysicsEditor {
    fn name(&self) -> &'static str { "物理" }

    fn draw(&mut self, gui: &mut Gui, rect: Rect, _state: &mut EditorState) -> f32 {
        let painter = gui.ui.painter_at(rect);
        let label_font = egui::FontId::proportional(11.0);
        let row_h = 26.0;

        painter.text(egui::pos2(rect.left(), rect.top()), egui::Align2::LEFT_CENTER,
            "物理", label_font, Color32::from_gray(90));
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

        y + 16.0
    }

    fn clone_box(&self) -> Box<dyn ComponentEditor> {
        Box::new(self.clone())
    }
}

pub fn draw(state: &mut EditorState, gui: &mut Gui, rect: Rect) {
    let h_scale = gui.ui.ctx().screen_rect().height() / 1080.0;
    let w_scale = gui.ui.ctx().screen_rect().width() / 1920.0;

    let painter = gui.ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));
    painter.add(Shape::line(
        vec![Pos2::new(rect.left(), rect.top()), Pos2::new(rect.left(), rect.bottom())],
        Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
    ));

    let search_h = 36.0 * h_scale;
    let search_round = 6.0 * h_scale;
    let pad8 = 8.0 * w_scale;
    painter.add(Shape::rect_filled(
        Rect::from_min_size(Pos2::new(rect.left() + pad8, rect.top() + 8.0 * h_scale),
            Vec2::new(rect.width() - pad8 * 2.0, search_h)),
        Rounding::same(search_round),
        Color32::from_rgb(30, 30, 34),
    ));
    painter.text(
        egui::pos2(rect.left() + 20.0 * w_scale, rect.top() + (8.0 * h_scale + search_h / 2.0)),
        egui::Align2::LEFT_CENTER,
        "🔍 搜索属性...",
        egui::FontId::proportional(12.0 * h_scale),
        Color32::from_gray(90),
    );

    let content_top = rect.top() + (8.0 * h_scale + search_h + 8.0 * h_scale);
    let content_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 12.0 * w_scale, content_top),
        Vec2::new(rect.width() - 24.0 * w_scale, rect.bottom() - content_top),
    );

    // Use a thread-local or lazy registry for now
    draw_inspector_components(gui, content_rect, state);
}

fn draw_inspector_components(gui: &mut Gui, rect: Rect, _state: &mut EditorState) {
    let mut transform = TransformEditor::default();
    let mut render = RenderEditor { material: "Default".into(), mesh: "Cube".into(), cast_shadow: true };
    let mut physics = PhysicsEditor { body_type: "Static".into(), collision_shape: "Box".into() };

    let mut y = rect.top();
    let r1 = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), 120.0));
    y = transform.draw(gui, r1, _state) + 8.0;

    let r2 = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), 120.0));
    y = render.draw(gui, r2, _state) + 8.0;

    let r3 = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), 100.0));
    physics.draw(gui, r3, _state);
}
