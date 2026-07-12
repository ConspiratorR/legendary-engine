use engine_core::app::{App, AppBuilder};
use engine_core::config::Config;
use engine_core::plugin::Plugin;
use engine_core::time::Time;
use engine_ecs::world::World;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

struct CounterPlugin {
    value: u32,
}

impl Plugin for CounterPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let world = app.world_mut();
        let e = world.spawn();
        world.add_component(e, self.value);
    }
}

struct OrderTracker {
    log: Arc<std::sync::Mutex<Vec<&'static str>>>,
    name: &'static str,
}

impl Plugin for OrderTracker {
    fn build(&self, app: &mut AppBuilder) {
        let log = self.log.clone();
        let name = self.name;
        log.lock().unwrap().push(name);
        app.insert_resource(log);
    }
}

// ---------------------------------------------------------------------------
// App creation tests
// ---------------------------------------------------------------------------

#[test]
fn test_app_creation() {
    let app = App::new();
    // App should have an InputManager resource
    assert!(
        app.world
            .get_resource::<engine_input::input_manager::InputManager>()
            .is_some()
    );
}

#[test]
fn test_app_default() {
    let app = App::default();
    assert!(
        app.world
            .get_resource::<engine_input::input_manager::InputManager>()
            .is_some()
    );
}

#[test]
fn test_app_builder_new() {
    let builder = AppBuilder::new();
    let app = builder.build();
    assert!(
        app.world
            .get_resource::<engine_input::input_manager::InputManager>()
            .is_some()
    );
}

// ---------------------------------------------------------------------------
// Plugin registration tests
// ---------------------------------------------------------------------------

#[test]
fn test_plugin_registration() {
    let mut builder = AppBuilder::new();
    builder.add_plugin(CounterPlugin { value: 42 });
    let app = builder.build();

    let entities = app.world.component_entities::<u32>();
    assert_eq!(entities.len(), 1);
    let val = app.world.get_by_index::<u32>(entities[0]).unwrap();
    assert_eq!(*val, 42);
}

#[test]
fn test_multiple_plugins() {
    let mut builder = AppBuilder::new();
    builder.add_plugin(CounterPlugin { value: 10 });
    builder.add_plugin(CounterPlugin { value: 20 });
    let app = builder.build();

    let entities = app.world.component_entities::<u32>();
    assert_eq!(entities.len(), 2);

    let mut values: Vec<u32> = entities
        .iter()
        .map(|&e| *app.world.get_by_index::<u32>(e).unwrap())
        .collect();
    values.sort();
    assert_eq!(values, vec![10, 20]);
}

#[test]
fn test_plugin_execution_order() {
    let log = Arc::new(std::sync::Mutex::new(Vec::<&'static str>::new()));
    let mut builder = AppBuilder::new();
    builder.add_plugin(OrderTracker {
        log: log.clone(),
        name: "first",
    });
    builder.add_plugin(OrderTracker {
        log: log.clone(),
        name: "second",
    });
    builder.add_plugin(OrderTracker {
        log: log.clone(),
        name: "third",
    });

    let entries = log.lock().unwrap();
    assert_eq!(*entries, vec!["first", "second", "third"]);
}

// ---------------------------------------------------------------------------
// System execution tests
// ---------------------------------------------------------------------------

#[test]
fn test_system_execution() {
    let counter = Arc::new(AtomicU32::new(0));
    let c = counter.clone();

    let mut builder = AppBuilder::new();
    builder.add_system(move |_world: &mut World| {
        c.fetch_add(1, Ordering::Relaxed);
    });

    let mut app = builder.build();
    app.run();
    app.run();
    app.run();

    assert_eq!(counter.load(Ordering::Relaxed), 3);
}

#[test]
fn test_system_reads_resource() {
    let mut builder = AppBuilder::new();
    builder.insert_resource(100u32);

    let result = Arc::new(AtomicU32::new(0));
    let r = result.clone();
    builder.add_system(move |world: &mut World| {
        if let Some(val) = world.get_resource::<u32>() {
            r.store(*val, Ordering::Relaxed);
        }
    });

    let mut app = builder.build();
    app.run();

    assert_eq!(result.load(Ordering::Relaxed), 100);
}

#[test]
fn test_system_modifies_component() {
    let mut builder = AppBuilder::new();
    let world = builder.world_mut();
    let e = world.spawn();
    world.add_component(e, 0u32);

    builder.add_system(|world: &mut World| {
        let entities = world.component_entities::<u32>();
        for &entity in &entities {
            if let Some(val) = world.get_by_index_mut::<u32>(entity) {
                *val += 1;
            }
        }
    });

    let mut app = builder.build();
    app.run();
    app.run();

    let entities = app.world.component_entities::<u32>();
    let val = app.world.get_by_index::<u32>(entities[0]).unwrap();
    assert_eq!(*val, 2);
}

#[test]
fn test_multiple_systems_sequential_order() {
    let log = Arc::new(std::sync::Mutex::new(Vec::<u32>::new()));

    let l1 = log.clone();
    let l2 = log.clone();
    let l3 = log.clone();

    let mut builder = AppBuilder::new();
    builder.add_system(move |_world: &mut World| {
        l1.lock().unwrap().push(1);
    });
    builder.add_system(move |_world: &mut World| {
        l2.lock().unwrap().push(2);
    });
    builder.add_system(move |_world: &mut World| {
        l3.lock().unwrap().push(3);
    });

    let mut app = builder.build();
    app.run();

    let entries = log.lock().unwrap();
    assert_eq!(*entries, vec![1, 2, 3]);
}

// ---------------------------------------------------------------------------
// Time management tests
// ---------------------------------------------------------------------------

#[test]
fn test_time_creation() {
    let time = Time::new();
    assert_eq!(time.frame_count(), 0);
    assert_eq!(time.elapsed_seconds(), 0.0);
    assert_eq!(time.delta_seconds(), 0.0); // no update yet
}

#[test]
fn test_time_default() {
    let time = Time::default();
    assert_eq!(time.frame_count(), 0);
}

#[test]
fn test_time_update() {
    let mut time = Time::new();
    time.update(0.016);
    assert_eq!(time.frame_count(), 1);
    assert!(time.elapsed_seconds() >= 0.0);
    assert!(time.delta_seconds() >= 0.0);
}

#[test]
fn test_time_multiple_updates() {
    let mut time = Time::new();
    for _ in 0..10 {
        time.update(0.016);
    }
    assert_eq!(time.frame_count(), 10);
    assert!(time.elapsed_seconds() > 0.0);
}

// ---------------------------------------------------------------------------
// Configuration tests
// ---------------------------------------------------------------------------

#[test]
fn test_config_default() {
    let config = Config::default();
    assert!(config.is_empty());
    assert_eq!(config.len(), 0);
}

#[test]
fn test_config_get_set() {
    let mut config = Config::new();
    config.set("name".to_string(), "rustengine".to_string());
    assert_eq!(config.get("name"), Some(&"rustengine".to_string()));
    assert_eq!(config.len(), 1);
}

#[test]
fn test_config_missing_key() {
    let config = Config::new();
    assert!(config.get("nonexistent").is_none());
}

#[test]
fn test_config_get_or() {
    let config = Config::new();
    assert_eq!(config.get_or("missing", "default"), "default");

    let mut config = Config::new();
    config.set("key".to_string(), "value".to_string());
    assert_eq!(config.get_or("key", "default"), "value");
}

#[test]
fn test_config_get_bool() {
    let mut config = Config::new();
    config.set("a".to_string(), "true".to_string());
    config.set("b".to_string(), "false".to_string());
    config.set("c".to_string(), "1".to_string());
    config.set("d".to_string(), "0".to_string());
    config.set("e".to_string(), "yes".to_string());
    config.set("f".to_string(), "no".to_string());
    config.set("g".to_string(), "invalid".to_string());

    assert_eq!(config.get_bool("a"), Some(true));
    assert_eq!(config.get_bool("b"), Some(false));
    assert_eq!(config.get_bool("c"), Some(true));
    assert_eq!(config.get_bool("d"), Some(false));
    assert_eq!(config.get_bool("e"), Some(true));
    assert_eq!(config.get_bool("f"), Some(false));
    assert_eq!(config.get_bool("g"), None);
    assert_eq!(config.get_bool("missing"), None);
}

#[test]
fn test_config_get_i32() {
    let mut config = Config::new();
    config.set("num".to_string(), "42".to_string());
    config.set("bad".to_string(), "abc".to_string());

    assert_eq!(config.get_i32("num"), Some(42));
    assert_eq!(config.get_i32("bad"), None);
    assert_eq!(config.get_i32("missing"), None);
}

#[test]
fn test_config_get_f32() {
    let mut config = Config::new();
    config.set("pi".to_string(), "3.14".to_string());
    config.set("bad".to_string(), "abc".to_string());

    assert!((config.get_f32("pi").unwrap() - std::f32::consts::PI).abs() < 0.01);
    assert_eq!(config.get_f32("bad"), None);
    assert_eq!(config.get_f32("missing"), None);
}

#[test]
fn test_config_from_toml() {
    let toml = r#"
# comment
name = "test"
count = "10"
enabled = "true"
"#;
    let config = Config::from_toml(toml).unwrap();
    assert_eq!(config.get("name"), Some(&"test".to_string()));
    assert_eq!(config.get_i32("count"), Some(10));
    assert_eq!(config.get_bool("enabled"), Some(true));
}

#[test]
fn test_config_keys() {
    let mut config = Config::new();
    config.set("a".to_string(), "1".to_string());
    config.set("b".to_string(), "2".to_string());
    config.set("c".to_string(), "3".to_string());

    let mut keys: Vec<&String> = config.keys().collect();
    keys.sort();
    assert_eq!(
        keys,
        vec![&"a".to_string(), &"b".to_string(), &"c".to_string()]
    );
}

// ---------------------------------------------------------------------------
// Integration: plugin + system + resource
// ---------------------------------------------------------------------------

#[test]
fn test_plugin_adds_system_that_modifies_resource() {
    struct IncrementPlugin;

    impl Plugin for IncrementPlugin {
        fn build(&self, app: &mut AppBuilder) {
            app.insert_resource(0u32);
            app.add_system(|world: &mut World| {
                if let Some(val) = world.get_resource_mut::<u32>() {
                    *val += 1;
                }
            });
        }
    }

    let mut builder = AppBuilder::new();
    builder.add_plugin(IncrementPlugin);
    let mut app = builder.build();

    app.run();
    app.run();
    app.run();

    let val = app.world.get_resource::<u32>().unwrap();
    assert_eq!(*val, 3);
}

#[test]
fn test_parallel_schedule_system_execution() {
    let counter = Arc::new(AtomicUsize::new(0));
    let c1 = counter.clone();
    let c2 = counter.clone();

    let mut builder = AppBuilder::new();
    builder.with_parallel_schedule(2);
    builder.add_system(move |_world: &mut World| {
        c1.fetch_add(1, Ordering::Relaxed);
    });
    builder.add_system(move |_world: &mut World| {
        c2.fetch_add(1, Ordering::Relaxed);
    });

    let mut app = builder.build();
    app.run();

    assert_eq!(counter.load(Ordering::Relaxed), 2);
}

// ---------------------------------------------------------------------------
// Additional hardening tests
// ---------------------------------------------------------------------------

#[test]
fn test_builder_insert_resource() {
    let mut builder = AppBuilder::new();
    builder.insert_resource(42u64);
    builder.insert_resource("hello".to_string());

    let app = builder.build();
    assert_eq!(*app.world.get_resource::<u64>().unwrap(), 42);
    assert_eq!(
        app.world.get_resource::<String>().unwrap().as_str(),
        "hello"
    );
}

#[test]
fn test_plugin_registers_resource_via_builder() {
    struct ConfigPlugin;

    impl Plugin for ConfigPlugin {
        fn build(&self, app: &mut AppBuilder) {
            app.insert_resource(Config::from_toml("name = \"test\"\nport = \"8080\"").unwrap());
        }
    }

    let mut builder = AppBuilder::new();
    builder.add_plugin(ConfigPlugin);
    let app = builder.build();

    let config = app.world.get_resource::<Config>().unwrap();
    assert_eq!(config.get("name"), Some(&"test".to_string()));
    assert_eq!(config.get_i32("port"), Some(8080));
}

#[test]
fn test_hook_execution_order() {
    let log = Arc::new(std::sync::Mutex::new(Vec::<&'static str>::new()));
    let l1 = log.clone();
    let l2 = log.clone();
    let l3 = log.clone();

    let mut builder = AppBuilder::new();
    builder.add_pre_update_hook(Box::new(move |_app| {
        l1.lock().unwrap().push("pre");
    }));
    builder.add_system(move |_world: &mut World| {
        l2.lock().unwrap().push("system");
    });
    builder.add_post_update_hook(Box::new(move |_app| {
        l3.lock().unwrap().push("post");
    }));

    let mut app = builder.build();
    app.run();

    let entries = log.lock().unwrap();
    assert_eq!(*entries, vec!["pre", "system", "post"]);
}

#[test]
fn test_config_overwrite() {
    let mut config = Config::new();
    config.set("key".to_string(), "old".to_string());
    config.set("key".to_string(), "new".to_string());
    assert_eq!(config.get("key"), Some(&"new".to_string()));
    assert_eq!(config.len(), 1);
}

#[test]
fn test_config_empty_toml() {
    let config = Config::from_toml("").unwrap();
    assert!(config.is_empty());
}

#[test]
fn test_config_comments_only() {
    let toml = "# just a comment\n# another comment\n";
    let config = Config::from_toml(toml).unwrap();
    assert!(config.is_empty());
}

#[test]
fn test_time_delta_seconds_positive_after_update() {
    let mut time = Time::new();
    // Small sleep to ensure measurable delta
    std::thread::sleep(std::time::Duration::from_millis(1));
    time.update(0.016);
    assert!(time.delta_seconds() > 0.0);
}

#[test]
fn test_time_fps_positive_after_update() {
    let mut time = Time::new();
    std::thread::sleep(std::time::Duration::from_millis(1));
    time.update(0.016);
    assert!(time.delta_seconds() > 0.0);
    assert!(time.delta_seconds() < 1_000_000.0); // sanity upper bound
}

#[test]
fn test_time_elapsed_grows_monotonically() {
    let mut time = Time::new();
    let mut prev_elapsed = time.elapsed_seconds();
    for _ in 0..5 {
        std::thread::sleep(std::time::Duration::from_millis(1));
        time.update(0.016);
        let current = time.elapsed_seconds();
        assert!(current >= prev_elapsed);
        prev_elapsed = current;
    }
}

#[test]
fn test_multiple_post_render_hooks() {
    let counter = Arc::new(AtomicUsize::new(0));
    let c1 = counter.clone();
    let c2 = counter.clone();

    let mut builder = AppBuilder::new();
    builder.add_post_render_hook(Box::new(move |_app| {
        c1.fetch_add(1, Ordering::Relaxed);
    }));
    builder.add_post_render_hook(Box::new(move |_app| {
        c2.fetch_add(1, Ordering::Relaxed);
    }));

    let app = builder.build();
    // post_render_hooks are only invoked in the event loop, not in app.run()
    // but we can verify they were registered
    assert_eq!(app.post_render_hooks.len(), 2);
}

#[test]
fn test_resource_registry_basic() {
    let mut builder = AppBuilder::new();
    builder.resources_mut().insert(100u32);
    let app = builder.build();
    assert_eq!(*app.resources.get::<u32>().unwrap(), 100);
}
