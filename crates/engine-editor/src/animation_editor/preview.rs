use super::{AnimationEditorState, TrackType};
use std::collections::HashMap;

fn sample_track(
    curves: &[super::ComponentCurve],
    track_type: TrackType,
    time: f32,
    default: f32,
) -> f32 {
    curves
        .iter()
        .find(|c| c.track_type == track_type && !c.keyframes.is_empty())
        .map(|c| c.sample(time))
        .unwrap_or(default)
}

/// Sample curves at the playhead and write into `node_transforms`.
///
/// Tracks with **no keyframes** leave the existing pose component unchanged
/// (avoid zeroing objects when a clip is partial). Callers should then write
/// through to the Unity World via `EditorState::apply_node_transform_to_world`.
pub fn apply_preview(
    state: &mut AnimationEditorState,
    node_transforms: &mut HashMap<u64, [f32; 9]>,
) {
    if !state.preview_enabled {
        return;
    }

    let target = match state.target_entity {
        Some(id) => id,
        None => return,
    };

    let time = state.player.time;

    // Seed from current snapshot so partial clips do not reset untracked axes.
    let current = node_transforms
        .get(&target)
        .copied()
        .unwrap_or([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);

    let pos_x = sample_track(&state.curves, TrackType::PositionX, time, current[0]);
    let pos_y = sample_track(&state.curves, TrackType::PositionY, time, current[1]);
    let pos_z = sample_track(&state.curves, TrackType::PositionZ, time, current[2]);

    let scl_x = sample_track(&state.curves, TrackType::ScaleX, time, current[6]);
    let scl_y = sample_track(&state.curves, TrackType::ScaleY, time, current[7]);
    let scl_z = sample_track(&state.curves, TrackType::ScaleZ, time, current[8]);

    let rot_x = sample_track(&state.curves, TrackType::RotationX, time, current[3]);
    let rot_y = sample_track(&state.curves, TrackType::RotationY, time, current[4]);
    let rot_z = sample_track(&state.curves, TrackType::RotationZ, time, current[5]);

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
    } else {
        node_transforms.insert(
            target,
            [pos_x, pos_y, pos_z, rot_x, rot_y, rot_z, scl_x, scl_y, scl_z],
        );
    }
}

pub fn advance_playback(state: &mut AnimationEditorState, dt: f32) {
    if let Some(ref clip) = state.clip {
        state.player.advance(dt, clip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_scene::keyframe::Interpolation;

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
    fn test_apply_preview_empty_curves_preserve_pose() {
        let mut state = AnimationEditorState::new();
        state.preview_enabled = true;
        state.target_entity = Some(7);
        // Default state has empty curves — pose must not drift to zero.
        let mut transforms = HashMap::new();
        transforms.insert(7, [1.0, 2.0, 3.0, 0.0, 0.0, 0.0, 2.0, 2.0, 2.0]);
        apply_preview(&mut state, &mut transforms);
        let t = &transforms[&7];
        assert_eq!(t[0], 1.0);
        assert_eq!(t[1], 2.0);
        assert_eq!(t[2], 3.0);
        assert_eq!(t[6], 2.0);
    }

    #[test]
    fn test_apply_preview_with_target() {
        let mut state = AnimationEditorState::new();
        state.preview_enabled = true;
        state.target_entity = Some(1);
        state.curves.push(super::super::ComponentCurve::new(
            super::super::TrackType::PositionX,
        ));
        state.curves[0].keyframes.push(super::super::CurveKeyframe {
            time: 0.0,
            value: 5.0,
            interpolation: Interpolation::Linear,
            tangent_in: 0.0,
            tangent_out: 0.0,
        });
        let mut transforms = HashMap::new();
        transforms.insert(1, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        apply_preview(&mut state, &mut transforms);
        assert_eq!(transforms[&1][0], 5.0);
        // Scale track empty — snapshot scale preserved.
        assert_eq!(transforms[&1][6], 1.0);
    }
}
