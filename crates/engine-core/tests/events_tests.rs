use engine_core::event::{EventBus, EventBusExt};
use engine_core::events::*;
use engine_core::gameobject::{Component, GameObject};
use engine_core::world::World;
use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

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
fn test_builtin_event_types() {
    let counter = Arc::new(AtomicUsize::new(0));

    let mut bus = EventBus::new();

    let c = counter.clone();
    bus.on_event::<CollisionEnter>(move |_, _| {
        c.fetch_add(1, Ordering::SeqCst);
    });
    let c = counter.clone();
    bus.on_event::<TriggerEnter>(move |_, _| {
        c.fetch_add(10, Ordering::SeqCst);
    });
    let c = counter.clone();
    bus.on_event::<MouseDown>(move |_, _| {
        c.fetch_add(100, Ordering::SeqCst);
    });

    let handle = engine_core::gameobject::GameObjectHandle::new(0, 0);

    let mut world = World::new();
    let mut events = EventBus::new();
    let time = engine_core::Time::default();
    let mut context = engine_core::context::Context::new(&mut world, time, 0, &mut events);

    bus.send(
        CollisionEnter {
            entity: handle,
            collision: Collision {
                other: handle,
                normal: engine_math::Vec3::Y,
                point: engine_math::Vec3::ZERO,
                relative_velocity: engine_math::Vec3::ZERO,
            },
        },
        &mut context,
    );
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    bus.send(
        TriggerEnter {
            entity: handle,
            trigger: TriggerData { other: handle },
        },
        &mut context,
    );
    assert_eq!(counter.load(Ordering::SeqCst), 11);

    bus.send(
        MouseDown {
            entity: handle,
            button: MouseButton::Left,
        },
        &mut context,
    );
    assert_eq!(counter.load(Ordering::SeqCst), 111);
}

#[test]
fn test_health_changed_event() {
    let counter = Arc::new(AtomicUsize::new(0));

    let mut bus = EventBus::new();

    let c = counter.clone();
    bus.on_event::<HealthChanged>(move |e, _| {
        if e.new_health <= 0.0 {
            c.fetch_add(1, Ordering::SeqCst);
        }
    });
    let c = counter.clone();
    bus.on_event::<EntityDied>(move |_, _| {
        c.fetch_add(10, Ordering::SeqCst);
    });

    let handle = engine_core::gameobject::GameObjectHandle::new(0, 0);

    let mut world = World::new();
    let mut events = EventBus::new();
    let time = engine_core::Time::default();
    let mut context = engine_core::context::Context::new(&mut world, time, 0, &mut events);

    bus.send(
        HealthChanged {
            entity: handle,
            old_health: 100.0,
            new_health: 50.0,
        },
        &mut context,
    );
    assert_eq!(counter.load(Ordering::SeqCst), 0);

    bus.send(
        HealthChanged {
            entity: handle,
            old_health: 50.0,
            new_health: 0.0,
        },
        &mut context,
    );
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    bus.send(EntityDied { entity: handle }, &mut context);
    assert_eq!(counter.load(Ordering::SeqCst), 11);
}

#[test]
fn test_event_with_gameobject() {
    let mut world = World::new();

    let mut player = GameObject::new("Player");
    player.add_component(Health {
        current: 100.0,
        max: 100.0,
    });

    let handle = world.spawn(player);

    let gameobject = world.get_gameobject(handle).unwrap();
    assert!(gameobject.has_component::<Health>());

    let health = gameobject.get_component::<Health>().unwrap();
    assert_eq!(health.current, 100.0);
    assert_eq!(health.max, 100.0);
}

#[test]
fn test_multiple_event_handlers() {
    let counter = Arc::new(AtomicUsize::new(0));

    let mut bus = EventBus::new();

    let c = counter.clone();
    bus.on_event::<CollisionEnter>(move |_, _| {
        c.fetch_add(1, Ordering::SeqCst);
    });
    let c = counter.clone();
    bus.on_event::<CollisionEnter>(move |_, _| {
        c.fetch_add(10, Ordering::SeqCst);
    });

    let handle = engine_core::gameobject::GameObjectHandle::new(0, 0);

    let mut world = World::new();
    let mut events = EventBus::new();
    let time = engine_core::Time::default();
    let mut context = engine_core::context::Context::new(&mut world, time, 0, &mut events);

    bus.send(
        CollisionEnter {
            entity: handle,
            collision: Collision {
                other: handle,
                normal: engine_math::Vec3::Y,
                point: engine_math::Vec3::ZERO,
                relative_velocity: engine_math::Vec3::ZERO,
            },
        },
        &mut context,
    );

    assert_eq!(counter.load(Ordering::SeqCst), 11);
}
