use crate::state::EditorState;
use egui::{Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use engine_ui::Gui;

pub fn draw(state: &mut EditorState, gui: &mut Gui, rect: Rect) {
    let h_scale = gui.ui.ctx().screen_rect().height() / 1080.0;
    let w_scale = gui.ui.ctx().screen_rect().width() / 1920.0;

    let painter = gui.ui.painter_at(rect);
    painter.add(Shape::rect_filled(
        rect,
        Rounding::ZERO,
        Color32::from_rgb(22, 22, 25),
    ));
    painter.add(Shape::line(
        vec![
            Pos2::new(rect.left(), rect.top()),
            Pos2::new(rect.left(), rect.bottom()),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    let search_h = 36.0 * h_scale;
    let pad8 = 8.0 * w_scale;
    painter.add(Shape::rect_filled(
        Rect::from_min_size(
            Pos2::new(rect.left() + pad8, rect.top() + 8.0 * h_scale),
            Vec2::new(rect.width() - pad8 * 2.0, search_h),
        ),
        Rounding::same(6.0 * h_scale),
        Color32::from_rgb(30, 30, 34),
    ));
    painter.text(
        egui::pos2(
            rect.left() + 20.0 * w_scale,
            rect.top() + (8.0 * h_scale + search_h / 2.0),
        ),
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

    let selected_id = state.selected_nodes.first().copied();

    if let Some(id) = selected_id {
        let name = state
            .scene_tree
            .nodes
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.name.clone())
            .unwrap_or_else(|| "—".into());
        draw_transform_section(gui, content_rect, state, id, &name);
    } else {
        let painter = gui.ui.painter_at(content_rect);
        painter.text(
            content_rect.center(),
            egui::Align2::CENTER_CENTER,
            "未选中对象",
            egui::FontId::proportional(12.0),
            Color32::from_gray(90),
        );
    }
}

fn draw_transform_section(gui: &mut Gui, rect: Rect, state: &mut EditorState, id: u64, name: &str) {
    let painter = gui.ui.painter_at(rect);
    let label_font = egui::FontId::proportional(11.0);
    let row_h = 26.0;
    let x = rect.left();
    let w = rect.width();

    painter.text(
        egui::pos2(x, rect.top()),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(13.0),
        Color32::WHITE,
    );
    let mut y = rect.top() + 24.0;

    if let Some(t) = state.node_transforms.get_mut(&id) {
        let sep_y = y;
        painter.add(Shape::line(
            vec![Pos2::new(x, sep_y), Pos2::new(x + w, sep_y)],
            Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
        ));
        y += 8.0;

        painter.text(
            egui::pos2(x, y),
            egui::Align2::LEFT_CENTER,
            "变换",
            label_font.clone(),
            Color32::from_gray(90),
        );
        y += 18.0;

        let pr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        let (mut px, mut py, mut pz) = (t[0], t[1], t[2]);
        gui.vec3_input(pr, "位置", &mut px, &mut py, &mut pz);
        t[0] = px;
        t[1] = py;
        t[2] = pz;
        y += row_h + 6.0;

        let rr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        let (mut rx, mut ry, mut rz) = (t[3], t[4], t[5]);
        gui.vec3_input(rr, "旋转", &mut rx, &mut ry, &mut rz);
        t[3] = rx;
        t[4] = ry;
        t[5] = rz;
        y += row_h + 6.0;

        let sr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        let (mut sx, mut sy, mut sz) = (t[6], t[7], t[8]);
        gui.vec3_input(sr, "缩放", &mut sx, &mut sy, &mut sz);
        t[6] = sx;
        t[7] = sy;
        t[8] = sz;
        y += 16.0;
    }

    let sep_y = y;
    painter.add(Shape::line(
        vec![Pos2::new(x, sep_y), Pos2::new(x + w, sep_y)],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));
    y += 8.0;

    painter.text(
        egui::pos2(x, y),
        egui::Align2::LEFT_CENTER,
        "渲染",
        label_font.clone(),
        Color32::from_gray(90),
    );
    y += 18.0;

    if let Some((mat, mesh, shadow)) = state.node_render.get_mut(&id) {
        let mr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.input_labeled(mr, "材质", mat);
        y += row_h + 6.0;

        let mer = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.input_labeled(mer, "网格", mesh);
        y += row_h + 6.0;

        let sr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.checkbox(sr, "投射阴影", shadow);
        y += 16.0;
    }

    let sep_y = y;
    painter.add(Shape::line(
        vec![Pos2::new(x, sep_y), Pos2::new(x + w, sep_y)],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));
    y += 8.0;

    painter.text(
        egui::pos2(x, y),
        egui::Align2::LEFT_CENTER,
        "物理",
        label_font.clone(),
        Color32::from_gray(90),
    );
    y += 18.0;

    if let Some((body, col)) = state.node_physics.get_mut(&id) {
        let br = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.input_labeled(br, "刚体", body);
        y += row_h + 6.0;

        let cr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.input_labeled(cr, "碰撞", col);
    }
}
