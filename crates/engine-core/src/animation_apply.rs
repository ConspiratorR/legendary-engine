//! Apply sampled keyframe poses onto a Unity World GameObject (phase 11 S4).
//!
//! Keyframe **format** remains `engine_scene::keyframe` (clip/interpolation).
//! Pose **authority** when applying is `engine_core::World`:
//! - feature on → `with_ecs_transform_mut` (ECS primary + array cache)
//! - feature off → array primary via the same helper's fallback
//!
//! `engine_scene::transform::Transform` is a scene-graph fallback for legacy
//! editor paths only — do not treat it as gameplay pose authority.

use crate::gameobject::GameObjectHandle;
use crate::world::World;
use engine_scene::keyframe::AnimationClip;

/// Sample `clip` at `time` and write local pose onto `handle`.
///
/// Tracks that are absent **or empty** (`Some(vec![])`) are left unchanged.
/// Returns `true` if any track wrote.
pub fn apply_clip_pose(
    world: &mut World,
    handle: GameObjectHandle,
    clip: &AnimationClip,
    time: f32,
) -> bool {
    if !world.is_valid(handle) {
        return false;
    }
    let tracked = |present: bool| present;
    let has_pos = tracked(clip.position_track.as_ref().is_some_and(|t| !t.is_empty()));
    let has_rot = tracked(clip.rotation_track.as_ref().is_some_and(|t| !t.is_empty()));
    let has_scale = tracked(clip.scale_track.as_ref().is_some_and(|t| !t.is_empty()));
    if !has_pos && !has_rot && !has_scale {
        return false;
    }
    let pos = if has_pos {
        clip.sample_position(time)
    } else {
        None
    };
    let rot = if has_rot {
        clip.sample_rotation(time)
    } else {
        None
    };
    let scale = if has_scale {
        clip.sample_scale(time)
    } else {
        None
    };
    if pos.is_none() && rot.is_none() && scale.is_none() {
        return false;
    }
    let _ = world.with_ecs_transform_mut(handle, |t| {
        if let Some(p) = pos {
            t.SetLocalPosition(p);
        }
        if let Some(r) = rot {
            t.SetLocalRotation(r);
        }
        if let Some(s) = scale {
            t.SetLocalScale(s);
        }
    });
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_math::{Quat, Vec3};
    use engine_scene::keyframe::{AnimationClip, RotationKeyframe, Vec3Keyframe};

    #[test]
    fn test_apply_clip_pose_position_track() {
        let mut world = World::new();
        let go = world.CreateGameObject("Anim");
        let clip = AnimationClip::new("walk", 1.0).with_position_track(vec![
            Vec3Keyframe::linear(0.0, Vec3::ZERO),
            Vec3Keyframe::linear(1.0, Vec3::new(10.0, 0.0, 0.0)),
        ]);
        assert!(apply_clip_pose(&mut world, go, &clip, 0.5));
        let t = world.GetTransform(go).expect("transform");
        assert!((t.LocalPosition().x - 5.0).abs() < 0.01);
        // Array cache / array authority still sees the write.
        let arr = world.GetTransformArray(go).expect("array transform");
        assert!((arr.LocalPosition().x - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_apply_clip_pose_leaves_missing_tracks() {
        let mut world = World::new();
        let go = world.CreateGameObject("ScaleOnly");
        world.SetLocalScale(go, Vec3::new(3.0, 3.0, 3.0));
        let clip = AnimationClip::new("rot", 1.0).with_rotation_track(vec![
            RotationKeyframe::linear(0.0, Quat::IDENTITY),
            RotationKeyframe::linear(1.0, Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        ]);
        assert!(apply_clip_pose(&mut world, go, &clip, 0.0));
        let t = world.GetTransform(go).expect("transform");
        assert_eq!(t.LocalScale(), Vec3::new(3.0, 3.0, 3.0));
    }

    #[test]
    fn test_apply_clip_pose_empty_is_noop() {
        let mut world = World::new();
        let go = world.CreateGameObject("Empty");
        world.SetLocalPosition(go, Vec3::new(1.0, 2.0, 3.0));
        let clip = AnimationClip::new("empty", 1.0);
        assert!(!apply_clip_pose(&mut world, go, &clip, 0.5));
        assert_eq!(
            world.GetTransform(go).unwrap().LocalPosition(),
            Vec3::new(1.0, 2.0, 3.0)
        );
    }

    #[test]
    fn test_apply_clip_pose_empty_but_present_track_is_noop() {
        let mut world = World::new();
        let go = world.CreateGameObject("EmptyTrack");
        world.SetLocalPosition(go, Vec3::new(4.0, 5.0, 6.0));
        let mut clip = AnimationClip::new("empty_pos", 1.0);
        clip.position_track = Some(Vec::new());
        clip.scale_track = Some(Vec::new());
        assert!(!apply_clip_pose(&mut world, go, &clip, 0.25));
        assert_eq!(
            world.GetTransform(go).unwrap().LocalPosition(),
            Vec3::new(4.0, 5.0, 6.0)
        );
    }
}
