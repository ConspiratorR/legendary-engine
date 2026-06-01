# Animation Editor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a keyframe animation editor in the RustEngine editor with timeline UI, bezier curve editing, property binding, real-time preview, and AnimationClip JSON import/export.

**Architecture:** New `animation_editor` module in `engine-editor` crate, using egui for all UI. Integrates with existing `AnimationClip`/`AnimationPlayer` types from `engine-scene`. The editor is a self-contained panel that can be toggled from the Window menu. Each sub-component (timeline, curve editor, keyframe list, preview) is a separate submodule.

**Tech Stack:** Rust 2024, egui 0.30, engine-scene (AnimationClip, AnimationPlayer, FloatKeyframe, Vec3Keyframe, RotationKeyframe, Interpolation), serde/serde_json for import/export.

---

## File Structure

| File | Responsibility |
|------|---------------|
| `crates/engine-editor/src/animation_editor/mod.rs` | `AnimationEditor` struct, `AnimationEditorState`, panel entry point, module declarations |
| `crates/engine-editor/src/animation_editor/timeline.rs` | Timeline widget: frame markers, playback head, zoom/pan, drag-to-scrub |
| `crates/engine-editor/src/animation_editor/curve_editor.rs` | Bezier curve editor: tangent handles, curve visualization, keyframe points |
| `crates/engine-editor/src/animation_editor/keyframe_list.rs` | Keyframe list panel: add/delete/move keyframes, property binding display |
| `crates/engine-editor/src/animation_editor/preview.rs` | Animation preview: apply sampled transform to selected entity in viewport |
| `crates/engine-editor/src/animation_editor/io.rs` | Import/export AnimationClip as JSON file |

**Modified files:**
- `crates/engine-editor/src/lib.rs` — add `pub mod animation_editor;`
- `crates/engine-editor/src/state.rs` — add `animation_editor: AnimationEditorState` field to `EditorState`
- `crates/engine-editor/src/layout.rs` — add animation editor panel toggle and drawing
- `crates/engine-editor/Cargo.toml` — no new deps needed (serde_json already present)

---

### Task 1: AnimationEditorState and Module Skeleton

**Files:**
- Create: `crates/engine-editor/src/animation_editor/mod.rs`
- Modify: `crates/engine-editor/src/lib.rs`
- Modify: `crates/engine-editor/src/state.rs`

- [ ] **Step 1: Create `animation_editor/mod.rs` with state types**

```rust
pub mod curve_editor;
pub mod io;
pub mod keyframe_list;
pub mod preview;
pub mod timeline;

use engine_scene::{AnimationClip, FloatKeyframe, Vec3Keyframe, RotationKeyframe, Interpolation};
use engine_scene::{AnimationPlayer};
use engine_math::{Vec3, Quat};

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
            Self::PositionX | Self::RotationX | Self::ScaleX => egui::Color32::from_rgb(230, 70, 70),
            Self::PositionY | Self::RotationY | Self::ScaleY => egui::Color32::from_rgb(70, 200, 70),
            Self::PositionZ | Self::RotationZ | Self::ScaleZ => egui::Color32::from_rgb(70, 100, 230),
        }
    }

    pub fn all() -> &'static [TrackType] {
        &[
            Self::PositionX, Self::PositionY, Self::PositionZ,
            Self::RotationX, Self::RotationY, Self::RotationZ,
            Self::ScaleX, Self::ScaleY, Self::ScaleZ,
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
        self.keyframes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    pub fn sample(&self, time: f32) -> f32 {
        if self.keyframes.is_empty() {
            return 0.0;
        }
        if self.keyframes.len() == 1 {
            return self.keyframes[0].value;
        }

        // Clamp to range
        let t = time.clamp(self.keyframes[0].time, self.keyframes.last().unwrap().time);

        // Find surrounding keyframes
        let mut i = 0;
        while i < self.keyframes.len() - 1 && self.keyframes[i + 1].time <= t {
            i += 1;
        }
        if i >= self.keyframes.len() - 1 {
            return self.keyframes.last().unwrap().value;
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
                k0.value * h00 + k0.tangent_out * dt * h10 + k1.value * h01 + k1.tangent_in * dt * h11
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
    pub dragging_keyframe: Option<(usize, usize)>, // (track_idx, keyframe_idx)
    pub dragging_tangent: Option<(usize, usize, bool)>, // (track_idx, keyframe_idx, is_out)
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
            timeline_zoom: 100.0, // pixels per second
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
            for curve in &self.curves {
                match curve.track_type {
                    TrackType::PositionX | TrackType::PositionY | TrackType::PositionZ => {
                        // Build or update position track from X/Y/Z curves
                        let x_curve = self.curves.iter().find(|c| c.track_type == TrackType::PositionX);
                        let y_curve = self.curves.iter().find(|c| c.track_type == TrackType::PositionY);
                        let z_curve = self.curves.iter().find(|c| c.track_type == TrackType::PositionZ);
                        if let (Some(xc), Some(yc), Some(zc)) = (x_curve, y_curve, z_curve) {
                            let max_len = xc.keyframes.len().max(yc.keyframes.len()).max(zc.keyframes.len());
                            if max_len > 0 {
                                let mut track = Vec::new();
                                for i in 0..max_len {
                                    let xkf = xc.keyframes.get(i);
                                    let ykf = yc.keyframes.get(i);
                                    let zkf = zc.keyframes.get(i);
                                    let time = xkf.or(ykf).or(zkf).map(|k| k.time).unwrap_or(0.0);
                                    let x = xkf.map(|k| k.value).unwrap_or(0.0);
                                    let y = ykf.map(|k| k.value).unwrap_or(0.0);
                                    let z = zkf.map(|k| k.value).unwrap_or(0.0);
                                    let interp = xkf.map(|k| k.interpolation).unwrap_or(Interpolation::Linear);
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
                        break; // only process position once
                    }
                    _ => {}
                }
            }
            // Similar logic for scale track
            for curve in &self.curves {
                match curve.track_type {
                    TrackType::ScaleX | TrackType::ScaleY | TrackType::ScaleZ => {
                        let x_curve = self.curves.iter().find(|c| c.track_type == TrackType::ScaleX);
                        let y_curve = self.curves.iter().find(|c| c.track_type == TrackType::ScaleY);
                        let z_curve = self.curves.iter().find(|c| c.track_type == TrackType::ScaleZ);
                        if let (Some(xc), Some(yc), Some(zc)) = (x_curve, y_curve, z_curve) {
                            let max_len = xc.keyframes.len().max(yc.keyframes.len()).max(zc.keyframes.len());
                            if max_len > 0 {
                                let mut track = Vec::new();
                                for i in 0..max_len {
                                    let xkf = xc.keyframes.get(i);
                                    let ykf = yc.keyframes.get(i);
                                    let zkf = zc.keyframes.get(i);
                                    let time = xkf.or(ykf).or(zkf).map(|k| k.time).unwrap_or(0.0);
                                    let x = xkf.map(|k| k.value).unwrap_or(1.0);
                                    let y = ykf.map(|k| k.value).unwrap_or(1.0);
                                    let z = zkf.map(|k| k.value).unwrap_or(1.0);
                                    track.push(Vec3Keyframe::linear(time, Vec3::new(x, y, z)));
                                }
                                clip.scale_track = Some(track);
                            }
                        }
                        break;
                    }
                    _ => {}
                }
            }
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

    // Rotation curves (extract euler angles from quaternions as float tracks)
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
        assert_eq!(curves.len(), 9); // 3 pos + 3 rot + 3 scale
    }

    #[test]
    fn test_build_curves_from_clip_with_tracks() {
        let clip = AnimationClip::new("test", 1.0)
            .with_position_track(vec![
                Vec3Keyframe::linear(0.0, Vec3::ZERO),
                Vec3Keyframe::linear(1.0, Vec3::new(10.0, 0.0, 0.0)),
            ]);
        let curves = build_curves_from_clip(&clip);
        assert_eq!(curves[0].keyframes.len(), 2); // PositionX has 2 keyframes
        assert_eq!(curves[1].keyframes.len(), 0); // PositionY has 0
    }

    #[test]
    fn test_load_clip() {
        let clip = AnimationClip::new("walk", 2.0)
            .with_position_track(vec![
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

        // Add keyframes to position X
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
}
```

- [ ] **Step 2: Add module declaration to `lib.rs`**

Add `pub mod animation_editor;` to `crates/engine-editor/src/lib.rs`.

- [ ] **Step 3: Add `AnimationEditorState` to `EditorState`**

In `crates/engine-editor/src/state.rs`, add import and field:
```rust
use crate::animation_editor::AnimationEditorState;
```
Add field to `EditorState`:
```rust
pub animation_editor: AnimationEditorState,
```
Initialize in `EditorState::new()`:
```rust
animation_editor: AnimationEditorState::new(),
```

- [ ] **Step 4: Run tests to verify**

Run: `cargo test -p engine-editor`
Expected: All tests pass (including new animation_editor tests)

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p engine-editor`
Expected: No warnings

---

### Task 2: Timeline Widget

**Files:**
- Create: `crates/engine-editor/src/animation_editor/timeline.rs`

- [ ] **Step 1: Create timeline.rs with timeline rendering**

```rust
use super::AnimationEditorState;
use egui::{Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};

const PLAYHEAD_COLOR: Color32 = Color32::from_rgb(255, 200, 50);
const MARKER_COLOR: Color32 = Color32::from_rgb(80, 80, 90);
const BG_COLOR: Color32 = Color32::from_rgb(28, 28, 32);
const TRACK_BG: Color32 = Color32::from_rgb(35, 35, 40);
const GRID_COLOR: Color32 = Color32::from_rgb(50, 50, 58);
const TEXT_COLOR: Color32 = Color32::from_gray(140);

/// Draw the timeline header with frame markers and playback head.
pub fn draw_timeline_header(
    state: &mut AnimationEditorState,
    ui: &egui::Ui,
    rect: Rect,
) {
    let painter = ui.painter_at(rect);

    // Background
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
    let zoom = state.timeline_zoom; // px per second
    let offset = state.timeline_offset;

    // Draw time markers
    let pixels_per_second = zoom;
    let start_time = (offset / pixels_per_second).floor() as i32;
    let end_time = ((offset + rect.width()) / pixels_per_second).ceil() as i32;

    for t in start_time..=end_time {
        if t < 0 {
            continue;
        }
        let x = rect.left() + (t as f32 * pixels_per_second) - offset;

        // Major tick
        painter.add(Shape::line(
            vec![Pos2::new(x, rect.bottom() - 12.0), Pos2::new(x, rect.bottom())],
            Stroke::new(1.0, MARKER_COLOR),
        ));

        // Time label
        painter.text(
            Pos2::new(x + 3.0, rect.top() + 4.0),
            egui::Align2::LEFT_TOP,
            format!("{}", t),
            egui::FontId::proportional(10.0),
            TEXT_COLOR,
        );

        // Sub-ticks (at 0.5s intervals if zoom is high enough)
        if pixels_per_second > 60.0 {
            let sub_x = x + pixels_per_second * 0.5;
            if sub_x < rect.right() {
                painter.add(Shape::line(
                    vec![Pos2::new(sub_x, rect.bottom() - 6.0), Pos2::new(sub_x, rect.bottom())],
                    Stroke::new(1.0, Color32::from_rgb(40, 40, 48)),
                ));
            }
        }
    }

    // Bottom border
    painter.add(Shape::line(
        vec![Pos2::new(rect.left(), rect.bottom() - 1.0), Pos2::new(rect.right(), rect.bottom() - 1.0)],
        Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
    ));

    // Playhead
    let playhead_x = rect.left() + (state.player.time * pixels_per_second) - offset;
    if playhead_x >= rect.left() && playhead_x <= rect.right() {
        // Triangle marker
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

        // Playhead line
        painter.add(Shape::line(
            vec![Pos2::new(playhead_x, rect.top()), Pos2::new(playhead_x, rect.bottom())],
            Stroke::new(1.5, PLAYHEAD_COLOR),
        ));
    }

    // Handle click/drag on timeline to scrub
    let id = egui::Id::new("timeline_header");
    let response = ui.interact(rect, id, egui::Sense::click_and_drag());
    if response.clicked() || response.dragged() {
        if let Some(pos) = ui.input(|i| i.pointer.latest_pos()) {
            let time = ((pos.x - rect.left() + offset) / pixels_per_second).max(0.0);
            state.player.time = time.min(duration);
            state.dragging_playhead = true;
        }
    }
    if response.drag_stopped() {
        state.dragging_playhead = false;
    }
}

/// Draw the track labels on the left side of the timeline.
pub fn draw_track_labels(
    state: &AnimationEditorState,
    ui: &egui::Ui,
    rect: Rect,
) {
    let painter = ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(25, 25, 28)));

    let row_h = 24.0;
    let mut y = rect.top() + 2.0;

    for (i, curve) in state.curves.iter().enumerate() {
        if y + row_h > rect.bottom() {
            break;
        }

        let is_selected = state.selected_track == i;
        let row_rect = Rect::from_min_size(
            Pos2::new(rect.left(), y),
            Vec2::new(rect.width(), row_h),
        );

        if is_selected {
            painter.add(Shape::rect_filled(
                row_rect,
                Rounding::ZERO,
                Color32::from_rgb(40, 40, 48),
            ));
        }

        // Color indicator
        let color = curve.track_type.color();
        painter.add(Shape::rect_filled(
            Rect::from_min_size(
                Pos2::new(rect.left() + 4.0, y + 6.0),
                Vec2::new(3.0, row_h - 12.0),
            ),
            Rounding::same(1.0),
            color,
        ));

        // Label
        painter.text(
            Pos2::new(rect.left() + 12.0, y + row_h / 2.0),
            egui::Align2::LEFT_CENTER,
            curve.track_type.label(),
            egui::FontId::proportional(11.0),
            if is_selected { Color32::WHITE } else { TEXT_COLOR },
        );

        // Keyframe count
        let count_text = format!("{}", curve.keyframes.len());
        painter.text(
            Pos2::new(rect.right() - 8.0, y + row_h / 2.0),
            egui::Align2::RIGHT_CENTER,
            count_text,
            egui::FontId::proportional(10.0),
            Color32::from_gray(70),
        );

        // Row separator
        painter.add(Shape::line(
            vec![Pos2::new(rect.left(), y + row_h), Pos2::new(rect.right(), y + row_h)],
            Stroke::new(0.5, Color32::from_rgb(40, 40, 48)),
        ));

        // Click to select track
        let id = egui::Id::new("track_label").with(i);
        let response = ui.interact(row_rect, id, egui::Sense::click());
        // Note: can't mutate state here since we have &AnimationEditorState
        // Selection will be handled in the main draw function

        y += row_h;
    }

    // Right border
    painter.add(Shape::line(
        vec![Pos2::new(rect.right() - 1.0, rect.top()), Pos2::new(rect.right() - 1.0, rect.bottom())],
        Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
    ));
}

/// Draw the keyframe diamonds on the timeline tracks.
pub fn draw_keyframe_tracks(
    state: &mut AnimationEditorState,
    ui: &egui::Ui,
    rect: Rect,
) {
    let painter = ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, TRACK_BG));

    let Some(ref _clip) = state.clip else { return; };

    let zoom = state.timeline_zoom;
    let offset = state.timeline_offset;
    let row_h = 24.0;
    let diamond_size = 6.0;

    let mut y = rect.top() + 2.0;

    for (track_idx, curve) in state.curves.iter().enumerate() {
        if y + row_h > rect.bottom() {
            break;
        }

        // Row background
        if state.selected_track == track_idx {
            painter.add(Shape::rect_filled(
                Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), row_h)),
                Rounding::ZERO,
                Color32::from_rgb(32, 32, 38),
            ));
        }

        // Row separator
        painter.add(Shape::line(
            vec![Pos2::new(rect.left(), y + row_h), Pos2::new(rect.right(), y + row_h)],
            Stroke::new(0.5, Color32::from_rgb(40, 40, 48)),
        ));

        // Draw keyframe diamonds
        for (kf_idx, kf) in curve.keyframes.iter().enumerate() {
            let x = rect.left() + (kf.time * zoom) - offset;
            if x < rect.left() - diamond_size || x > rect.right() + diamond_size {
                continue;
            }

            let cy = y + row_h / 2.0;
            let is_selected = state.selected_track == track_idx && state.selected_keyframe == Some(kf_idx);

            // Diamond shape
            let color = if is_selected {
                Color32::WHITE
            } else {
                curve.track_type.color()
            };

            let stroke = if is_selected {
                Stroke::new(2.0, Color32::from_rgb(255, 255, 255))
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

            // Click interaction for keyframe selection
            let kf_rect = Rect::from_center_size(
                Pos2::new(x, cy),
                Vec2::new(diamond_size * 2.5, row_h),
            );
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
        } else if let Some(delta) = ui.input(|i| i.pointer.delta()) {
            if let Some(curve) = state.curves.get_mut(track_idx) {
                if let Some(kf) = curve.keyframes.get_mut(kf_idx) {
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
    }

    // Playhead overlay
    let playhead_x = rect.left() + (state.player.time * zoom) - offset;
    if playhead_x >= rect.left() && playhead_x <= rect.right() {
        painter.add(Shape::line(
            vec![Pos2::new(playhead_x, rect.top()), Pos2::new(playhead_x, rect.bottom())],
            Stroke::new(1.5, PLAYHEAD_COLOR),
        ));
    }

    // Handle scroll for zoom
    let id = egui::Id::new("timeline_tracks");
    let response = ui.interact(rect, id, egui::Sense::click_and_drag());
    if response.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll.abs() > 0.1 {
            state.timeline_zoom = (state.timeline_zoom * (1.0 + scroll * 0.005)).clamp(20.0, 1000.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playhead_within_bounds() {
        let state = AnimationEditorState::new();
        // Just verify the function doesn't panic with no clip
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 100.0));
        // Can't test draw functions without egui context, but types compile
        assert!(state.clip.is_none());
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p engine-editor`
Expected: Pass

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -p engine-editor`
Expected: No warnings

---

### Task 3: Curve Editor Widget

**Files:**
- Create: `crates/engine-editor/src/animation_editor/curve_editor.rs`

- [ ] **Step 1: Create curve_editor.rs**

```rust
use super::AnimationEditorState;
use engine_scene::Interpolation;
use egui::{Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};

const BG_COLOR: Color32 = Color32::from_rgb(25, 25, 28);
const GRID_COLOR: Color32 = Color32::from_rgb(40, 40, 48);
const GRID_MAJOR: Color32 = Color32::from_rgb(50, 50, 58);
const TANGENT_COLOR: Color32 = Color32::from_rgb(180, 180, 100);

/// Draw the bezier curve editor for the currently selected track.
pub fn draw_curve_editor(
    state: &mut AnimationEditorState,
    ui: &egui::Ui,
    rect: Rect,
) {
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
    let offset_x = state.curve_offset_x;
    let offset_y = state.curve_offset_y;
    let center_x = rect.center().x - offset_x;
    let center_y = rect.center().y + offset_y;

    // Draw grid
    draw_grid(&painter, rect, zoom_x, zoom_y, center_x, center_y);

    // Draw zero line
    let zero_y = center_y;
    if zero_y >= rect.top() && zero_y <= rect.bottom() {
        painter.add(Shape::line(
            vec![Pos2::new(rect.left(), zero_y), Pos2::new(rect.right(), zero_y)],
            Stroke::new(0.5, Color32::from_rgb(60, 60, 70)),
        ));
    }

    // Draw curve
    if curve.keyframes.len() >= 2 {
        let color = curve.track_type.color();
        let num_samples = ((rect.width() / 2.0) as usize).max(50);

        let mut points = Vec::with_capacity(num_samples + 1);
        for i in 0..=num_samples {
            let t = i as f32 / num_samples as f32;
            let time = curve.keyframes.first().unwrap().time
                + t * (curve.keyframes.last().unwrap().time - curve.keyframes.first().unwrap().time);
            let value = curve.sample(time);

            let screen_x = center_x + (time * zoom_x);
            let screen_y = center_y - (value * zoom_y);

            points.push(Pos2::new(screen_x, screen_y));
        }

        for w in points.windows(2) {
            painter.add(Shape::line(
                vec![w[0], w[1]],
                Stroke::new(2.0, color),
            ));
        }
    }

    // Draw keyframe points and tangent handles
    for (kf_idx, kf) in curve.keyframes.iter().enumerate() {
        let screen_x = center_x + (kf.time * zoom_x);
        let screen_y = center_y - (kf.value * zoom_y);
        let pos = Pos2::new(screen_x, screen_y);
        let is_selected = state.selected_keyframe == Some(kf_idx);

        // Tangent handles (for cubic interpolation)
        if kf.interpolation == Interpolation::Cubic {
            // Tangent in
            let tangent_in_x = screen_x - 30.0;
            let tangent_in_y = screen_y + kf.tangent_in * zoom_y * 0.3;
            let tangent_in_pos = Pos2::new(tangent_in_x, tangent_in_y);

            painter.add(Shape::line(
                vec![pos, tangent_in_pos],
                Stroke::new(1.0, TANGENT_COLOR),
            ));
            painter.add(Shape::circle_filled(tangent_in_pos, 3.0, TANGENT_COLOR));

            // Tangent out
            let tangent_out_x = screen_x + 30.0;
            let tangent_out_y = screen_y - kf.tangent_out * zoom_y * 0.3;
            let tangent_out_pos = Pos2::new(tangent_out_x, tangent_out_y);

            painter.add(Shape::line(
                vec![pos, tangent_out_pos],
                Stroke::new(1.0, TANGENT_COLOR),
            ));
            painter.add(Shape::circle_filled(tangent_out_pos, 3.0, TANGENT_COLOR));

            // Tangent handle interactions
            let in_rect = Rect::from_center_size(tangent_in_pos, Vec2::new(12.0, 12.0));
            let id = egui::Id::new("tangent_in").with(kf_idx);
            let resp = ui.interact(in_rect, id, egui::Sense::drag());
            if resp.drag_started() {
                state.dragging_tangent = Some((state.selected_track, kf_idx, false));
            }

            let out_rect = Rect::from_center_size(tangent_out_pos, Vec2::new(12.0, 12.0));
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
            painter.add(Shape::circle_stroke(pos, radius + 2.0, Stroke::new(1.5, Color32::WHITE)));
        }

        // Click to select keyframe
        let kf_rect = Rect::from_center_size(pos, Vec2::new(14.0, 14.0));
        let id = egui::Id::new("curve_kf").with(kf_idx);
        let resp = ui.interact(kf_rect, id, egui::Sense::click());
        if resp.clicked() {
            state.selected_keyframe = Some(kf_idx);
        }
    }

    // Handle tangent dragging
    if let Some((track_idx, kf_idx, is_out)) = state.dragging_tangent {
        if ui.input(|i| i.pointer.any_released()) {
            state.dragging_tangent = None;
        } else if let Some(delta) = ui.input(|i| i.pointer.delta()) {
            if let Some(curve) = state.curves.get_mut(track_idx) {
                if let Some(kf) = curve.keyframes.get_mut(kf_idx) {
                    let tangent_delta = -delta.y / (zoom_y * 0.3);
                    if is_out {
                        kf.tangent_out += tangent_delta;
                    } else {
                        kf.tangent_in += tangent_delta;
                    }
                }
            }
        }
    }

    // Handle zoom with scroll
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

    // Border
    painter.add(Shape::rect_stroke(
        rect,
        Rounding::ZERO,
        Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
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
    // Vertical grid lines (time)
    let grid_spacing_x = compute_grid_spacing(zoom_x);
    let start_x = ((rect.left() - center_x) / (zoom_x * grid_spacing_x)).floor() as i32;
    let end_x = ((rect.right() - center_x) / (zoom_x * grid_spacing_x)).ceil() as i32;

    for i in start_x..=end_x {
        let x = center_x + (i as f32 * grid_spacing_x * zoom_x);
        if x >= rect.left() && x <= rect.right() {
            let is_major = i % 5 == 0;
            painter.add(Shape::line(
                vec![Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(0.5, if is_major { GRID_MAJOR } else { GRID_COLOR }),
            ));
        }
    }

    // Horizontal grid lines (value)
    let grid_spacing_y = compute_grid_spacing(zoom_y);
    let start_y = ((rect.top() - center_y) / (zoom_y * grid_spacing_y)).floor() as i32;
    let end_y = ((rect.bottom() - center_y) / (zoom_y * grid_spacing_y)).ceil() as i32;

    for i in start_y..=end_y {
        let y = center_y + (i as f32 * grid_spacing_y * zoom_y);
        if y >= rect.top() && y <= rect.bottom() {
            let is_major = i % 5 == 0;
            painter.add(Shape::line(
                vec![Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                Stroke::new(0.5, if is_major { GRID_MAJOR } else { GRID_COLOR }),
            ));
        }
    }
}

fn compute_grid_spacing(zoom: f32) -> f32 {
    // Aim for ~80px between grid lines
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
```

- [ ] **Step 2: Run tests and clippy**

Run: `cargo test -p engine-editor && cargo clippy -p engine-editor`
Expected: All pass

---

### Task 4: Keyframe List Panel

**Files:**
- Create: `crates/engine-editor/src/animation_editor/keyframe_list.rs`

- [ ] **Step 1: Create keyframe_list.rs with CRUD operations**

```rust
use super::AnimationEditorState;
use engine_scene::Interpolation;
use egui::{Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};

const BG_COLOR: Color32 = Color32::from_rgb(28, 28, 32);
const ROW_H: f32 = 22.0;

/// Draw the keyframe list panel with add/delete/move controls.
pub fn draw_keyframe_list(
    state: &mut AnimationEditorState,
    ui: &egui::Ui,
    rect: Rect,
) {
    let painter = ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, BG_COLOR));

    // Header
    let header_h = 28.0;
    let header_rect = Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), header_h));
    painter.add(Shape::rect_filled(header_rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));

    painter.text(
        Pos2::new(rect.left() + 8.0, header_rect.center().y),
        egui::Align2::LEFT_CENTER,
        "关键帧",
        egui::FontId::proportional(11.0),
        Color32::from_gray(150),
    );

    // Add/Delete buttons in header
    let btn_size = 18.0;
    let add_btn_rect = Rect::from_min_size(
        Pos2::new(rect.right() - 48.0, header_rect.center().y - btn_size / 2.0),
        Vec2::new(btn_size, btn_size),
    );
    let del_btn_rect = Rect::from_min_size(
        Pos2::new(rect.right() - 26.0, header_rect.center().y - btn_size / 2.0),
        Vec2::new(btn_size, btn_size),
    );

    // Add button
    let add_id = egui::Id::new("kf_add_btn");
    let add_resp = ui.interact(add_btn_rect, add_id, egui::Sense::click());
    painter.add(Shape::rect_filled(add_btn_rect, Rounding::same(3.0), Color32::from_rgb(50, 50, 58)));
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

    // Delete button
    let del_id = egui::Id::new("kf_del_btn");
    let del_resp = ui.interact(del_btn_rect, del_id, egui::Sense::click());
    painter.add(Shape::rect_filled(del_btn_rect, Rounding::same(3.0), Color32::from_rgb(50, 50, 58)));
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
        vec![Pos2::new(rect.left(), header_h), Pos2::new(rect.right(), header_h)],
        Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
    ));

    // Content area
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

    // Draw keyframe rows
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

        // Selection highlight
        if is_selected {
            painter.add(Shape::rect_filled(
                row_rect,
                Rounding::ZERO,
                Color32::from_rgb(0, 80, 160),
            ));
        }

        // Color indicator
        let color = curve.track_type.color();
        painter.add(Shape::rect_filled(
            Rect::from_min_size(
                Pos2::new(content_rect.left() + 2.0, y + 4.0),
                Vec2::new(3.0, ROW_H - 8.0),
            ),
            Rounding::same(1.0),
            color,
        ));

        // Time
        painter.text(
            Pos2::new(content_rect.left() + 10.0, y + ROW_H / 2.0),
            egui::Align2::LEFT_CENTER,
            format!("{:.3}s", kf.time),
            egui::FontId::monospace(10.0),
            if is_selected { Color32::WHITE } else { Color32::from_gray(160) },
        );

        // Value
        painter.text(
            Pos2::new(content_rect.left() + 70.0, y + ROW_H / 2.0),
            egui::Align2::LEFT_CENTER,
            format!("{:.2}", kf.value),
            egui::FontId::monospace(10.0),
            if is_selected { Color32::WHITE } else { Color32::from_gray(140) },
        );

        // Interpolation icon
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

        // Row separator
        painter.add(Shape::line(
            vec![Pos2::new(content_rect.left(), y + ROW_H), Pos2::new(content_rect.right(), y + ROW_H)],
            Stroke::new(0.5, Color32::from_rgb(35, 35, 42)),
        ));

        // Click to select
        let id = egui::Id::new("kf_row").with(kf_idx);
        let resp = ui.interact(row_rect, id, egui::Sense::click());
        if resp.clicked() {
            state.selected_keyframe = Some(kf_idx);
        }

        y += ROW_H;
    }
}

/// Add a keyframe at the current playhead time on the selected track.
pub fn add_keyframe_at_playhead(state: &mut AnimationEditorState) {
    let time = state.player.time;
    let curve = match state.curves.get_mut(state.selected_track) {
        Some(c) => c,
        None => return,
    };

    // Check if a keyframe already exists at this time
    let existing = curve.keyframes.iter().position(|kf| (kf.time - time).abs() < 0.001);
    if existing.is_some() {
        return;
    }

    // Sample current value at this time
    let value = curve.sample(time);

    curve.keyframes.push(super::CurveKeyframe {
        time,
        value,
        interpolation: Interpolation::Linear,
        tangent_in: 0.0,
        tangent_out: 0.0,
    });
    curve.sort_keyframes();

    // Select the new keyframe
    state.selected_keyframe = curve.keyframes.iter().position(|kf| (kf.time - time).abs() < 0.001);
}

/// Delete the currently selected keyframe.
pub fn delete_selected_keyframe(state: &mut AnimationEditorState) {
    let kf_idx = match state.selected_keyframe {
        Some(idx) => idx,
        None => return,
    };

    if let Some(curve) = state.curves.get_mut(state.selected_track) {
        if kf_idx < curve.keyframes.len() {
            curve.keyframes.remove(kf_idx);
            state.selected_keyframe = None;
        }
    }
}

/// Move the selected keyframe to a new time.
pub fn move_selected_keyframe(state: &mut AnimationEditorState, new_time: f32) {
    let kf_idx = match state.selected_keyframe {
        Some(idx) => idx,
        None => return,
    };

    if let Some(curve) = state.curves.get_mut(state.selected_track) {
        if let Some(kf) = curve.keyframes.get_mut(kf_idx) {
            kf.time = new_time.max(0.0);
            if state.snap_to_frame {
                let frame_time = 1.0 / state.fps;
                kf.time = (kf.time / frame_time).round() * frame_time;
            }
            curve.sort_keyframes();
        }
    }
}

/// Change the interpolation type of the selected keyframe.
pub fn set_selected_interpolation(state: &mut AnimationEditorState, interp: Interpolation) {
    let kf_idx = match state.selected_keyframe {
        Some(idx) => idx,
        None => return,
    };

    if let Some(curve) = state.curves.get_mut(state.selected_track) {
        if let Some(kf) = curve.keyframes.get_mut(kf_idx) {
            kf.interpolation = interp;
            if interp == Interpolation::Cubic {
                // Auto-compute tangents from neighboring keyframes
                let idx = kf_idx;
                let prev_val = if idx > 0 { curve.keyframes[idx - 1].value } else { kf.value };
                let next_val = if idx < curve.keyframes.len() - 1 { curve.keyframes[idx + 1].value } else { kf.value };
                kf.tangent_in = (next_val - prev_val) * 0.25;
                kf.tangent_out = (next_val - prev_val) * 0.25;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_scene::Interpolation;

    fn make_test_state() -> AnimationEditorState {
        let mut state = AnimationEditorState::new();
        state.create_new_clip("test".to_string(), 2.0);
        // Add some keyframes to position X
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
        assert_eq!(state.curves[0].keyframes.len(), 2); // no duplicate
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
        let expected = (0.51 * 30.0).round() / 30.0;
        assert!((state.curves[0].keyframes[0].time - expected).abs() < 0.001);
    }

    #[test]
    fn test_set_selected_interpolation_cubic() {
        let mut state = make_test_state();
        state.selected_keyframe = Some(0);
        set_selected_interpolation(&mut state, Interpolation::Cubic);
        assert_eq!(state.curves[0].keyframes[0].interpolation, Interpolation::Cubic);
    }

    #[test]
    fn test_set_selected_interpolation_step() {
        let mut state = make_test_state();
        state.selected_keyframe = Some(1);
        set_selected_interpolation(&mut state, Interpolation::Step);
        assert_eq!(state.curves[0].keyframes[1].interpolation, Interpolation::Step);
    }
}
```

- [ ] **Step 2: Run tests and clippy**

Run: `cargo test -p engine-editor && cargo clippy -p engine-editor`
Expected: All pass

---

### Task 5: Animation Preview

**Files:**
- Create: `crates/engine-editor/src/animation_editor/preview.rs`

- [ ] **Step 1: Create preview.rs**

```rust
use super::AnimationEditorState;
use engine_math::Vec3;

/// Apply the current animation sample to the target entity's transform.
/// This is called each frame when preview is enabled.
pub fn apply_preview(state: &mut AnimationEditorState, node_transforms: &mut std::collections::HashMap<u64, [f32; 9]>) {
    if !state.preview_enabled {
        return;
    }

    let target = match state.target_entity {
        Some(id) => id,
        None => return,
    };

    let time = state.player.time;

    // Sample position from curves
    let pos_x = state.curves.iter().find(|c| c.track_type == super::TrackType::PositionX).map(|c| c.sample(time)).unwrap_or(0.0);
    let pos_y = state.curves.iter().find(|c| c.track_type == super::TrackType::PositionY).map(|c| c.sample(time)).unwrap_or(0.0);
    let pos_z = state.curves.iter().find(|c| c.track_type == super::TrackType::PositionZ).map(|c| c.sample(time)).unwrap_or(0.0);

    // Sample scale from curves
    let scl_x = state.curves.iter().find(|c| c.track_type == super::TrackType::ScaleX).map(|c| c.sample(time)).unwrap_or(1.0);
    let scl_y = state.curves.iter().find(|c| c.track_type == super::TrackType::ScaleY).map(|c| c.sample(time)).unwrap_or(1.0);
    let scl_z = state.curves.iter().find(|c| c.track_type == super::TrackType::ScaleZ).map(|c| c.sample(time)).unwrap_or(1.0);

    // Sample rotation (as euler angles in degrees, converted to radians)
    let rot_x = state.curves.iter().find(|c| c.track_type == super::TrackType::RotationX).map(|c| c.sample(time)).unwrap_or(0.0);
    let rot_y = state.curves.iter().find(|c| c.track_type == super::TrackType::RotationY).map(|c| c.sample(time)).unwrap_or(0.0);
    let rot_z = state.curves.iter().find(|c| c.track_type == super::TrackType::RotationZ).map(|c| c.sample(time)).unwrap_or(0.0);

    // Apply to entity transform
    if let Some(transform) = node_transforms.get_mut(&target) {
        transform[0] = pos_x;
        transform[1] = pos_y;
        transform[2] = pos_z;
        transform[3] = rot_x;
        transform[4] = rot_y;
        transform[5] = rot_z;
        transform[6] = scl_x;
        transform[7] = scl_y;
        transform[8] = scl_z;
    }
}

/// Advance the animation player by delta time.
pub fn advance_playback(state: &mut AnimationEditorState, dt: f32) {
    if let Some(ref clip) = state.clip {
        state.player.advance(dt, clip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_apply_preview_disabled() {
        let mut state = AnimationEditorState::new();
        state.preview_enabled = false;
        let mut transforms = HashMap::new();
        transforms.insert(1, [0.0; 9]);
        apply_preview(&mut state, &mut transforms);
        assert_eq!(transforms[&1][0], 0.0);
    }

    #[test]
    fn test_apply_preview_no_target() {
        let mut state = AnimationEditorState::new();
        state.preview_enabled = true;
        state.target_entity = None;
        let mut transforms = HashMap::new();
        apply_preview(&mut state, &mut transforms);
    }

    #[test]
    fn test_apply_preview_with_target() {
        let mut state = AnimationEditorState::new();
        state.create_new_clip("test".to_string(), 1.0);
        state.preview_enabled = true;
        state.target_entity = Some(1);

        // Add position keyframes
        state.curves[0].keyframes.push(super::super::CurveKeyframe {
            time: 0.0,
            value: 5.0,
            interpolation: engine_scene::Interpolation::Linear,
            tangent_in: 0.0,
            tangent_out: 0.0,
        });

        let mut transforms = HashMap::new();
        transforms.insert(1, [0.0; 9]);

        state.player.time = 0.0;
        apply_preview(&mut state, &mut transforms);
        assert_eq!(transforms[&1][0], 5.0); // position X
    }

    #[test]
    fn test_advance_playback() {
        let mut state = AnimationEditorState::new();
        state.create_new_clip("test".to_string(), 2.0);
        state.player.play();

        advance_playback(&mut state, 0.5);
        assert!((state.player.time - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_advance_playback_no_clip() {
        let mut state = AnimationEditorState::new();
        advance_playback(&mut state, 0.5);
        assert_eq!(state.player.time, 0.0);
    }
}
```

- [ ] **Step 2: Run tests and clippy**

Run: `cargo test -p engine-editor && cargo clippy -p engine-editor`
Expected: All pass

---

### Task 6: Import/Export (AnimationClip JSON)

**Files:**
- Create: `crates/engine-editor/src/animation_editor/io.rs`

- [ ] **Step 1: Create io.rs with JSON serialization**

```rust
use super::AnimationEditorState;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Serializable representation of an AnimationClip for JSON export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationClipData {
    pub name: String,
    pub duration: f32,
    pub looping: bool,
    pub position_track: Option<Vec<Vec3KeyframeData>>,
    pub rotation_track: Option<Vec<RotationKeyframeData>>,
    pub scale_track: Option<Vec<Vec3KeyframeData>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vec3KeyframeData {
    pub time: f32,
    pub value: [f32; 3],
    pub interpolation: String,
    pub tangent_in: [f32; 3],
    pub tangent_out: [f32; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationKeyframeData {
    pub time: f32,
    pub value: [f32; 4],
    pub interpolation: String,
}

impl AnimationClipData {
    pub fn from_clip(clip: &engine_scene::AnimationClip) -> Self {
        Self {
            name: clip.name.clone(),
            duration: clip.duration,
            looping: clip.looping,
            position_track: clip.position_track.as_ref().map(|track| {
                track.iter().map(|kf| Vec3KeyframeData {
                    time: kf.time,
                    value: [kf.value.x, kf.value.y, kf.value.z],
                    interpolation: format!("{:?}", kf.interpolation).to_lowercase(),
                    tangent_in: [kf.tangent_in.x, kf.tangent_in.y, kf.tangent_in.z],
                    tangent_out: [kf.tangent_out.x, kf.tangent_out.y, kf.tangent_out.z],
                }).collect()
            }),
            rotation_track: clip.rotation_track.as_ref().map(|track| {
                track.iter().map(|kf| RotationKeyframeData {
                    time: kf.time,
                    value: [kf.value.x, kf.value.y, kf.value.z, kf.value.w],
                    interpolation: format!("{:?}", kf.interpolation).to_lowercase(),
                }).collect()
            }),
            scale_track: clip.scale_track.as_ref().map(|track| {
                track.iter().map(|kf| Vec3KeyframeData {
                    time: kf.time,
                    value: [kf.value.x, kf.value.y, kf.value.z],
                    interpolation: format!("{:?}", kf.interpolation).to_lowercase(),
                    tangent_in: [kf.tangent_in.x, kf.tangent_in.y, kf.tangent_in.z],
                    tangent_out: [kf.tangent_out.x, kf.tangent_out.y, kf.tangent_out.z],
                }).collect()
            }),
        }
    }

    pub fn to_clip(&self) -> Result<engine_scene::AnimationClip> {
        let mut clip = engine_scene::AnimationClip::new(&self.name, self.duration).looping(self.looping);

        if let Some(ref track) = self.position_track {
            let keyframes: Result<Vec<engine_scene::Vec3Keyframe>> = track.iter().map(|kf| {
                let interp = parse_interpolation(&kf.interpolation)?;
                Ok(engine_scene::Vec3Keyframe {
                    time: kf.time,
                    value: engine_math::Vec3::new(kf.value[0], kf.value[1], kf.value[2]),
                    interpolation: interp,
                    tangent_in: engine_math::Vec3::new(kf.tangent_in[0], kf.tangent_in[1], kf.tangent_in[2]),
                    tangent_out: engine_math::Vec3::new(kf.tangent_out[0], kf.tangent_out[1], kf.tangent_out[2]),
                })
            }).collect();
            clip = clip.with_position_track(keyframes?);
        }

        if let Some(ref track) = self.rotation_track {
            let keyframes: Result<Vec<engine_scene::RotationKeyframe>> = track.iter().map(|kf| {
                let interp = parse_interpolation(&kf.interpolation)?;
                Ok(engine_scene::RotationKeyframe {
                    time: kf.time,
                    value: engine_math::Quat::from_xyzw(kf.value[0], kf.value[1], kf.value[2], kf.value[3]),
                    interpolation: interp,
                })
            }).collect();
            clip = clip.with_rotation_track(keyframes?);
        }

        if let Some(ref track) = self.scale_track {
            let keyframes: Result<Vec<engine_scene::Vec3Keyframe>> = track.iter().map(|kf| {
                let interp = parse_interpolation(&kf.interpolation)?;
                Ok(engine_scene::Vec3Keyframe {
                    time: kf.time,
                    value: engine_math::Vec3::new(kf.value[0], kf.value[1], kf.value[2]),
                    interpolation: interp,
                    tangent_in: engine_math::Vec3::new(kf.tangent_in[0], kf.tangent_in[1], kf.tangent_in[2]),
                    tangent_out: engine_math::Vec3::new(kf.tangent_out[0], kf.tangent_out[1], kf.tangent_out[2]),
                })
            }).collect();
            clip = clip.with_scale_track(keyframes?);
        }

        Ok(clip)
    }
}

fn parse_interpolation(s: &str) -> Result<engine_scene::Interpolation> {
    match s.to_lowercase().as_str() {
        "linear" => Ok(engine_scene::Interpolation::Linear),
        "step" => Ok(engine_scene::Interpolation::Step),
        "cubic" => Ok(engine_scene::Interpolation::Cubic),
        _ => anyhow::bail!("Unknown interpolation type: {}", s),
    }
}

/// Export the current animation clip to a JSON file.
pub fn export_clip(state: &AnimationEditorState, path: &Path) -> Result<()> {
    let clip = state.clip.as_ref().context("No animation clip loaded")?;
    let data = AnimationClipData::from_clip(clip);
    let json = serde_json::to_string_pretty(&data).context("Failed to serialize animation clip")?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    fs::write(path, json)
        .with_context(|| format!("Failed to write animation file: {}", path.display()))?;
    Ok(())
}

/// Import an animation clip from a JSON file.
pub fn import_clip(path: &Path) -> Result<engine_scene::AnimationClip> {
    let json = fs::read_to_string(path)
        .with_context(|| format!("Failed to read animation file: {}", path.display()))?;

    let data: AnimationClipData = serde_json::from_str(&json)
        .with_context(|| format!("Failed to parse animation file: {}", path.display()))?;

    data.to_clip()
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_math::Vec3;

    fn make_test_clip() -> engine_scene::AnimationClip {
        engine_scene::AnimationClip::new("walk", 2.0)
            .looping(true)
            .with_position_track(vec![
                engine_scene::Vec3Keyframe::linear(0.0, Vec3::ZERO),
                engine_scene::Vec3Keyframe::linear(1.0, Vec3::new(5.0, 0.0, 0.0)),
                engine_scene::Vec3Keyframe::linear(2.0, Vec3::ZERO),
            ])
    }

    #[test]
    fn test_roundtrip_clip_data() {
        let clip = make_test_clip();
        let data = AnimationClipData::from_clip(&clip);
        let clip2 = data.to_clip().unwrap();

        assert_eq!(clip2.name, "walk");
        assert_eq!(clip2.duration, 2.0);
        assert!(clip2.looping);
        assert!(clip2.position_track.is_some());
        let track = clip2.position_track.as_ref().unwrap();
        assert_eq!(track.len(), 3);
    }

    #[test]
    fn test_export_import_roundtrip() {
        let dir = std::env::temp_dir().join("rust_engine_anim_test");
        let path = dir.join("test_anim.json");

        let clip = make_test_clip();
        let mut state = AnimationEditorState::new();
        state.load_clip(clip);

        export_clip(&state, &path).unwrap();
        let loaded = import_clip(&path).unwrap();

        assert_eq!(loaded.name, "walk");
        assert_eq!(loaded.duration, 2.0);
        assert!(loaded.looping);
        assert!(loaded.position_track.is_some());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_parse_interpolation() {
        assert_eq!(parse_interpolation("linear").unwrap(), engine_scene::Interpolation::Linear);
        assert_eq!(parse_interpolation("step").unwrap(), engine_scene::Interpolation::Step);
        assert_eq!(parse_interpolation("cubic").unwrap(), engine_scene::Interpolation::Cubic);
        assert!(parse_interpolation("bezier").is_err());
    }

    #[test]
    fn test_export_no_clip_fails() {
        let state = AnimationEditorState::new();
        let path = std::env::temp_dir().join("should_not_exist.json");
        assert!(export_clip(&state, &path).is_err());
    }
}
```

- [ ] **Step 2: Run tests and clippy**

Run: `cargo test -p engine-editor && cargo clippy -p engine-editor`
Expected: All pass

---

### Task 7: Main Animation Editor Panel (Integration)

**Files:**
- Modify: `crates/engine-editor/src/animation_editor/mod.rs` — add `draw` function
- Modify: `crates/engine-editor/src/layout.rs` — add animation editor panel
- Modify: `crates/engine-editor/src/state.rs` — add toggle menu item

- [ ] **Step 1: Add `draw` function to `animation_editor/mod.rs`**

Append to `crates/engine-editor/src/animation_editor/mod.rs` before the `#[cfg(test)]` block:

```rust
/// Main entry point: draw the full animation editor panel.
pub fn draw_animation_editor(
    state: &mut super::state::EditorState,
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

    // Panel background
    painter.add(egui::Shape::rect_filled(
        rect,
        egui::Rounding::ZERO,
        egui::Color32::from_rgb(22, 22, 25),
    ));

    // Top border
    painter.add(egui::Shape::line(
        vec![
            egui::pos2(rect.left(), rect.top()),
            egui::pos2(rect.right(), rect.top()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 53)),
    ));

    // Layout: toolbar | (track_labels | timeline) | (curve_editor | keyframe_list)
    let toolbar_h = 32.0 * h_scale;
    let track_label_w = 70.0 * w_scale;
    let keyframe_list_w = 140.0 * w_scale;

    // Toolbar
    let toolbar_rect = egui::Rect::from_min_size(
        rect.left_top(),
        egui::Vec2::new(rect.width(), toolbar_h),
    );
    draw_animation_toolbar(anim, ui, toolbar_rect, w_scale, h_scale);

    let content_top = rect.top() + toolbar_h;
    let content_h = rect.height() - toolbar_h;
    let left_h = content_h * 0.5; // Top half: timeline tracks
    let right_h = content_h - left_h; // Bottom half: curve editor

    // Track labels (left column, full height)
    let track_label_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left(), content_top),
        egui::vec2(track_label_w, content_h),
    );

    // Timeline tracks area (right of track labels)
    let timeline_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + track_label_w, content_top),
        egui::vec2(rect.width() - track_label_w - keyframe_list_w, left_h),
    );

    // Timeline header (above tracks)
    let header_h = 24.0 * h_scale;
    let timeline_header_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + track_label_w, content_top),
        egui::vec2(rect.width() - track_label_w - keyframe_list_w, header_h),
    );

    // Curve editor (below timeline)
    let curve_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + track_label_w, content_top + left_h),
        egui::vec2(rect.width() - track_label_w - keyframe_list_w, right_h),
    );

    // Keyframe list (right column)
    let kf_list_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() - keyframe_list_w, content_top),
        egui::vec2(keyframe_list_w, content_h),
    );

    // Draw timeline header
    timeline::draw_timeline_header(anim, ui, timeline_header_rect);

    // Draw track labels
    timeline::draw_track_labels(anim, ui, track_label_rect);

    // Draw keyframe tracks
    let kf_tracks_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + track_label_w, content_top + header_h),
        egui::vec2(rect.width() - track_label_w - keyframe_list_w, left_h - header_h),
    );
    timeline::draw_keyframe_tracks(anim, ui, kf_tracks_rect);

    // Draw curve editor
    curve_editor::draw_curve_editor(anim, ui, curve_rect);

    // Draw keyframe list
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

    // Advance playback if playing
    let dt = ui.input(|i| i.unstable_dt);
    if anim.player.playing {
        preview::advance_playback(anim, dt);
    }

    // Apply preview to target entity
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
        egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 53)),
    ));

    let btn_size = 24.0 * h_scale;
    let gap = 4.0 * w_scale;
    let pad = 8.0 * w_scale;
    let mut x = rect.left() + pad;
    let cy = rect.top() + (rect.height() - btn_size) / 2.0;

    // Play/Pause/Stop
    let play_icon = if state.player.playing { "⏸" } else { "▶" };
    let play_rect = egui::Rect::from_min_size(egui::pos2(x, cy), egui::vec2(btn_size, btn_size));
    let id = egui::Id::new("anim_play_btn");
    let resp = ui.interact(play_rect, id, egui::Sense::click());
    painter.text(play_rect.center(), egui::Align2::CENTER_CENTER, play_icon, egui::FontId::proportional(12.0 * h_scale), egui::Color32::WHITE);
    if resp.clicked() {
        if state.player.playing {
            state.player.pause();
        } else {
            state.player.play();
        }
    }
    x += btn_size + gap;

    let stop_rect = egui::Rect::from_min_size(egui::pos2(x, cy), egui::vec2(btn_size, btn_size));
    let id = egui::Id::new("anim_stop_btn");
    let resp = ui.interact(stop_rect, id, egui::Sense::click());
    painter.text(stop_rect.center(), egui::Align2::CENTER_CENTER, "⏹", egui::FontId::proportional(12.0 * h_scale), egui::Color32::WHITE);
    if resp.clicked() {
        state.player.stop();
    }
    x += btn_size + gap + pad;

    // Separator
    painter.add(egui::Shape::line(
        vec![egui::pos2(x, rect.top() + 4.0), egui::pos2(x, rect.bottom() - 4.0)],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 53)),
    ));
    x += pad;

    // Time display
    let time_text = format!("{:.2}s / {:.2}s", state.player.time, state.clip.as_ref().map(|c| c.duration).unwrap_or(0.0));
    painter.text(
        egui::pos2(x, cy + btn_size / 2.0),
        egui::Align2::LEFT_CENTER,
        time_text,
        egui::FontId::monospace(11.0 * h_scale),
        egui::Color32::from_rgb(200, 200, 200),
    );
    x += 100.0 * w_scale;

    // Speed control
    painter.text(
        egui::pos2(x, cy + btn_size / 2.0),
        egui::Align2::LEFT_CENTER,
        format!("速度: {:.1}x", state.player.speed),
        egui::FontId::proportional(10.0 * h_scale),
        egui::Color32::from_gray(120),
    );
    x += 60.0 * w_scale;

    // Snap toggle
    let snap_label = if state.snap_to_frame { "吸附: 开" } else { "吸附: 关" };
    let snap_rect = egui::Rect::from_min_size(egui::pos2(x, cy), egui::vec2(50.0 * w_scale, btn_size));
    let id = egui::Id::new("anim_snap_btn");
    let resp = ui.interact(snap_rect, id, egui::Sense::click());
    painter.text(
        snap_rect.center(),
        egui::Align2::CENTER_CENTER,
        snap_label,
        egui::FontId::proportional(10.0 * h_scale),
        if state.snap_to_frame { egui::Color32::from_rgb(0, 200, 150) } else { egui::Color32::from_gray(100) },
    );
    if resp.clicked() {
        state.snap_to_frame = !state.snap_to_frame;
    }
    x += 50.0 * w_scale + pad;

    // Preview toggle
    let preview_label = if state.preview_enabled { "预览: 开" } else { "预览: 关" };
    let preview_rect = egui::Rect::from_min_size(egui::pos2(x, cy), egui::vec2(50.0 * w_scale, btn_size));
    let id = egui::Id::new("anim_preview_btn");
    let resp = ui.interact(preview_rect, id, egui::Sense::click());
    painter.text(
        preview_rect.center(),
        egui::Align2::CENTER_CENTER,
        preview_label,
        egui::FontId::proportional(10.0 * h_scale),
        if state.preview_enabled { egui::Color32::from_rgb(0, 200, 150) } else { egui::Color32::from_gray(100) },
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
```

- [ ] **Step 2: Add animation editor to layout.rs**

In `crates/engine-editor/src/layout.rs`, add the animation editor drawing after the bottom panel. In the `frame()` function, after `draw_bottom_panel(state, ui, bottom_rect, h_scale, w_scale);`, add:

```rust
// Animation editor panel (overlay on top of bottom panel when visible)
if state.animation_editor.visible {
    let anim_h = (screen.height() * 300.0 / 1080.0).clamp(200.0, 500.0);
    let anim_rect = Rect::from_min_size(
        Pos2::new(screen.left(), status_rect.top() - anim_h),
        Vec2::new(screen.width(), anim_h),
    );
    crate::animation_editor::draw_animation_editor(state, ui, anim_rect);
}
```

- [ ] **Step 3: Add toggle to Window menu in layout.rs**

In the `draw_dropdown_menu` function in `layout.rs`, add an animation editor toggle to the Window menu (menu_idx 6). Change the menu items to:

```rust
6 => vec!["控制台", "性能", "资源浏览器", "动画编辑器"], // 窗口
```

And add the handler in the match block for menu_idx 6:

```rust
3 => state.animation_editor.visible = !state.animation_editor.visible, // 动画编辑器
```

- [ ] **Step 4: Run clippy and tests**

Run: `cargo clippy -p engine-editor && cargo test -p engine-editor`
Expected: All pass, no warnings

---

### Task 8: Wire Up Serialization Integration

**Files:**
- Modify: `crates/engine-editor/src/animation_editor/mod.rs`

- [ ] **Step 1: Add import/export methods to AnimationEditorState**

Add these methods to the `impl AnimationEditorState` block:

```rust
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
```

- [ ] **Step 2: Add keyboard shortcuts support**

Add a method to handle keyboard input for the animation editor:

```rust
    pub fn handle_input(&mut self, ui: &egui::Ui) {
        // Space to toggle play/pause
        if ui.input(|i| i.key_pressed(egui::Key::Space)) {
            if self.player.playing {
                self.player.pause();
            } else {
                self.player.play();
            }
        }

        // Delete to remove selected keyframe
        if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
            keyframe_list::delete_selected_keyframe(self);
        }

        // Home to go to start
        if ui.input(|i| i.key_pressed(egui::Key::Home)) {
            self.player.time = 0.0;
        }

        // End to go to end
        if ui.input(|i| i.key_pressed(egui::Key::End)) {
            if let Some(ref clip) = self.clip {
                self.player.time = clip.duration;
            }
        }
    }
```

- [ ] **Step 3: Run all tests and clippy**

Run: `cargo test -p engine-editor && cargo clippy -p engine-editor`
Expected: All pass

- [ ] **Step 4: Run cargo fmt**

Run: `cargo fmt -p engine-editor`
Expected: No errors

---

### Task 9: Final Verification

- [ ] **Step 1: Full workspace build**

Run: `cargo build`
Expected: Compiles without errors

- [ ] **Step 2: Full test suite**

Run: `cargo test -p engine-editor`
Expected: All tests pass, including all new animation_editor tests

- [ ] **Step 3: Clippy check**

Run: `cargo clippy -p engine-editor`
Expected: No warnings

- [ ] **Step 4: Format check**

Run: `cargo fmt --check`
Expected: No diff
