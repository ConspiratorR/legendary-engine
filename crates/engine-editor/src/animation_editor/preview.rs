use super::AnimationEditorState;
use std::collections::HashMap;

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

    let pos_x = state
        .curves
        .iter()
        .find(|c| c.track_type == super::TrackType::PositionX)
        .map(|c| c.sample(time))
        .unwrap_or(0.0);
    let pos_y = state
        .curves
        .iter()
        .find(|c| c.track_type == super::TrackType::PositionY)
        .map(|c| c.sample(time))
        .unwrap_or(0.0);
    let pos_z = state
        .curves
        .iter()
        .find(|c| c.track_type == super::TrackType::PositionZ)
        .map(|c| c.sample(time))
        .unwrap_or(0.0);

    let scl_x = state
        .curves
        .iter()
        .find(|c| c.track_type == super::TrackType::ScaleX)
        .map(|c| c.sample(time))
        .unwrap_or(1.0);
    let scl_y = state
        .curves
        .iter()
        .find(|c| c.track_type == super::TrackType::ScaleY)
        .map(|c| c.sample(time))
        .unwrap_or(1.0);
    let scl_z = state
        .curves
        .iter()
        .find(|c| c.track_type == super::TrackType::ScaleZ)
        .map(|c| c.sample(time))
        .unwrap_or(1.0);

    let rot_x = state
        .curves
        .iter()
        .find(|c| c.track_type == super::TrackType::RotationX)
        .map(|c| c.sample(time))
        .unwrap_or(0.0);
    let rot_y = state
        .curves
        .iter()
        .find(|c| c.track_type == super::TrackType::RotationY)
        .map(|c| c.sample(time))
        .unwrap_or(0.0);
    let rot_z = state
        .curves
        .iter()
        .find(|c| c.track_type == super::TrackType::RotationZ)
        .map(|c| c.sample(time))
        .unwrap_or(0.0);

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
    fn test_apply_preview_with_target() {
        let mut state = AnimationEditorState::new();
        state.create_new_clip("test".to_string(), 1.0);
        state.preview_enabled = true;
        state.target_entity = Some(1);

        state.curves[0].keyframes.push(super::super::CurveKeyframe {
            time: 0.0,
            value: 5.0,
            interpolation: Interpolation::Linear,
            tangent_in: 0.0,
            tangent_out: 0.0,
        });

        let mut transforms = HashMap::new();
        transforms.insert(1, [0.0; 9]);

        state.player.time = 0.0;
        apply_preview(&mut state, &mut transforms);
        assert_eq!(transforms[&1][0], 5.0);
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
