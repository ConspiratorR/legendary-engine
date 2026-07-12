use engine_core::Phase;
use engine_core::app::AppBuilder;
use engine_core::context::Context;
use engine_core::event::EventBus;
use engine_core::gameobject::{Component, GameObject, GameObjectHandle};
use engine_core::transform::Transform;
use engine_core::world::World;
use std::any::Any;
use std::sync::atomic::{AtomicUsize, Ordering};

// NOTE: Module-level AtomicUsize statics work here because each test manually
// resets at the top, but they're fragile if tests ever run in parallel.
// A scoped approach would be more idiomatic.
static UPDATE_CALLED: AtomicUsize = AtomicUsize::new(0);
static LATE_CALLED: AtomicUsize = AtomicUsize::new(0);
static FIXED_CALLED: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
struct Health {
    current: f32,
    _max: f32,
}

impl Component for Health {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Debug)]
struct Ammo {
    _count: u32,
}

impl Component for Ammo {
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
        _max: 100.0,
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

    // Extend: multi-level hierarchy (grandparent -> parent -> child)
    let grandchild = world.spawn(GameObject::new("Grandchild"));
    world.set_parent(grandchild, Some(child));
    assert_eq!(world.get_parent(grandchild), Some(child));
    assert!(world.get_children(child).contains(&grandchild));
    // Grandchild is NOT a direct child of parent
    assert!(!world.get_children(parent).contains(&grandchild));

    // Extend: cycle prevention — while chain is intact (parent -> child -> grandchild),
    // setting parent's parent to grandchild would create a cycle and should be rejected
    world.set_parent(parent, Some(grandchild));
    assert_eq!(world.get_parent(parent), None); // parent stays root

    // Extend: detach child from parent
    world.set_parent(child, None);
    assert_eq!(world.get_parent(child), None);
    assert!(!world.get_children(parent).contains(&child));
    // Grandchild should still be a child of child
    assert!(world.get_children(child).contains(&grandchild));
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
    let mut events = EventBus::new();
    for frame in 0..3 {
        if app.is_running() {
            let mut ctx = Context::new(&mut world, time.clone(), frame, &mut events);
            app.player_loop_mut().run(&mut ctx);
        }
    }

    assert_eq!(UPDATE_CALLED.load(Ordering::SeqCst), 3);
    assert_eq!(LATE_CALLED.load(Ordering::SeqCst), 3);
    assert_eq!(FIXED_CALLED.load(Ordering::SeqCst), 3);
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
        _max: 100.0,
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

    // Extend: include_inactive = false — inactive objects should be excluded
    world.get_gameobject_mut(h2).unwrap().set_active(false);
    let active_players = world.find_gameobjects_with_tag("Player", false);
    assert_eq!(active_players.len(), 1);
    assert!(active_players.contains(&h1));
    assert!(!active_players.contains(&h2));

    // All players (including inactive)
    let all_players = world.find_gameobjects_with_tag("Player", true);
    assert_eq!(all_players.len(), 2);
}

#[test]
fn test_find_gameobject_missing_name_returns_none() {
    let mut world = World::new();

    let mut go = GameObject::new("Foo");
    go.set_tag("Bar");
    world.spawn(go);

    assert_eq!(world.find_gameobject("Nonexistent"), None);
    assert_eq!(
        world.find_gameobjects_with_tag("Nonexistent", true).len(),
        0
    );
}

#[test]
fn test_get_component_wrong_type_returns_none() {
    let mut world = World::new();

    let mut go = GameObject::new("Entity");
    go.add_component(Health {
        current: 50.0,
        _max: 100.0,
    });
    let handle = world.spawn(go);

    let go = world.get_gameobject(handle).unwrap();
    assert!(go.get_component::<Ammo>().is_none());
    assert!(go.get_component::<Transform>().is_none());
}

#[test]
fn test_get_gameobject_invalid_handle_returns_none() {
    let mut world = World::new();

    let go = GameObject::new("Temporary");
    let handle = world.spawn(go);
    world.despawn(handle);

    // After despawn, the handle should be invalid
    assert!(world.get_gameobject(handle).is_none());
    assert!(world.get_gameobject_mut(handle).is_none());
}

#[test]
fn test_set_parent_invalid_child_is_noop() {
    let mut world = World::new();

    let parent = world.spawn(GameObject::new("Parent"));
    let invalid_child = GameObjectHandle::new(999, 0);

    world.set_parent(invalid_child, Some(parent));
    // Invalid handle — should be a no-op
    assert!(world.get_children(parent).is_empty());
}
