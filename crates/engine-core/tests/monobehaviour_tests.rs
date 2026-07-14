use engine_core::Component;
use engine_core::Time;
use engine_core::behaviour::Behaviour;
use engine_core::context::Context;
use engine_core::event::{Event, EventBus, EventBusExt};
use engine_core::gameobject::GameObjectHandle;
use engine_core::monobehaviour::{MonoBehaviour, MonoBehaviourHolder};
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

impl Behaviour for PlayerController {
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

impl MonoBehaviour for PlayerController {
    fn Awake(&mut self, _context: &mut Context) {
        AWAKE_CALLED.fetch_add(1, Ordering::SeqCst);
    }

    fn Start(&mut self, _context: &mut Context) {
        START_CALLED.fetch_add(1, Ordering::SeqCst);
    }

    fn Update(&mut self, _context: &mut Context) {
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
    assert!(holder.Enabled());
    assert!(!holder.TypeName().is_empty());

    // Test lifecycle methods
    let mut world = World::new();
    let mut events = EventBus::new();
    let time = Time::default();
    let mut context = Context::new(&mut world, time, 0, &mut events);

    holder.GetMut().Awake(&mut context);
    assert_eq!(AWAKE_CALLED.load(Ordering::SeqCst), 1);

    holder.GetMut().Start(&mut context);
    assert_eq!(START_CALLED.load(Ordering::SeqCst), 1);

    holder.GetMut().Update(&mut context);
    assert_eq!(UPDATE_CALLED.load(Ordering::SeqCst), 1);
}

#[test]
fn test_monobehaviour_enabled() {
    let player = PlayerController { speed: 5.0 };
    let mut holder = MonoBehaviourHolder::new(player);

    assert!(holder.Enabled());

    holder.SetEnabled(false);
    assert!(!holder.Enabled());

    holder.SetEnabled(true);
    assert!(holder.Enabled());
}

#[test]
fn test_monobehaviour_with_gameobject() {
    let mut world = World::new();

    let handle = world.CreateGameObject("Player");
    world.AddComponent(handle, PlayerController { speed: 5.0 });

    // Verify components exist
    assert!(world.HasComponent::<PlayerController>(handle));

    // Get component and verify
    let controller = world.GetComponent::<PlayerController>(handle).unwrap();
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
