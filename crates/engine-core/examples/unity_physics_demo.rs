//! Unity physics bridge demo — Rigidbody gravity → World pose via fixed step.
//!
//! Run: `cargo run -p engine-core --example unity_physics_demo`
//!
//! Uses `engine_physics::unity_physics_fixed_step` (runtime counterpart of
//! editor Play-host physics sync). No GPU / window.

use engine_core::app::AppBuilder;
use engine_core::components::{Rigidbody, SphereCollider};
use engine_core::plugins::CorePlugins;
use engine_core::scene_runtime::SceneRuntime;
use engine_math::Vec3;

fn main() {
    println!("=== Unity Physics Bridge Demo ===\n");

    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins);
    builder.add_plugin(engine_physics::UnityPhysicsPlugin);

    let mut app = builder.build();
    app.set_running(true);

    // Spawn a falling body on the first lifecycle frame via Phase::Update? Prefer
    // mutate SceneRuntime directly after build.
    {
        let runtime = app
            .world_mut()
            .get_resource_mut::<SceneRuntime>()
            .expect("SceneRuntime from CorePlugins");
        let go = runtime.world.CreateGameObject("Ball");
        runtime.world.SetLocalPosition(go, Vec3::new(0.0, 8.0, 0.0));
        runtime.world.AddComponent(
            go,
            Rigidbody {
                use_gravity: true,
                mass: 1.0,
                ..Default::default()
            },
        );
        runtime.world.AddComponent(
            go,
            SphereCollider {
                center: Vec3::ZERO,
                radius: 0.5,
                is_trigger: false,
            },
        );
    }

    println!("Start Y = 8.0");
    for frame in 0..30 {
        app.run_with_lifecycle(0.02);

        // Deterministic extra bridge step (plugin FixedUpdate may also run).
        {
            let ecs = app.world_mut();
            if let Some(mut runtime) = ecs.remove_resource::<SceneRuntime>() {
                engine_physics::unity_physics_fixed_step(&mut runtime, ecs, 0.02);
                ecs.insert_resource(runtime);
            }
        }

        if frame % 5 == 0 || frame == 29 {
            let y = {
                let rt = app.unity_world_ref().expect("unity world");
                let h = rt.Find("Ball").expect("Ball");
                rt.GetTransform(h).unwrap().LocalPosition().y
            };
            println!("  frame {:>2} Y = {:.4}", frame + 1, y);
        }
    }

    let end_y = {
        let rt = app.unity_world_ref().expect("unity world");
        let h = rt.Find("Ball").expect("Ball");
        rt.GetTransform(h).unwrap().LocalPosition().y
    };
    println!("\nEnd Y = {end_y:.4}");
    assert!(
        end_y < 8.0 - 0.1,
        "World Y should fall under gravity via unity physics bridge"
    );
    println!("=== Demo Complete ===");
}
