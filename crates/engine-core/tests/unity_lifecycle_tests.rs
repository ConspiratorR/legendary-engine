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
fn test_set_active_immediate_dispatches_now() {
    #[derive(Debug, Default)]
    struct EnProbe {
        enables: Arc<AtomicU32>,
        disables: Arc<AtomicU32>,
    }

    impl Component for EnProbe {
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }
    impl Behaviour for EnProbe {
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
    impl MonoBehaviour for EnProbe {
        fn OnEnable(&mut self, _ctx: &mut Context) {
            self.enables.fetch_add(1, Ordering::SeqCst);
        }
        fn OnDisable(&mut self, _ctx: &mut Context) {
            self.disables.fetch_add(1, Ordering::SeqCst);
        }
    }

    let mut world = World::new();
    let obj = world.CreateGameObject("Toggle");
    let enables = Arc::new(AtomicU32::new(0));
    let disables = Arc::new(AtomicU32::new(0));
    world.AddMonoBehaviour(
        obj,
        EnProbe {
            enables: enables.clone(),
            disables: disables.clone(),
        },
    );

    let time = Time::default();
    let mut events = engine_core::event::EventBus::new();
    world.SetActiveImmediate(obj, false, time.clone(), 0, &mut events);
    assert_eq!(disables.load(Ordering::SeqCst), 1);
    assert_eq!(world.pending_enable_disable_count(), 0);

    world.SetActiveImmediate(obj, true, time, 1, &mut events);
    assert_eq!(enables.load(Ordering::SeqCst), 1);
}

#[test]
fn test_get_component_finds_monobehaviour() {
    let mut world = World::new();
    let obj = world.CreateGameObject("Scripted");
    world.AddMonoBehaviour(
        obj,
        Probe {
            counters: Counters::new(),
        },
    );
    assert!(world.GetComponent::<Probe>(obj).is_some());
    assert!(world.HasComponent::<Probe>(obj));
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

#[test]
fn test_wait_until_condition() {
    use engine_core::coroutine::CoroutineStep;
    use std::sync::atomic::AtomicBool;

    let mut world = World::new();
    let obj = world.CreateGameObject("Waiter");
    let flag = Arc::new(AtomicBool::new(false));
    let flag2 = flag.clone();

    let done = Arc::new(AtomicU32::new(0));
    let done2 = done.clone();

    world.StartCoroutine(
        obj,
        "Until",
        vec![
            CoroutineStep::WaitUntil(Arc::new(move |_w, _o| flag2.load(Ordering::SeqCst))),
            CoroutineStep::Action(Arc::new(move |_w, _o| {
                done2.fetch_add(1, Ordering::SeqCst);
            })),
        ],
    );

    // Condition false → still waiting
    world.tick_coroutines(0.016, false);
    assert_eq!(done.load(Ordering::SeqCst), 0);
    assert_eq!(world.CoroutineCount(), 1);

    // Flip condition → next tick completes
    flag.store(true, Ordering::SeqCst);
    world.tick_coroutines(0.016, false);
    assert_eq!(done.load(Ordering::SeqCst), 1);
    assert_eq!(world.CoroutineCount(), 0);
}

#[test]
fn test_wait_while_condition() {
    use engine_core::coroutine::CoroutineStep;
    use std::sync::atomic::AtomicBool;

    let mut world = World::new();
    let obj = world.CreateGameObject("While");
    let busy = Arc::new(AtomicBool::new(true));
    let busy2 = busy.clone();

    let after = Arc::new(AtomicU32::new(0));
    let after2 = after.clone();

    world.StartCoroutine(
        obj,
        "While",
        vec![
            CoroutineStep::WaitWhile(Arc::new(move |_w, _o| busy2.load(Ordering::SeqCst))),
            CoroutineStep::Action(Arc::new(move |_w, _o| {
                after2.fetch_add(1, Ordering::SeqCst);
            })),
        ],
    );

    world.tick_coroutines(0.016, false);
    assert_eq!(after.load(Ordering::SeqCst), 0);

    busy.store(false, Ordering::SeqCst);
    world.tick_coroutines(0.016, false);
    assert_eq!(after.load(Ordering::SeqCst), 1);
}

#[test]
fn test_send_message_dispatches_to_on_message() {
    #[derive(Debug, Default)]
    struct MsgProbe {
        log: Arc<Mutex<Vec<String>>>,
    }

    impl Component for MsgProbe {
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }
    impl Behaviour for MsgProbe {
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
    impl MonoBehaviour for MsgProbe {
        fn on_message(&mut self, method: &str, value: Option<&dyn Any>, _ctx: &mut Context) {
            let extra = value
                .and_then(|v| v.downcast_ref::<i32>())
                .map(|n| format!(":{n}"))
                .unwrap_or_default();
            self.log.lock().unwrap().push(format!("{method}{extra}"));
        }
    }

    let mut world = World::new();
    let obj = world.CreateGameObject("Receiver");
    let log = Arc::new(Mutex::new(Vec::new()));
    world.AddMonoBehaviour(obj, MsgProbe { log: log.clone() });

    world.SendMessage(obj, "Explode");
    world.SendMessageWithValue(obj, "Hit", &5i32);

    assert_eq!(
        &*log.lock().unwrap(),
        &["Explode".to_string(), "Hit:5".to_string()]
    );
}

#[test]
fn test_invoke_calls_on_message() {
    #[derive(Debug, Default)]
    struct InvProbe {
        hits: Arc<AtomicU32>,
    }

    impl Component for InvProbe {
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }
    impl Behaviour for InvProbe {
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
    impl MonoBehaviour for InvProbe {
        fn on_message(&mut self, method: &str, _v: Option<&dyn Any>, _ctx: &mut Context) {
            if method == "Boom" {
                self.hits.fetch_add(1, Ordering::SeqCst);
            }
        }
    }

    let mut world = World::new();
    let obj = world.CreateGameObject("Bomb");
    let hits = Arc::new(AtomicU32::new(0));
    world.AddMonoBehaviour(obj, InvProbe { hits: hits.clone() });

    world.Invoke(obj, "Boom", 0.05);
    world.tick_invokes(0.06);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[test]
fn test_context_start_coroutine() {
    use engine_core::coroutine::CoroutineStep;

    let mut world = World::new();
    let obj = world.CreateGameObject("ViaCtx");
    let fired = Arc::new(AtomicU32::new(0));
    let fired2 = fired.clone();

    {
        let time = Time::default();
        let mut events = engine_core::event::EventBus::new();
        let mut ctx = Context::new(&mut world, time, 0, &mut events);
        let id = ctx.StartCoroutine(
            obj,
            "FromContext",
            vec![CoroutineStep::Action(Arc::new(move |_w, _o| {
                fired2.fetch_add(1, Ordering::SeqCst);
            }))],
        );
        assert_ne!(id, engine_core::CoroutineId::INVALID);
    }

    world.tick_coroutines(0.016, false);
    assert_eq!(fired.load(Ordering::SeqCst), 1);
}

#[test]
fn test_scene_unload_stops_coroutines() {
    use engine_core::coroutine::CoroutineStep;

    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins);
    let mut app = builder.build();

    let (scene_handle, keeper) = {
        let rt = app.scene_runtime_mut().unwrap();
        let keeper = rt.spawn("Persistent");
        rt.world.DontDestroyOnLoad(keeper);
        rt.world
            .StartCoroutine(keeper, "Keep", vec![CoroutineStep::Wait(99.0)]);

        let temp = rt.spawn("TempRoot");
        rt.world
            .StartCoroutine(temp, "Die", vec![CoroutineStep::Wait(99.0)]);

        let handle = rt.scenes.register_loaded_scene("Level", vec![temp]);
        assert_eq!(rt.world.CoroutineCount(), 2);
        (handle, keeper)
    };

    app.scene_runtime_mut()
        .unwrap()
        .unload_scene(scene_handle)
        .unwrap();

    let rt = app.scene_runtime().unwrap();
    // TempRoot destroyed → its coroutine stopped; Persistent still running
    assert_eq!(rt.world.CoroutineCount(), 1);
    assert!(rt.world.is_valid(keeper));
}

#[test]
fn test_scenedata_components_survive_runtime_load_and_tick() {
    use engine_core::SceneRuntime;
    use engine_core::components::{Material, SpriteRenderer};
    use engine_core::event::EventBus;
    use engine_core::serialization::SceneSerializer;

    // Author a small scene with components
    let mut author = World::new();
    let root = author.CreateGameObject("Hero");
    author.AddComponent(
        root,
        Material {
            base_color: [0.2, 0.8, 1.0, 1.0],
            ..Default::default()
        },
    );
    author.AddComponent(
        root,
        SpriteRenderer {
            sprite: "hero.png".into(),
            color: [1.0, 1.0, 1.0, 1.0],
            sorting_order: 2,
            ..Default::default()
        },
    );
    let child = author.CreateGameObject("Weapon");
    author.SetParent(child, Some(root));

    let serializer = SceneSerializer::new();
    let scene = serializer.Save(&author, "Level");
    let json = serde_json::to_string_pretty(&scene).unwrap();
    assert!(json.contains("\"Material\""));
    assert!(json.contains("\"SpriteRenderer\""));
    assert!(json.contains("hero.png"));

    // Load into a fresh SceneRuntime (editor export → play / standalone)
    let mut rt = SceneRuntime::new();
    let loaded: engine_core::serialization::SceneData = serde_json::from_str(&json).unwrap();
    let handles = serializer.Load(&loaded, &mut rt.world);
    assert_eq!(handles.len(), 1);
    rt.mark_needs_awake();

    let hero = rt.world.Find("Hero").expect("Hero loaded");
    let m = rt.world.GetComponent::<Material>(hero).unwrap();
    assert_eq!(m.base_color, [0.2, 0.8, 1.0, 1.0]);
    let sr = rt.world.GetComponent::<SpriteRenderer>(hero).unwrap();
    assert_eq!(sr.sprite, "hero.png");
    assert_eq!(sr.sorting_order, 2);
    assert_eq!(rt.world.GetChildCount(hero), 1);

    // Drive one lifecycle frame without App (Play-host style)
    let mut time = Time::default();
    let mut events = EventBus::new();
    time.update(0.016);
    rt.tick(&time, time.frameCount(), &mut events);
    assert!(time.frameCount() >= 1);
    assert!(rt.world.is_valid(hero));
}

#[test]
fn test_app_load_scenedata_json_into_unity_world() {
    use engine_core::components::Material;
    use engine_core::serialization::SaveSceneJson;

    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins);
    let mut app = builder.build();

    let json = {
        let mut author = World::new();
        let go = author.CreateGameObject("Crate");
        author.AddComponent(
            go,
            Material {
                base_color: [1.0, 0.2, 0.2, 1.0],
                ..Default::default()
            },
        );
        SaveSceneJson(&author, "CrateScene").unwrap()
    };

    {
        let rt = app.scene_runtime_mut().unwrap();
        rt.load_scene_json(
            "CrateScene",
            &json,
            engine_core::scene_management::LoadSceneMode::Single,
        )
        .expect("load scene json");
    }

    app.run_with_lifecycle(0.016);

    let world = app.unity_world().unwrap();
    let crate_go = world.Find("Crate").expect("Crate in unity world");
    let m = world.GetComponent::<Material>(crate_go).unwrap();
    assert_eq!(m.base_color, [1.0, 0.2, 0.2, 1.0]);
    // Identity bridge should have auto-linked after lifecycle sync
    assert_eq!(app.entity_for_gameobject(crate_go).is_some(), true);
}

#[test]
fn test_wait_realtime_advances_when_paused() {
    use engine_core::coroutine::CoroutineStep;

    let mut world = World::new();
    let obj = world.CreateGameObject("PausedUI");
    let done = Arc::new(AtomicU32::new(0));
    let done2 = done.clone();

    world.StartCoroutine(
        obj,
        "Intro",
        vec![
            CoroutineStep::WaitRealtime(0.05),
            CoroutineStep::Action(Arc::new(move |_w, _o| {
                done2.fetch_add(1, Ordering::SeqCst);
            })),
        ],
    );

    // timeScale=0 → scaled Wait would freeze; WaitRealtime still ticks with unscaled dt
    world.tick_coroutines_full(0.0, 0.06, false);
    assert_eq!(done.load(Ordering::SeqCst), 1);
    assert_eq!(world.CoroutineCount(), 0);
}

#[test]
fn test_stop_coroutine_by_name() {
    use engine_core::coroutine::CoroutineStep;

    let mut world = World::new();
    let obj = world.CreateGameObject("NamedCo");
    world.StartCoroutine(obj, "Blink", vec![CoroutineStep::Wait(10.0)]);
    world.StartCoroutine(obj, "Spin", vec![CoroutineStep::Wait(10.0)]);
    assert_eq!(world.CoroutineCount(), 2);

    assert!(world.StopCoroutineByName(obj, "Blink"));
    assert_eq!(world.CoroutineCount(), 1);
    assert!(!world.StopCoroutineByName(obj, "Blink"));
}

#[test]
fn test_setname_setactive_gettransform_stable_under_any_storage_mode() {
    let mut world = World::new();
    let go = world.CreateGameObject("Before");
    world.SetName(go, "After");
    world.SetActive(go, false);
    assert_eq!(world.GetName(go), "After");
    assert!(!world.IsActive(go));
    if let Some(t) = world.GetTransformMut(go) {
        t.SetLocalPosition(engine_math::Vec3::new(1.0, 2.0, 3.0));
    }
    let t = world.GetTransform(go).unwrap();
    assert_eq!(t.LocalPosition(), engine_math::Vec3::new(1.0, 2.0, 3.0));
}

#[cfg(feature = "unity-world-primary")]
#[test]
fn test_lifecycle_with_unity_world_primary_dual_read() {
    use engine_core::components::Material;
    use engine_core::scene_management::LoadSceneMode;
    use engine_core::serialization::SaveSceneJson;

    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins);
    let mut app = builder.build();

    let json = {
        let mut author = World::new();
        let go = author.CreateGameObject("Flagged");
        author.SetTag(go, "Enemy");
        author.AddComponent(
            go,
            Material {
                base_color: [1.0, 0.0, 0.0, 1.0],
                ..Default::default()
            },
        );
        SaveSceneJson(&author, "FlaggedScene").unwrap()
    };

    {
        let rt = app.scene_runtime_mut().unwrap();
        rt.load_scene_json("FlaggedScene", &json, LoadSceneMode::Single)
            .unwrap();
    }

    // Rename through API — dual-read GetName must see it
    {
        let world = app.unity_world().expect("SceneRuntime");
        let go = world.Find("Flagged").expect("loaded");
        let _ = world.GetTransform(go).expect("transform");
        assert_eq!(world.GetTag(go), "Enemy");
    }
    {
        let world = app.unity_world().unwrap();
        let go = world.Find("Flagged").unwrap();
        world.SetName(go, "FlaggedRenamed");
    }

    app.run_with_lifecycle(0.016);

    let world = app.unity_world().unwrap();
    let go = world
        .Find("FlaggedRenamed")
        .or_else(|| world.Find("Flagged"))
        .expect("object after rename");
    assert_eq!(world.GetName(go), "FlaggedRenamed");
    assert_eq!(world.GetTag(go), "Enemy");
    let _ = world.GetTransform(go).unwrap();
}
