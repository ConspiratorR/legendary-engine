//! Built-in sample MonoBehaviours registered for SceneData roundtrip (D2 / phase 14 R2).
//!
//! These types are intentionally small and `Default` so
//! [`crate::monobehaviour::MonoBehaviourRegistry`] can rebuild them from
//! `script_type` + props after scene load.

use crate::animation_apply::{AnimationClip, apply_clip_pose};
use crate::behaviour::BehaviourState;
use crate::context::Context;
use crate::gameobject::GameObjectHandle;
use crate::monobehaviour::MonoBehaviour;
use crate::{Behaviour, Component};
use engine_math::Vec3;
use std::any::Any;

/// Translate the host transform every Update.
#[derive(Debug, Default, Clone)]
pub struct Mover {
    pub speed: f32,
    pub direction: Vec3,
    pub state: BehaviourState,
}

impl Component for Mover {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Behaviour for Mover {
    fn Enabled(&self) -> bool {
        self.state.enabled()
    }
    fn SetEnabled(&mut self, enabled: bool) {
        self.state.set_enabled(enabled);
    }
    fn IsActiveAndEnabled(&self) -> bool {
        self.state.enabled()
    }
    fn set_gameobject(&mut self, handle: GameObjectHandle) {
        self.state.set_gameobject(handle);
    }
    fn gameobject_handle(&self) -> Option<GameObjectHandle> {
        self.state.gameobject()
    }
}

impl MonoBehaviour for Mover {
    fn TypeName(&self) -> &str {
        "Mover"
    }

    fn SerializeProps(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "speed": self.speed,
            "direction": [self.direction.x, self.direction.y, self.direction.z],
        }))
    }

    fn DeserializeProps(&mut self, props: &serde_json::Value) {
        self.speed = props.get("speed").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        if let Some(d) = props.get("direction").and_then(|v| v.as_array()) {
            self.direction = Vec3::new(
                d.first().and_then(|x| x.as_f64()).unwrap_or(0.0) as f32,
                d.get(1).and_then(|x| x.as_f64()).unwrap_or(0.0) as f32,
                d.get(2).and_then(|x| x.as_f64()).unwrap_or(0.0) as f32,
            );
        }
    }

    fn Update(&mut self, ctx: &mut Context) {
        let Some(me) = self.gameobject_handle() else {
            return;
        };
        let dt = ctx.DeltaTime();
        let delta = self.direction * (self.speed * dt);
        // ECS-primary under unity-world-primary; array-primary when the feature is off.
        let _ = ctx.world.with_ecs_transform_mut(me, |t| {
            let p = t.LocalPosition();
            t.SetLocalPosition(p + delta);
        });
    }
}

/// Spin the host on the Y axis each frame.
#[derive(Debug, Default, Clone)]
pub struct Rotator {
    pub degrees_per_second: f32,
    pub state: BehaviourState,
}

impl Component for Rotator {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Behaviour for Rotator {
    fn Enabled(&self) -> bool {
        self.state.enabled()
    }
    fn SetEnabled(&mut self, enabled: bool) {
        self.state.set_enabled(enabled);
    }
    fn IsActiveAndEnabled(&self) -> bool {
        self.state.enabled()
    }
    fn set_gameobject(&mut self, handle: GameObjectHandle) {
        self.state.set_gameobject(handle);
    }
    fn gameobject_handle(&self) -> Option<GameObjectHandle> {
        self.state.gameobject()
    }
}

impl MonoBehaviour for Rotator {
    fn TypeName(&self) -> &str {
        "Rotator"
    }

    fn SerializeProps(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "degrees_per_second": self.degrees_per_second }))
    }

    fn DeserializeProps(&mut self, props: &serde_json::Value) {
        self.degrees_per_second = props
            .get("degrees_per_second")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
    }

    fn Update(&mut self, ctx: &mut Context) {
        let Some(me) = self.gameobject_handle() else {
            return;
        };
        let dt = ctx.DeltaTime();
        let angle = self.degrees_per_second * dt;
        let _ = ctx.world.with_ecs_transform_mut(me, |t| {
            t.Rotate(Vec3::new(0.0, angle, 0.0));
        });
    }
}

/// Destroy the host after `lifetime` seconds.
#[derive(Debug, Default, Clone)]
pub struct Lifetime {
    pub seconds: f32,
    pub elapsed: f32,
    pub state: BehaviourState,
}

impl Component for Lifetime {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Behaviour for Lifetime {
    fn Enabled(&self) -> bool {
        self.state.enabled()
    }
    fn SetEnabled(&mut self, enabled: bool) {
        self.state.set_enabled(enabled);
    }
    fn IsActiveAndEnabled(&self) -> bool {
        self.state.enabled()
    }
    fn set_gameobject(&mut self, handle: GameObjectHandle) {
        self.state.set_gameobject(handle);
    }
    fn gameobject_handle(&self) -> Option<GameObjectHandle> {
        self.state.gameobject()
    }
}

impl MonoBehaviour for Lifetime {
    fn TypeName(&self) -> &str {
        "Lifetime"
    }

    fn SerializeProps(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "seconds": self.seconds }))
    }

    fn DeserializeProps(&mut self, props: &serde_json::Value) {
        self.seconds = props.get("seconds").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
    }

    fn Update(&mut self, ctx: &mut Context) {
        self.elapsed += ctx.DeltaTime();
        if self.seconds > 0.0 && self.elapsed >= self.seconds {
            if let Some(me) = self.gameobject_handle() {
                ctx.world.Destroy(me);
            }
        }
    }
}

/// Runtime animation clip player — samples [`AnimationClip`] onto Unity World
/// via [`apply_clip_pose`] (phase 14 R2). Clip format stays engine-scene
/// keyframe math; pose authority is `World` / storage-authority path.
#[derive(Debug, Default, Clone)]
pub struct AnimationClipPlayer {
    pub clip: Option<AnimationClip>,
    pub time: f32,
    pub speed: f32,
    pub playing: bool,
    pub state: BehaviourState,
}

impl Component for AnimationClipPlayer {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Behaviour for AnimationClipPlayer {
    fn Enabled(&self) -> bool {
        self.state.enabled()
    }
    fn SetEnabled(&mut self, enabled: bool) {
        self.state.set_enabled(enabled);
    }
    fn IsActiveAndEnabled(&self) -> bool {
        self.state.enabled()
    }
    fn set_gameobject(&mut self, handle: GameObjectHandle) {
        self.state.set_gameobject(handle);
    }
    fn gameobject_handle(&self) -> Option<GameObjectHandle> {
        self.state.gameobject()
    }
}

impl MonoBehaviour for AnimationClipPlayer {
    fn TypeName(&self) -> &str {
        "AnimationClipPlayer"
    }

    fn SerializeProps(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "clip": self.clip,
            "time": self.time,
            "speed": self.speed,
            "playing": self.playing,
        }))
    }

    fn DeserializeProps(&mut self, props: &serde_json::Value) {
        self.clip = props
            .get("clip")
            .and_then(|c| serde_json::from_value(c.clone()).ok());
        self.time = props.get("time").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        self.speed = props
            .get("speed")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .filter(|s| *s != 0.0)
            .unwrap_or(1.0);
        self.playing = props
            .get("playing")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
    }

    fn Update(&mut self, ctx: &mut Context) {
        if !self.playing {
            return;
        }
        let Some(me) = self.gameobject_handle() else {
            return;
        };
        let Some(clip) = self.clip.clone() else {
            return;
        };
        let speed = if self.speed == 0.0 { 1.0 } else { self.speed };
        self.time += ctx.DeltaTime() * speed;
        if clip.duration > 0.0 {
            if clip.looping {
                self.time = self.time.rem_euclid(clip.duration);
            } else if self.time >= clip.duration {
                self.time = clip.duration;
                self.playing = false;
            }
        }
        apply_clip_pose(ctx.world, me, &clip, self.time);
    }
}

/// Register sample scripts under their short [`MonoBehaviour::TypeName`] keys.
///
/// Called from [`crate::plugins::CorePlugins`] so SceneData load can rebuild them.
pub fn register_sample_scripts() {
    use crate::monobehaviour::MonoBehaviourRegistry;

    let mut reg = MonoBehaviourRegistry::global()
        .lock()
        .expect("MonoBehaviourRegistry");
    reg.register("Mover", || {
        Box::new(Mover {
            speed: 1.0,
            direction: Vec3::new(0.0, 0.0, 1.0),
            state: BehaviourState::new(),
        })
    });
    reg.register("Rotator", || {
        Box::new(Rotator {
            degrees_per_second: 90.0,
            state: BehaviourState::new(),
        })
    });
    reg.register("Lifetime", || {
        Box::new(Lifetime {
            seconds: 1.0,
            elapsed: 0.0,
            state: BehaviourState::new(),
        })
    });
    reg.register("AnimationClipPlayer", || {
        Box::new(AnimationClipPlayer::default())
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::{LoadSceneJson, SaveSceneJsonPrepared};
    use crate::world::World;

    #[test]
    fn test_sample_scripts_scenedata_roundtrip() {
        register_sample_scripts();

        let mut world = World::new();
        let go = world.CreateGameObject("MoverGO");
        world.AddMonoBehaviour(
            go,
            Mover {
                speed: 3.5,
                direction: Vec3::new(1.0, 0.0, 0.0),
                state: BehaviourState::new(),
            },
        );
        world.AddMonoBehaviour(
            go,
            Rotator {
                degrees_per_second: 45.0,
                state: BehaviourState::new(),
            },
        );

        let json = SaveSceneJsonPrepared(&mut world, "SampleScripts").unwrap();
        assert!(json.contains("Mover"));
        assert!(json.contains("Rotator"));

        let mut loaded = World::new();
        let handles = LoadSceneJson(&json, &mut loaded).unwrap();
        let h = handles[0];
        assert_eq!(loaded.MonoBehaviourCount(h), 2);
        let collected = loaded.CollectMonoBehaviours(h);
        let mover = collected
            .iter()
            .find(|(n, _, _)| n == "Mover")
            .expect("Mover restored");
        assert_eq!(mover.2.as_ref().unwrap()["speed"], 3.5);
        let rot = collected
            .iter()
            .find(|(n, _, _)| n == "Rotator")
            .expect("Rotator restored");
        assert_eq!(rot.2.as_ref().unwrap()["degrees_per_second"], 45.0);
    }

    #[test]
    fn test_animation_clip_player_applies_pose_and_roundtrip() {
        use crate::animation_apply::{AnimationClip, Vec3Keyframe};

        register_sample_scripts();

        let clip = AnimationClip::new("move", 1.0)
            .with_position_track(vec![
                Vec3Keyframe::linear(0.0, Vec3::ZERO),
                Vec3Keyframe::linear(1.0, Vec3::new(10.0, 0.0, 0.0)),
            ])
            .looping(false);

        let mut world = World::new();
        let go = world.CreateGameObject("AnimGO");
        let mut player = AnimationClipPlayer {
            clip: Some(clip.clone()),
            time: 0.0,
            speed: 1.0,
            playing: true,
            state: BehaviourState::new(),
        };
        player.set_gameobject(go);
        world.AddMonoBehaviour(go, player);
        assert_eq!(world.MonoBehaviourCount(go), 1);

        // Authority write path: apply at known sample time.
        crate::animation_apply::apply_clip_pose(&mut world, go, &clip, 0.5);
        let t = world.GetTransform(go).expect("transform");
        assert!(
            (t.LocalPosition().x - 5.0).abs() < 0.5,
            "clip sample should write World pose"
        );

        // SceneData props roundtrip restores player + clip JSON.
        let json = SaveSceneJsonPrepared(&mut world, "Anim").unwrap();
        assert!(json.contains("AnimationClipPlayer"));
        assert!(json.contains("move"));
        let mut loaded = World::new();
        let handles = LoadSceneJson(&json, &mut loaded).unwrap();
        let collected = loaded.CollectMonoBehaviours(handles[0]);
        let p = collected
            .iter()
            .find(|(n, _, _)| n == "AnimationClipPlayer")
            .expect("player restored");
        let props = p.2.as_ref().unwrap();
        assert_eq!(props["speed"], 1.0);
        assert!(props.get("clip").is_some());
    }

    /// Phase 14 review residual — Update path through World tick drives pose.
    #[test]
    fn test_animation_clip_player_update_writes_world_pose() {
        use crate::animation_apply::{AnimationClip, Vec3Keyframe};
        use crate::time::Time;

        register_sample_scripts();
        let clip = AnimationClip::new("u", 2.0)
            .with_position_track(vec![
                Vec3Keyframe::linear(0.0, Vec3::ZERO),
                Vec3Keyframe::linear(2.0, Vec3::new(2.0, 0.0, 0.0)),
            ])
            .looping(false);

        let mut world = World::new();
        let go = world.CreateGameObject("Upd");
        world.SetLocalPosition(go, Vec3::new(-1.0, 0.0, 0.0));
        world.AddMonoBehaviour(
            go,
            AnimationClipPlayer {
                clip: Some(clip),
                time: 0.0,
                speed: 1.0,
                playing: true,
                state: BehaviourState::new(),
            },
        );

        let mut events = crate::event::EventBus::new();
        for _ in 0..8 {
            let mut t = Time::default();
            t.update(0.25);
            world.tick_update(t, 0, &mut events);
        }

        let x = world.GetTransform(go).expect("transform").LocalPosition().x;
        // Player must have written a clip-sampled pose (not left at -1 forever).
        assert!(
            x > -1.0 + 0.01,
            "Update should apply clip pose onto World, got x={x}"
        );
    }
}
