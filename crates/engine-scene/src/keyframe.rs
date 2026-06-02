use engine_math::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Interpolation method between keyframes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Interpolation {
    /// Linear interpolation.
    Linear,
    /// Step function (no interpolation — snap to next value).
    Step,
    /// Cubic spline interpolation (uses tangent values).
    Cubic,
}

/// A single keyframe for a float value.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FloatKeyframe {
    pub time: f32,
    pub value: f32,
    pub interpolation: Interpolation,
    /// Cubic tangent in (used with Cubic interpolation).
    pub tangent_in: f32,
    /// Cubic tangent out.
    pub tangent_out: f32,
}

impl FloatKeyframe {
    pub fn linear(time: f32, value: f32) -> Self {
        Self {
            time,
            value,
            interpolation: Interpolation::Linear,
            tangent_in: 0.0,
            tangent_out: 0.0,
        }
    }

    pub fn step(time: f32, value: f32) -> Self {
        Self {
            time,
            value,
            interpolation: Interpolation::Step,
            tangent_in: 0.0,
            tangent_out: 0.0,
        }
    }

    pub fn cubic(time: f32, value: f32, tangent_in: f32, tangent_out: f32) -> Self {
        Self {
            time,
            value,
            interpolation: Interpolation::Cubic,
            tangent_in,
            tangent_out,
        }
    }
}

/// A single keyframe for a Vec3 value.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Vec3Keyframe {
    pub time: f32,
    pub value: Vec3,
    pub interpolation: Interpolation,
    pub tangent_in: Vec3,
    pub tangent_out: Vec3,
}

impl Vec3Keyframe {
    pub fn linear(time: f32, value: Vec3) -> Self {
        Self {
            time,
            value,
            interpolation: Interpolation::Linear,
            tangent_in: Vec3::ZERO,
            tangent_out: Vec3::ZERO,
        }
    }

    pub fn step(time: f32, value: Vec3) -> Self {
        Self {
            time,
            value,
            interpolation: Interpolation::Step,
            tangent_in: Vec3::ZERO,
            tangent_out: Vec3::ZERO,
        }
    }
}

/// A single keyframe for a rotation (quaternion).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RotationKeyframe {
    pub time: f32,
    pub value: Quat,
    pub interpolation: Interpolation,
}

impl RotationKeyframe {
    pub fn linear(time: f32, value: Quat) -> Self {
        Self {
            time,
            value,
            interpolation: Interpolation::Linear,
        }
    }

    pub fn step(time: f32, value: Quat) -> Self {
        Self {
            time,
            value,
            interpolation: Interpolation::Step,
        }
    }
}

/// A transform animation clip containing position, rotation, and scale tracks.
///
/// Each track is optional — only the tracks that are present will be applied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationClip {
    pub name: String,
    pub duration: f32,
    pub looping: bool,
    pub position_track: Option<Vec<Vec3Keyframe>>,
    pub rotation_track: Option<Vec<RotationKeyframe>>,
    pub scale_track: Option<Vec<Vec3Keyframe>>,
}

impl AnimationClip {
    pub fn new(name: impl Into<String>, duration: f32) -> Self {
        Self {
            name: name.into(),
            duration,
            looping: false,
            position_track: None,
            rotation_track: None,
            scale_track: None,
        }
    }

    pub fn looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    pub fn with_position_track(mut self, track: Vec<Vec3Keyframe>) -> Self {
        self.position_track = Some(track);
        self
    }

    pub fn with_rotation_track(mut self, track: Vec<RotationKeyframe>) -> Self {
        self.rotation_track = Some(track);
        self
    }

    pub fn with_scale_track(mut self, track: Vec<Vec3Keyframe>) -> Self {
        self.scale_track = Some(track);
        self
    }

    /// Sample the position at a given time.
    pub fn sample_position(&self, time: f32) -> Option<Vec3> {
        self.position_track
            .as_ref()
            .map(|track| sample_vec3_track(track, time, self.looping, self.duration))
    }

    /// Sample the rotation at a given time.
    pub fn sample_rotation(&self, time: f32) -> Option<Quat> {
        self.rotation_track
            .as_ref()
            .map(|track| sample_rotation_track(track, time, self.looping, self.duration))
    }

    /// Sample the scale at a given time.
    pub fn sample_scale(&self, time: f32) -> Option<Vec3> {
        self.scale_track
            .as_ref()
            .map(|track| sample_vec3_track(track, time, self.looping, self.duration))
    }
}

/// Animation player component — tracks playback state for an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationPlayer {
    pub clip_name: String,
    pub time: f32,
    pub speed: f32,
    pub playing: bool,
    pub loop_count: i32,
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self {
            clip_name: String::new(),
            time: 0.0,
            speed: 1.0,
            playing: false,
            loop_count: 0,
        }
    }
}

impl AnimationPlayer {
    pub fn new(clip_name: impl Into<String>) -> Self {
        Self {
            clip_name: clip_name.into(),
            playing: true,
            ..Default::default()
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

    /// Advance time by delta. Returns the current time (wrapped if looping).
    pub fn advance(&mut self, delta: f32, clip: &AnimationClip) -> f32 {
        if !self.playing {
            return self.time;
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
        self.time
    }
}

// ── Interpolation helpers ────────────────────────────────────────────

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

    // Find the two keyframes surrounding time t
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

    match k0.interpolation {
        Interpolation::Linear => k0.value.slerp(k1.value, alpha),
        Interpolation::Step => k0.value,
        Interpolation::Cubic => {
            // For cubic, fall back to slerp (proper cubic quaternion is complex)
            k0.value.slerp(k1.value, alpha)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-4;

    fn approx_eq_vec3(a: Vec3, b: Vec3) -> bool {
        (a.x - b.x).abs() < EPS && (a.y - b.y).abs() < EPS && (a.z - b.z).abs() < EPS
    }

    #[test]
    fn test_float_keyframe_linear() {
        let k = FloatKeyframe::linear(0.0, 10.0);
        assert_eq!(k.interpolation, Interpolation::Linear);
        assert!((k.value - 10.0).abs() < EPS);
    }

    #[test]
    fn test_animation_clip_sample_position() {
        let clip = AnimationClip::new("walk", 1.0).with_position_track(vec![
            Vec3Keyframe::linear(0.0, Vec3::ZERO),
            Vec3Keyframe::linear(1.0, Vec3::new(10.0, 0.0, 0.0)),
        ]);

        let pos = clip.sample_position(0.5).unwrap();
        assert!(approx_eq_vec3(pos, Vec3::new(5.0, 0.0, 0.0)));
    }

    #[test]
    fn test_animation_clip_sample_rotation() {
        let clip = AnimationClip::new("spin", 1.0).with_rotation_track(vec![
            RotationKeyframe::linear(0.0, Quat::IDENTITY),
            RotationKeyframe::linear(1.0, Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        ]);

        let rot = clip.sample_rotation(0.5).unwrap();
        // Should be ~45 degrees around Y
        let expected_angle = std::f32::consts::FRAC_PI_4;
        let (_, angle) = rot.to_axis_angle();
        assert!((angle - expected_angle).abs() < 0.05);
    }

    #[test]
    fn test_animation_clip_sample_scale() {
        let clip = AnimationClip::new("scale", 1.0).with_scale_track(vec![
            Vec3Keyframe::linear(0.0, Vec3::ONE),
            Vec3Keyframe::linear(1.0, Vec3::new(2.0, 2.0, 2.0)),
        ]);

        let scale = clip.sample_scale(0.5).unwrap();
        assert!(approx_eq_vec3(scale, Vec3::new(1.5, 1.5, 1.5)));
    }

    #[test]
    fn test_animation_clip_looping() {
        let clip = AnimationClip::new("loop", 1.0)
            .looping(true)
            .with_position_track(vec![
                Vec3Keyframe::linear(0.0, Vec3::ZERO),
                Vec3Keyframe::linear(1.0, Vec3::new(10.0, 0.0, 0.0)),
            ]);

        let pos = clip.sample_position(1.5).unwrap();
        assert!(approx_eq_vec3(pos, Vec3::new(5.0, 0.0, 0.0)));
    }

    #[test]
    fn test_animation_clip_step_interpolation() {
        let clip = AnimationClip::new("step", 1.0).with_position_track(vec![
            Vec3Keyframe::step(0.0, Vec3::ZERO),
            Vec3Keyframe::step(1.0, Vec3::new(10.0, 0.0, 0.0)),
        ]);

        let pos = clip.sample_position(0.5).unwrap();
        assert!(approx_eq_vec3(pos, Vec3::ZERO));
    }

    #[test]
    fn test_animation_player_advance() {
        let clip = AnimationClip::new("test", 2.0).looping(true);
        let mut player = AnimationPlayer::new("test");
        player.speed = 1.0;

        player.advance(0.5, &clip);
        assert!((player.time - 0.5).abs() < EPS);

        player.advance(2.0, &clip);
        assert!((player.time - 0.5).abs() < EPS); // wrapped
        assert_eq!(player.loop_count, 1);
    }

    #[test]
    fn test_animation_player_stop() {
        let clip = AnimationClip::new("test", 1.0);
        let mut player = AnimationPlayer::new("test");
        player.advance(0.5, &clip);
        player.stop();
        assert!((player.time).abs() < EPS);
        assert!(!player.playing);
    }

    #[test]
    fn test_animation_clip_empty_tracks() {
        let clip = AnimationClip::new("empty", 1.0);
        assert!(clip.sample_position(0.5).is_none());
        assert!(clip.sample_rotation(0.5).is_none());
        assert!(clip.sample_scale(0.5).is_none());
    }

    #[test]
    fn test_animation_clip_single_keyframe() {
        let clip = AnimationClip::new("single", 1.0)
            .with_position_track(vec![Vec3Keyframe::linear(0.0, Vec3::new(5.0, 0.0, 0.0))]);

        let pos = clip.sample_position(0.5).unwrap();
        assert!(approx_eq_vec3(pos, Vec3::new(5.0, 0.0, 0.0)));
    }
}
