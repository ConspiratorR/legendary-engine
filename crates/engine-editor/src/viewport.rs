use crate::state::EditorState;
use egui::{Color32, FontId, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use engine_math::Vec3Ext;
use engine_ui::Gui;

fn draw_viewport_header(
    state: &mut EditorState,
    gui: &mut Gui,
    rect: Rect,
    header_h: f32,
    w_scale: f32,
    h_scale: f32,
) {
    let painter = gui.ui.painter_at(Rect::from_min_size(
        rect.left_top(),
        Vec2::new(rect.width(), header_h),
    ));
    painter.add(Shape::rect_filled(
        Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), header_h)),
        Rounding::ZERO,
        Color32::from_rgb(22, 22, 25),
    ));
    painter.add(Shape::line(
        vec![
            Pos2::new(rect.left(), header_h - 1.0),
            Pos2::new(rect.right(), header_h - 1.0),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    let char_w = 8.0 * w_scale;
    let tab_pad = 12.0 * w_scale;
    let tab_font = 12.0 * h_scale;
    let tab_gap = 16.0 * w_scale;
    let mut tx = rect.left() + 12.0 * w_scale;
    let tabs = &["场景", "游戏", "物理"];
    for (i, label) in tabs.iter().enumerate() {
        let text_w = label.len() as f32 * char_w;
        let tab_rect = Rect::from_min_size(
            Pos2::new(tx, rect.top()),
            Vec2::new(text_w + tab_pad * 2.0, header_h),
        );
        let id = egui::Id::new("vp_tab").with(i as u64);
        let response = gui.ui.interact(tab_rect, id, egui::Sense::click());
        if state.active_viewport_tab == i {
            let line_rect = Rect::from_min_size(
                Pos2::new(tab_rect.left(), tab_rect.bottom() - 2.0 * h_scale),
                Vec2::new(tab_rect.width(), 2.0 * h_scale),
            );
            painter.add(Shape::rect_filled(
                line_rect,
                Rounding::ZERO,
                Color32::from_rgb(0, 212, 170),
            ));
            painter.text(
                tab_rect.center(),
                egui::Align2::CENTER_CENTER,
                *label,
                FontId::proportional(tab_font),
                Color32::from_rgb(0, 212, 170),
            );
        } else {
            painter.text(
                tab_rect.center(),
                egui::Align2::CENTER_CENTER,
                *label,
                FontId::proportional(tab_font),
                Color32::from_gray(90),
            );
        }
        if response.clicked() {
            state.active_viewport_tab = i;
        }
        tx += text_w + tab_pad * 2.0 + tab_gap;
    }

    let tool_btn = 24.0 * h_scale;
    let tool_gap = 4.0 * w_scale;
    let tool_font = 12.0 * h_scale;
    let tool_icons = &["📐", "#", "⌖"];
    let rounding = 4.0 * h_scale;
    let mut tool_x =
        rect.right() - 12.0 * w_scale - tool_icons.len() as f32 * (tool_btn + tool_gap);
    for icon in tool_icons {
        let tool_rect = Rect::from_min_size(
            Pos2::new(tool_x, rect.top() + (header_h - tool_btn) / 2.0),
            Vec2::new(tool_btn, tool_btn),
        );
        let id = egui::Id::new("vp_tool").with(tool_x as u64);
        let response = gui.ui.interact(tool_rect, id, egui::Sense::click());
        if response.hovered() {
            painter.add(Shape::rect_filled(
                tool_rect,
                Rounding::same(rounding),
                Color32::from_rgb(30, 30, 34),
            ));
        }
        painter.text(
            tool_rect.center(),
            egui::Align2::CENTER_CENTER,
            *icon,
            FontId::proportional(tool_font),
            Color32::from_gray(90),
        );
        tool_x += tool_btn + tool_gap;
    }
}

pub fn draw(state: &mut EditorState, gui: &mut Gui, rect: Rect) {
    let h_scale = gui.ui.ctx().screen_rect().height() / 1080.0;
    let w_scale = gui.ui.ctx().screen_rect().width() / 1920.0;

    let header_h = 32.0 * h_scale;
    draw_viewport_header(state, gui, rect, header_h, w_scale, h_scale);

    let canvas_rect = Rect::from_min_size(
        Pos2::new(rect.left(), rect.top() + header_h),
        Vec2::new(rect.width(), rect.height() - header_h),
    );

    let painter = gui.ui.painter_at(canvas_rect);

    let gradient_steps = 20;
    let step_h = canvas_rect.height() / gradient_steps as f32;
    for i in 0..gradient_steps {
        let t = i as f32 / (gradient_steps - 1) as f32;
        let r = (10.0 + t * 10.0) as u8;
        let g = (10.0 + t * 10.0) as u8;
        let b = (12.0 + t * 16.0) as u8;
        let strip = Rect::from_min_size(
            Pos2::new(canvas_rect.left(), canvas_rect.top() + i as f32 * step_h),
            Vec2::new(canvas_rect.width(), step_h + 1.0),
        );
        painter.add(Shape::rect_filled(
            strip,
            Rounding::ZERO,
            Color32::from_rgb(r, g, b),
        ));
    }

    if state.show_grid {
        let grid_size = 50.0 * w_scale;
        let grid_color = Color32::from_rgba_premultiplied(37, 37, 48, 128);
        let mut x = canvas_rect.left();
        while x <= canvas_rect.right() {
            painter.add(Shape::line(
                vec![
                    Pos2::new(x, canvas_rect.top()),
                    Pos2::new(x, canvas_rect.bottom()),
                ],
                Stroke::new(1.0_f32, grid_color),
            ));
            x += grid_size;
        }
        let mut y = canvas_rect.top();
        while y <= canvas_rect.bottom() {
            painter.add(Shape::line(
                vec![
                    Pos2::new(canvas_rect.left(), y),
                    Pos2::new(canvas_rect.right(), y),
                ],
                Stroke::new(1.0_f32, grid_color),
            ));
            y += grid_size;
        }
    }

    let axes = [
        ("X", Color32::from_rgb(255, 107, 107)),
        ("Y", Color32::from_rgb(46, 213, 115)),
        ("Z", Color32::from_rgb(77, 171, 247)),
    ];
    for (i, (label, color)) in axes.iter().enumerate() {
        painter.text(
            egui::pos2(
                canvas_rect.left() + 20.0 * w_scale,
                canvas_rect.top() + 20.0 * h_scale + i as f32 * 14.0 * h_scale,
            ),
            egui::Align2::LEFT_CENTER,
            *label,
            FontId::proportional(10.0 * h_scale),
            *color,
        );
    }

    draw_scene_objects(state, gui, canvas_rect, h_scale, w_scale);

    if !state.selected_nodes.is_empty() {
        crate::gizmo::draw(state, &painter, canvas_rect, h_scale, w_scale);
    }

    draw_transform_overlay(state, &painter, canvas_rect, h_scale, w_scale);

    handle_camera_input(state, gui, canvas_rect);
}

fn draw_scene_objects(
    state: &mut EditorState,
    gui: &mut Gui,
    canvas_rect: Rect,
    h_scale: f32,
    _w_scale: f32,
) {
    let painter = gui.ui.painter_at(canvas_rect);
    let aspect = canvas_rect.width() / canvas_rect.height().max(1.0);
    let view_proj = state.camera.projection_matrix(aspect) * state.camera.view_matrix();

    for node in &state.scene_tree.nodes {
        if node.parent.is_none() {
            continue;
        }
        let t = state
            .node_transforms
            .get(&node.id)
            .copied()
            .unwrap_or([0.0; 9]);
        let world_pos = engine_math::Vec3::new(t[0], t[1], t[2]);
        let clip = view_proj * world_pos.extend_with_w(1.0);
        if clip.w <= 0.0 {
            continue;
        }
        let ndc = clip.truncate() / clip.w;
        let screen_x = canvas_rect.center().x + ndc.x * canvas_rect.width() * 0.5;
        let screen_y = canvas_rect.center().y - ndc.y * canvas_rect.height() * 0.5;

        let size = 50.0 * h_scale;
        let obj_rect = Rect::from_center_size(Pos2::new(screen_x, screen_y), Vec2::new(size, size));

        let is_selected = state.selected_nodes.contains(&node.id);
        let border_color = if is_selected {
            Color32::from_rgb(255, 107, 53)
        } else {
            Color32::from_rgb(0, 212, 170)
        };

        let glow_expand = 8.0 * h_scale;
        let glow_rect = obj_rect.expand(glow_expand);
        painter.add(Shape::rect_filled(
            glow_rect,
            Rounding::same(glow_expand),
            Color32::from_rgba_premultiplied(0, 212, 170, 20),
        ));
        painter.add(Shape::rect_filled(
            obj_rect,
            Rounding::same(4.0 * h_scale),
            Color32::from_rgb(42, 42, 53),
        ));
        let inner_grad = Rect::from_min_size(
            obj_rect.left_top(),
            Vec2::new(obj_rect.width(), obj_rect.height() / 2.0),
        );
        painter.add(Shape::rect_filled(
            inner_grad,
            Rounding::same(4.0 * h_scale),
            Color32::from_rgba_premultiplied(255, 255, 255, 8),
        ));
        painter.rect_stroke(
            obj_rect,
            Rounding::same(4.0 * h_scale),
            Stroke::new(2.0_f32, border_color),
        );
        painter.text(
            obj_rect.center(),
            egui::Align2::CENTER_CENTER,
            &node.icon,
            FontId::proportional(22.0 * h_scale),
            Color32::WHITE,
        );
    }
}

fn draw_transform_overlay(
    state: &EditorState,
    painter: &egui::Painter,
    canvas_rect: Rect,
    h_scale: f32,
    w_scale: f32,
) {
    let transform_bar_h = 28.0 * h_scale;
    let transform_w = 200.0 * w_scale;
    let transform_rect = Rect::from_min_size(
        Pos2::new(
            canvas_rect.left() + 20.0 * w_scale,
            canvas_rect.bottom() - 44.0 * h_scale,
        ),
        Vec2::new(transform_w, transform_bar_h),
    );
    painter.add(Shape::rect_filled(
        transform_rect,
        Rounding::same(6.0 * h_scale),
        Color32::from_rgba_premultiplied(22, 22, 25, 230),
    ));

    let sel_trans = state
        .selected_nodes
        .first()
        .and_then(|id| state.node_transforms.get(id).copied())
        .unwrap_or([0.0; 9]);

    let transform_axes = [
        ("X", sel_trans[0] as i32, Color32::from_rgb(255, 107, 107)),
        ("Y", sel_trans[1] as i32, Color32::from_rgb(46, 213, 115)),
        ("Z", sel_trans[2] as i32, Color32::from_rgb(77, 171, 247)),
    ];
    for (i, (label, val, color)) in transform_axes.iter().enumerate() {
        painter.text(
            egui::pos2(
                transform_rect.left() + 12.0 * w_scale + i as f32 * 60.0 * w_scale,
                transform_rect.center().y,
            ),
            egui::Align2::LEFT_CENTER,
            format!("{} {}", label, val),
            FontId::proportional(11.0 * h_scale),
            *color,
        );
    }
}

fn handle_camera_input(state: &mut EditorState, gui: &mut Gui, canvas_rect: Rect) {
    let ctx = gui.ui.ctx();

    if !canvas_rect.contains(ctx.pointer_interact_pos().unwrap_or(Pos2::ZERO)) {
        return;
    }

    let canvas_id = egui::Id::new("viewport_canvas");
    let canvas_response = gui
        .ui
        .interact(canvas_rect, canvas_id, egui::Sense::click_and_drag());

    if canvas_response.dragged_by(egui::PointerButton::Secondary) {
        let delta = canvas_response.drag_delta();
        state.camera.orbit(delta.x, -delta.y);
    }

    if canvas_response.dragged_by(egui::PointerButton::Middle) {
        let delta = canvas_response.drag_delta();
        state.camera.pan(delta.x, delta.y);
    }

    let scroll = ctx.input(|i| i.raw_scroll_delta);
    if scroll.y != 0.0 {
        state.camera.zoom(scroll.y / 120.0);
    }
}
