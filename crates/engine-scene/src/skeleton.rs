use std::collections::HashMap;

use engine_math::{Mat4, Quat, Vec3};

use crate::keyframe::{RotationKeyframe, Vec3Keyframe};

/// A single joint (bone) in a skeleton.
#[derive(Debug, Clone)]
pub struct Joint {
    pub id: u32,
    pub name: String,
    /// Index into the skeleton's joint list, or `None` for the root.
    pub parent_index: Option<usize>,
    /// Transforms mesh vertices from mesh space into this joint's local space at bind pose.
    pub inverse_bind_pose: Mat4,
    /// The joint's local transform relative to its parent at bind pose.
    pub local_bind_pose: Mat4,
}

/// A hierarchical collection of joints forming a skeleton.
#[derive(Debug, Clone)]
pub struct Skeleton {
    joints: Vec<Joint>,
    joint_names: HashMap<String, usize>,
}

impl Skeleton {
    pub fn new(joints: Vec<Joint>) -> Self {
        let mut joint_names = HashMap::with_capacity(joints.len());
        for (i, joint) in joints.iter().enumerate() {
            joint_names.insert(joint.name.clone(), i);
        }
        Self {
            joints,
            joint_names,
        }
    }

    pub fn joint_count(&self) -> usize {
        self.joints.len()
    }

    pub fn joints(&self) -> &[Joint] {
        &self.joints
    }

    /// Look up a joint index by name.
    pub fn find_joint(&self, name: &str) -> Option<usize> {
        self.joint_names.get(name).copied()
    }

    /// Return the chain of ancestor indices from `joint_index` up to (and including) the root.
    /// Order is `[joint_index, parent, grandparent, …, root]`.
    pub fn parent_chain(&self, joint_index: usize) -> Vec<usize> {
        let mut chain = Vec::new();
        let mut current = Some(joint_index);
        while let Some(idx) = current {
            chain.push(idx);
            current = self.joints[idx].parent_index;
        }
        chain
    }
}

/// Per-vertex skinning data referencing a skeleton.
#[derive(Debug, Clone)]
pub struct Skin {
    /// Name of the skeleton this skin references.
    pub skeleton_name: String,
    /// Per-vertex joint indices (up to 4 influences per vertex).
    pub joint_indices: Vec<[u16; 4]>,
    /// Per-vertex joint weights (matching `joint_indices`).
    pub joint_weights: Vec<[f32; 4]>,
}

impl Skin {
    pub fn new(skeleton_name: impl Into<String>, vertex_count: usize) -> Self {
        Self {
            skeleton_name: skeleton_name.into(),
            joint_indices: vec![[0; 4]; vertex_count],
            joint_weights: vec![[0.0; 4]; vertex_count],
        }
    }

    pub fn vertex_count(&self) -> usize {
        self.joint_indices.len()
    }
}

/// Local transform for a single joint (position, rotation, scale).
#[derive(Debug, Clone, Copy)]
pub struct JointTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for JointTransform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl JointTransform {
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// Extract a `JointTransform` from a matrix (assumes no shear).
    pub fn from_matrix(m: Mat4) -> Self {
        let (scale, rotation, translation) = m.to_scale_rotation_translation();
        Self {
            translation,
            rotation,
            scale,
        }
    }

    /// Linearly interpolate between two joint transforms.
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            translation: self.translation + (other.translation - self.translation) * t,
            rotation: self.rotation.slerp(other.rotation, t),
            scale: self.scale + (other.scale - self.scale) * t,
        }
    }
}

/// A per-joint animation clip mapping joint names to animation tracks.
#[derive(Debug, Clone)]
pub struct SkeletalAnimationClip {
    pub name: String,
    pub duration: f32,
    pub looping: bool,
    /// Maps joint name → position track.
    pub position_tracks: HashMap<String, Vec<Vec3Keyframe>>,
    /// Maps joint name → rotation track.
    pub rotation_tracks: HashMap<String, Vec<RotationKeyframe>>,
    /// Maps joint name → scale track.
    pub scale_tracks: HashMap<String, Vec<Vec3Keyframe>>,
}

impl SkeletalAnimationClip {
    pub fn new(name: impl Into<String>, duration: f32) -> Self {
        Self {
            name: name.into(),
            duration,
            looping: false,
            position_tracks: HashMap::new(),
            rotation_tracks: HashMap::new(),
            scale_tracks: HashMap::new(),
        }
    }

    pub fn looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    pub fn with_position_track(
        mut self,
        joint_name: impl Into<String>,
        track: Vec<Vec3Keyframe>,
    ) -> Self {
        self.position_tracks.insert(joint_name.into(), track);
        self
    }

    pub fn with_rotation_track(
        mut self,
        joint_name: impl Into<String>,
        track: Vec<RotationKeyframe>,
    ) -> Self {
        self.rotation_tracks.insert(joint_name.into(), track);
        self
    }

    pub fn with_scale_track(
        mut self,
        joint_name: impl Into<String>,
        track: Vec<Vec3Keyframe>,
    ) -> Self {
        self.scale_tracks.insert(joint_name.into(), track);
        self
    }

    /// Sample the local transform for a given joint at `time`.
    pub fn sample_joint(&self, joint_name: &str, time: f32) -> JointTransform {
        let translation = self
            .position_tracks
            .get(joint_name)
            .map(|track| sample_vec3_track(track, time, self.looping, self.duration))
            .unwrap_or(Vec3::ZERO);

        let rotation = self
            .rotation_tracks
            .get(joint_name)
            .map(|track| sample_rotation_track(track, time, self.looping, self.duration))
            .unwrap_or(Quat::IDENTITY);

        let scale = self
            .scale_tracks
            .get(joint_name)
            .map(|track| sample_vec3_track(track, time, self.looping, self.duration))
            .unwrap_or(Vec3::ONE);

        JointTransform {
            translation,
            rotation,
            scale,
        }
    }
}

/// Player that advances a skeletal animation and produces a joint matrix palette.
#[derive(Debug, Clone)]
pub struct SkeletalAnimationPlayer {
    pub clip_name: String,
    pub time: f32,
    pub speed: f32,
    pub playing: bool,
    pub loop_count: i32,
    /// Per-joint local transforms for the current pose (indexed by joint index).
    local_pose: Vec<JointTransform>,
}

impl SkeletalAnimationPlayer {
    pub fn new(clip_name: impl Into<String>, skeleton: &Skeleton) -> Self {
        let local_pose = skeleton
            .joints()
            .iter()
            .map(|joint| JointTransform::from_matrix(joint.local_bind_pose))
            .collect();
        Self {
            clip_name: clip_name.into(),
            time: 0.0,
            speed: 1.0,
            playing: true,
            loop_count: 0,
            local_pose,
        }
    }

    pub fn play(&mut self) {
        self.playing = true;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn stop(&mut self) {
        self.playing = false;
        self.time = 0.0;
        self.loop_count = 0;
    }

    pub fn local_pose(&self) -> &[JointTransform] {
        &self.local_pose
    }

    /// Advance time and sample the clip, storing per-joint local transforms.
    pub fn advance(&mut self, delta: f32, clip: &SkeletalAnimationClip) {
        if !self.playing {
            return;
        }
        self.time += delta * self.speed;
        if clip.looping {
            while self.time >= clip.duration {
                self.time -= clip.duration;
                self.loop_count += 1;
            }
            while self.time < 0.0 {
                self.time += clip.duration;
                self.loop_count -= 1;
            }
        } else {
            self.time = self.time.clamp(0.0, clip.duration);
        }
    }

    /// Sample local pose from the clip for every joint in the skeleton.
    pub fn sample_pose(&mut self, skeleton: &Skeleton, clip: &SkeletalAnimationClip) {
        for (i, joint) in skeleton.joints().iter().enumerate() {
            if i < self.local_pose.len() {
                self.local_pose[i] = clip.sample_joint(&joint.name, self.time);
            }
        }
    }

    /// Compute the final matrix palette for GPU skinning.
    ///
    /// For each joint: `final_matrix[j] = global_transform[j] * inverse_bind_pose[j]`
    pub fn compute_matrix_palette(&self, skeleton: &Skeleton) -> Vec<Mat4> {
        let joint_count = skeleton.joint_count();
        let mut world_transforms = vec![Mat4::IDENTITY; joint_count];

        // Traverse in parent-first order so parents are always computed before children.
        for i in 0..joint_count {
            let local = self.local_pose[i].to_matrix();
            world_transforms[i] = match skeleton.joints()[i].parent_index {
                Some(parent) => world_transforms[parent] * local,
                None => local,
            };
        }

        // final = world * inverse_bind_pose
        world_transforms
            .into_iter()
            .zip(skeleton.joints().iter())
            .map(|(world, joint)| world * joint.inverse_bind_pose)
            .collect()
    }
}

/// Blend two skeletal animation clips and return the blended local pose.
///
/// For each joint in the skeleton, samples both clips at their respective times
/// and lerps the resulting `JointTransform`s by `alpha` (0.0 = clip_a, 1.0 = clip_b).
pub fn blend_skeletal_poses(
    skeleton: &Skeleton,
    clip_a: &SkeletalAnimationClip,
    time_a: f32,
    clip_b: &SkeletalAnimationClip,
    time_b: f32,
    alpha: f32,
) -> Vec<JointTransform> {
    skeleton
        .joints()
        .iter()
        .map(|joint| {
            let a = clip_a.sample_joint(&joint.name, time_a);
            let b = clip_b.sample_joint(&joint.name, time_b);
            a.lerp(&b, alpha)
        })
        .collect()
}

/// Compute a matrix palette from an arbitrary set of local poses (e.g. after blending).
pub fn compute_palette_from_pose(skeleton: &Skeleton, local_pose: &[JointTransform]) -> Vec<Mat4> {
    let joint_count = skeleton.joint_count();
    let mut world_transforms = vec![Mat4::IDENTITY; joint_count];

    for i in 0..joint_count {
        let local = local_pose[i].to_matrix();
        world_transforms[i] = match skeleton.joints()[i].parent_index {
            Some(parent) => world_transforms[parent] * local,
            None => local,
        };
    }

    world_transforms
        .into_iter()
        .zip(skeleton.joints().iter())
        .map(|(world, joint)| world * joint.inverse_bind_pose)
        .collect()
}

// ── Interpolation helpers (reused from keyframe module logic) ────────

fn sample_vec3_track(track: &[Vec3Keyframe], time: f32, looping: bool, duration: f32) -> Vec3 {
    if track.is_empty() {
        return Vec3::ZERO;
    }
    if track.len() == 1 {
        return track[0].value;
    }

    let t = if looping {
        time.rem_euclid(duration)
    } else {
        time.clamp(track[0].time, track.last().unwrap().time)
    };

    let mut i = 0;
    while i < track.len() - 1 && track[i + 1].time <= t {
        i += 1;
    }
    if i >= track.len() - 1 {
        return track.last().unwrap().value;
    }

    let k0 = &track[i];
    let k1 = &track[i + 1];
    let dt = k1.time - k0.time;
    if dt < 1e-6 {
        return k0.value;
    }

    let alpha = (t - k0.time) / dt;
    k0.value + (k1.value - k0.value) * alpha
}

fn sample_rotation_track(
    track: &[RotationKeyframe],
    time: f32,
    looping: bool,
    duration: f32,
) -> Quat {
    if track.is_empty() {
        return Quat::IDENTITY;
    }
    if track.len() == 1 {
        return track[0].value;
    }

    let t = if looping {
        time.rem_euclid(duration)
    } else {
        time.clamp(track[0].time, track.last().unwrap().time)
    };

    let mut i = 0;
    while i < track.len() - 1 && track[i + 1].time <= t {
        i += 1;
    }
    if i >= track.len() - 1 {
        return track.last().unwrap().value;
    }

    let k0 = &track[i];
    let k1 = &track[i + 1];
    let dt = k1.time - k0.time;
    if dt < 1e-6 {
        return k0.value;
    }

    let alpha = (t - k0.time) / dt;
    k0.value.slerp(k1.value, alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-4;

    fn approx_mat4(a: Mat4, b: Mat4) -> bool {
        for row in 0..4 {
            for col in 0..4 {
                if (a.row(row)[col] - b.row(row)[col]).abs() > EPS {
                    return false;
                }
            }
        }
        true
    }

    /// Build a simple 2-joint chain: root → child.
    /// Root at (0,0,0), child translated (10,0,0) in root space.
    fn two_joint_skeleton() -> Skeleton {
        Skeleton::new(vec![
            Joint {
                id: 0,
                name: "root".to_string(),
                parent_index: None,
                inverse_bind_pose: Mat4::IDENTITY,
                local_bind_pose: Mat4::IDENTITY,
            },
            Joint {
                id: 1,
                name: "child".to_string(),
                parent_index: Some(0),
                inverse_bind_pose: Mat4::from_translation(Vec3::new(-10.0, 0.0, 0.0)),
                local_bind_pose: Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0)),
            },
        ])
    }

    #[test]
    fn test_skeleton_construction_and_lookup() {
        let skeleton = two_joint_skeleton();
        assert_eq!(skeleton.joint_count(), 2);
        assert_eq!(skeleton.find_joint("root"), Some(0));
        assert_eq!(skeleton.find_joint("child"), Some(1));
        assert_eq!(skeleton.find_joint("missing"), None);
    }

    #[test]
    fn test_parent_chain() {
        let skeleton = two_joint_skeleton();
        let chain = skeleton.parent_chain(1);
        assert_eq!(chain, vec![1, 0]);

        let root_chain = skeleton.parent_chain(0);
        assert_eq!(root_chain, vec![0]);
    }

    #[test]
    fn test_matrix_palette_identity_at_bind_pose() {
        let skeleton = two_joint_skeleton();
        let player = SkeletalAnimationPlayer::new("bind", &skeleton);
        // No animation applied — local pose is default (identity).
        let palette = player.compute_matrix_palette(&skeleton);

        // Root: world=IDENTITY, inverse_bind=IDENTITY → final=IDENTITY
        assert!(approx_mat4(palette[0], Mat4::IDENTITY));

        // Child: world=from_translation(10,0,0), inverse_bind=from_translation(-10,0,0)
        // → final = translate(10) * translate(-10) = IDENTITY
        assert!(approx_mat4(palette[1], Mat4::IDENTITY));
    }

    #[test]
    fn test_matrix_palette_with_transform() {
        let skeleton = two_joint_skeleton();
        let mut player = SkeletalAnimationPlayer::new("test", &skeleton);

        // Move root forward by 5 units.
        player.local_pose[0] = JointTransform {
            translation: Vec3::new(5.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };

        let palette = player.compute_matrix_palette(&skeleton);

        // Root: world=translate(5), inverse_bind=IDENTITY → final=translate(5)
        let expected_root = Mat4::from_translation(Vec3::new(5.0, 0.0, 0.0));
        assert!(approx_mat4(palette[0], expected_root));

        // Child: world = translate(5) * translate(10) = translate(15)
        // final = translate(15) * translate(-10) = translate(5)
        let expected_child = Mat4::from_translation(Vec3::new(5.0, 0.0, 0.0));
        assert!(approx_mat4(palette[1], expected_child));
    }

    #[test]
    fn test_skeletal_animation_player_advance() {
        let skeleton = two_joint_skeleton();
        let clip = SkeletalAnimationClip::new("walk", 2.0)
            .looping(true)
            .with_position_track(
                "root",
                vec![
                    Vec3Keyframe::linear(0.0, Vec3::ZERO),
                    Vec3Keyframe::linear(2.0, Vec3::new(10.0, 0.0, 0.0)),
                ],
            );

        let mut player = SkeletalAnimationPlayer::new("walk", &skeleton);
        player.advance(0.5, &clip);
        assert!((player.time - 0.5).abs() < EPS);

        player.advance(2.0, &clip);
        assert!((player.time - 0.5).abs() < EPS);
        assert_eq!(player.loop_count, 1);
    }

    #[test]
    fn test_skeletal_animation_sample_and_palette() {
        let skeleton = two_joint_skeleton();
        let clip = SkeletalAnimationClip::new("move", 1.0).with_position_track(
            "child",
            vec![
                Vec3Keyframe::linear(0.0, Vec3::ZERO),
                Vec3Keyframe::linear(1.0, Vec3::new(5.0, 0.0, 0.0)),
            ],
        );

        let mut player = SkeletalAnimationPlayer::new("move", &skeleton);
        player.time = 0.5;
        player.sample_pose(&skeleton, &clip);

        // Child local translation should be (2.5, 0, 0) at t=0.5
        let child_local = player.local_pose()[1];
        assert!((child_local.translation.x - 2.5).abs() < EPS);

        let palette = player.compute_matrix_palette(&skeleton);
        // Child world = translate(2.5), inverse_bind = translate(-10)
        // final = translate(2.5) * translate(-10) = translate(-7.5)
        let expected = Mat4::from_translation(Vec3::new(-7.5, 0.0, 0.0));
        assert!(approx_mat4(palette[1], expected));
    }

    #[test]
    fn test_blend_skeletal_poses() {
        let skeleton = two_joint_skeleton();

        let clip_a = SkeletalAnimationClip::new("a", 1.0).with_position_track(
            "root",
            vec![
                Vec3Keyframe::linear(0.0, Vec3::ZERO),
                Vec3Keyframe::linear(1.0, Vec3::new(10.0, 0.0, 0.0)),
            ],
        );

        let clip_b = SkeletalAnimationClip::new("b", 1.0).with_position_track(
            "root",
            vec![
                Vec3Keyframe::linear(0.0, Vec3::new(10.0, 0.0, 0.0)),
                Vec3Keyframe::linear(1.0, Vec3::ZERO),
            ],
        );

        // At t=0.5: clip_a → (5,0,0), clip_b → (5,0,0), blend 50/50 → (5,0,0)
        let blended = blend_skeletal_poses(&skeleton, &clip_a, 0.5, &clip_b, 0.5, 0.5);
        assert!((blended[0].translation.x - 5.0).abs() < EPS);

        // At t=0.0: clip_a → (0,0,0), clip_b → (10,0,0), blend 50/50 → (5,0,0)
        let blended2 = blend_skeletal_poses(&skeleton, &clip_a, 0.0, &clip_b, 0.0, 0.5);
        assert!((blended2[0].translation.x - 5.0).abs() < EPS);

        // Full weight on clip_a
        let blended3 = blend_skeletal_poses(&skeleton, &clip_a, 0.5, &clip_b, 0.5, 0.0);
        assert!((blended3[0].translation.x - 5.0).abs() < EPS);
    }

    #[test]
    fn test_skin_vertex_count() {
        let skin = Skin::new("humanoid", 100);
        assert_eq!(skin.vertex_count(), 100);
        assert_eq!(skin.skeleton_name, "humanoid");
    }

    #[test]
    fn test_joint_transform_lerp() {
        let a = JointTransform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        let b = JointTransform {
            translation: Vec3::new(10.0, 0.0, 0.0),
            rotation: Quat::from_rotation_y(std::f32::consts::PI),
            scale: Vec3::new(2.0, 2.0, 2.0),
        };
        let mid = a.lerp(&b, 0.5);
        assert!((mid.translation.x - 5.0).abs() < EPS);
        assert!((mid.scale.x - 1.5).abs() < EPS);
    }

    #[test]
    fn test_compute_palette_from_pose() {
        let skeleton = two_joint_skeleton();
        // Use bind-pose local transforms (root=ID, child=translate(10,0,0)).
        let pose: Vec<JointTransform> = skeleton
            .joints()
            .iter()
            .map(|j| JointTransform::from_matrix(j.local_bind_pose))
            .collect();
        let palette = compute_palette_from_pose(&skeleton, &pose);

        // At bind pose: final = world * inverse_bind_pose = IDENTITY for both joints.
        assert!(approx_mat4(palette[0], Mat4::IDENTITY));
        assert!(approx_mat4(palette[1], Mat4::IDENTITY));
    }

    #[test]
    fn test_skeletal_clip_sample_rotation() {
        let clip = SkeletalAnimationClip::new("rot", 1.0).with_rotation_track(
            "root",
            vec![
                RotationKeyframe::linear(0.0, Quat::IDENTITY),
                RotationKeyframe::linear(1.0, Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
            ],
        );

        let t = clip.sample_joint("root", 0.5);
        let (_, angle) = t.rotation.to_axis_angle();
        assert!((angle - std::f32::consts::FRAC_PI_4).abs() < 0.05);
    }
}
