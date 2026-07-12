use engine_core::Phase;
use engine_core::app::AppBuilder;
use engine_core::context::Context;
use engine_core::gameobject::{Component, GameObject};
use engine_core::transform::Transform;
use engine_core::world::World;
use std::any::Any;
use std::sync::atomic::{AtomicUsize, Ordering};

static UPDATE_CALLED: AtomicUsize = AtomicUsize::new(0);
static LATE_CALLED: AtomicUsize = AtomicUsize::new(0);
static FIXED_CALLED: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
struct Health {
    current: f32,
    max: f32,
}

impl Component for Health {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[test]
fn test_unity_like_gameobject_creation() {
    let mut world = World::new();

    // Create player like Unity
    let mut player = GameObject::new("Player");
    player.add_component(Transform::from_xyz(0.0, 1.0, 0.0));
    player.add_component(Health {
        current: 100.0,
        max: 100.0,
    });
    player.set_tag("Player");
    player.set_layer(6);

    let handle = world.spawn(player);

    // Verify
    let player = world.get_gameobject(handle).unwrap();
    assert_eq!(player.name(), "Player");
    assert_eq!(player.tag(), "Player");
    assert_eq!(player.layer(), 6);
    assert!(player.has_component::<Transform>());
    assert!(player.has_component::<Health>());

    let health = player.get_component::<Health>().unwrap();
    assert_eq!(health.current, 100.0);
}

#[test]
fn test_unity_like_hierarchy() {
    let mut world = World::new();

    // Create parent-child like Unity
    let parent = world.spawn(GameObject::new("Parent"));
    let child = world.spawn(GameObject::new("Child"));

    world.set_parent(child, Some(parent));

    // Verify
    assert_eq!(world.get_parent(child), Some(parent));
    assert!(world.get_children(parent).contains(&child));
}

#[test]
fn test_unity_like_player_loop() {
    UPDATE_CALLED.store(0, Ordering::SeqCst);
    LATE_CALLED.store(0, Ordering::SeqCst);
    FIXED_CALLED.store(0, Ordering::SeqCst);

    let mut builder = AppBuilder::new();

    builder.add_system_to_phase(Phase::Update, |_: &mut engine_core::context::Context| {
        UPDATE_CALLED.fetch_add(1, Ordering::SeqCst);
    });

    builder.add_late_update_system(|_: &mut engine_core::context::Context| {
        LATE_CALLED.fetch_add(1, Ordering::SeqCst);
    });

    builder.add_fixed_update_system(|_: &mut engine_core::context::Context| {
        FIXED_CALLED.fetch_add(1, Ordering::SeqCst);
    });

    let mut app = builder.build();
    app.set_running(true);

    // Run frames by invoking the player loop directly
    let mut world = World::new();
    let time = engine_core::time::Time::new();
    for frame in 0..3 {
        if app.is_running() {
            let mut ctx = Context::new(&mut world, time.clone(), frame);
            app.player_loop_mut().run(&mut ctx);
        }
    }

    assert_eq!(UPDATE_CALLED.load(Ordering::SeqCst), 3);
    assert_eq!(LATE_CALLED.load(Ordering::SeqCst), 3);
}

#[test]
fn test_unity_like_time_management() {
    let builder = AppBuilder::new();
    let mut app = builder.build();

    // Initial time
    assert_eq!(app.time_ref().time(), 0.0);
    assert_eq!(app.time_ref().frame_count(), 0);

    // Update
    app.update(0.016);
    assert!((app.time_ref().time() - 0.016).abs() < 0.001);
    assert_eq!(app.time_ref().frame_count(), 1);

    // Update again
    app.update(0.016);
    assert!((app.time_ref().time() - 0.032).abs() < 0.001);
    assert_eq!(app.time_ref().frame_count(), 2);
}

#[test]
fn test_unity_like_component_lifecycle() {
    let mut world = World::new();

    let mut go = GameObject::new("TestObject");
    go.add_component(Health {
        current: 100.0,
        max: 100.0,
    });

    let handle = world.spawn(go);

    // Get component and modify
    {
        let go = world.get_gameobject_mut(handle).unwrap();
        let health = go.get_component_mut::<Health>().unwrap();
        health.current -= 25.0;
    }

    // Verify modification
    let go = world.get_gameobject(handle).unwrap();
    let health = go.get_component::<Health>().unwrap();
    assert_eq!(health.current, 75.0);
}

#[test]
fn test_unity_like_find_gameobjects() {
    let mut world = World::new();

    let mut player1 = GameObject::new("Player");
    player1.set_tag("Player");
    let h1 = world.spawn(player1);

    let mut player2 = GameObject::new("Player");
    player2.set_tag("Player");
    let h2 = world.spawn(player2);

    let mut enemy = GameObject::new("Enemy");
    enemy.set_tag("Enemy");
    let h3 = world.spawn(enemy);

    // Find by name
    let found = world.find_gameobject("Player");
    assert!(found.is_some());

    // Find by tag
    let players = world.find_gameobjects_with_tag("Player", true);
    assert_eq!(players.len(), 2);
    assert!(players.contains(&h1));
    assert!(players.contains(&h2));

    let enemies = world.find_gameobjects_with_tag("Enemy", true);
    assert_eq!(enemies.len(), 1);
    assert!(enemies.contains(&h3));
}
