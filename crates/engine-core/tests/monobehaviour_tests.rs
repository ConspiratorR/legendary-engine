use engine_core::Time;
use engine_core::context::Context;
use engine_core::event::{Event, EventBus, EventBusExt};
use engine_core::gameobject::{Component, GameObject};
use engine_core::monobehaviour::{MonoBehaviour, MonoBehaviourHolder};
use engine_core::transform::Transform;
use engine_core::world::World;
use std::any::Any;
use std::sync::atomic::{AtomicUsize, Ordering};

static AWAKE_CALLED: AtomicUsize = AtomicUsize::new(0);
static START_CALLED: AtomicUsize = AtomicUsize::new(0);
static UPDATE_CALLED: AtomicUsize = AtomicUsize::new(0);
static DAMAGE_RECEIVED: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
struct PlayerController {
    speed: f32,
}

impl Component for PlayerController {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl MonoBehaviour for PlayerController {
    fn awake(&mut self, _context: &mut Context) {
        AWAKE_CALLED.fetch_add(1, Ordering::SeqCst);
    }

    fn start(&mut self, _context: &mut Context) {
        START_CALLED.fetch_add(1, Ordering::SeqCst);
    }

    fn update(&mut self, _context: &mut Context) {
        UPDATE_CALLED.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn test_monobehaviour_trait_object() {
    AWAKE_CALLED.store(0, Ordering::SeqCst);
    START_CALLED.store(0, Ordering::SeqCst);
    UPDATE_CALLED.store(0, Ordering::SeqCst);

    let player = PlayerController { speed: 5.0 };
    let mut holder = MonoBehaviourHolder::new(player);

    // Test trait object
    assert!(holder.is_enabled());
    assert!(!holder.type_name().is_empty());

    // Test lifecycle methods
    let mut world = World::new();
    let mut events = EventBus::new();
    let time = Time::default();
    let mut context = Context::new(&mut world, time, 0, &mut events);

    holder.get_mut().awake(&mut context);
    assert_eq!(AWAKE_CALLED.load(Ordering::SeqCst), 1);

    holder.get_mut().start(&mut context);
    assert_eq!(START_CALLED.load(Ordering::SeqCst), 1);

    holder.get_mut().update(&mut context);
    assert_eq!(UPDATE_CALLED.load(Ordering::SeqCst), 1);
}

#[test]
fn test_monobehaviour_enabled() {
    let player = PlayerController { speed: 5.0 };
    let mut holder = MonoBehaviourHolder::new(player);

    assert!(holder.is_enabled());

    holder.set_enabled(false);
    assert!(!holder.is_enabled());

    holder.set_enabled(true);
    assert!(holder.is_enabled());
}

#[test]
fn test_monobehaviour_with_gameobject() {
    let mut world = World::new();

    let mut player = GameObject::new("Player");
    player.add_component(Transform::from_xyz(0.0, 1.0, 0.0));
    player.add_component(PlayerController { speed: 5.0 });

    let handle = world.spawn(player);

    // Verify components exist
    let gameobject = world.get_gameobject(handle).unwrap();
    assert!(gameobject.has_component::<Transform>());
    assert!(gameobject.has_component::<PlayerController>());

    // Get component and verify
    let controller = gameobject.get_component::<PlayerController>().unwrap();
    assert_eq!(controller.speed, 5.0);
}

#[test]
fn test_event_bus_with_monobehaviour() {
    #[derive(Clone)]
    struct PlayerDamaged {
        damage: f32,
    }

    impl Event for PlayerDamaged {}

    DAMAGE_RECEIVED.store(0, Ordering::SeqCst);

    let mut events = EventBus::new();
    events.on_event::<PlayerDamaged>(|e, _| {
        DAMAGE_RECEIVED.fetch_add(e.damage as usize, Ordering::SeqCst);
    });

    let mut world = World::new();
    let mut ctx_events = EventBus::new();
    let mut ctx = Context::new(&mut world, Time::default(), 0, &mut ctx_events);

    events.send(PlayerDamaged { damage: 10.0 }, &mut ctx);
    assert_eq!(DAMAGE_RECEIVED.load(Ordering::SeqCst), 10);
}
