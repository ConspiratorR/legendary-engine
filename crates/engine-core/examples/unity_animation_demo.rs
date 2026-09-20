//! Unity animation authority demo — clip → SceneData → SceneRuntime → World pose.
//!
//! Exercises phases 12–16 without GPU:
//! - `AnimationClipPlayer` samples `engine_core::AnimationClip` via `apply_clip_pose`
//! - Scene export uses **prepared** save (ECS storage authority when default-on)
//! - SceneRuntime lifecycle ticks drive MonoBehaviour Update
//!
//! Run: `cargo run -p engine-core --example unity_animation_demo`

use engine_core::SceneRuntime;
use engine_core::animation_apply::{AnimationClip, Vec3Keyframe};
use engine_core::behaviour::BehaviourState;
use engine_core::event::EventBus;
use engine_core::sample_scripts::{AnimationClipPlayer, register_sample_scripts};
use engine_core::scene_management::LoadSceneMode;
use engine_core::serialization::SaveSceneJsonPrepared;
use engine_core::time::Time;
use engine_core::world::World;

fn main() {
    println!("=== Unity Animation Authority Demo ===\n");
    register_sample_scripts();

    println!(
        "Storage authority feature (unity-world-primary): {}",
        World::unity_world_primary_feature()
    );

    // 1. Author a clip + player on a GameObject (editor/script authoring).
    let clip = AnimationClip::new("hero_move", 1.0)
        .with_position_track(vec![
            Vec3Keyframe::linear(0.0, engine_math::Vec3::new(0.0, 0.0, 0.0)),
            Vec3Keyframe::linear(1.0, engine_math::Vec3::new(8.0, 0.0, 0.0)),
        ])
        .looping(false);

    let mut author = World::new();
    let hero = author.CreateGameObject("AnimHero");
    author.SetLocalPosition(hero, engine_math::Vec3::ZERO);
    author.AddMonoBehaviour(
        hero,
        AnimationClipPlayer {
            clip: Some(clip),
            time: 0.0,
            speed: 1.0,
            playing: true,
            state: BehaviourState::new(),
        },
    );

    // 2. Prepared scene export (refreshes pose/hierarchy caches from ECS when on).
    let json = SaveSceneJsonPrepared(&mut author, "AnimDemo").expect("prepared save");
    println!("Scene JSON {} bytes", json.len());
    assert!(
        json.contains("AnimationClipPlayer"),
        "player must survive SceneData"
    );
    assert!(json.contains("hero_move"), "clip name must serialize");

    // 3. Load into SceneRuntime (same path as editor Play / runtime).
    let mut rt = SceneRuntime::new();
    rt.load_scene_json("AnimDemo", &json, LoadSceneMode::Single)
        .expect("load");
    rt.mark_needs_awake();

    let hero = rt.world.Find("AnimHero").expect("AnimHero");
    let start = rt
        .world
        .GetTransform(hero)
        .expect("transform")
        .LocalPosition();
    println!("Pose after load: {start:?}");

    // 4. Drive lifecycle frames so AnimationClipPlayer::Update runs.
    let mut time = Time::new();
    let mut events = EventBus::new();
    for frame in 0..40 {
        time.update(0.05);
        rt.tick(&time, time.frameCount(), &mut events);
        if frame % 10 == 0 || frame == 39 {
            let p = rt.world.GetTransform(hero).unwrap().LocalPosition();
            println!("  frame {:>2} local_position = {:?}", frame + 1, p);
        }
    }

    let end = rt
        .world
        .GetTransform(hero)
        .expect("transform")
        .LocalPosition();
    println!("\nPose after ticks: {end:?}");
    println!("Delta X = {:.3} (clip target ~8.0 at t=1)", end.x - start.x);

    // 5. Authority assertions: World pose written by the player path.
    assert!(
        end.x > start.x + 0.5,
        "AnimationClipPlayer should advance World local pose via apply_clip_pose; start={start:?} end={end:?}"
    );
    assert!(
        (end.x - 8.0).abs() < 0.05,
        "non-looping clip should park near track end (8.0), got {}",
        end.x
    );
    println!("Non-looping clip parked at duration (playing stops when time >= duration).");
    println!("\n=== Demo Complete ===");
}
