use super::AnimationEditorState;
use egui::{Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use engine_scene::keyframe::Interpolation;

const BG_COLOR: Color32 = Color32::from_rgb(28, 28, 32);
const ROW_H: f32 = 22.0;

pub fn draw_keyframe_list(state: &mut AnimationEditorState, ui: &egui::Ui, rect: Rect) {
    let painter = ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, BG_COLOR));

    // Header
    let header_h = 28.0;
    let header_rect = Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), header_h));
    painter.add(Shape::rect_filled(
        header_rect,
        Rounding::ZERO,
        Color32::from_rgb(22, 22, 25),
    ));

    painter.text(
        Pos2::new(rect.left() + 8.0, header_rect.center().y),
        egui::Align2::LEFT_CENTER,
        "关键帧",
        egui::FontId::proportional(11.0),
        Color32::from_gray(150),
    );

    // Add/Delete buttons
    let btn_size = 18.0;
    let add_btn_rect = Rect::from_min_size(
        Pos2::new(rect.right() - 48.0, header_rect.center().y - btn_size / 2.0),
        Vec2::new(btn_size, btn_size),
    );
    let del_btn_rect = Rect::from_min_size(
        Pos2::new(rect.right() - 26.0, header_rect.center().y - btn_size / 2.0),
        Vec2::new(btn_size, btn_size),
    );

    let add_id = egui::Id::new("kf_add_btn");
    let add_resp = ui.interact(add_btn_rect, add_id, egui::Sense::click());
    painter.add(Shape::rect_filled(
        add_btn_rect,
        Rounding::same(3.0),
        Color32::from_rgb(50, 50, 58),
    ));
    painter.text(
        add_btn_rect.center(),
        egui::Align2::CENTER_CENTER,
        "+",
        egui::FontId::proportional(14.0),
        Color32::from_rgb(100, 200, 100),
    );
    if add_resp.clicked() {
        add_keyframe_at_playhead(state);
    }

    let del_id = egui::Id::new("kf_del_btn");
    let del_resp = ui.interact(del_btn_rect, del_id, egui::Sense::click());
    painter.add(Shape::rect_filled(
        del_btn_rect,
        Rounding::same(3.0),
        Color32::from_rgb(50, 50, 58),
    ));
    painter.text(
        del_btn_rect.center(),
        egui::Align2::CENTER_CENTER,
        "-",
        egui::FontId::proportional(14.0),
        Color32::from_rgb(200, 100, 100),
    );
    if del_resp.clicked() {
        delete_selected_keyframe(state);
    }

    // Header separator
    painter.add(Shape::line(
        vec![
            Pos2::new(rect.left(), header_h),
            Pos2::new(rect.right(), header_h),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    let content_rect = Rect::from_min_size(
        Pos2::new(rect.left(), rect.top() + header_h),
        Vec2::new(rect.width(), rect.height() - header_h),
    );

    let Some(ref _clip) = state.clip else {
        painter.text(
            content_rect.center(),
            egui::Align2::CENTER_CENTER,
            "无关键帧",
            egui::FontId::proportional(11.0),
            Color32::from_gray(70),
        );
        return;
    };

    let curve = match state.curves.get(state.selected_track) {
        Some(c) => c,
        None => return,
    };

    let mut y = content_rect.top() + 2.0;
    for (kf_idx, kf) in curve.keyframes.iter().enumerate() {
        if y + ROW_H > content_rect.bottom() {
            break;
        }

        let is_selected = state.selected_keyframe == Some(kf_idx);
        let row_rect = Rect::from_min_size(
            Pos2::new(content_rect.left(), y),
            Vec2::new(content_rect.width(), ROW_H),
        );

        if is_selected {
            painter.add(Shape::rect_filled(
                row_rect,
                Rounding::ZERO,
                Color32::from_rgb(0, 80, 160),
            ));
        }

        let color = curve.track_type.color();
        painter.add(Shape::rect_filled(
            Rect::from_min_size(
                Pos2::new(content_rect.left() + 2.0, y + 4.0),
                Vec2::new(3.0, ROW_H - 8.0),
            ),
            Rounding::same(1.0),
            color,
        ));

        painter.text(
            Pos2::new(content_rect.left() + 10.0, y + ROW_H / 2.0),
            egui::Align2::LEFT_CENTER,
            format!("{:.3}s", kf.time),
            egui::FontId::monospace(10.0),
            if is_selected {
                Color32::WHITE
            } else {
                Color32::from_gray(160)
            },
        );

        painter.text(
            Pos2::new(content_rect.left() + 70.0, y + ROW_H / 2.0),
            egui::Align2::LEFT_CENTER,
            format!("{:.2}", kf.value),
            egui::FontId::monospace(10.0),
            if is_selected {
                Color32::WHITE
            } else {
                Color32::from_gray(140)
            },
        );

        let interp_label = match kf.interpolation {
            Interpolation::Linear => "Lin",
            Interpolation::Step => "Stp",
            Interpolation::Cubic => "Bzr",
        };
        painter.text(
            Pos2::new(content_rect.right() - 8.0, y + ROW_H / 2.0),
            egui::Align2::RIGHT_CENTER,
            interp_label,
            egui::FontId::proportional(9.0),
            Color32::from_gray(100),
        );

        painter.add(Shape::line(
            vec![
                Pos2::new(content_rect.left(), y + ROW_H),
                Pos2::new(content_rect.right(), y + ROW_H),
            ],
            Stroke::new(0.5_f32, Color32::from_rgb(35, 35, 42)),
        ));

        let id = egui::Id::new("kf_row").with(kf_idx);
        let resp = ui.interact(row_rect, id, egui::Sense::click());
        if resp.clicked() {
            state.selected_keyframe = Some(kf_idx);
        }

        y += ROW_H;
    }
}

pub fn add_keyframe_at_playhead(state: &mut AnimationEditorState) {
    let time = state.player.time;
    let curve = match state.curves.get_mut(state.selected_track) {
        Some(c) => c,
        None => return,
    };

    let existing = curve
        .keyframes
        .iter()
        .position(|kf| (kf.time - time).abs() < 0.001);
    if existing.is_some() {
        return;
    }

    let value = curve.sample(time);

    curve.keyframes.push(super::CurveKeyframe {
        time,
        value,
        interpolation: Interpolation::Linear,
        tangent_in: 0.0,
        tangent_out: 0.0,
    });
    curve.sort_keyframes();

    state.selected_keyframe = curve
        .keyframes
        .iter()
        .position(|kf| (kf.time - time).abs() < 0.001);
}

pub fn delete_selected_keyframe(state: &mut AnimationEditorState) {
    let kf_idx = match state.selected_keyframe {
        Some(idx) => idx,
        None => return,
    };

    if let Some(curve) = state.curves.get_mut(state.selected_track)
        && kf_idx < curve.keyframes.len()
    {
        curve.keyframes.remove(kf_idx);
        state.selected_keyframe = None;
    }
}

pub fn move_selected_keyframe(state: &mut AnimationEditorState, new_time: f32) {
    let kf_idx = match state.selected_keyframe {
        Some(idx) => idx,
        None => return,
    };

    if let Some(curve) = state.curves.get_mut(state.selected_track)
        && let Some(kf) = curve.keyframes.get_mut(kf_idx)
    {
        kf.time = new_time.max(0.0);
        if state.snap_to_frame {
            let frame_time = 1.0 / state.fps;
            kf.time = (kf.time / frame_time).round() * frame_time;
        }
        curve.sort_keyframes();
    }
}

pub fn set_selected_interpolation(state: &mut AnimationEditorState, interp: Interpolation) {
    let kf_idx = match state.selected_keyframe {
        Some(idx) => idx,
        None => return,
    };

    if let Some(curve) = state.curves.get_mut(state.selected_track) {
        if kf_idx >= curve.keyframes.len() {
            return;
        }

        if interp == Interpolation::Cubic {
            // Auto-compute tangents from neighboring keyframes
            let prev_val = if kf_idx > 0 {
                curve.keyframes[kf_idx - 1].value
            } else {
                curve.keyframes[kf_idx].value
            };
            let next_val = if kf_idx < curve.keyframes.len() - 1 {
                curve.keyframes[kf_idx + 1].value
            } else {
                curve.keyframes[kf_idx].value
            };
            let tangent = (next_val - prev_val) * 0.25;
            let kf = &mut curve.keyframes[kf_idx];
            kf.interpolation = interp;
            kf.tangent_in = tangent;
            kf.tangent_out = tangent;
        } else {
            curve.keyframes[kf_idx].interpolation = interp;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_state() -> AnimationEditorState {
        let mut state = AnimationEditorState::new();
        state.create_new_clip("test".to_string(), 2.0);
        state.curves[0].keyframes.push(super::super::CurveKeyframe {
            time: 0.0,
            value: 0.0,
            interpolation: Interpolation::Linear,
            tangent_in: 0.0,
            tangent_out: 0.0,
        });
        state.curves[0].keyframes.push(super::super::CurveKeyframe {
            time: 1.0,
            value: 10.0,
            interpolation: Interpolation::Linear,
            tangent_in: 0.0,
            tangent_out: 0.0,
        });
        state
    }

    #[test]
    fn test_add_keyframe_at_playhead() {
        let mut state = make_test_state();
        state.player.time = 0.5;
        add_keyframe_at_playhead(&mut state);
        assert_eq!(state.curves[0].keyframes.len(), 3);
        assert!((state.curves[0].keyframes[1].time - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_add_keyframe_no_duplicate() {
        let mut state = make_test_state();
        state.player.time = 0.0;
        add_keyframe_at_playhead(&mut state);
        assert_eq!(state.curves[0].keyframes.len(), 2);
    }

    #[test]
    fn test_delete_selected_keyframe() {
        let mut state = make_test_state();
        state.selected_keyframe = Some(0);
        delete_selected_keyframe(&mut state);
        assert_eq!(state.curves[0].keyframes.len(), 1);
        assert!(state.selected_keyframe.is_none());
    }

    #[test]
    fn test_delete_no_selection() {
        let mut state = make_test_state();
        state.selected_keyframe = None;
        delete_selected_keyframe(&mut state);
        assert_eq!(state.curves[0].keyframes.len(), 2);
    }

    #[test]
    fn test_move_selected_keyframe() {
        let mut state = make_test_state();
        state.selected_keyframe = Some(0);
        state.snap_to_frame = false;
        move_selected_keyframe(&mut state, 0.5);
        assert!((state.curves[0].keyframes[0].time - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_move_with_snap() {
        let mut state = make_test_state();
        state.selected_keyframe = Some(0);
        state.snap_to_frame = true;
        state.fps = 30.0;
        move_selected_keyframe(&mut state, 0.51);
        let expected = (0.51_f32 * 30.0_f32).round() / 30.0_f32;
        assert!((state.curves[0].keyframes[0].time - expected).abs() < 0.001);
    }

    #[test]
    fn test_set_selected_interpolation_cubic() {
        let mut state = make_test_state();
        state.selected_keyframe = Some(0);
        set_selected_interpolation(&mut state, Interpolation::Cubic);
        assert_eq!(
            state.curves[0].keyframes[0].interpolation,
            Interpolation::Cubic
        );
    }

    #[test]
    fn test_set_selected_interpolation_step() {
        let mut state = make_test_state();
        state.selected_keyframe = Some(1);
        set_selected_interpolation(&mut state, Interpolation::Step);
        assert_eq!(
            state.curves[0].keyframes[1].interpolation,
            Interpolation::Step
        );
    }
}
