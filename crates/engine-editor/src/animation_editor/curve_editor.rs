use super::AnimationEditorState;
use egui::{Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use engine_scene::keyframe::Interpolation;

const BG_COLOR: Color32 = Color32::from_rgb(25, 25, 28);
const GRID_COLOR: Color32 = Color32::from_rgb(40, 40, 48);
const GRID_MAJOR: Color32 = Color32::from_rgb(50, 50, 58);
const TANGENT_COLOR: Color32 = Color32::from_rgb(180, 180, 100);

pub fn draw_curve_editor(state: &mut AnimationEditorState, ui: &egui::Ui, rect: Rect) {
    let painter = ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, BG_COLOR));

    let Some(ref _clip) = state.clip else {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "无动画片段",
            egui::FontId::proportional(12.0),
            Color32::from_gray(80),
        );
        return;
    };

    let curve = match state.curves.get(state.selected_track) {
        Some(c) => c,
        None => return,
    };

    let zoom_x = state.curve_zoom_x;
    let zoom_y = state.curve_zoom_y;
    let center_x = rect.center().x - state.curve_offset_x;
    let center_y = rect.center().y + state.curve_offset_y;

    draw_grid(&painter, rect, zoom_x, zoom_y, center_x, center_y);

    // Zero line
    let zero_y = center_y;
    if zero_y >= rect.top() && zero_y <= rect.bottom() {
        painter.add(Shape::line(
            vec![
                Pos2::new(rect.left(), zero_y),
                Pos2::new(rect.right(), zero_y),
            ],
            Stroke::new(0.5_f32, Color32::from_rgb(60, 60, 70)),
        ));
    }

    // Curve line
    if curve.keyframes.len() >= 2 {
        let color = curve.track_type.color();
        let num_samples = ((rect.width() / 2.0) as usize).max(50);

        let mut points = Vec::with_capacity(num_samples + 1);
        for i in 0..=num_samples {
            let t = i as f32 / num_samples as f32;
            let time = curve.keyframes.first().unwrap().time
                + t * (curve.keyframes.last().unwrap().time
                    - curve.keyframes.first().unwrap().time);
            let value = curve.sample(time);
            let screen_x = center_x + (time * zoom_x);
            let screen_y = center_y - (value * zoom_y);
            points.push(Pos2::new(screen_x, screen_y));
        }

        for w in points.windows(2) {
            painter.add(Shape::line(vec![w[0], w[1]], Stroke::new(2.0_f32, color)));
        }
    }

    // Keyframe points and tangent handles
    for (kf_idx, kf) in curve.keyframes.iter().enumerate() {
        let screen_x = center_x + (kf.time * zoom_x);
        let screen_y = center_y - (kf.value * zoom_y);
        let pos = Pos2::new(screen_x, screen_y);
        let is_selected = state.selected_keyframe == Some(kf_idx);

        // Tangent handles for cubic
        if kf.interpolation == Interpolation::Cubic {
            let in_x = screen_x - 30.0;
            let in_y = screen_y + kf.tangent_in * zoom_y * 0.3;
            let in_pos = Pos2::new(in_x, in_y);
            painter.add(Shape::line(
                vec![pos, in_pos],
                Stroke::new(1.0_f32, TANGENT_COLOR),
            ));
            painter.add(Shape::circle_filled(in_pos, 3.0, TANGENT_COLOR));

            let out_x = screen_x + 30.0;
            let out_y = screen_y - kf.tangent_out * zoom_y * 0.3;
            let out_pos = Pos2::new(out_x, out_y);
            painter.add(Shape::line(
                vec![pos, out_pos],
                Stroke::new(1.0_f32, TANGENT_COLOR),
            ));
            painter.add(Shape::circle_filled(out_pos, 3.0, TANGENT_COLOR));

            let in_rect = Rect::from_center_size(in_pos, Vec2::new(12.0, 12.0));
            let id = egui::Id::new("tangent_in").with(kf_idx);
            let resp = ui.interact(in_rect, id, egui::Sense::drag());
            if resp.drag_started() {
                state.dragging_tangent = Some((state.selected_track, kf_idx, false));
            }

            let out_rect = Rect::from_center_size(out_pos, Vec2::new(12.0, 12.0));
            let id = egui::Id::new("tangent_out").with(kf_idx);
            let resp = ui.interact(out_rect, id, egui::Sense::drag());
            if resp.drag_started() {
                state.dragging_tangent = Some((state.selected_track, kf_idx, true));
            }
        }

        // Keyframe point
        let point_color = if is_selected {
            Color32::WHITE
        } else {
            curve.track_type.color()
        };
        let radius = if is_selected { 5.0 } else { 4.0 };
        painter.add(Shape::circle_filled(pos, radius, point_color));
        if is_selected {
            painter.add(Shape::circle_stroke(
                pos,
                radius + 2.0,
                Stroke::new(1.5_f32, Color32::WHITE),
            ));
        }

        let kf_rect = Rect::from_center_size(pos, Vec2::new(14.0, 14.0));
        let id = egui::Id::new("curve_kf").with(kf_idx);
        let resp = ui.interact(kf_rect, id, egui::Sense::click());
        if resp.clicked() {
            state.selected_keyframe = Some(kf_idx);
        }
    }

    // Tangent dragging
    if let Some((track_idx, kf_idx, is_out)) = state.dragging_tangent {
        if ui.input(|i| i.pointer.any_released()) {
            state.dragging_tangent = None;
        } else {
            let delta = ui.input(|i| i.pointer.delta());
            if delta.y.abs() > 0.01
                && let Some(curve) = state.curves.get_mut(track_idx)
                && let Some(kf) = curve.keyframes.get_mut(kf_idx)
            {
                let tangent_delta = -delta.y / (zoom_y * 0.3);
                if is_out {
                    kf.tangent_out += tangent_delta;
                } else {
                    kf.tangent_in += tangent_delta;
                }
            }
        }
    }

    // Zoom
    let id = egui::Id::new("curve_area");
    let resp = ui.interact(rect, id, egui::Sense::click_and_drag());
    if resp.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta);
        if scroll.y.abs() > 0.1 {
            let factor = 1.0 + scroll.y * 0.005;
            state.curve_zoom_x = (state.curve_zoom_x * factor).clamp(20.0, 2000.0);
            state.curve_zoom_y = (state.curve_zoom_y * factor).clamp(10.0, 1000.0);
        }
    }

    painter.add(Shape::rect_stroke(
        rect,
        Rounding::ZERO,
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));
}

fn draw_grid(
    painter: &egui::Painter,
    rect: Rect,
    zoom_x: f32,
    zoom_y: f32,
    center_x: f32,
    center_y: f32,
) {
    let grid_spacing_x = compute_grid_spacing(zoom_x);
    let start_x = ((rect.left() - center_x) / (zoom_x * grid_spacing_x)).floor() as i32;
    let end_x = ((rect.right() - center_x) / (zoom_x * grid_spacing_x)).ceil() as i32;

    for i in start_x..=end_x {
        let x = center_x + (i as f32 * grid_spacing_x * zoom_x);
        if x >= rect.left() && x <= rect.right() {
            let is_major = i % 5 == 0;
            painter.add(Shape::line(
                vec![Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(0.5_f32, if is_major { GRID_MAJOR } else { GRID_COLOR }),
            ));
        }
    }

    let grid_spacing_y = compute_grid_spacing(zoom_y);
    let start_y = ((rect.top() - center_y) / (zoom_y * grid_spacing_y)).floor() as i32;
    let end_y = ((rect.bottom() - center_y) / (zoom_y * grid_spacing_y)).ceil() as i32;

    for i in start_y..=end_y {
        let y = center_y + (i as f32 * grid_spacing_y * zoom_y);
        if y >= rect.top() && y <= rect.bottom() {
            let is_major = i % 5 == 0;
            painter.add(Shape::line(
                vec![Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                Stroke::new(0.5_f32, if is_major { GRID_MAJOR } else { GRID_COLOR }),
            ));
        }
    }
}

fn compute_grid_spacing(zoom: f32) -> f32 {
    let target_px = 80.0;
    let raw = target_px / zoom;
    let magnitude = 10_f32.powf(raw.log10().floor());
    let normalized = raw / magnitude;

    let step = if normalized < 1.5 {
        1.0
    } else if normalized < 3.5 {
        2.0
    } else if normalized < 7.5 {
        5.0
    } else {
        10.0
    };

    step * magnitude
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_grid_spacing() {
        let spacing = compute_grid_spacing(100.0);
        assert!(spacing > 0.0);
        assert!(spacing < 10.0);
    }

    #[test]
    fn test_compute_grid_spacing_zoom_in() {
        let spacing = compute_grid_spacing(1000.0);
        assert!(spacing > 0.0);
    }

    #[test]
    fn test_compute_grid_spacing_zoom_out() {
        let spacing = compute_grid_spacing(10.0);
        assert!(spacing > 0.0);
    }
}
