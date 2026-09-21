//! Combined Unity sim demo — animation + physics bridge + SceneData (phase 35).
//!
//! Headless evidence for branch `phase12-array-authority`:
//! - AnimationClipPlayer → apply_clip_pose → World pose
//! - Rigidbody gravity via UnityPhysicsPlugin / unity_physics_fixed_step
//! - SaveSceneJsonPrepared / SceneData roundtrip
//!
//! Run: `cargo run -p engine-core --example unity_sim_demo`

use engine_core::animation_apply::{AnimationClip, Vec3Keyframe};
use engine_core::app::AppBuilder;
use engine_core::behaviour::BehaviourState;
use engine_core::components::{Rigidbody, SphereCollider};
use engine_core::plugins::CorePlugins;
use engine_core::sample_scripts::{AnimationClipPlayer, register_sample_scripts};
use engine_core::scene_runtime::SceneRuntime;
use engine_core::serialization::SaveSceneJsonPrepared;
use engine_core::world::World;
use engine_math::Vec3;

fn main() {
    println!("=== Unity Sim Demo (animation + physics + SceneData) ===\n");
    println!(
        "unity_world_primary_feature = {}",
        World::unity_world_primary_feature()
    );

    register_sample_scripts();

    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins);
    builder.add_plugin(engine_physics::UnityPhysicsPlugin);
    let mut app = builder.build();
    app.set_running(true);

    // ── Author ──
    {
        let rt = app
            .world_mut()
            .get_resource_mut::<SceneRuntime>()
            .expect("SceneRuntime");

        // Animated hero
        let hero = rt.world.CreateGameObject("Hero");
        rt.world.SetLocalPosition(hero, Vec3::ZERO);
        let clip = AnimationClip::new("walk", 1.0)
            .with_position_track(vec![
                Vec3Keyframe::linear(0.0, Vec3::ZERO),
                Vec3Keyframe::linear(1.0, Vec3::new(6.0, 0.0, 0.0)),
            ])
            .looping(false);
        rt.world.AddMonoBehaviour(
            hero,
            AnimationClipPlayer {
                clip: Some(clip),
                time: 0.0,
                speed: 1.0,
                playing: true,
                state: BehaviourState::new(),
            },
        );

        // Falling ball
        let ball = rt.world.CreateGameObject("Ball");
        rt.world.SetLocalPosition(ball, Vec3::new(0.0, 5.0, 0.0));
        rt.world.AddComponent(
            ball,
            Rigidbody {
                use_gravity: true,
                mass: 1.0,
                ..Default::default()
            },
        );
        rt.world.AddComponent(
            ball,
            SphereCollider {
                center: Vec3::ZERO,
                radius: 0.3,
                is_trigger: false,
            },
        );

        // Floor
        let floor = rt.world.CreateGameObject("Floor");
        rt.world.SetLocalPosition(floor, Vec3::new(0.0, 0.0, 0.0));
        rt.world.AddComponent(
            floor,
            Rigidbody {
                use_gravity: false,
                is_kinematic: true,
                mass: 1.0,
                ..Default::default()
            },
        );
        rt.world.AddComponent(
            floor,
            SphereCollider {
                center: Vec3::ZERO,
                radius: 2.0,
                is_trigger: false,
            },
        );
    }

    // ── Prepared SceneData ──
    let json = {
        let rt = app
            .world_mut()
            .get_resource_mut::<SceneRuntime>()
            .expect("SceneRuntime");
        SaveSceneJsonPrepared(&mut rt.world, "SimDemo").expect("prepared save")
    };
    println!("Scene JSON {} bytes", json.len());
    assert!(json.contains("AnimationClipPlayer"));
    assert!(json.contains("SphereCollider"));
    assert!(json.contains("Hero"));

    // Reload into a clean SceneRuntime on the same App ECS (drop + insert).
    {
        let ecs = app.world_mut();
        let _ = ecs.remove_resource::<SceneRuntime>();
        let mut rt = SceneRuntime::new();
        rt.load_scene_json(
            "SimDemo",
            &json,
            engine_core::scene_management::LoadSceneMode::Single,
        )
        .expect("load");
        rt.mark_needs_awake();
        ecs.insert_resource(rt);
    }

    let (hero_x0, ball_y0) = {
        let rt = app
            .world_mut()
            .get_resource::<SceneRuntime>()
            .expect("SceneRuntime");
        let hero = rt.world.Find("Hero").expect("Hero");
        let ball = rt.world.Find("Ball").expect("Ball");
        (
            rt.world.GetTransform(hero).unwrap().LocalPosition().x,
            rt.world.GetTransform(ball).unwrap().LocalPosition().y,
        )
    };
    println!("After load: hero_x={hero_x0:.3} ball_y={ball_y0:.3}");

    // ── Simulate (plugin FixedUpdate only; no manual double-step) ──
    for frame in 0..40 {
        app.run_with_lifecycle(0.02);
        if frame == 19 || frame == 39 {
            let (hx, by) = {
                let rt = app.world_mut().get_resource::<SceneRuntime>().unwrap();
                let hero = rt.world.Find("Hero").unwrap();
                let ball = rt.world.Find("Ball").unwrap();
                (
                    rt.world.GetTransform(hero).unwrap().LocalPosition().x,
                    rt.world.GetTransform(ball).unwrap().LocalPosition().y,
                )
            };
            println!("  frame {:>2} hero_x={hx:.3} ball_y={by:.3}", frame + 1);
        }
    }

    let (hero_x, ball_y) = {
        let rt = app
            .world_mut()
            .get_resource::<SceneRuntime>()
            .expect("SceneRuntime");
        let hero = rt.world.Find("Hero").expect("Hero");
        let ball = rt.world.Find("Ball").expect("Ball");
        (
            rt.world.GetTransform(hero).unwrap().LocalPosition().x,
            rt.world.GetTransform(ball).unwrap().LocalPosition().y,
        )
    };

    println!("\nFinal: hero_x={hero_x:.3} ball_y={ball_y:.3}");
    assert!(
        hero_x > hero_x0 + 0.5,
        "animation should advance Hero X; {hero_x0} -> {hero_x}"
    );
    assert!(
        ball_y < ball_y0 - 0.3,
        "physics should drop Ball Y; {ball_y0} -> {ball_y}"
    );
    println!("=== Demo Complete ===");
}
