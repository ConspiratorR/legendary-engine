//! Clip management, curve editing, timeline scrubbing.
//!
//! TODO: Migrate from direct egui to IMGUI wrapper (engine_ui::imgui)
//! Unity Reference: https://docs.unity3d.com/ScriptReference/AnimationWindow.html

pub mod curve_editor;
pub mod io;
pub mod keyframe_list;
pub mod preview;
pub mod timeline;

use engine_math::Vec3;
use engine_scene::keyframe::{
    AnimationClip, AnimationPlayer, FloatKeyframe, Interpolation, Vec3Keyframe,
};

/// Which property track is currently selected for editing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackType {
    PositionX,
    PositionY,
    PositionZ,
    RotationX,
    RotationY,
    RotationZ,
    ScaleX,
    ScaleY,
    ScaleZ,
}

impl TrackType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::PositionX => "Pos X",
            Self::PositionY => "Pos Y",
            Self::PositionZ => "Pos Z",
            Self::RotationX => "Rot X",
            Self::RotationY => "Rot Y",
            Self::RotationZ => "Rot Z",
            Self::ScaleX => "Scl X",
            Self::ScaleY => "Scl Y",
            Self::ScaleZ => "Scl Z",
        }
    }

    pub fn color(&self) -> egui::Color32 {
        match self {
            Self::PositionX | Self::RotationX | Self::ScaleX => {
                egui::Color32::from_rgb(230, 70, 70)
            }
            Self::PositionY | Self::RotationY | Self::ScaleY => {
                egui::Color32::from_rgb(70, 200, 70)
            }
            Self::PositionZ | Self::RotationZ | Self::ScaleZ => {
                egui::Color32::from_rgb(70, 100, 230)
            }
        }
    }

    pub fn all() -> &'static [TrackType] {
        &[
            Self::PositionX,
            Self::PositionY,
            Self::PositionZ,
            Self::RotationX,
            Self::RotationY,
            Self::RotationZ,
            Self::ScaleX,
            Self::ScaleY,
            Self::ScaleZ,
        ]
    }
}

/// A single float keyframe displayed in the curve editor (extracted from a Vec3 or float track).
#[derive(Debug, Clone)]
pub struct CurveKeyframe {
    pub time: f32,
    pub value: f32,
    pub interpolation: Interpolation,
    pub tangent_in: f32,
    pub tangent_out: f32,
}

impl CurveKeyframe {
    pub fn from_float(kf: &FloatKeyframe) -> Self {
        Self {
            time: kf.time,
            value: kf.value,
            interpolation: kf.interpolation,
            tangent_in: kf.tangent_in,
            tangent_out: kf.tangent_out,
        }
    }

    pub fn to_float(&self) -> FloatKeyframe {
        FloatKeyframe {
            time: self.time,
            value: self.value,
            interpolation: self.interpolation,
            tangent_in: self.tangent_in,
            tangent_out: self.tangent_out,
        }
    }
}

/// Editable curve for one component (e.g., Position.X).
#[derive(Debug, Clone)]
pub struct ComponentCurve {
    pub track_type: TrackType,
    pub keyframes: Vec<CurveKeyframe>,
}

impl ComponentCurve {
    pub fn new(track_type: TrackType) -> Self {
        Self {
            track_type,
            keyframes: Vec::new(),
        }
    }

    pub fn sort_keyframes(&mut self) {
        self.keyframes.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    pub fn sample(&self, time: f32) -> f32 {
        if self.keyframes.is_empty() {
            return 0.0;
        }
        if self.keyframes.len() == 1 {
            return self.keyframes[0].value;
        }

        let t = time.clamp(
            self.keyframes[0].time,
            self.keyframes
                .last()
                .expect("checked: keyframes is non-empty")
                .time,
        );

        let mut i = 0;
        while i < self.keyframes.len() - 1 && self.keyframes[i + 1].time <= t {
            i += 1;
        }
        if i >= self.keyframes.len() - 1 {
            return self
                .keyframes
                .last()
                .expect("checked: keyframes is non-empty")
                .value;
        }

        let k0 = &self.keyframes[i];
        let k1 = &self.keyframes[i + 1];
        let dt = k1.time - k0.time;
        if dt < 1e-6 {
            return k0.value;
        }

        let alpha = (t - k0.time) / dt;
        match k0.interpolation {
            Interpolation::Linear => k0.value + (k1.value - k0.value) * alpha,
            Interpolation::Step => k0.value,
            Interpolation::Cubic => {
                let alpha2 = alpha * alpha;
                let alpha3 = alpha2 * alpha;
                let h00 = 2.0 * alpha3 - 3.0 * alpha2 + 1.0;
                let h10 = alpha3 - 2.0 * alpha2 + alpha;
                let h01 = -2.0 * alpha3 + 3.0 * alpha2;
                let h11 = alpha3 - alpha2;
                k0.value * h00
                    + k0.tangent_out * dt * h10
                    + k1.value * h01
                    + k1.tangent_in * dt * h11
            }
        }
    }
}

/// Full state for the animation editor panel.
#[derive(Debug, Clone)]
pub struct AnimationEditorState {
    pub visible: bool,
    pub clip: Option<AnimationClip>,
    pub player: AnimationPlayer,
    pub curves: Vec<ComponentCurve>,
    pub selected_track: usize,
    pub selected_keyframe: Option<usize>,
    pub timeline_zoom: f32,
    pub timeline_offset: f32,
    pub snap_to_frame: bool,
    pub fps: f32,
    pub curve_zoom_x: f32,
    pub curve_zoom_y: f32,
    pub curve_offset_x: f32,
    pub curve_offset_y: f32,
    pub dragging_playhead: bool,
    pub dragging_keyframe: Option<(usize, usize)>,
    pub dragging_tangent: Option<(usize, usize, bool)>,
    pub preview_enabled: bool,
    pub target_entity: Option<u64>,
}

impl Default for AnimationEditorState {
    fn default() -> Self {
        Self {
            visible: false,
            clip: None,
            player: AnimationPlayer::default(),
            curves: Vec::new(),
            selected_track: 0,
            selected_keyframe: None,
            timeline_zoom: 100.0,
            timeline_offset: 0.0,
            snap_to_frame: true,
            fps: 30.0,
            curve_zoom_x: 200.0,
            curve_zoom_y: 100.0,
            curve_offset_x: 0.0,
            curve_offset_y: 0.0,
            dragging_playhead: false,
            dragging_keyframe: None,
            dragging_tangent: None,
            preview_enabled: false,
            target_entity: None,
        }
    }
}

impl AnimationEditorState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_clip(&mut self, clip: AnimationClip) {
        self.curves = build_curves_from_clip(&clip);
        self.player = AnimationPlayer::new(&clip.name);
        self.clip = Some(clip);
        self.selected_track = 0;
        self.selected_keyframe = None;
    }

    pub fn create_new_clip(&mut self, name: String, duration: f32) {
        let clip = AnimationClip::new(name, duration);
        self.load_clip(clip);
    }

    pub fn sync_clip_from_curves(&mut self) {
        if let Some(ref mut clip) = self.clip {
            // Build position track from X/Y/Z curves
            let x_curve = self
                .curves
                .iter()
                .find(|c| c.track_type == TrackType::PositionX);
            let y_curve = self
                .curves
                .iter()
                .find(|c| c.track_type == TrackType::PositionY);
            let z_curve = self
                .curves
                .iter()
                .find(|c| c.track_type == TrackType::PositionZ);
            if let (Some(xc), Some(yc), Some(zc)) = (x_curve, y_curve, z_curve) {
                let max_len = xc
                    .keyframes
                    .len()
                    .max(yc.keyframes.len())
                    .max(zc.keyframes.len());
                if max_len > 0 {
                    let mut track = Vec::new();
                    for idx in 0..max_len {
                        let xkf = xc.keyframes.get(idx);
                        let ykf = yc.keyframes.get(idx);
                        let zkf = zc.keyframes.get(idx);
                        let time = xkf.or(ykf).or(zkf).map(|k| k.time).unwrap_or(0.0);
                        let x = xkf.map(|k| k.value).unwrap_or(0.0);
                        let y = ykf.map(|k| k.value).unwrap_or(0.0);
                        let z = zkf.map(|k| k.value).unwrap_or(0.0);
                        let interp = xkf
                            .map(|k| k.interpolation)
                            .unwrap_or(Interpolation::Linear);
                        match interp {
                            Interpolation::Cubic => {
                                let tx_in = xkf.map(|k| k.tangent_in).unwrap_or(0.0);
                                let ty_in = ykf.map(|k| k.tangent_in).unwrap_or(0.0);
                                let tz_in = zkf.map(|k| k.tangent_in).unwrap_or(0.0);
                                let tx_out = xkf.map(|k| k.tangent_out).unwrap_or(0.0);
                                let ty_out = ykf.map(|k| k.tangent_out).unwrap_or(0.0);
                                let tz_out = zkf.map(|k| k.tangent_out).unwrap_or(0.0);
                                track.push(Vec3Keyframe {
                                    time,
                                    value: Vec3::new(x, y, z),
                                    interpolation: Interpolation::Cubic,
                                    tangent_in: Vec3::new(tx_in, ty_in, tz_in),
                                    tangent_out: Vec3::new(tx_out, ty_out, tz_out),
                                });
                            }
                            Interpolation::Step => {
                                track.push(Vec3Keyframe::step(time, Vec3::new(x, y, z)));
                            }
                            _ => {
                                track.push(Vec3Keyframe::linear(time, Vec3::new(x, y, z)));
                            }
                        }
                    }
                    clip.position_track = Some(track);
                }
            }

            // Build scale track from X/Y/Z curves
            let x_curve = self
                .curves
                .iter()
                .find(|c| c.track_type == TrackType::ScaleX);
            let y_curve = self
                .curves
                .iter()
                .find(|c| c.track_type == TrackType::ScaleY);
            let z_curve = self
                .curves
                .iter()
                .find(|c| c.track_type == TrackType::ScaleZ);
            if let (Some(xc), Some(yc), Some(zc)) = (x_curve, y_curve, z_curve) {
                let max_len = xc
                    .keyframes
                    .len()
                    .max(yc.keyframes.len())
                    .max(zc.keyframes.len());
                if max_len > 0 {
                    let mut track = Vec::new();
                    for idx in 0..max_len {
                        let xkf = xc.keyframes.get(idx);
                        let ykf = yc.keyframes.get(idx);
                        let zkf = zc.keyframes.get(idx);
                        let time = xkf.or(ykf).or(zkf).map(|k| k.time).unwrap_or(0.0);
                        let x = xkf.map(|k| k.value).unwrap_or(1.0);
                        let y = ykf.map(|k| k.value).unwrap_or(1.0);
                        let z = zkf.map(|k| k.value).unwrap_or(1.0);
                        track.push(Vec3Keyframe::linear(time, Vec3::new(x, y, z)));
                    }
                    clip.scale_track = Some(track);
                }
            }
        }
    }

    pub fn export_to_file(&self, path: &std::path::Path) -> anyhow::Result<()> {
        io::export_clip(self, path)
    }

    pub fn import_from_file(&mut self, path: &std::path::Path) -> anyhow::Result<()> {
        let clip = io::import_clip(path)?;
        self.load_clip(clip);
        Ok(())
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.player.speed = speed.clamp(0.1, 10.0);
    }

    pub fn handle_input(&mut self, ui: &egui::Ui) {
        if ui.input(|i| i.key_pressed(egui::Key::Space)) {
            if self.player.playing {
                self.player.pause();
            } else {
                self.player.play();
            }
        }

        if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
            keyframe_list::delete_selected_keyframe(self);
        }

        if ui.input(|i| i.key_pressed(egui::Key::Home)) {
            self.player.time = 0.0;
        }

        if ui.input(|i| i.key_pressed(egui::Key::End))
            && let Some(ref clip) = self.clip
        {
            self.player.time = clip.duration;
        }
    }
}

fn build_curves_from_clip(clip: &AnimationClip) -> Vec<ComponentCurve> {
    let mut curves = Vec::new();

    // Position curves
    if let Some(ref track) = clip.position_track {
        let mut x_curve = ComponentCurve::new(TrackType::PositionX);
        let mut y_curve = ComponentCurve::new(TrackType::PositionY);
        let mut z_curve = ComponentCurve::new(TrackType::PositionZ);
        for kf in track {
            x_curve.keyframes.push(CurveKeyframe {
                time: kf.time,
                value: kf.value.x,
                interpolation: kf.interpolation,
                tangent_in: kf.tangent_in.x,
                tangent_out: kf.tangent_out.x,
            });
            y_curve.keyframes.push(CurveKeyframe {
                time: kf.time,
                value: kf.value.y,
                interpolation: kf.interpolation,
                tangent_in: kf.tangent_in.y,
                tangent_out: kf.tangent_out.y,
            });
            z_curve.keyframes.push(CurveKeyframe {
                time: kf.time,
                value: kf.value.z,
                interpolation: kf.interpolation,
                tangent_in: kf.tangent_in.z,
                tangent_out: kf.tangent_out.z,
            });
        }
        curves.push(x_curve);
        curves.push(y_curve);
        curves.push(z_curve);
    } else {
        curves.push(ComponentCurve::new(TrackType::PositionX));
        curves.push(ComponentCurve::new(TrackType::PositionY));
        curves.push(ComponentCurve::new(TrackType::PositionZ));
    }

    // Rotation curves (placeholder — euler angles extracted at runtime)
    curves.push(ComponentCurve::new(TrackType::RotationX));
    curves.push(ComponentCurve::new(TrackType::RotationY));
    curves.push(ComponentCurve::new(TrackType::RotationZ));

    // Scale curves
    if let Some(ref track) = clip.scale_track {
        let mut x_curve = ComponentCurve::new(TrackType::ScaleX);
        let mut y_curve = ComponentCurve::new(TrackType::ScaleY);
        let mut z_curve = ComponentCurve::new(TrackType::ScaleZ);
        for kf in track {
            x_curve.keyframes.push(CurveKeyframe {
                time: kf.time,
                value: kf.value.x,
                interpolation: kf.interpolation,
                tangent_in: kf.tangent_in.x,
                tangent_out: kf.tangent_out.x,
            });
            y_curve.keyframes.push(CurveKeyframe {
                time: kf.time,
                value: kf.value.y,
                interpolation: kf.interpolation,
                tangent_in: kf.tangent_in.y,
                tangent_out: kf.tangent_out.y,
            });
            z_curve.keyframes.push(CurveKeyframe {
                time: kf.time,
                value: kf.value.z,
                interpolation: kf.interpolation,
                tangent_in: kf.tangent_in.z,
                tangent_out: kf.tangent_out.z,
            });
        }
        curves.push(x_curve);
        curves.push(y_curve);
        curves.push(z_curve);
    } else {
        curves.push(ComponentCurve::new(TrackType::ScaleX));
        curves.push(ComponentCurve::new(TrackType::ScaleY));
        curves.push(ComponentCurve::new(TrackType::ScaleZ));
    }

    curves
}

/// Main entry point: draw the full animation editor panel.
pub fn draw_animation_editor(
    state: &mut crate::state::EditorState,
    ui: &egui::Ui,
    rect: egui::Rect,
) {
    let anim = &mut state.animation_editor;
    if !anim.visible {
        return;
    }

    let h_scale = ui.ctx().screen_rect().height() / 1080.0;
    let w_scale = ui.ctx().screen_rect().width() / 1920.0;

    let painter = ui.painter_at(rect);

    painter.add(egui::Shape::rect_filled(
        rect,
        egui::Rounding::ZERO,
        egui::Color32::from_rgb(22, 22, 25),
    ));
    painter.add(egui::Shape::line(
        vec![
            egui::pos2(rect.left(), rect.top()),
            egui::pos2(rect.right(), rect.top()),
        ],
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 45, 53)),
    ));

    let toolbar_h = 32.0 * h_scale;
    let track_label_w = 70.0 * w_scale;
    let keyframe_list_w = 140.0 * w_scale;

    let toolbar_rect =
        egui::Rect::from_min_size(rect.left_top(), egui::Vec2::new(rect.width(), toolbar_h));
    draw_animation_toolbar(anim, ui, toolbar_rect, w_scale, h_scale);

    let content_top = rect.top() + toolbar_h;
    let content_h = rect.height() - toolbar_h;
    let header_h = 24.0 * h_scale;
    let left_h = content_h * 0.5;
    let right_h = content_h - left_h;

    let track_label_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left(), content_top),
        egui::vec2(track_label_w, content_h),
    );

    let timeline_header_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + track_label_w, content_top),
        egui::vec2(rect.width() - track_label_w - keyframe_list_w, header_h),
    );

    let kf_tracks_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + track_label_w, content_top + header_h),
        egui::vec2(
            rect.width() - track_label_w - keyframe_list_w,
            left_h - header_h,
        ),
    );

    let curve_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + track_label_w, content_top + left_h),
        egui::vec2(rect.width() - track_label_w - keyframe_list_w, right_h),
    );

    let kf_list_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() - keyframe_list_w, content_top),
        egui::vec2(keyframe_list_w, content_h),
    );

    timeline::draw_timeline_header(anim, ui, timeline_header_rect);
    timeline::draw_track_labels(anim, ui, track_label_rect);
    timeline::draw_keyframe_tracks(anim, ui, kf_tracks_rect);
    curve_editor::draw_curve_editor(anim, ui, curve_rect);
    keyframe_list::draw_keyframe_list(anim, ui, kf_list_rect);

    // Handle track label clicks
    let row_h = 24.0;
    let mut y = track_label_rect.top() + 2.0;
    for i in 0..anim.curves.len() {
        if y + row_h > track_label_rect.bottom() {
            break;
        }
        let row_rect = egui::Rect::from_min_size(
            egui::pos2(track_label_rect.left(), y),
            egui::vec2(track_label_rect.width(), row_h),
        );
        let id = egui::Id::new("anim_track_click").with(i);
        let resp = ui.interact(row_rect, id, egui::Sense::click());
        if resp.clicked() {
            anim.selected_track = i;
            anim.selected_keyframe = None;
        }
        y += row_h;
    }

    // Advance playback
    let dt = ui.input(|i| i.unstable_dt);
    if anim.player.playing {
        preview::advance_playback(anim, dt);
    }

    // Apply preview
    preview::apply_preview(anim, &mut state.node_transforms);
}

fn draw_animation_toolbar(
    state: &mut AnimationEditorState,
    ui: &egui::Ui,
    rect: egui::Rect,
    w_scale: f32,
    h_scale: f32,
) {
    let painter = ui.painter_at(rect);
    painter.add(egui::Shape::rect_filled(
        rect,
        egui::Rounding::ZERO,
        egui::Color32::from_rgb(30, 30, 34),
    ));
    painter.add(egui::Shape::line(
        vec![
            egui::pos2(rect.left(), rect.bottom() - 1.0),
            egui::pos2(rect.right(), rect.bottom() - 1.0),
        ],
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 45, 53)),
    ));

    let btn_size = 24.0 * h_scale;
    let gap = 4.0 * w_scale;
    let pad = 8.0 * w_scale;
    let mut x = rect.left() + pad;
    let cy = rect.top() + (rect.height() - btn_size) / 2.0;

    // Play/Pause
    let play_icon = if state.player.playing { "⏸" } else { "▶" };
    let play_rect = egui::Rect::from_min_size(egui::pos2(x, cy), egui::vec2(btn_size, btn_size));
    let id = egui::Id::new("anim_play_btn");
    let resp = ui.interact(play_rect, id, egui::Sense::click());
    painter.text(
        play_rect.center(),
        egui::Align2::CENTER_CENTER,
        play_icon,
        egui::FontId::proportional(12.0 * h_scale),
        egui::Color32::WHITE,
    );
    if resp.clicked() {
        if state.player.playing {
            state.player.pause();
        } else {
            state.player.play();
        }
    }
    x += btn_size + gap;

    // Stop
    let stop_rect = egui::Rect::from_min_size(egui::pos2(x, cy), egui::vec2(btn_size, btn_size));
    let id = egui::Id::new("anim_stop_btn");
    let resp = ui.interact(stop_rect, id, egui::Sense::click());
    painter.text(
        stop_rect.center(),
        egui::Align2::CENTER_CENTER,
        "⏹",
        egui::FontId::proportional(12.0 * h_scale),
        egui::Color32::WHITE,
    );
    if resp.clicked() {
        state.player.stop();
    }
    x += btn_size + gap + pad;

    // Separator
    painter.add(egui::Shape::line(
        vec![
            egui::pos2(x, rect.top() + 4.0),
            egui::pos2(x, rect.bottom() - 4.0),
        ],
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 45, 53)),
    ));
    x += pad;

    // Time display
    let time_text = format!(
        "{:.2}s / {:.2}s",
        state.player.time,
        state.clip.as_ref().map(|c| c.duration).unwrap_or(0.0)
    );
    painter.text(
        egui::pos2(x, cy + btn_size / 2.0),
        egui::Align2::LEFT_CENTER,
        time_text,
        egui::FontId::monospace(11.0 * h_scale),
        egui::Color32::from_rgb(200, 200, 200),
    );
    x += 100.0 * w_scale;

    // Speed
    painter.text(
        egui::pos2(x, cy + btn_size / 2.0),
        egui::Align2::LEFT_CENTER,
        format!("速度: {:.1}x", state.player.speed),
        egui::FontId::proportional(10.0 * h_scale),
        egui::Color32::from_gray(120),
    );
    x += 60.0 * w_scale;

    // Snap toggle
    let snap_label = if state.snap_to_frame {
        "吸附: 开"
    } else {
        "吸附: 关"
    };
    let snap_rect =
        egui::Rect::from_min_size(egui::pos2(x, cy), egui::vec2(50.0 * w_scale, btn_size));
    let id = egui::Id::new("anim_snap_btn");
    let resp = ui.interact(snap_rect, id, egui::Sense::click());
    painter.text(
        snap_rect.center(),
        egui::Align2::CENTER_CENTER,
        snap_label,
        egui::FontId::proportional(10.0 * h_scale),
        if state.snap_to_frame {
            egui::Color32::from_rgb(0, 200, 150)
        } else {
            egui::Color32::from_gray(100)
        },
    );
    if resp.clicked() {
        state.snap_to_frame = !state.snap_to_frame;
    }
    x += 50.0 * w_scale + pad;

    // Preview toggle
    let preview_label = if state.preview_enabled {
        "预览: 开"
    } else {
        "预览: 关"
    };
    let preview_rect =
        egui::Rect::from_min_size(egui::pos2(x, cy), egui::vec2(50.0 * w_scale, btn_size));
    let id = egui::Id::new("anim_preview_btn");
    let resp = ui.interact(preview_rect, id, egui::Sense::click());
    painter.text(
        preview_rect.center(),
        egui::Align2::CENTER_CENTER,
        preview_label,
        egui::FontId::proportional(10.0 * h_scale),
        if state.preview_enabled {
            egui::Color32::from_rgb(0, 200, 150)
        } else {
            egui::Color32::from_gray(100)
        },
    );
    if resp.clicked() {
        state.preview_enabled = !state.preview_enabled;
    }

    // Clip name
    let clip_name = state.clip.as_ref().map(|c| c.name.as_str()).unwrap_or("无");
    painter.text(
        egui::pos2(rect.right() - pad, cy + btn_size / 2.0),
        egui::Align2::RIGHT_CENTER,
        format!("片段: {}", clip_name),
        egui::FontId::proportional(11.0 * h_scale),
        egui::Color32::from_gray(140),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_type_labels() {
        assert_eq!(TrackType::PositionX.label(), "Pos X");
        assert_eq!(TrackType::ScaleZ.label(), "Scl Z");
    }

    #[test]
    fn test_component_curve_sample_empty() {
        let curve = ComponentCurve::new(TrackType::PositionX);
        assert_eq!(curve.sample(0.5), 0.0);
    }

    #[test]
    fn test_component_curve_sample_single() {
        let mut curve = ComponentCurve::new(TrackType::PositionX);
        curve.keyframes.push(CurveKeyframe {
            time: 0.0,
            value: 5.0,
            interpolation: Interpolation::Linear,
            tangent_in: 0.0,
            tangent_out: 0.0,
        });
        assert_eq!(curve.sample(0.5), 5.0);
    }

    #[test]
    fn test_component_curve_sample_linear() {
        let mut curve = ComponentCurve::new(TrackType::PositionX);
        curve.keyframes.push(CurveKeyframe {
            time: 0.0,
            value: 0.0,
            interpolation: Interpolation::Linear,
            tangent_in: 0.0,
            tangent_out: 0.0,
        });
        curve.keyframes.push(CurveKeyframe {
            time: 1.0,
            value: 10.0,
            interpolation: Interpolation::Linear,
            tangent_in: 0.0,
            tangent_out: 0.0,
        });
        assert!((curve.sample(0.5) - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_component_curve_sample_step() {
        let mut curve = ComponentCurve::new(TrackType::PositionX);
        curve.keyframes.push(CurveKeyframe {
            time: 0.0,
            value: 0.0,
            interpolation: Interpolation::Step,
            tangent_in: 0.0,
            tangent_out: 0.0,
        });
        curve.keyframes.push(CurveKeyframe {
            time: 1.0,
            value: 10.0,
            interpolation: Interpolation::Step,
            tangent_in: 0.0,
            tangent_out: 0.0,
        });
        assert_eq!(curve.sample(0.5), 0.0);
    }

    #[test]
    fn test_build_curves_from_empty_clip() {
        let clip = AnimationClip::new("test", 1.0);
        let curves = build_curves_from_clip(&clip);
        assert_eq!(curves.len(), 9);
    }

    #[test]
    fn test_build_curves_from_clip_with_tracks() {
        let clip = AnimationClip::new("test", 1.0).with_position_track(vec![
            Vec3Keyframe::linear(0.0, Vec3::ZERO),
            Vec3Keyframe::linear(1.0, Vec3::new(10.0, 0.0, 0.0)),
        ]);
        let curves = build_curves_from_clip(&clip);
        // All 3 position component curves get 2 keyframes each
        assert_eq!(curves[0].keyframes.len(), 2); // PositionX
        assert_eq!(curves[1].keyframes.len(), 2); // PositionY
        assert_eq!(curves[2].keyframes.len(), 2); // PositionZ
        // X values differ
        assert!((curves[0].keyframes[1].value - 10.0).abs() < 0.01);
        // Y and Z values are zero
        assert!((curves[1].keyframes[1].value).abs() < 0.01);
        assert!((curves[2].keyframes[1].value).abs() < 0.01);
    }

    #[test]
    fn test_load_clip() {
        let clip = AnimationClip::new("walk", 2.0).with_position_track(vec![
            Vec3Keyframe::linear(0.0, Vec3::ZERO),
            Vec3Keyframe::linear(1.0, Vec3::new(5.0, 0.0, 0.0)),
        ]);
        let mut state = AnimationEditorState::new();
        state.load_clip(clip);
        assert!(state.clip.is_some());
        assert_eq!(state.curves.len(), 9);
    }

    #[test]
    fn test_create_new_clip() {
        let mut state = AnimationEditorState::new();
        state.create_new_clip("idle".to_string(), 2.0);
        assert!(state.clip.is_some());
        let clip = state.clip.as_ref().unwrap();
        assert_eq!(clip.name, "idle");
        assert_eq!(clip.duration, 2.0);
    }

    #[test]
    fn test_sync_clip_from_curves() {
        let mut state = AnimationEditorState::new();
        state.create_new_clip("test".to_string(), 1.0);

        state.curves[0].keyframes.push(CurveKeyframe {
            time: 0.0,
            value: 0.0,
            interpolation: Interpolation::Linear,
            tangent_in: 0.0,
            tangent_out: 0.0,
        });
        state.curves[0].keyframes.push(CurveKeyframe {
            time: 1.0,
            value: 10.0,
            interpolation: Interpolation::Linear,
            tangent_in: 0.0,
            tangent_out: 0.0,
        });

        state.sync_clip_from_curves();
        let clip = state.clip.as_ref().unwrap();
        assert!(clip.position_track.is_some());
    }

    #[test]
    fn test_set_speed_clamp() {
        let mut state = AnimationEditorState::new();
        state.set_speed(15.0);
        assert_eq!(state.player.speed, 10.0);
        state.set_speed(0.01);
        assert_eq!(state.player.speed, 0.1);
    }
}
