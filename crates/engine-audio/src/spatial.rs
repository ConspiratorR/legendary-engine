//! 3D spatial audio: listener, source, distance attenuation, doppler, and stereo panning.

use engine_math::Vec3;

/// Represents the listener's position and orientation in 3D space.
#[derive(Debug, Clone)]
pub struct AudioListener {
    pub position: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
    pub velocity: Vec3,
}

impl Default for AudioListener {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            forward: Vec3::NEG_Z,
            up: Vec3::Y,
            velocity: Vec3::ZERO,
        }
    }
}

/// A spatial audio source positioned in 3D space.
#[derive(Debug, Clone)]
pub struct SpatialAudioSource {
    pub position: Vec3,
    pub velocity: Vec3,
    pub min_distance: f32,
    pub max_distance: f32,
    pub rolloff_factor: f32,
    pub reference_distance: f32,
}

impl Default for SpatialAudioSource {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            min_distance: 1.0,
            max_distance: 100.0,
            rolloff_factor: 1.0,
            reference_distance: 1.0,
        }
    }
}

/// Distance attenuation model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistanceModel {
    Linear,
    Inverse,
    Exponential,
}

/// Configuration for the spatial audio system.
#[derive(Debug, Clone)]
pub struct SpatialAudioConfig {
    pub doppler_factor: f32,
    pub speed_of_sound: f32,
    pub distance_model: DistanceModel,
}

impl Default for SpatialAudioConfig {
    fn default() -> Self {
        Self {
            doppler_factor: 1.0,
            speed_of_sound: 343.3,
            distance_model: DistanceModel::Inverse,
        }
    }
}

/// Compute the distance between listener and source.
pub fn distance(listener: &AudioListener, source: &SpatialAudioSource) -> f32 {
    listener.position.distance(source.position)
}

/// Compute distance-based volume attenuation.
///
/// Returns a value in `[0.0, 1.0]`. When the source is at or closer than
/// `min_distance`, the volume is 1.0. Beyond `max_distance` it is 0.0.
pub fn compute_volume(
    listener: &AudioListener,
    source: &SpatialAudioSource,
    config: &SpatialAudioConfig,
) -> f32 {
    let dist = distance(listener, source);

    if dist <= source.min_distance {
        return 1.0;
    }
    if dist >= source.max_distance {
        return 0.0;
    }

    let vol = match config.distance_model {
        DistanceModel::Inverse => {
            let denom = source.reference_distance
                + source.rolloff_factor * (dist - source.reference_distance);
            if denom.abs() < f32::EPSILON {
                1.0
            } else {
                (source.reference_distance / denom).clamp(0.0, 1.0)
            }
        }
        DistanceModel::Exponential => {
            if source.reference_distance.abs() < f32::EPSILON || dist.abs() < f32::EPSILON {
                1.0
            } else {
                (dist / source.reference_distance)
                    .powf(-source.rolloff_factor)
                    .clamp(0.0, 1.0)
            }
        }
        DistanceModel::Linear => {
            let range = source.max_distance - source.reference_distance;
            if range.abs() < f32::EPSILON {
                1.0
            } else {
                (1.0 - source.rolloff_factor * (dist - source.reference_distance) / range)
                    .clamp(0.0, 1.0)
            }
        }
    };

    vol.clamp(0.0, 1.0)
}

/// Compute the doppler frequency ratio.
///
/// A ratio > 1.0 means the source is approaching (higher pitch).
/// A ratio < 1.0 means the source is receding (lower pitch).
pub fn compute_doppler(
    listener: &AudioListener,
    source: &SpatialAudioSource,
    config: &SpatialAudioConfig,
) -> f32 {
    let dir_to_listener = (listener.position - source.position).normalize_or_zero();
    let v_listener = listener.velocity.dot(dir_to_listener);
    let v_source = source.velocity.dot(dir_to_listener);

    let denom = config.speed_of_sound - config.doppler_factor * v_source;
    if denom.abs() < f32::EPSILON {
        return 1.0;
    }

    ((config.speed_of_sound - config.doppler_factor * v_listener) / denom).max(0.0)
}

/// Compute stereo panning based on the source's position relative to the listener.
///
/// Returns `(left_volume, right_volume)` where each is in `[0.0, 1.0]`.
pub fn compute_stereo_pan(listener: &AudioListener, source: &SpatialAudioSource) -> (f32, f32) {
    let to_source = source.position - listener.position;
    let dist = to_source.length();

    if dist < f32::EPSILON {
        return (1.0, 1.0);
    }

    let right = listener.forward.cross(listener.up).normalize();
    let to_source_norm = to_source / dist;
    let pan = to_source_norm.dot(right).clamp(-1.0, 1.0);

    let angle = (pan + 1.0) * 0.5;
    let left_vol = (1.0 - angle).clamp(0.0, 1.0);
    let right_vol = angle.clamp(0.0, 1.0);

    (left_vol, right_vol)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listener_at(origin: Vec3) -> AudioListener {
        AudioListener {
            position: origin,
            forward: Vec3::NEG_Z,
            up: Vec3::Y,
            velocity: Vec3::ZERO,
        }
    }

    fn source_at(pos: Vec3) -> SpatialAudioSource {
        SpatialAudioSource {
            position: pos,
            ..Default::default()
        }
    }

    fn config(model: DistanceModel) -> SpatialAudioConfig {
        SpatialAudioConfig {
            distance_model: model,
            ..Default::default()
        }
    }

    // ── Distance attenuation ───────────────────────────────────────────

    #[test]
    fn test_volume_at_min_distance() {
        let listener = listener_at(Vec3::ZERO);
        let source = source_at(Vec3::new(1.0, 0.0, 0.0)); // at min_distance=1.0
        let cfg = config(DistanceModel::Inverse);
        let vol = compute_volume(&listener, &source, &cfg);
        assert!((vol - 1.0).abs() < 1e-6, "expected 1.0, got {vol}");
    }

    #[test]
    fn test_volume_inside_min_distance() {
        let listener = listener_at(Vec3::ZERO);
        let source = source_at(Vec3::new(0.5, 0.0, 0.0));
        let cfg = config(DistanceModel::Inverse);
        let vol = compute_volume(&listener, &source, &cfg);
        assert!((vol - 1.0).abs() < 1e-6, "expected 1.0, got {vol}");
    }

    #[test]
    fn test_volume_beyond_max_distance() {
        let listener = listener_at(Vec3::ZERO);
        let mut source = source_at(Vec3::new(200.0, 0.0, 0.0));
        source.max_distance = 100.0;
        let cfg = config(DistanceModel::Inverse);
        let vol = compute_volume(&listener, &source, &cfg);
        assert!(vol.abs() < 1e-6, "expected 0.0, got {vol}");
    }

    #[test]
    fn test_volume_inverse_decreases_with_distance() {
        let listener = listener_at(Vec3::ZERO);
        let cfg = config(DistanceModel::Inverse);

        let near = compute_volume(&listener, &source_at(Vec3::new(5.0, 0.0, 0.0)), &cfg);
        let far = compute_volume(&listener, &source_at(Vec3::new(20.0, 0.0, 0.0)), &cfg);
        assert!(near > far, "near {near} should be > far {far}");
    }

    #[test]
    fn test_volume_exponential_decreases_with_distance() {
        let listener = listener_at(Vec3::ZERO);
        let cfg = config(DistanceModel::Exponential);

        let near = compute_volume(&listener, &source_at(Vec3::new(2.0, 0.0, 0.0)), &cfg);
        let far = compute_volume(&listener, &source_at(Vec3::new(10.0, 0.0, 0.0)), &cfg);
        assert!(near > far, "near {near} should be > far {far}");
    }

    #[test]
    fn test_volume_linear_decreases_with_distance() {
        let listener = listener_at(Vec3::ZERO);
        let cfg = config(DistanceModel::Linear);

        let near = compute_volume(&listener, &source_at(Vec3::new(5.0, 0.0, 0.0)), &cfg);
        let far = compute_volume(&listener, &source_at(Vec3::new(50.0, 0.0, 0.0)), &cfg);
        assert!(near > far, "near {near} should be > far {far}");
    }

    // ── Doppler ────────────────────────────────────────────────────────

    #[test]
    fn test_doppler_approaching() {
        let listener = listener_at(Vec3::new(0.0, 0.0, -10.0));
        let source = SpatialAudioSource {
            position: Vec3::new(0.0, 0.0, 10.0),
            velocity: Vec3::new(0.0, 0.0, -20.0),
            ..Default::default()
        };
        let cfg = SpatialAudioConfig::default();
        let ratio = compute_doppler(&listener, &source, &cfg);
        assert!(
            ratio > 1.0,
            "approaching source ratio {ratio} should be > 1.0"
        );
    }

    #[test]
    fn test_doppler_receding() {
        let listener = listener_at(Vec3::ZERO);
        let source = SpatialAudioSource {
            position: Vec3::new(0.0, 0.0, -10.0),
            velocity: Vec3::new(0.0, 0.0, -20.0),
            ..Default::default()
        };
        let cfg = SpatialAudioConfig::default();
        let ratio = compute_doppler(&listener, &source, &cfg);
        assert!(ratio < 1.0, "receding source ratio {ratio} should be < 1.0");
    }

    #[test]
    fn test_doppler_stationary() {
        let listener = listener_at(Vec3::ZERO);
        let source = source_at(Vec3::new(10.0, 0.0, 0.0));
        let cfg = SpatialAudioConfig::default();
        let ratio = compute_doppler(&listener, &source, &cfg);
        assert!(
            (ratio - 1.0).abs() < 1e-6,
            "stationary ratio {ratio} should be 1.0"
        );
    }

    #[test]
    fn test_doppler_listener_approaching() {
        let listener = AudioListener {
            position: Vec3::new(0.0, 0.0, -10.0),
            forward: Vec3::NEG_Z,
            up: Vec3::Y,
            velocity: Vec3::new(0.0, 0.0, 20.0),
        };
        let source = source_at(Vec3::new(0.0, 0.0, 10.0));
        let cfg = SpatialAudioConfig::default();
        let ratio = compute_doppler(&listener, &source, &cfg);
        assert!(
            ratio > 1.0,
            "approaching listener ratio {ratio} should be > 1.0"
        );
    }

    // ── Stereo panning ─────────────────────────────────────────────────

    #[test]
    fn test_pan_source_center() {
        let listener = listener_at(Vec3::ZERO);
        let source = source_at(Vec3::new(0.0, 0.0, -10.0));
        let (left, right) = compute_stereo_pan(&listener, &source);
        assert!(
            (left - right).abs() < 0.1,
            "center pan: L={left}, R={right}"
        );
    }

    #[test]
    fn test_pan_source_right() {
        let listener = listener_at(Vec3::ZERO);
        let source = source_at(Vec3::new(10.0, 0.0, 0.0));
        let (left, right) = compute_stereo_pan(&listener, &source);
        assert!(right > left, "right pan: L={left}, R={right}");
    }

    #[test]
    fn test_pan_source_left() {
        let listener = listener_at(Vec3::ZERO);
        let source = source_at(Vec3::new(-10.0, 0.0, 0.0));
        let (left, right) = compute_stereo_pan(&listener, &source);
        assert!(left > right, "left pan: L={left}, R={right}");
    }

    #[test]
    fn test_pan_behind_listener() {
        let listener = listener_at(Vec3::ZERO);
        let source = source_at(Vec3::new(0.0, 0.0, 10.0));
        let (left, right) = compute_stereo_pan(&listener, &source);
        assert!(
            (left - right).abs() < 0.1,
            "behind pan: L={left}, R={right}"
        );
    }

    // ── Edge cases ─────────────────────────────────────────────────────

    #[test]
    fn test_volume_zero_distance() {
        let listener = listener_at(Vec3::ZERO);
        let source = source_at(Vec3::ZERO);
        let cfg = config(DistanceModel::Inverse);
        let vol = compute_volume(&listener, &source, &cfg);
        assert!(
            (vol - 1.0).abs() < 1e-6,
            "zero distance volume {vol} should be 1.0"
        );
    }

    #[test]
    fn test_pan_zero_distance() {
        let listener = listener_at(Vec3::ZERO);
        let source = source_at(Vec3::ZERO);
        let (left, right) = compute_stereo_pan(&listener, &source);
        assert!((left - 1.0).abs() < 1e-6, "zero dist left {left}");
        assert!((right - 1.0).abs() < 1e-6, "zero dist right {right}");
    }

    #[test]
    fn test_defaults() {
        let listener = AudioListener::default();
        assert_eq!(listener.position, Vec3::ZERO);
        assert_eq!(listener.forward, Vec3::NEG_Z);

        let source = SpatialAudioSource::default();
        assert_eq!(source.min_distance, 1.0);
        assert_eq!(source.max_distance, 100.0);

        let cfg = SpatialAudioConfig::default();
        assert_eq!(cfg.distance_model, DistanceModel::Inverse);
        assert!((cfg.speed_of_sound - 343.3).abs() < 1e-6);
    }

    #[test]
    fn test_exponential_volume_at_reference_distance() {
        let listener = listener_at(Vec3::ZERO);
        let source = SpatialAudioSource {
            position: Vec3::new(1.0, 0.0, 0.0),
            reference_distance: 1.0,
            rolloff_factor: 1.0,
            min_distance: 0.5,
            max_distance: 100.0,
            ..Default::default()
        };
        let cfg = config(DistanceModel::Exponential);
        let vol = compute_volume(&listener, &source, &cfg);
        // At reference distance, exponential model should return 1.0
        assert!((vol - 1.0).abs() < 1e-6, "at ref_distance vol={vol}");
    }

    #[test]
    fn test_linear_volume_at_reference_distance() {
        let listener = listener_at(Vec3::ZERO);
        let source = SpatialAudioSource {
            position: Vec3::new(1.0, 0.0, 0.0),
            reference_distance: 1.0,
            rolloff_factor: 1.0,
            min_distance: 0.5,
            max_distance: 100.0,
            ..Default::default()
        };
        let cfg = config(DistanceModel::Linear);
        let vol = compute_volume(&listener, &source, &cfg);
        // At reference distance, linear model should return 1.0
        assert!((vol - 1.0).abs() < 1e-6, "at ref_distance vol={vol}");
    }

    #[test]
    fn test_inverse_volume_halves_at_double_distance() {
        let listener = listener_at(Vec3::ZERO);
        let source = SpatialAudioSource {
            position: Vec3::new(2.0, 0.0, 0.0),
            reference_distance: 1.0,
            rolloff_factor: 1.0,
            min_distance: 0.5,
            max_distance: 100.0,
            ..Default::default()
        };
        let cfg = config(DistanceModel::Inverse);
        let vol = compute_volume(&listener, &source, &cfg);
        // Inverse: ref / (ref + rolloff * (dist - ref)) = 1 / (1 + 1) = 0.5
        assert!((vol - 0.5).abs() < 1e-6, "expected 0.5, got {vol}");
    }

    #[test]
    fn test_doppler_zero_speed_of_sound_zero_velocity() {
        let listener = listener_at(Vec3::ZERO);
        let source = source_at(Vec3::new(0.0, 0.0, -10.0));
        let cfg = SpatialAudioConfig {
            speed_of_sound: 0.0,
            doppler_factor: 1.0,
            ..Default::default()
        };
        let ratio = compute_doppler(&listener, &source, &cfg);
        // Zero speed + zero velocities → denom=0 → returns 1.0
        assert!((ratio - 1.0).abs() < 1e-6, "expected 1.0, got {ratio}");
    }

    #[test]
    fn test_doppler_zero_speed_with_velocity() {
        let listener = listener_at(Vec3::ZERO);
        let source = SpatialAudioSource {
            position: Vec3::new(0.0, 0.0, -10.0),
            velocity: Vec3::new(0.0, 0.0, -20.0),
            ..Default::default()
        };
        let cfg = SpatialAudioConfig {
            speed_of_sound: 0.0,
            doppler_factor: 1.0,
            ..Default::default()
        };
        let ratio = compute_doppler(&listener, &source, &cfg);
        // denom = 0 - (-20) = 20, num = 0 - 0 = 0 → ratio = 0
        assert!(ratio.abs() < 1e-6, "expected 0.0, got {ratio}");
    }

    #[test]
    fn test_pan_full_right() {
        let listener = listener_at(Vec3::ZERO);
        let source = source_at(Vec3::new(100.0, 0.0, 0.0));
        let (left, right) = compute_stereo_pan(&listener, &source);
        assert!(right > 0.9, "expected right>0.9, got R={right}");
        assert!(left < 0.1, "expected left<0.1, got L={left}");
    }

    #[test]
    fn test_pan_full_left() {
        let listener = listener_at(Vec3::ZERO);
        let source = source_at(Vec3::new(-100.0, 0.0, 0.0));
        let (left, right) = compute_stereo_pan(&listener, &source);
        assert!(left > 0.9, "expected left>0.9, got L={left}");
        assert!(right < 0.1, "expected right<0.1, got R={right}");
    }

    #[test]
    fn test_volume_clamped_at_boundary() {
        let listener = listener_at(Vec3::ZERO);
        let source = SpatialAudioSource {
            position: Vec3::new(50.0, 0.0, 0.0),
            min_distance: 1.0,
            max_distance: 100.0,
            rolloff_factor: 10.0,
            ..Default::default()
        };
        let cfg = config(DistanceModel::Linear);
        let vol = compute_volume(&listener, &source, &cfg);
        assert!((0.0..=1.0).contains(&vol), "volume {vol} out of [0,1]");
    }

    #[test]
    fn test_distance_function() {
        let listener = listener_at(Vec3::ZERO);
        let source = source_at(Vec3::new(3.0, 4.0, 0.0));
        let dist = distance(&listener, &source);
        assert!((dist - 5.0).abs() < 1e-6, "expected 5.0, got {dist}");
    }
}
