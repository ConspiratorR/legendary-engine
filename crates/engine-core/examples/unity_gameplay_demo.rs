//! Unity Gameplay Demo — gameplay on SceneRuntime / MonoBehaviour only.
//!
//! Demonstrates the roadmap exit condition: a game loop that talks to
//! **Unity World** APIs (GameObject, Transform, MonoBehaviour, coroutines)
//! without hand-built ECS components.
//!
//! Run: `cargo run -p engine-core --example unity_gameplay_demo`

use engine_core::app::AppBuilder;
use engine_core::components::Material;
use engine_core::context::Context;
use engine_core::coroutine::CoroutineStep;
use engine_core::monobehaviour::MonoBehaviour;
use engine_core::plugins::CorePlugins;
use engine_core::{Behaviour, BehaviourState, Component, GameObjectHandle};
use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Moves the host every Update (Unity-style script).
struct Spinner {
    state: BehaviourState,
    turns: Arc<AtomicU32>,
}

impl Component for Spinner {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Behaviour for Spinner {
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

impl MonoBehaviour for Spinner {
    fn Start(&mut self, ctx: &mut Context) {
        let me = self.gameobject_handle().expect("host");
        println!("[Spinner] Start");
        ctx.StartCoroutine(
            me,
            "Pulse",
            vec![
                CoroutineStep::Wait(0.08),
                CoroutineStep::Call("Pulse".into()),
                CoroutineStep::Wait(0.08),
                CoroutineStep::Call("Pulse".into()),
            ],
        );
    }

    fn Update(&mut self, ctx: &mut Context) {
        let me = self.gameobject_handle().expect("host");
        let dt = ctx.DeltaTime().max(0.0);
        let time = ctx.Time();
        if let Some(t) = ctx.world.GetTransformMut(me) {
            let p = t.LocalPosition();
            t.SetLocalPosition(engine_math::Vec3::new(
                p.x + 120.0 * dt,
                40.0 * (time * 4.0).sin(),
                p.z,
            ));
            t.Rotate(engine_math::Vec3::new(0.0, 0.0, 90.0 * dt));
        }
    }

    fn on_message(&mut self, method: &str, _value: Option<&dyn Any>, _ctx: &mut Context) {
        if method == "Pulse" {
            let n = self.turns.fetch_add(1, Ordering::SeqCst) + 1;
            println!("[Spinner] Pulse #{n}");
        }
    }
}

fn main() {
    println!("=== Unity Gameplay Demo (SceneRuntime only) ===\n");

    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins); // SceneRuntimePlugin included

    let turns = Arc::new(AtomicU32::new(0));
    let turns_in = turns.clone();

    // run_with_lifecycle drives PlayerLoop phases (not startup list);
    // spawn on the first Update frame.
    {
        let spawned = Arc::new(AtomicBool::new(false));
        let spawned_sys = spawned.clone();
        let turns_in = turns_in.clone();
        builder.add_system_to_phase(
            engine_core::player_loop::Phase::Update,
            move |ctx: &mut Context| {
                if spawned_sys.load(Ordering::SeqCst) {
                    return;
                }
                let player = ctx.world.CreateGameObject("Player");
                ctx.world.AddComponent(
                    player,
                    Material {
                        base_color: [0.3, 0.8, 1.0, 1.0],
                        ..Default::default()
                    },
                );
                if let Some(t) = ctx.world.GetTransformMut(player) {
                    t.SetLocalPosition(engine_math::Vec3::new(0.0, 0.0, 0.0));
                }
                ctx.world.AddMonoBehaviour(
                    player,
                    Spinner {
                        state: BehaviourState::new(),
                        turns: turns_in.clone(),
                    },
                );
                spawned_sys.store(true, Ordering::SeqCst);
                println!("Spawned Player + Spinner via Unity World API");
            },
        );
    }

    let mut app = builder.build();
    app.set_running(true);

    for frame in 0..40 {
        app.run_with_lifecycle(0.033);
        if frame % 8 == 0 {
            let world = app.unity_world_ref().expect("SceneRuntime");
            let pos = world
                .Find("Player")
                .and_then(|h| world.GetTransform(h).map(|t| t.LocalPosition()));
            println!(
                "frame {:>2}  Player position = {:?}  pulses={}",
                frame + 1,
                pos,
                turns.load(Ordering::SeqCst)
            );
        }
    }

    let pulses = turns.load(Ordering::SeqCst);
    assert!(pulses >= 2, "coroutine Call should have fired");
    println!("\n=== Demo Complete ===");
    println!(
        "Gameplay used only: CreateGameObject / AddComponent / Transform / MonoBehaviour / coroutine."
    );
}
