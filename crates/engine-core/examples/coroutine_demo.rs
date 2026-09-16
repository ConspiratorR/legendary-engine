//! # Coroutine Demo
//!
//! Demonstrates Unity-style coroutines:
//! - `WaitForSeconds` / `WaitForSecondsRealtime` (`Wait` / `WaitRealtime`)
//! - `WaitForFixedUpdate` / `WaitForEndOfFrame`
//! - `WaitUntil` / `WaitWhile`
//! - `SendMessage` via `Call` and `on_message`
//! - `StopCoroutine` by name and `StopAllCoroutines`
//! - Scene unload keeps DontDestroyOnLoad coroutines, stops the rest
//!
//! Run: `cargo run -p engine-core --example coroutine_demo`

use engine_core::app::AppBuilder;
use engine_core::context::Context;
use engine_core::coroutine::{CoroutineId, CoroutineStep};
use engine_core::monobehaviour::MonoBehaviour;
use engine_core::plugins::CorePlugins;
use engine_core::time::Time;
use engine_core::{Behaviour, BehaviourState, Component, GameObjectHandle};
use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Counts frame-end ticks so WaitEndOfFrame / sequence progress is visible.
struct PulseCounter {
    hits: Arc<AtomicU32>,
    state: BehaviourState,
}

impl Component for PulseCounter {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Behaviour for PulseCounter {
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

impl MonoBehaviour for PulseCounter {
    fn on_message(&mut self, method: &str, _value: Option<&dyn Any>, _ctx: &mut Context) {
        if method == "Pulse" {
            let n = self.hits.fetch_add(1, Ordering::SeqCst) + 1;
            println!("  [{}] on_message Pulse #{}", self.name(), n);
        }
    }
}

impl PulseCounter {
    fn new() -> Self {
        Self {
            hits: Arc::new(AtomicU32::new(0)),
            state: BehaviourState::new(),
        }
    }

    fn name(&self) -> &'static str {
        "PulseCounter"
    }

    fn hits(&self) -> Arc<AtomicU32> {
        self.hits.clone()
    }
}

fn print_count(label: &str, co: usize) {
    println!("  {label}: {co} coroutine(s) running");
}

fn main() {
    println!("=== Coroutine Demo ===\n");

    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins); // includes SceneRuntimePlugin
    let mut app = builder.build();

    // --- Part 1: WaitForSeconds blink ---
    println!("--- Part 1: WaitForSeconds blink ---");
    {
        let rt = app.scene_runtime_mut().expect("SceneRuntime");
        let flash = rt.spawn("Flash");
        rt.world.StartCoroutine(
            flash,
            "Blink",
            vec![
                CoroutineStep::Wait(0.04),
                CoroutineStep::SetActive(false),
                CoroutineStep::Wait(0.04),
                CoroutineStep::SetActive(true),
                CoroutineStep::Call("Pulse".into()),
            ],
        );
        print_count("after Start", rt.world.CoroutineCount());

        // Drive ~10 frames at 0.03s scaled time (pause simulation via App lifecycle)
        for frame in 0..10 {
            app.run_with_lifecycle(0.03);
            let rt = app.scene_runtime().unwrap();
            if frame % 3 == 0 || rt.world.CoroutineCount() == 0 {
                let active = rt.world.IsActive(flash);
                println!(
                    "  frame {} active={active} cors={}",
                    frame + 1,
                    rt.world.CoroutineCount()
                );
            }
        }
        let rt = app.scene_runtime().unwrap();
        print_count("after blink", rt.world.CoroutineCount());
    }

    // --- Part 2: WaitUntil + WaitWhile ---
    println!("\n--- Part 2: WaitUntil / WaitWhile ---");
    {
        let ready = Arc::new(AtomicBool::new(false));
        let busy = Arc::new(AtomicBool::new(true));
        let ready2 = ready.clone();
        let busy2 = busy.clone();

        let rt = app.scene_runtime_mut().unwrap();
        let agent = rt.spawn("Agent");
        rt.world.StartCoroutine(
            agent,
            "Mission",
            vec![
                CoroutineStep::WaitWhile(Arc::new(move |_w, _o| busy2.load(Ordering::SeqCst))),
                CoroutineStep::WaitUntil(Arc::new(move |_w, _o| ready2.load(Ordering::SeqCst))),
                CoroutineStep::Call("Pulse".into()),
            ],
        );

        app.run_with_lifecycle(0.016);
        print_count(
            "still busy",
            app.scene_runtime().unwrap().world.CoroutineCount(),
        );

        busy.store(false, Ordering::SeqCst);
        app.run_with_lifecycle(0.016);
        print_count(
            "busy cleared, waiting ready",
            app.scene_runtime().unwrap().world.CoroutineCount(),
        );

        ready.store(true, Ordering::SeqCst);
        app.run_with_lifecycle(0.016);
        print_count(
            "mission done",
            app.scene_runtime().unwrap().world.CoroutineCount(),
        );
    }

    // --- Part 3: StopCoroutine by name / StopAllCoroutines ---
    println!("\n--- Part 3: StopCoroutine by name ---");
    {
        let rt = app.scene_runtime_mut().unwrap();
        let spinner = rt.spawn("Spinner");
        let id: CoroutineId = rt.world.StartCoroutine(
            spinner,
            "Spin",
            vec![
                CoroutineStep::Wait(99.0),
                CoroutineStep::Call("Pulse".into()),
            ],
        );
        rt.world
            .StartCoroutine(spinner, "SpinSlow", vec![CoroutineStep::Wait(99.0)]);
        print_count("two on Spinner", rt.world.CoroutineCount());

        assert!(rt.world.StopCoroutineByName(spinner, "Spin"));
        print_count(
            "after StopCoroutineByName(\"Spin\")",
            rt.world.CoroutineCount(),
        );

        // Still can stop by id
        let _ = id;
        rt.world.StopAllCoroutines(spinner);
        print_count("after StopAllCoroutines", rt.world.CoroutineCount());
    }

    // --- Part 4: WaitForFixedUpdate ---
    println!("\n--- Part 4: WaitForFixedUpdate ---");
    {
        let rt = app.scene_runtime_mut().unwrap();
        let phys = rt.spawn("PhysProbe");
        let fired = Arc::new(AtomicU32::new(0));
        let fired2 = fired.clone();
        rt.world.StartCoroutine(
            phys,
            "EachFixed",
            vec![
                CoroutineStep::WaitFixedUpdate,
                CoroutineStep::Action(Arc::new(move |_w, _o| {
                    fired2.fetch_add(1, Ordering::SeqCst);
                })),
            ],
        );

        // Yield WaitForFixedUpdate waits until a later frame's FixedUpdate has run.
        // 0.05s → ~2 fixed steps (0.02); need an extra frame after the yield.
        for frame in 0..3 {
            app.run_with_lifecycle(0.05);
            let n = fired.load(Ordering::SeqCst);
            println!("  frame {} → fired {n} time(s)", frame + 1);
        }
        let n = fired.load(Ordering::SeqCst);
        assert!(n >= 1, "expected FixedUpdate resume");
        app.scene_runtime_mut()
            .unwrap()
            .world
            .StopAllCoroutines(phys);
    }

    // --- Part 5: DontDestroyOnLoad survives unload ---
    println!("\n--- Part 5: DontDestroyOnLoad coroutines ---");
    {
        let rt = app.scene_runtime_mut().unwrap();
        let keeper = rt.spawn("Persistent");
        rt.world.DontDestroyOnLoad(keeper);
        rt.world
            .StartCoroutine(keeper, "KeepAlive", vec![CoroutineStep::Wait(99.0)]);

        let temp = rt.spawn("LevelRoot");
        rt.world
            .StartCoroutine(temp, "LevelLogic", vec![CoroutineStep::Wait(99.0)]);

        let handle = rt.scenes.register_loaded_scene("Level", vec![temp]);
        print_count("before unload", rt.world.CoroutineCount());

        rt.unload_scene(handle).unwrap();
        let rt = app.scene_runtime().unwrap();
        print_count("after unload (DDOL kept)", rt.world.CoroutineCount());
        assert!(rt.world.is_valid(keeper));
        assert_eq!(rt.world.CoroutineCount(), 1);
    }

    // --- Part 6: MonoBehaviour via Context.StartCoroutine ---
    println!("\n--- Part 6: MonoBehaviour + Context.StartCoroutine ---");
    {
        let mut probe = PulseCounter::new();
        let hits = probe.hits();
        let rt = app.scene_runtime_mut().unwrap();
        let host = rt.spawn("ScriptHost");
        rt.world.AddMonoBehaviour(host, probe);
        // Scripts use Context inside lifecycle callbacks:
        {
            let time = Time::default();
            let mut events = engine_core::event::EventBus::new();
            let mut ctx = Context::new(&mut rt.world, time, 0, &mut events);
            let me = host;
            ctx.StartCoroutine(
                me,
                "FromScript",
                vec![
                    CoroutineStep::Wait(0.01),
                    CoroutineStep::Action(Arc::new(move |_w, _o| {
                        println!("  [FromScript] ran via Context.StartCoroutine");
                    })),
                ],
            );
        }

        for _ in 0..5 {
            app.run_with_lifecycle(0.02);
        }
        println!(
            "  Pulse hits from SendMessage = {}",
            hits.load(Ordering::SeqCst)
        );
    }

    println!("\n=== Demo Complete ===");
    println!("Coroutines: Wait / WaitRealtime / WaitFixedUpdate / WaitUntil / WaitWhile");
    println!("Stop by id or name; DDOL objects survive scene unload.");
}
