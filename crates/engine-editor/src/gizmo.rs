use crate::state::{EditorState, ToolType};
use egui::{Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};

const AXIS_COLORS: [Color32; 3] = [
    Color32::from_rgb(255, 107, 107),
    Color32::from_rgb(46, 213, 115),
    Color32::from_rgb(77, 171, 247),
];

const AXIS_DIRS: [Vec2; 3] = [
    Vec2::new(1.0, 0.0),
    Vec2::new(0.0, -1.0),
    Vec2::new(-0.7, 0.7),
];

pub fn draw(
    state: &mut EditorState,
    painter: &egui::Painter,
    canvas_rect: Rect,
    h_scale: f32,
    _w_scale: f32,
) {
    let gizmo_center = Pos2::new(canvas_rect.right() - 100.0, canvas_rect.top() + 80.0);
    let gizmo_size = 60.0 * h_scale;

    match state.active_tool {
        ToolType::Translate => {
            draw_translate_gizmo(painter, gizmo_center, gizmo_size);
            handle_translate_interaction(state, canvas_rect, gizmo_center, gizmo_size);
        }
        ToolType::Rotate => draw_rotate_gizmo(painter, gizmo_center, gizmo_size),
        ToolType::Scale => draw_scale_gizmo(painter, gizmo_center, gizmo_size),
        ToolType::Select => {}
    }
}

fn handle_translate_interaction(
    state: &mut EditorState,
    _canvas_rect: Rect,
    _center: Pos2,
    _size: f32,
) {
    if state.selected_nodes.is_empty() {
        return;
    }
    // Full gizmo interaction requires egui drag events routed through the viewport.
    // For now, gizmo is visual-only; transform editing works via the inspector.
    _ = state.selected_nodes[0];
}

fn draw_translate_gizmo(painter: &egui::Painter, center: Pos2, size: f32) {
    for (i, &dir) in AXIS_DIRS.iter().enumerate() {
        let tip = Pos2::new(center.x + dir.x * size, center.y + dir.y * size);
        let color = AXIS_COLORS[i];
        painter.add(Shape::line(vec![center, tip], Stroke::new(3.0, color)));
        let arrow_base = Pos2::new(
            center.x + dir.x * (size - 8.0),
            center.y + dir.y * (size - 8.0),
        );
        let perp = Vec2::new(-dir.y, dir.x);
        painter.add(Shape::convex_polygon(
            vec![tip, arrow_base + perp * 4.0, arrow_base - perp * 4.0],
            color,
            Stroke::NONE,
        ));
    }
}

fn draw_rotate_gizmo(painter: &egui::Painter, center: Pos2, size: f32) {
    for (i, &start_angle) in [0.0, 90.0_f32.to_radians(), 180.0_f32.to_radians()]
        .iter()
        .enumerate()
    {
        let color = AXIS_COLORS[i];
        let mut points = Vec::with_capacity(31);
        for a in 0..=30 {
            let angle = start_angle + a as f32 * 120.0_f32.to_radians() / 30.0;
            let p = Pos2::new(center.x + angle.cos() * size, center.y + angle.sin() * size);
            points.push(p);
        }
        painter.add(Shape::line(points, Stroke::new(2.0, color)));
    }
}

fn draw_scale_gizmo(painter: &egui::Painter, center: Pos2, size: f32) {
    for (i, &dir) in AXIS_DIRS.iter().enumerate() {
        let tip = Pos2::new(center.x + dir.x * size, center.y + dir.y * size);
        let color = AXIS_COLORS[i];
        painter.add(Shape::line(
            vec![center, tip],
            Stroke::new(
                2.0,
                Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 100),
            ),
        ));
        let cube_rect = Rect::from_center_size(tip, Vec2::new(10.0, 10.0));
        painter.add(Shape::rect_filled(cube_rect, Rounding::ZERO, color));
    }
    let center_cube = Rect::from_center_size(center, Vec2::new(10.0, 10.0));
    painter.add(Shape::rect_filled(
        center_cube,
        Rounding::same(2.0),
        Color32::WHITE,
    ));
}
