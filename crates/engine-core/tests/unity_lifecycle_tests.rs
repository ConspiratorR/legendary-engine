//! End-to-end Unity lifecycle integration tests.
//!
//! Covers FixedUpdate 0+ steps, SetActive callbacks, Invoke, coroutines,
//! and destroy cleanup against `App::run_with_lifecycle`.

use engine_core::app::AppBuilder;
use engine_core::context::Context;
use engine_core::coroutine::CoroutineStep;
use engine_core::gameobject::GameObjectHandle;
use engine_core::plugins::{CorePlugins, SceneRuntimePlugin};
use engine_core::time::Time;
use engine_core::world::World;
use engine_core::{Behaviour, Component, MonoBehaviour};
use std::any::Any;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct Counters {
    fixed: AtomicU32,
    update: AtomicU32,
    late: AtomicU32,
    start: AtomicU32,
    awake: AtomicU32,
}

impl Counters {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            fixed: AtomicU32::new(0),
            update: AtomicU32::new(0),
            late: AtomicU32::new(0),
            start: AtomicU32::new(0),
            awake: AtomicU32::new(0),
        })
    }
}

#[derive(Debug)]
struct Probe {
    counters: Arc<Counters>,
}

impl Component for Probe {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Behaviour for Probe {
    fn Enabled(&self) -> bool {
        true
    }
    fn SetEnabled(&mut self, _enabled: bool) {}
    fn IsActiveAndEnabled(&self) -> bool {
        true
    }
    fn set_gameobject(&mut self, _handle: GameObjectHandle) {}
    fn gameobject_handle(&self) -> Option<GameObjectHandle> {
        None
    }
}

impl MonoBehaviour for Probe {
    fn Awake(&mut self, _ctx: &mut Context) {
        self.counters.awake.fetch_add(1, Ordering::SeqCst);
    }
    fn Start(&mut self, _ctx: &mut Context) {
        self.counters.start.fetch_add(1, Ordering::SeqCst);
    }
    fn FixedUpdate(&mut self, _ctx: &mut Context) {
        self.counters.fixed.fetch_add(1, Ordering::SeqCst);
    }
    fn Update(&mut self, _ctx: &mut Context) {
        self.counters.update.fetch_add(1, Ordering::SeqCst);
    }
    fn LateUpdate(&mut self, _ctx: &mut Context) {
        self.counters.late.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn test_fixed_update_runs_zero_or_more_times() {
    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins);
    let mut app = builder.build();

    let counters = Counters::new();
    {
        let rt = app.scene_runtime_mut().unwrap();
        let obj = rt.spawn("Probe");
        rt.world.AddMonoBehaviour(
            obj,
            Probe {
                counters: counters.clone(),
            },
        );
    }

    // Small frame: 0.01s → 0 FixedUpdate steps (fixedDelta=0.02)
    app.run_with_lifecycle(0.01);
    assert_eq!(counters.fixed.load(Ordering::SeqCst), 0);
    assert_eq!(counters.update.load(Ordering::SeqCst), 1);
    assert_eq!(counters.late.load(Ordering::SeqCst), 1);
    assert_eq!(counters.awake.load(Ordering::SeqCst), 1);
    assert_eq!(counters.start.load(Ordering::SeqCst), 1);

    // Large frame: accumulator 0.01+0.05=0.06 → 3 FixedUpdate steps
    app.run_with_lifecycle(0.05);
    assert_eq!(counters.fixed.load(Ordering::SeqCst), 3);
    assert_eq!(counters.update.load(Ordering::SeqCst), 2);
    assert_eq!(counters.late.load(Ordering::SeqCst), 2);
}

#[test]
fn test_fixed_ecs_system_runs_per_fixed_step() {
    let hits = Arc::new(AtomicU32::new(0));
    let hits2 = hits.clone();

    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins);
    builder.add_fixed_ecs_system(move |_w: &mut engine_ecs::world::World| {
        hits2.fetch_add(1, Ordering::SeqCst);
    });
    let mut app = builder.build();

    app.run_with_lifecycle(0.01); // 0 fixed steps
    assert_eq!(hits.load(Ordering::SeqCst), 0);

    app.run_with_lifecycle(0.05); // acc 0.06 → 3 fixed steps
    assert_eq!(hits.load(Ordering::SeqCst), 3);
}

#[test]
fn test_time_scale_zero_freezes_fixed_update() {
    let counters = Counters::new();
    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins);
    let mut app = builder.build();
    app.time_mut().set_timeScale(0.0);

    {
        let rt = app.scene_runtime_mut().unwrap();
        let obj = rt.spawn("Probe");
        rt.world.AddMonoBehaviour(
            obj,
            Probe {
                counters: counters.clone(),
            },
        );
    }

    app.run_with_lifecycle(0.05);
    // timeScale=0 → scaled delta 0 → no FixedUpdate; Update still runs once/frame
    assert_eq!(counters.fixed.load(Ordering::SeqCst), 0);
    assert_eq!(counters.update.load(Ordering::SeqCst), 1);
}

#[test]
fn test_set_active_flushes_enable_disable_queue() {
    let mut world = World::new();
    let obj = world.CreateGameObject("Toggle");
    world.SetActive(obj, false);
    assert_eq!(world.pending_enable_disable_count(), 1);

    let time = Time::default();
    let mut events = engine_core::event::EventBus::new();
    world.tick_end_of_frame(0.016, time, 0, &mut events);
    assert_eq!(world.pending_enable_disable_count(), 0);
}

#[test]
fn test_invoke_and_coroutine_cleaned_on_destroy() {
    let mut world = World::new();
    let obj = world.CreateGameObject("Timed");
    world.Invoke(obj, "Boom", 5.0);
    let co = world.StartCoroutine(obj, "Wait", vec![CoroutineStep::Wait(10.0)]);
    assert_eq!(world.pending_invoke_count(), 1);
    assert_eq!(world.CoroutineCount(), 1);

    world.DestroyImmediate(obj);
    assert_eq!(world.pending_invoke_count(), 0);
    assert_eq!(world.CoroutineCount(), 0);
    assert!(!world.IsCoroutineRunning(co));
}

#[test]
fn test_coroutine_set_active_via_lifecycle() {
    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins);
    let mut app = builder.build();

    let obj = {
        let rt = app.scene_runtime_mut().unwrap();
        let obj = rt.spawn("Flash");
        rt.world.StartCoroutine(
            obj,
            "BlinkOff",
            vec![CoroutineStep::Wait(0.0), CoroutineStep::SetActive(false)],
        );
        obj
    };

    // One frame: Wait(0) completes, SetActive runs at EoF tick
    app.run_with_lifecycle(0.016);

    let active = {
        let rt = app.scene_runtime().unwrap();
        rt.world.IsActive(obj)
    };
    assert!(!active);
}

#[test]
fn test_full_frame_pipeline_with_scene_runtime_plugin() {
    let mut builder = AppBuilder::new();
    builder.add_plugin(SceneRuntimePlugin);
    assert!(
        builder
            .world_mut()
            .get_resource::<engine_core::SceneRuntime>()
            .is_some()
    );
}

#[test]
fn test_destroy_end_of_frame_calls_destroy_sequence() {
    #[derive(Debug, Default)]
    struct DestroyProbe {
        log: Arc<Mutex<Vec<&'static str>>>,
    }

    impl Component for DestroyProbe {
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }
    impl Behaviour for DestroyProbe {
        fn Enabled(&self) -> bool {
            true
        }
        fn SetEnabled(&mut self, _e: bool) {}
        fn IsActiveAndEnabled(&self) -> bool {
            true
        }
        fn set_gameobject(&mut self, _h: GameObjectHandle) {}
        fn gameobject_handle(&self) -> Option<GameObjectHandle> {
            None
        }
    }
    impl MonoBehaviour for DestroyProbe {
        fn OnDisable(&mut self, _ctx: &mut Context) {
            self.log.lock().unwrap().push("OnDisable");
        }
        fn OnDestroy(&mut self, _ctx: &mut Context) {
            self.log.lock().unwrap().push("OnDestroy");
        }
    }

    let mut world = World::new();
    let obj = world.CreateGameObject("Doomed");
    let log = Arc::new(Mutex::new(Vec::new()));
    world.AddMonoBehaviour(obj, DestroyProbe { log: log.clone() });

    world.Destroy(obj);
    let time = Time::default();
    let mut events = engine_core::event::EventBus::new();
    world.tick_end_of_frame(0.016, time, 0, &mut events);

    assert_eq!(&*log.lock().unwrap(), &["OnDisable", "OnDestroy"]);
    assert!(!world.is_valid(obj));
}
