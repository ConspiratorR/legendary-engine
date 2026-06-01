use super::AnimationEditorState;
use anyhow::{Context, Result};
use engine_scene::keyframe::{AnimationClip, Interpolation, RotationKeyframe, Vec3Keyframe};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

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
    pub fn from_clip(clip: &AnimationClip) -> Self {
        Self {
            name: clip.name.clone(),
            duration: clip.duration,
            looping: clip.looping,
            position_track: clip.position_track.as_ref().map(|track| {
                track
                    .iter()
                    .map(|kf| Vec3KeyframeData {
                        time: kf.time,
                        value: [kf.value.x, kf.value.y, kf.value.z],
                        interpolation: format!("{:?}", kf.interpolation).to_lowercase(),
                        tangent_in: [kf.tangent_in.x, kf.tangent_in.y, kf.tangent_in.z],
                        tangent_out: [kf.tangent_out.x, kf.tangent_out.y, kf.tangent_out.z],
                    })
                    .collect()
            }),
            rotation_track: clip.rotation_track.as_ref().map(|track| {
                track
                    .iter()
                    .map(|kf| RotationKeyframeData {
                        time: kf.time,
                        value: [kf.value.x, kf.value.y, kf.value.z, kf.value.w],
                        interpolation: format!("{:?}", kf.interpolation).to_lowercase(),
                    })
                    .collect()
            }),
            scale_track: clip.scale_track.as_ref().map(|track| {
                track
                    .iter()
                    .map(|kf| Vec3KeyframeData {
                        time: kf.time,
                        value: [kf.value.x, kf.value.y, kf.value.z],
                        interpolation: format!("{:?}", kf.interpolation).to_lowercase(),
                        tangent_in: [kf.tangent_in.x, kf.tangent_in.y, kf.tangent_in.z],
                        tangent_out: [kf.tangent_out.x, kf.tangent_out.y, kf.tangent_out.z],
                    })
                    .collect()
            }),
        }
    }

    pub fn to_clip(&self) -> Result<AnimationClip> {
        let mut clip = AnimationClip::new(&self.name, self.duration).looping(self.looping);

        if let Some(ref track) = self.position_track {
            let keyframes: Result<Vec<Vec3Keyframe>> = track
                .iter()
                .map(|kf| {
                    let interp = parse_interpolation(&kf.interpolation)?;
                    Ok(Vec3Keyframe {
                        time: kf.time,
                        value: engine_math::Vec3::new(kf.value[0], kf.value[1], kf.value[2]),
                        interpolation: interp,
                        tangent_in: engine_math::Vec3::new(
                            kf.tangent_in[0],
                            kf.tangent_in[1],
                            kf.tangent_in[2],
                        ),
                        tangent_out: engine_math::Vec3::new(
                            kf.tangent_out[0],
                            kf.tangent_out[1],
                            kf.tangent_out[2],
                        ),
                    })
                })
                .collect();
            clip = clip.with_position_track(keyframes?);
        }

        if let Some(ref track) = self.rotation_track {
            let keyframes: Result<Vec<RotationKeyframe>> = track
                .iter()
                .map(|kf| {
                    let interp = parse_interpolation(&kf.interpolation)?;
                    Ok(RotationKeyframe {
                        time: kf.time,
                        value: engine_math::Quat::from_xyzw(
                            kf.value[0],
                            kf.value[1],
                            kf.value[2],
                            kf.value[3],
                        ),
                        interpolation: interp,
                    })
                })
                .collect();
            clip = clip.with_rotation_track(keyframes?);
        }

        if let Some(ref track) = self.scale_track {
            let keyframes: Result<Vec<Vec3Keyframe>> = track
                .iter()
                .map(|kf| {
                    let interp = parse_interpolation(&kf.interpolation)?;
                    Ok(Vec3Keyframe {
                        time: kf.time,
                        value: engine_math::Vec3::new(kf.value[0], kf.value[1], kf.value[2]),
                        interpolation: interp,
                        tangent_in: engine_math::Vec3::new(
                            kf.tangent_in[0],
                            kf.tangent_in[1],
                            kf.tangent_in[2],
                        ),
                        tangent_out: engine_math::Vec3::new(
                            kf.tangent_out[0],
                            kf.tangent_out[1],
                            kf.tangent_out[2],
                        ),
                    })
                })
                .collect();
            clip = clip.with_scale_track(keyframes?);
        }

        Ok(clip)
    }
}

fn parse_interpolation(s: &str) -> Result<Interpolation> {
    match s.to_lowercase().as_str() {
        "linear" => Ok(Interpolation::Linear),
        "step" => Ok(Interpolation::Step),
        "cubic" => Ok(Interpolation::Cubic),
        _ => anyhow::bail!("Unknown interpolation type: {}", s),
    }
}

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

pub fn import_clip(path: &Path) -> Result<AnimationClip> {
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

    fn make_test_clip() -> AnimationClip {
        AnimationClip::new("walk", 2.0)
            .looping(true)
            .with_position_track(vec![
                Vec3Keyframe::linear(0.0, Vec3::ZERO),
                Vec3Keyframe::linear(1.0, Vec3::new(5.0, 0.0, 0.0)),
                Vec3Keyframe::linear(2.0, Vec3::ZERO),
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
        assert_eq!(
            parse_interpolation("linear").unwrap(),
            Interpolation::Linear
        );
        assert_eq!(parse_interpolation("step").unwrap(), Interpolation::Step);
        assert_eq!(parse_interpolation("cubic").unwrap(), Interpolation::Cubic);
        assert!(parse_interpolation("bezier").is_err());
    }

    #[test]
    fn test_export_no_clip_fails() {
        let state = AnimationEditorState::new();
        let path = std::env::temp_dir().join("should_not_exist.json");
        assert!(export_clip(&state, &path).is_err());
    }
}
