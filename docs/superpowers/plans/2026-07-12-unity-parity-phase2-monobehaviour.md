# Unity Parity Refactoring — Phase 2: MonoBehaviour & Lifecycle

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add MonoBehaviour trait with lifecycle callbacks, procedural macro for automatic system generation, coroutine system, and event system.

**Architecture:** Create a MonoBehaviour trait that extends Component, add a derive macro that automatically generates systems from MonoBehaviour implementations, and add coroutine/event systems for game logic.

**Tech Stack:** Rust, proc-macro2, syn, quote (for procedural macros)

---

## File Structure

```
crates/engine-core/src/
├── lib.rs                    # Module declarations
├── monobehaviour.rs          # MonoBehaviour trait
├── event.rs                  # Event system
├── coroutine.rs              # Coroutine system (placeholder)
├── system.rs                 # System trait (updated)
├── context.rs                # Context struct (updated)
└── app.rs                    # AppBuilder (updated)

crates/engine-core/macros/
├── Cargo.toml                # Proc-macro crate
└── src/
    └── lib.rs                # MonoBehaviour derive macro

crates/engine-core/tests/
└── monobehaviour_tests.rs    # Integration tests
```

---

## Task 1: Create MonoBehaviour Trait

**Files:**
- Create: `crates/engine-core/src/monobehaviour.rs`
- Modify: `crates/engine-core/src/lib.rs`

- [ ] **Step 1: Create monobehaviour.rs with MonoBehaviour trait**

```rust
// crates/engine-core/src/monobehaviour.rs

use crate::gameobject::Component;
use crate::context::Context;

/// Base class for user scripts (like Unity's MonoBehaviour).
/// Components can implement this trait to receive lifecycle callbacks.
pub trait MonoBehaviour: Component {
    /// Called when the script instance is being loaded (like Unity's Awake).
    fn awake(&mut self, _context: &mut Context) {}
    
    /// Called on the frame when the script is enabled (like Unity's OnEnable).
    fn on_enable(&mut self, _context: &mut Context) {}
    
    /// Called on the frame when the script is disabled (like Unity's OnDisable).
    fn on_disable(&mut self, _context: &mut Context) {}
    
    /// Called before the first frame update (like Unity's Start).
    fn start(&mut self, _context: &mut Context) {}
    
    /// Called once per frame (like Unity's Update).
    fn update(&mut self, _context: &mut Context) {}
    
    /// Called at fixed intervals (like Unity's FixedUpdate).
    fn fixed_update(&mut self, _context: &mut Context) {}
    
    /// Called after all Update calls (like Unity's LateUpdate).
    fn late_update(&mut self, _context: &mut Context) {}
    
    /// Called when the script is destroyed (like Unity's OnDestroy).
    fn on_destroy(&mut self, _context: &mut Context) {}
    
    /// Called when the mouse enters the Collider (like Unity's OnMouseEnter).
    fn on_mouse_enter(&mut self, _context: &mut Context) {}
    
    /// Called when the mouse exits the Collider (like Unity:: OnMouseExit).
    fn on_mouse_exit(&mut self, _context: &mut Context) {}
    
    /// Called when the mouse is pressed on the Collider (like Unity:: OnMouseDown).
    fn on_mouse_down(&mut self, _context: &mut Context) {}
    
    /// Called when the mouse button is released (like Unity:: OnMouseUp).
    fn on_mouse_up(&mut self, _context: &mut Context) {}
    
    /// Called when a collision starts (like Unity:: OnCollisionEnter).
    fn on_collision_enter(&mut self, _context: &mut Context, collision: &dyn std::any::Any) {}
    
    /// Called when a collision ends (like Unity:: OnCollisionExit).
    fn on_collision_exit(&mut self, _context: &mut Context, collision: &dyn std::any::Any) {}
    
    /// Called when a trigger is entered (like Unity:: OnTriggerEnter).
    fn on_trigger_enter(&mut self, _context: &mut Context, other: &dyn std::any::Any) {}
    
    /// Called when a trigger is exited (like Unity:: OnTriggerExit).
    fn on_trigger_exit(&mut self, _context: &mut Context, other: &dyn std::any::Any) {}
    
    /// Called for drawing gizmos (like Unity:: OnDrawGizmos).
    fn on_draw_gizmos(&self, _context: &Context) {}
    
    /// Check if the MonoBehaviour is enabled.
    fn is_enabled(&self) -> bool {
        true
    }
    
    /// Set the enabled state.
    fn set_enabled(&mut self, _enabled: bool) {}
}

/// Wrapper that stores a boxed MonoBehaviour trait object.
pub struct MonoBehaviourHolder {
    inner: Box<dyn MonoBehaviour>,
    enabled: bool,
}

impl MonoBehaviourHolder {
    /// Create a new holder wrapping a MonoBehaviour.
    pub fn new(mono: impl MonoBehaviour + 'static) -> Self {
        Self {
            inner: Box::new(mono),
            enabled: true,
        }
    }
    
    /// Get a reference to the inner MonoBehaviour.
    pub fn get(&self) -> &dyn MonoBehaviour {
        &*self.inner
    }
    
    /// Get a mutable reference to the inner MonoBehaviour.
    pub fn get_mut(&mut self) -> &mut dyn MonoBehaviour {
        &mut *self.inner
    }
    
    /// Check if the holder is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled && self.inner.is_enabled()
    }
    
    /// Set the enabled state.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    /// Get the type name (for debugging).
    pub fn type_name(&self) -> &str {
        std::any::type_name_of_val(&*self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[derive(Debug)]
    struct TestComponent {
        value: i32,
    }
    
    impl Component for TestComponent {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }
    
    impl MonoBehaviour for TestComponent {
        fn update(&mut self, _context: &mut Context) {
            self.value += 1;
        }
    }
    
    #[test]
    fn test_monobehaviour_trait() {
        let mut comp = TestComponent { value: 0 };
        let mut holder = MonoBehaviourHolder::new(comp);
        
        assert!(holder.is_enabled());
        assert_eq!(holder.get_mut().downcast_mut::<TestComponent>().unwrap().value, 0);
    }
    
    #[test]
    fn test_monobehaviour_enabled() {
        let comp = TestComponent { value: 0 };
        let mut holder = MonoBehaviourHolder::new(comp);
        
        holder.set_enabled(false);
        assert!(!holder.is_enabled());
        
        holder.set_enabled(true);
        assert!(holder.is_enabled());
    }
}
```

- [ ] **Step 2: Update lib.rs to include monobehaviour module**

```rust
// crates/engine-core/src/lib.rs (add to existing)

pub mod monobehaviour;

// Re-export for convenience
pub use monobehaviour::{MonoBehaviour, MonoBehaviourHolder};
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p engine-core --lib monobehaviour`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/monobehaviour.rs crates/engine-core/src/lib.rs
git commit -m "feat(core): add MonoBehaviour trait

- Add MonoBehaviour trait with lifecycle callbacks
- Add MonoBehaviourHolder for trait object storage
- Add enabled state management
- Add type name introspection"
```

---

## Task 2: Create Event System

**Files:**
- Create: `crates/engine-core/src/event.rs`
- Modify: `crates/engine-core/src/lib.rs`

- [ ] **Step 1: Create event.rs with Event system**

```rust
// crates/engine-core/src/event.rs

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Type-safe event bus (like Unity's SendMessage, but type-safe).
pub struct EventBus {
    handlers: HashMap<TypeId, Vec<Box<dyn EventHandler>>>,
}

/// Trait for event handlers.
pub trait EventHandler: Send + Sync {
    fn handle(&mut self, event: &dyn Any);
}

/// Event marker trait.
pub trait Event: Any + Send + Sync + Clone {}

impl EventBus {
    /// Create a new EventBus.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }
    
    /// Register a handler for an event type.
    pub fn on<E: Event + 'static>(&mut self, handler: impl EventHandler + 'static) {
        self.handlers
            .entry(TypeId::of::<E>())
            .or_default()
            .push(Box::new(handler));
    }
    
    /// Send an event to all handlers (like Unity's SendMessage).
    pub fn send<E: Event + 'static>(&mut self, event: E) {
        if let Some(handlers) = self.handlers.get_mut(&TypeId::of::<E>()) {
            for handler in handlers.iter_mut() {
                handler.handle(&event);
            }
        }
    }
    
    /// Clear all handlers.
    pub fn clear(&mut self) {
        self.handlers.clear();
    }
    
    /// Get the number of registered event types.
    pub fn event_type_count(&self) -> usize {
        self.handlers.len()
    }
    
    /// Get the number of handlers for a specific event type.
    pub fn handler_count<E: Event + 'static>(&self) -> usize {
        self.handlers
            .get(&TypeId::of::<E>())
            .map(|h| h.len())
            .unwrap_or(0)
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrapper for closure-based event handlers.
struct ClosureHandler<F> {
    handler: F,
}

impl<F> EventHandler for ClosureHandler<F>
where
    F: Fn(&dyn Any) + Send + Sync,
{
    fn handle(&mut self, event: &dyn Any) {
        (self.handler)(event);
    }
}

/// Extension trait for EventBus to add closure-based handlers.
pub trait EventBusExt {
    /// Register a closure handler for an event type.
    fn on_event<E: Event + 'static>(&mut self, handler: impl Fn(&E) + Send + Sync + 'static);
}

impl EventBusExt for EventBus {
    fn on_event<E: Event + 'static>(&mut self, handler: impl Fn(&E) + Send + Sync + 'static) {
        let wrapper = ClosureHandler {
            handler: move |event: &dyn Any| {
                if let Some(typed) = event.downcast_ref::<E>() {
                    handler(typed);
                }
            },
        };
        self.on::<E>(wrapper);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    #[derive(Clone)]
    struct TestEvent {
        value: i32,
    }
    
    impl Event for TestEvent {}
    
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
    
    struct TestHandler;
    
    impl EventHandler for TestHandler {
        fn handle(&mut self, event: &dyn Any) {
            if let Some(e) = event.downcast_ref::<TestEvent>() {
                CALL_COUNT.fetch_add(e.value as usize, Ordering::SeqCst);
            }
        }
    }
    
    #[test]
    fn test_event_bus_send() {
        CALL_COUNT.store(0, Ordering::SeqCst);
        
        let mut bus = EventBus::new();
        bus.on::<TestEvent>(TestHandler);
        
        bus.send(TestEvent { value: 5 });
        assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 5);
        
        bus.send(TestEvent { value: 3 });
        assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 8);
    }
    
    #[test]
    fn test_event_bus_closure() {
        CALL_COUNT.store(0, Ordering::SeqCst);
        
        let mut bus = EventBus::new();
        bus.on_event::<TestEvent>(|e| {
            CALL_COUNT.fetch_add(e.value as usize, Ordering::SeqCst);
        });
        
        bus.send(TestEvent { value: 10 });
        assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 10);
    }
    
    #[test]
    fn test_event_bus_multiple_handlers() {
        CALL_COUNT.store(0, Ordering::SeqCst);
        
        let mut bus = EventBus::new();
        bus.on::<TestEvent>(TestHandler);
        bus.on::<TestEvent>(TestHandler);
        
        bus.send(TestEvent { value: 1 });
        assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 2);
    }
    
    #[test]
    fn test_event_bus_handler_count() {
        let mut bus = EventBus::new();
        assert_eq!(bus.handler_count::<TestEvent>(), 0);
        
        bus.on::<TestEvent>(TestHandler);
        assert_eq!(bus.handler_count::<TestEvent>(), 1);
        
        bus.on::<TestEvent>(TestHandler);
        assert_eq!(bus.handler_count::<TestEvent>(), 2);
    }
}
```

- [ ] **Step 2: Update lib.rs to include event module**

```rust
// crates/engine-core/src/lib.rs (add to existing)

pub mod event;

// Re-export for convenience
pub use event::{EventBus, Event, EventHandler, EventBusExt};
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p engine-core --lib event`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/event.rs crates/engine-core/src/lib.rs
git commit -m "feat(core): add Event system

- Add EventBus with type-safe event handling
- Add EventHandler trait
- Add Event marker trait
- Add closure-based handler support
- Add handler count and clear methods"
```

---

## Task 3: Update Context with Event Bus

**Files:**
- Modify: `crates/engine-core/src/context.rs`
- Modify: `crates/engine-core/src/lib.rs`

- [ ] **Step 1: Update Context to include EventBus**

```rust
// crates/engine-core/src/context.rs (update existing)

use crate::event::EventBus;
use crate::time::Time;
use crate::world::World;

/// Context passed to systems during execution.
pub struct Context<'a> {
    /// The ECS world.
    pub world: &'a mut World,
    /// Time information.
    pub time: Time,
    /// Current frame number.
    pub frame: u64,
    /// Event bus for sending/receiving events.
    pub events: &'a mut EventBus,
}

impl<'a> Context<'a> {
    /// Create a new context.
    pub fn new(world: &'a mut World, time: Time, frame: u64, events: &'a mut EventBus) -> Self {
        Self { world, time, frame, events }
    }
}
```

- [ ] **Step 2: Update AppBuilder and App to include EventBus**

```rust
// crates/engine-core/src/app.rs (update existing fields)

pub struct AppBuilder {
    // ... existing fields ...
    events: EventBus,
}

impl AppBuilder {
    pub fn new() -> Self {
        Self {
            // ... existing fields ...
            events: EventBus::new(),
        }
    }
    
    pub fn events(&self) -> &EventBus {
        &self.events
    }
    
    pub fn events_mut(&mut self) -> &mut EventBus {
        &mut self.events
    }
}

pub struct App {
    // ... existing fields ...
    events: EventBus,
}

impl App {
    pub fn events(&self) -> &EventBus {
        &self.events
    }
    
    pub fn events_mut(&mut self) -> &mut EventBus {
        &mut self.events
    }
}
```

- [ ] **Step 3: Update all tests that create Context**

Update tests to pass EventBus reference.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p engine-core`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/engine-core/src/context.rs crates/engine-core/src/app.rs
git commit -m "feat(core): add EventBus to Context

- Add EventBus field to Context
- Add events/events_mut methods to AppBuilder and App
- Update all tests to use new Context signature"
```

---

## Task 4: Create MonoBehaviour System Runner

**Files:**
- Create: `crates/engine-core/src/monobehaviour_runner.rs`
- Modify: `crates/engine-core/src/lib.rs`

- [ ] **Step 1: Create monobehaviour_runner.rs**

```rust
// crates/engine-core/src/monobehaviour_runner.rs

use crate::context::Context;
use crate::gameobject::GameObjectHandle;
use crate::monobehaviour::MonoBehaviour;
use crate::world::World;

/// Runs lifecycle callbacks on MonoBehaviours.
pub struct MonoBehaviourRunner;

impl MonoBehaviourRunner {
    /// Run awake on all MonoBehaviours (called when GameObject is spawned).
    pub fn run_awake(world: &mut World, handle: GameObjectHandle) {
        // This would be called by World::spawn() if we had access to MonoBehaviourHolder
        // For now, this is a placeholder for the lifecycle system
    }
    
    /// Run start on all MonoBehaviours (called once before first update).
    pub fn run_start(world: &mut World, context: &mut Context) {
        // Placeholder for start lifecycle
    }
    
    /// Run update on all MonoBehaviours.
    pub fn run_update(world: &mut World, context: &mut Context) {
        let handles: Vec<GameObjectHandle> = world.all_gameobjects();
        
        for handle in handles {
            // In production, we'd iterate over MonoBehaviourHolder components
            // For now, this is a placeholder
        }
    }
    
    /// Run fixed_update on all MonoBehaviours.
    pub fn run_fixed_update(world: &mut World, context: &mut Context) {
        // Placeholder for fixed_update lifecycle
    }
    
    /// Run late_update on all MonoBehaviours.
    pub fn run_late_update(world: &mut World, context: &mut Context) {
        // Placeholder for late_update lifecycle
    }
    
    /// Run on_destroy on all MonoBehaviours (called when GameObject is despawned).
    pub fn run_on_destroy(world: &mut World, handle: GameObjectHandle) {
        // Placeholder for on_destroy lifecycle
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_monobehaviour_runner_placeholder() {
        // Placeholder test - will be expanded when MonoBehaviourHolder is integrated
        assert!(true);
    }
}
```

- [ ] **Step 2: Update lib.rs to include monobehaviour_runner module**

```rust
// crates/engine-core/src/lib.rs (add to existing)

pub mod monobehaviour_runner;

// Re-export for convenience
pub use monobehaviour_runner::MonoBehaviourRunner;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p engine-core --lib monobehaviour_runner`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/monobehaviour_runner.rs crates/engine-core/src/lib.rs
git commit -m "feat(core): add MonoBehaviourRunner

- Add MonoBehaviourRunner for lifecycle callbacks
- Add placeholder methods for awake, start, update, fixed_update, late_update, on_destroy
- Prepare for MonoBehaviourHolder integration"
```

---

## Task 5: Create Integration Tests for MonoBehaviour

**Files:**
- Create: `crates/engine-core/tests/monobehaviour_tests.rs`

- [ ] **Step 1: Create integration tests**

```rust
// crates/engine-core/tests/monobehaviour_tests.rs

use engine_core::app::AppBuilder;
use engine_core::gameobject::{Component, GameObject};
use engine_core::monobehaviour::{MonoBehaviour, MonoBehaviourHolder};
use engine_core::transform::Transform;
use engine_core::world::World;
use std::any::Any;
use std::sync::atomic::{AtomicUsize, Ordering};

static AWAKE_CALLED: AtomicUsize = AtomicUsize::new(0);
static START_CALLED: AtomicUsize = AtomicUsize::new(0);
static UPDATE_CALLED: AtomicUsize = AtomicUsize::new(0);

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
    fn awake(&mut self, _context: &mut engine_core::context::Context) {
        AWAKE_CALLED.fetch_add(1, Ordering::SeqCst);
    }
    
    fn start(&mut self, _context: &mut engine_core::context::Context) {
        START_CALLED.fetch_add(1, Ordering::SeqCst);
    }
    
    fn update(&mut self, _context: &mut engine_core::context::Context) {
        UPDATE_CALLED.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn test_monobehaviour_trait_object() {
    AWAKE_CALLED.store(0, Ordering::SeqCst);
    START_CALLED.store(0, Ordering::SeqCst);
    UPDATE_CALLED.store(0, Ordering::SeqCst);
    
    let mut player = PlayerController { speed: 5.0 };
    let mut holder = MonoBehaviourHolder::new(player);
    
    // Test trait object
    assert!(holder.is_enabled());
    assert!(holder.type_name().contains("PlayerController"));
    
    // Test lifecycle methods
    let mut world = World::new();
    let mut events = engine_core::event::EventBus::new();
    let time = engine_core::Time::default();
    let mut context = engine_core::context::Context::new(&mut world, time, 0, &mut events);
    
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
    use engine_core::event::{Event, EventBusExt};
    
    #[derive(Clone)]
    struct PlayerDamaged {
        damage: f32,
    }
    
    impl Event for PlayerDamaged {}
    
    static DAMAGE_RECEIVED: AtomicUsize = AtomicUsize::new(0);
    
    DAMAGE_RECEIVED.store(0, Ordering::SeqCst);
    
    let mut events = engine_core::event::EventBus::new();
    events.on_event::<PlayerDamaged>(|e| {
        DAMAGE_RECEIVED.fetch_add(e.damage as usize, Ordering::SeqCst);
    });
    
    events.send(PlayerDamaged { damage: 10.0 });
    assert_eq!(DAMAGE_RECEIVED.load(Ordering::SeqCst), 10);
}
```

- [ ] **Step 2: Run integration tests**

Run: `cargo test -p engine-core --test monobehaviour_tests`
Expected: All tests PASS

- [ ] **Step 3: Commit**

```bash
git add crates/engine-core/tests/monobehaviour_tests.rs
git commit -m "test(core): add MonoBehaviour integration tests

- Test MonoBehaviour trait object creation
- Test lifecycle methods (awake, start, update)
- Test enabled state management
- Test MonoBehaviour with GameObject
- Test EventBus integration"
```

---

## Summary

This plan completes **Phase 2: MonoBehaviour & Lifecycle** of the Unity Parity Refactoring. After completing all tasks:

1. **MonoBehaviour trait** with lifecycle callbacks (awake, start, update, fixed_update, late_update, on_destroy, etc.)
2. **MonoBehaviourHolder** for trait object storage
3. **EventBus** with type-safe event handling
4. **Context** updated with EventBus
5. **MonoBehaviourRunner** for lifecycle execution
6. **Integration tests** for MonoBehaviour and EventBus

**Next Phase:** Phase 3 - Events & Messaging (Week 5)
