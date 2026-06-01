use super::AnimationEditorState;
use egui::{Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};

const PLAYHEAD_COLOR: Color32 = Color32::from_rgb(255, 200, 50);
const BG_COLOR: Color32 = Color32::from_rgb(28, 28, 32);
const TRACK_BG: Color32 = Color32::from_rgb(35, 35, 40);
const TEXT_COLOR: Color32 = Color32::from_gray(140);

pub fn draw_timeline_header(state: &mut AnimationEditorState, ui: &egui::Ui, rect: Rect) {
    let painter = ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, BG_COLOR));

    let Some(ref clip) = state.clip else {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "无动画片段",
            egui::FontId::proportional(12.0),
            Color32::from_gray(80),
        );
        return;
    };

    let duration = clip.duration;
    let zoom = state.timeline_zoom;
    let offset = state.timeline_offset;

    let start_time = (offset / zoom).floor() as i32;
    let end_time = ((offset + rect.width()) / zoom).ceil() as i32;

    for t in start_time..=end_time {
        if t < 0 {
            continue;
        }
        let x = rect.left() + (t as f32 * zoom) - offset;

        painter.add(Shape::line(
            vec![
                Pos2::new(x, rect.bottom() - 12.0),
                Pos2::new(x, rect.bottom()),
            ],
            Stroke::new(1.0_f32, Color32::from_rgb(80, 80, 90)),
        ));

        painter.text(
            Pos2::new(x + 3.0, rect.top() + 4.0),
            egui::Align2::LEFT_TOP,
            format!("{}", t),
            egui::FontId::proportional(10.0),
            TEXT_COLOR,
        );

        if zoom > 60.0 {
            let sub_x = x + zoom * 0.5;
            if sub_x < rect.right() {
                painter.add(Shape::line(
                    vec![
                        Pos2::new(sub_x, rect.bottom() - 6.0),
                        Pos2::new(sub_x, rect.bottom()),
                    ],
                    Stroke::new(1.0_f32, Color32::from_rgb(40, 40, 48)),
                ));
            }
        }
    }

    painter.add(Shape::line(
        vec![
            Pos2::new(rect.left(), rect.bottom() - 1.0),
            Pos2::new(rect.right(), rect.bottom() - 1.0),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    let playhead_x = rect.left() + (state.player.time * zoom) - offset;
    if playhead_x >= rect.left() && playhead_x <= rect.right() {
        let tri_size = 6.0;
        painter.add(Shape::convex_polygon(
            vec![
                Pos2::new(playhead_x, rect.bottom() - 1.0),
                Pos2::new(playhead_x - tri_size, rect.bottom() - tri_size - 1.0),
                Pos2::new(playhead_x + tri_size, rect.bottom() - tri_size - 1.0),
            ],
            PLAYHEAD_COLOR,
            Stroke::NONE,
        ));
        painter.add(Shape::line(
            vec![
                Pos2::new(playhead_x, rect.top()),
                Pos2::new(playhead_x, rect.bottom()),
            ],
            Stroke::new(1.5_f32, PLAYHEAD_COLOR),
        ));
    }

    let id = egui::Id::new("timeline_header");
    let response = ui.interact(rect, id, egui::Sense::click_and_drag());
    if (response.clicked() || response.dragged())
        && let Some(pos) = ui.input(|i| i.pointer.latest_pos())
    {
        let time = ((pos.x - rect.left() + offset) / zoom).max(0.0);
        state.player.time = time.min(duration);
        state.dragging_playhead = true;
    }
    if response.drag_stopped() {
        state.dragging_playhead = false;
    }
}

pub fn draw_track_labels(state: &AnimationEditorState, ui: &egui::Ui, rect: Rect) {
    let painter = ui.painter_at(rect);
    painter.add(Shape::rect_filled(
        rect,
        Rounding::ZERO,
        Color32::from_rgb(25, 25, 28),
    ));

    let row_h = 24.0;
    let mut y = rect.top() + 2.0;

    for (i, curve) in state.curves.iter().enumerate() {
        if y + row_h > rect.bottom() {
            break;
        }

        let is_selected = state.selected_track == i;
        let row_rect =
            Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), row_h));

        if is_selected {
            painter.add(Shape::rect_filled(
                row_rect,
                Rounding::ZERO,
                Color32::from_rgb(40, 40, 48),
            ));
        }

        let color = curve.track_type.color();
        painter.add(Shape::rect_filled(
            Rect::from_min_size(
                Pos2::new(rect.left() + 4.0, y + 6.0),
                Vec2::new(3.0, row_h - 12.0),
            ),
            Rounding::same(1.0),
            color,
        ));

        painter.text(
            Pos2::new(rect.left() + 12.0, y + row_h / 2.0),
            egui::Align2::LEFT_CENTER,
            curve.track_type.label(),
            egui::FontId::proportional(11.0),
            if is_selected {
                Color32::WHITE
            } else {
                TEXT_COLOR
            },
        );

        let count_text = format!("{}", curve.keyframes.len());
        painter.text(
            Pos2::new(rect.right() - 8.0, y + row_h / 2.0),
            egui::Align2::RIGHT_CENTER,
            count_text,
            egui::FontId::proportional(10.0),
            Color32::from_gray(70),
        );

        painter.add(Shape::line(
            vec![
                Pos2::new(rect.left(), y + row_h),
                Pos2::new(rect.right(), y + row_h),
            ],
            Stroke::new(0.5_f32, Color32::from_rgb(40, 40, 48)),
        ));

        y += row_h;
    }

    painter.add(Shape::line(
        vec![
            Pos2::new(rect.right() - 1.0, rect.top()),
            Pos2::new(rect.right() - 1.0, rect.bottom()),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));
}

pub fn draw_keyframe_tracks(state: &mut AnimationEditorState, ui: &egui::Ui, rect: Rect) {
    let painter = ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, TRACK_BG));

    let Some(ref _clip) = state.clip else {
        return;
    };

    let zoom = state.timeline_zoom;
    let offset = state.timeline_offset;
    let row_h = 24.0;
    let diamond_size = 6.0;

    let mut y = rect.top() + 2.0;

    for (track_idx, curve) in state.curves.iter().enumerate() {
        if y + row_h > rect.bottom() {
            break;
        }

        if state.selected_track == track_idx {
            painter.add(Shape::rect_filled(
                Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), row_h)),
                Rounding::ZERO,
                Color32::from_rgb(32, 32, 38),
            ));
        }

        painter.add(Shape::line(
            vec![
                Pos2::new(rect.left(), y + row_h),
                Pos2::new(rect.right(), y + row_h),
            ],
            Stroke::new(0.5_f32, Color32::from_rgb(40, 40, 48)),
        ));

        for (kf_idx, kf) in curve.keyframes.iter().enumerate() {
            let x = rect.left() + (kf.time * zoom) - offset;
            if x < rect.left() - diamond_size || x > rect.right() + diamond_size {
                continue;
            }

            let cy = y + row_h / 2.0;
            let is_selected =
                state.selected_track == track_idx && state.selected_keyframe == Some(kf_idx);

            let color = if is_selected {
                Color32::WHITE
            } else {
                curve.track_type.color()
            };

            let stroke = if is_selected {
                Stroke::new(2.0_f32, Color32::from_rgb(255, 255, 255))
            } else {
                Stroke::NONE
            };

            painter.add(Shape::convex_polygon(
                vec![
                    Pos2::new(x, cy - diamond_size),
                    Pos2::new(x + diamond_size, cy),
                    Pos2::new(x, cy + diamond_size),
                    Pos2::new(x - diamond_size, cy),
                ],
                color,
                stroke,
            ));

            let kf_rect =
                Rect::from_center_size(Pos2::new(x, cy), Vec2::new(diamond_size * 2.5, row_h));
            let id = egui::Id::new("kf").with(track_idx).with(kf_idx);
            let response = ui.interact(kf_rect, id, egui::Sense::click_and_drag());

            if response.clicked() {
                state.selected_track = track_idx;
                state.selected_keyframe = Some(kf_idx);
            }

            if response.drag_started() {
                state.dragging_keyframe = Some((track_idx, kf_idx));
            }
        }

        y += row_h;
    }

    // Handle keyframe dragging
    if let Some((track_idx, kf_idx)) = state.dragging_keyframe {
        if ui.input(|i| i.pointer.any_released()) {
            state.dragging_keyframe = None;
        } else {
            let delta = ui.input(|i| i.pointer.delta());
            if (delta.x.abs() > 0.01 || delta.y.abs() > 0.01)
                && let Some(curve) = state.curves.get_mut(track_idx)
                && let Some(kf) = curve.keyframes.get_mut(kf_idx)
            {
                kf.time += delta.x / zoom;
                kf.time = kf.time.max(0.0);
                if state.snap_to_frame {
                    let frame_time = 1.0 / state.fps;
                    kf.time = (kf.time / frame_time).round() * frame_time;
                }
                curve.sort_keyframes();
            }
        }
    }

    // Playhead overlay
    let playhead_x = rect.left() + (state.player.time * zoom) - offset;
    if playhead_x >= rect.left() && playhead_x <= rect.right() {
        painter.add(Shape::line(
            vec![
                Pos2::new(playhead_x, rect.top()),
                Pos2::new(playhead_x, rect.bottom()),
            ],
            Stroke::new(1.5_f32, PLAYHEAD_COLOR),
        ));
    }

    // Scroll to zoom
    let id = egui::Id::new("timeline_tracks");
    let resp = ui.interact(rect, id, egui::Sense::click_and_drag());
    if resp.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll.abs() > 0.1 {
            state.timeline_zoom =
                (state.timeline_zoom * (1.0 + scroll * 0.005)).clamp(20.0, 1000.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playhead_no_clip() {
        let state = AnimationEditorState::new();
        assert!(state.clip.is_none());
    }
}
