//! Unity physics bridge demo — Rigidbody gravity → World pose via `UnityPhysicsPlugin`.
//!
//! Run: `cargo run -p engine-core --example unity_physics_demo`
//!
//! Uses only the plugin FixedUpdate path (no manual double-step). No GPU/window.

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

    println!("Start Y = 8.0 (plugin FixedUpdate only)");
    for frame in 0..30 {
        app.run_with_lifecycle(0.02);
        if frame % 5 == 0 || frame == 29 {
            let (y, vy) = {
                let rt = app.unity_world_ref().expect("unity world");
                let h = rt.Find("Ball").expect("Ball");
                let t = rt.GetTransform(h).unwrap();
                let rb = rt.GetComponent::<Rigidbody>(h).unwrap();
                (t.LocalPosition().y, rb.velocity.y)
            };
            println!("  frame {:>2} Y = {:.4}  vy = {:.4}", frame + 1, y, vy);
        }
    }

    let end_y = {
        let rt = app.unity_world_ref().expect("unity world");
        let h = rt.Find("Ball").expect("Ball");
        rt.GetTransform(h).unwrap().LocalPosition().y
    };
    println!("\nEnd Y = {end_y:.4}");
    // 30 × 0.02 s ≈ 0.6 s free-fall → Δy ≳ 1 m when velocity accumulates.
    assert!(
        end_y < 7.0,
        "World Y must fall substantially under accumulated gravity; got {end_y}"
    );
    println!("=== Demo Complete ===");
}
