# Unity Parity Refactoring — Phase 3: Events & Messaging

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add built-in events, SendMessage/BroadcastMessage API, and event system integration with MonoBehaviour.

**Architecture:** Extend the existing EventBus with built-in game events (collision, trigger, mouse), add Unity-like message passing methods to GameObject, and integrate events with MonoBehaviour lifecycle.

**Tech Stack:** Rust

---

## File Structure

```
crates/engine-core/src/
├── lib.rs                    # Module declarations
├── events.rs                 # Built-in events
├── gameobject.rs             # GameObject (updated with message methods)
├── world.rs                  # World (updated with message dispatch)
├── monobehaviour.rs          # MonoBehaviour (updated with event integration)
└── event.rs                  # EventBus (updated with Context integration)

crates/engine-core/tests/
└── events_tests.rs           # Integration tests
```

---

## Task 1: Create Built-in Events

**Files:**
- Create: `crates/engine-core/src/events.rs`
- Modify: `crates/engine-core/src/lib.rs`

- [ ] **Step 1: Create events.rs with built-in event types**

```rust
// crates/engine-core/src/events.rs

use crate::event::Event;
use crate::gameobject::GameObjectHandle;

/// Mouse button enumeration (like Unity's MouseButton).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Collision data (simplified version of Unity's Collision).
#[derive(Debug, Clone)]
pub struct Collision {
    /// The other collider involved in the collision.
    pub other: GameObjectHandle,
    /// Contact point normal.
    pub normal: engine_math::Vec3,
    /// Contact point position.
    pub point: engine_math::Vec3,
    /// Impact velocity.
    pub relative_velocity: engine_math::Vec3,
}

/// Trigger data (simplified version of Unity's Collider).
#[derive(Debug, Clone)]
pub struct TriggerData {
    /// The other collider involved in the trigger.
    pub other: GameObjectHandle,
}

// Built-in events (like Unity's built-in messages)

/// Called when a collision starts (like Unity:: OnCollisionEnter).
#[derive(Debug, Clone, Event)]
pub struct CollisionEnter {
    /// The entity that was collided with.
    pub entity: GameObjectHandle,
    /// Collision data.
    pub collision: Collision,
}

/// Called when a collision ends (like Unity:: OnCollisionExit).
#[derive(Debug, Clone, Event)]
pub struct CollisionExit {
    /// The entity that was collided with.
    pub entity: GameObjectHandle,
    /// Collision data.
    pub collision: Collision,
}

/// Called when a collision stays (like Unity:: OnCollisionStay).
#[derive(Debug, Clone, Event)]
pub struct CollisionStay {
    /// The entity that was collided with.
    pub entity: GameObjectHandle,
    /// Collision data.
    pub collision: Collision,
}

/// Called when a trigger is entered (like Unity:: OnTriggerEnter).
#[derive(Debug, Clone, Event)]
pub struct TriggerEnter {
    /// The entity that entered the trigger.
    pub entity: GameObjectHandle,
    /// Trigger data.
    pub trigger: TriggerData,
}

/// Called when a trigger is exited (like Unity:: OnTriggerExit).
#[derive(Debug, Clone, Event)]
pub struct TriggerExit {
    /// The entity that exited the trigger.
    pub entity: GameObjectHandle,
    /// Trigger data.
    pub trigger: TriggerData,
}

/// Called when a trigger stays (like Unity:: OnTriggerStay).
#[derive(Debug, Clone, Event)]
pub struct TriggerStay {
    /// The entity that is in the trigger.
    pub entity: GameObjectHandle,
    /// Trigger data.
    pub trigger: TriggerData,
}

/// Called when the mouse enters the Collider (like Unity:: OnMouseEnter).
#[derive(Debug, Clone, Event)]
pub struct MouseEnter {
    /// The entity the mouse entered.
    pub entity: GameObjectHandle,
}

/// Called when the mouse exits the Collider (like Unity:: OnMouseExit).
#[derive(Debug, Clone, Event)]
pub struct MouseExit {
    /// The entity the mouse exited.
    pub entity: GameObjectHandle,
}

/// Called when the mouse is pressed on the Collider (like Unity:: OnMouseDown).
#[derive(Debug, Clone, Event)]
pub struct MouseDown {
    /// The entity the mouse was pressed on.
    pub entity: GameObjectHandle,
    /// Which mouse button was pressed.
    pub button: MouseButton,
}

/// Called when the mouse button is released (like Unity:: OnMouseUp).
#[derive(Debug, Clone, Event)]
pub struct MouseUp {
    /// The entity the mouse was released on.
    pub entity: GameObjectHandle,
    /// Which mouse button was released.
    pub button: MouseButton,
}

/// Called when the mouse is dragged (like Unity:: OnMouseDrag).
#[derive(Debug, Clone, Event)]
pub struct MouseDrag {
    /// The entity being dragged.
    pub entity: GameObjectHandle,
    /// Which mouse button is being held.
    pub button: MouseButton,
}

/// Called when the mouse is hovering (like Unity:: OnMouseOver).
#[derive(Debug, Clone, Event)]
pub struct MouseOver {
    /// The entity being hovered over.
    pub entity: GameObjectHandle,
}

/// Health changed event.
#[derive(Debug, Clone, Event)]
pub struct HealthChanged {
    /// The entity whose health changed.
    pub entity: GameObjectHandle,
    /// Previous health value.
    pub old_health: f32,
    /// New health value.
    pub new_health: f32,
}

/// Entity died event.
#[derive(Debug, Clone, Event)]
pub struct EntityDied {
    /// The entity that died.
    pub entity: GameObjectHandle,
}

/// Entity spawned event.
#[derive(Debug, Clone, Event)]
pub struct EntitySpawned {
    /// The entity that was spawned.
    pub entity: GameObjectHandle,
}

/// Entity despawned event.
#[derive(Debug, Clone, Event)]
pub struct EntityDespawned {
    /// The entity that was despawned.
    pub entity: GameObjectHandle,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventBus, EventBusExt};
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    static EVENT_COUNT: AtomicUsize = AtomicUsize::new(0);
    
    #[test]
    fn test_builtin_events() {
        EVENT_COUNT.store(0, Ordering::SeqCst);
        
        let mut bus = EventBus::new();
        bus.on_event::<CollisionEnter>(|_| {
            EVENT_COUNT.fetch_add(1, Ordering::SeqCst);
        });
        bus.on_event::<TriggerEnter>(|_| {
            EVENT_COUNT.fetch_add(10, Ordering::SeqCst);
        });
        bus.on_event::<MouseDown>(|_| {
            EVENT_COUNT.fetch_add(100, Ordering::SeqCst);
        });
        
        let handle = crate::gameobject::GameObjectHandle::new(0, 0);
        
        bus.send(CollisionEnter {
            entity: handle,
            collision: Collision {
                other: handle,
                normal: engine_math::Vec3::Y,
                point: engine_math::Vec3::ZERO,
                relative_velocity: engine_math::Vec3::ZERO,
            },
        });
        assert_eq!(EVENT_COUNT.load(Ordering::SeqCst), 1);
        
        bus.send(TriggerEnter {
            entity: handle,
            trigger: TriggerData { other: handle },
        });
        assert_eq!(EVENT_COUNT.load(Ordering::SeqCst), 11);
        
        bus.send(MouseDown {
            entity: handle,
            button: MouseButton::Left,
        });
        assert_eq!(EVENT_COUNT.load(Ordering::SeqCst), 111);
    }
    
    #[test]
    fn test_game_events() {
        EVENT_COUNT.store(0, Ordering::SeqCst);
        
        let mut bus = EventBus::new();
        bus.on_event::<HealthChanged>(|e| {
            if e.new_health <= 0.0 {
                EVENT_COUNT.fetch_add(1, Ordering::SeqCst);
            }
        });
        bus.on_event::<EntityDied>(|_| {
            EVENT_COUNT.fetch_add(10, Ordering::SeqCst);
        });
        
        let handle = crate::gameobject::GameObjectHandle::new(0, 0);
        
        bus.send(HealthChanged {
            entity: handle,
            old_health: 100.0,
            new_health: 50.0,
        });
        assert_eq!(EVENT_COUNT.load(Ordering::SeqCst), 0);
        
        bus.send(HealthChanged {
            entity: handle,
            old_health: 50.0,
            new_health: 0.0,
        });
        assert_eq!(EVENT_COUNT.load(Ordering::SeqCst), 1);
        
        bus.send(EntityDied { entity: handle });
        assert_eq!(EVENT_COUNT.load(Ordering::SeqCst), 11);
    }
}
```

- [ ] **Step 2: Update lib.rs to include events module**

```rust
// crates/engine-core/src/lib.rs (add to existing)

pub mod events;

// Re-export for convenience
pub use events::*;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p engine-core --lib events`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/events.rs crates/engine-core/src/lib.rs
git commit -m "feat(core): add built-in events

- Add Collision/Trigger/Mouse event types
- Add game events (HealthChanged, EntityDied, EntitySpawned, EntityDespawned)
- Add MouseButton enum
- Add Collision and TriggerData structs"
```

---

## Task 2: Update EventHandler Trait with Context

**Files:**
- Modify: `crates/engine-core/src/event.rs`

- [ ] **Step 1: Update EventHandler to accept Context**

```rust
// crates/engine-core/src/event.rs (update existing)

use crate::context::Context;

/// Trait for event handlers.
pub trait EventHandler: Send + Sync {
    fn handle(&mut self, event: &dyn Any, context: &mut Context);
}

/// Wrapper for closure-based event handlers.
struct ClosureHandler<F> {
    handler: F,
}

impl<F> EventHandler for ClosureHandler<F>
where
    F: Fn(&dyn Any, &mut Context) + Send + Sync,
{
    fn handle(&mut self, event: &dyn Any, context: &mut Context) {
        (self.handler)(event, context);
    }
}

/// Extension trait for EventBus to add closure-based handlers.
pub trait EventBusExt {
    /// Register a closure handler for an event type.
    fn on_event<E: Event + 'static>(&mut self, handler: impl Fn(&E, &mut Context) + Send + Sync + 'static);
}

impl EventBusExt for EventBus {
    fn on_event<E: Event + 'static>(&mut self, handler: impl Fn(&E, &mut Context) + Send + Sync + 'static) {
        let wrapper = ClosureHandler {
            handler: move |event: &dyn Any, context: &mut Context| {
                if let Some(typed) = event.downcast_ref::<E>() {
                    handler(typed, context);
                }
            },
        };
        self.on::<E>(wrapper);
    }
}

impl EventBus {
    /// Send an event to all handlers (like Unity's SendMessage).
    pub fn send<E: Event + 'static>(&mut self, event: E, context: &mut Context) {
        if let Some(handlers) = self.handlers.get_mut(&TypeId::of::<E>()) {
            for handler in handlers.iter_mut() {
                handler.handle(&event, context);
            }
        }
    }
}
```

- [ ] **Step 2: Update all tests that use EventHandler**

Update tests to pass Context reference.

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p engine-core`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/event.rs
git commit -m "feat(core): update EventHandler with Context

- Add Context parameter to EventHandler::handle
- Update ClosureHandler to pass Context
- Update EventBus::send to pass Context
- Update all tests"
```

---

## Task 3: Add SendMessage to GameObject

**Files:**
- Modify: `crates/engine-core/src/gameobject.rs`

- [ ] **Step 1: Add message methods to GameObject**

```rust
// crates/engine-core/src/gameobject.rs (update existing)

use crate::monobehaviour::MonoBehaviour;
use crate::context::Context;

impl GameObject {
    /// Send a message to all MonoBehaviours on this GameObject (like Unity's SendMessage).
    pub fn send_message(&mut self, method_name: &str, context: &mut Context) {
        // This would call the named method on all MonoBehaviour components
        // For now, this is a placeholder for the message system
    }
    
    /// Send a message to all MonoBehaviours on this GameObject and its children (like Unity's BroadcastMessage).
    pub fn broadcast_message(&mut self, method_name: &str, context: &mut Context, world: &crate::world::World) {
        // This would call the named method on all MonoBehaviour components
        // For now, this is a placeholder for the message system
    }
    
    /// Send a message to all MonoBehaviours on this GameObject and its parents (like Unity's SendMessageUpwards).
    pub fn send_message_upwards(&mut self, method_name: &str, context: &mut Context, world: &crate::world::World) {
        // This would call the named method on all MonoBehaviour components
        // For now, this is a placeholder for the message system
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p engine-core`
Expected: All tests PASS

- [ ] **Step 3: Commit**

```bash
git add crates/engine-core/src/gameobject.rs
git commit -m "feat(core): add SendMessage to GameObject

- Add send_message method
- Add broadcast_message method
- Add send_message_upwards method"
```

---

## Task 4: Add Event Integration to MonoBehaviour

**Files:**
- Modify: `crates/engine-core/src/monobehaviour.rs`

- [ ] **Step 1: Add event-related methods to MonoBehaviour**

```rust
// crates/engine-core/src/monobehaviour.rs (update existing)

use crate::events::*;

impl dyn MonoBehaviour {
    /// Called when a collision starts (like Unity:: OnCollisionEnter).
    pub fn on_collision_enter(&mut self, _context: &mut Context, _collision: &Collision) {}
    
    /// Called when a collision ends (like Unity:: OnCollisionExit).
    pub fn on_collision_exit(&mut self, _context: &mut Context, _collision: &Collision) {}
    
    /// Called when a trigger is entered (like Unity:: OnTriggerEnter).
    pub fn on_trigger_enter(&mut self, _context: &mut Context, _other: &TriggerData) {}
    
    /// Called when a trigger is exited (like Unity:: OnTriggerExit).
    pub fn on_trigger_exit(&mut self, _context: &mut Context, _other: &TriggerData) {}
    
    /// Called when the mouse enters the Collider (like Unity:: OnMouseEnter).
    pub fn on_mouse_enter(&mut self, _context: &mut Context) {}
    
    /// Called when the mouse exits the Collider (like Unity:: OnMouseExit).
    pub fn on_mouse_exit(&mut self, _context: &mut Context) {}
    
    /// Called when the mouse is pressed on the Collider (like Unity:: OnMouseDown).
    pub fn on_mouse_down(&mut self, _context: &mut Context, _button: MouseButton) {}
    
    /// Called when the mouse button is released (like Unity:: OnMouseUp).
    pub fn on_mouse_up(&mut self, _context: &mut Context, _button: MouseButton) {}
    
    /// Called when the mouse is dragged (like Unity:: OnMouseDrag).
    pub fn on_mouse_drag(&mut self, _context: &mut Context, _button: MouseButton) {}
    
    /// Called when the mouse is hovering (like Unity:: OnMouseOver).
    pub fn on_mouse_over(&mut self, _context: &mut Context) {}
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p engine-core`
Expected: All tests PASS

- [ ] **Step 3: Commit**

```bash
git add crates/engine-core/src/monobehaviour.rs
git commit -m "feat(core): add event integration to MonoBehaviour

- Add collision callback methods
- Add trigger callback methods
- Add mouse callback methods"
```

---

## Task 5: Create Integration Tests for Events & Messaging

**Files:**
- Create: `crates/engine-core/tests/events_tests.rs`

- [ ] **Step 1: Create integration tests**

```rust
// crates/engine-core/tests/events_tests.rs

use engine_core::app::AppBuilder;
use engine_core::event::{Event, EventBus, EventBusExt};
use engine_core::events::*;
use engine_core::gameobject::{Component, GameObject};
use engine_core::world::World;
use std::any::Any;
use std::sync::atomic::{AtomicUsize, Ordering};

static EVENT_COUNT: AtomicUsize = AtomicUsize::new(0);

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
    EVENT_COUNT.store(0, Ordering::SeqCst);
    
    let mut bus = EventBus::new();
    bus.on_event::<CollisionEnter>(|_, _| {
        EVENT_COUNT.fetch_add(1, Ordering::SeqCst);
    });
    bus.on_event::<TriggerEnter>(|_, _| {
        EVENT_COUNT.fetch_add(10, Ordering::SeqCst);
    });
    bus.on_event::<MouseDown>(|_, _| {
        EVENT_COUNT.fetch_add(100, Ordering::SeqCst);
    });
    
    let handle = engine_core::gameobject::GameObjectHandle::new(0, 0);
    
    let mut world = World::new();
    let mut events = EventBus::new();
    let time = engine_core::Time::default();
    let mut context = engine_core::context::Context::new(&mut world, time, 0, &mut events);
    
    bus.send(CollisionEnter {
        entity: handle,
        collision: Collision {
            other: handle,
            normal: engine_math::Vec3::Y,
            point: engine_math::Vec3::ZERO,
            relative_velocity: engine_math::Vec3::ZERO,
        },
    }, &mut context);
    assert_eq!(EVENT_COUNT.load(Ordering::SeqCst), 1);
    
    bus.send(TriggerEnter {
        entity: handle,
        trigger: TriggerData { other: handle },
    }, &mut context);
    assert_eq!(EVENT_COUNT.load(Ordering::SeqCst), 11);
    
    bus.send(MouseDown {
        entity: handle,
        button: MouseButton::Left,
    }, &mut context);
    assert_eq!(EVENT_COUNT.load(Ordering::SeqCst), 111);
}

#[test]
fn test_health_changed_event() {
    EVENT_COUNT.store(0, Ordering::SeqCst);
    
    let mut bus = EventBus::new();
    bus.on_event::<HealthChanged>(|e, _| {
        if e.new_health <= 0.0 {
            EVENT_COUNT.fetch_add(1, Ordering::SeqCst);
        }
    });
    bus.on_event::<EntityDied>(|_, _| {
        EVENT_COUNT.fetch_add(10, Ordering::SeqCst);
    });
    
    let handle = engine_core::gameobject::GameObjectHandle::new(0, 0);
    
    let mut world = World::new();
    let mut events = EventBus::new();
    let time = engine_core::Time::default();
    let mut context = engine_core::context::Context::new(&mut world, time, 0, &mut events);
    
    bus.send(HealthChanged {
        entity: handle,
        old_health: 100.0,
        new_health: 50.0,
    }, &mut context);
    assert_eq!(EVENT_COUNT.load(Ordering::SeqCst), 0);
    
    bus.send(HealthChanged {
        entity: handle,
        old_health: 50.0,
        new_health: 0.0,
    }, &mut context);
    assert_eq!(EVENT_COUNT.load(Ordering::SeqCst), 1);
    
    bus.send(EntityDied { entity: handle }, &mut context);
    assert_eq!(EVENT_COUNT.load(Ordering::SeqCst), 11);
}

#[test]
fn test_event_with_gameobject() {
    let mut world = World::new();
    
    let mut player = GameObject::new("Player");
    player.add_component(Health { current: 100.0, max: 100.0 });
    
    let handle = world.spawn(player);
    
    // Verify component exists
    let gameobject = world.get_gameobject(handle).unwrap();
    assert!(gameobject.has_component::<Health>());
    
    // Get component and verify
    let health = gameobject.get_component::<Health>().unwrap();
    assert_eq!(health.current, 100.0);
    assert_eq!(health.max, 100.0);
}

#[test]
fn test_multiple_event_handlers() {
    EVENT_COUNT.store(0, Ordering::SeqCst);
    
    let mut bus = EventBus::new();
    bus.on_event::<CollisionEnter>(|_, _| {
        EVENT_COUNT.fetch_add(1, Ordering::SeqCst);
    });
    bus.on_event::<CollisionEnter>(|_, _| {
        EVENT_COUNT.fetch_add(10, Ordering::SeqCst);
    });
    
    let handle = engine_core::gameobject::GameObjectHandle::new(0, 0);
    
    let mut world = World::new();
    let mut events = EventBus::new();
    let time = engine_core::Time::default();
    let mut context = engine_core::context::Context::new(&mut world, time, 0, &mut events);
    
    bus.send(CollisionEnter {
        entity: handle,
        collision: Collision {
            other: handle,
            normal: engine_math::Vec3::Y,
            point: engine_math::Vec3::ZERO,
            relative_velocity: engine_math::Vec3::ZERO,
        },
    }, &mut context);
    
    assert_eq!(EVENT_COUNT.load(Ordering::SeqCst), 11);
}
```

- [ ] **Step 2: Run integration tests**

Run: `cargo test -p engine-core --test events_tests`
Expected: All tests PASS

- [ ] **Step 3: Commit**

```bash
git add crates/engine-core/tests/events_tests.rs
git commit -m "test(core): add Events & Messaging integration tests

- Test built-in event types
- Test health changed event
- Test event with GameObject
- Test multiple event handlers"
```

---

## Summary

This plan completes **Phase 3: Events & Messaging** of the Unity Parity Refactoring. After completing all tasks:

1. **Built-in events** — Collision, Trigger, Mouse events
2. **EventHandler with Context** — Updated to accept Context for game logic
3. **SendMessage API** — Unity-like message passing on GameObjects
4. **MonoBehaviour event integration** — Collision, Trigger, Mouse callbacks
5. **Integration tests** — Verify event system works correctly

**Next Phase:** Phase 4 - ScriptableObject & Assets (Weeks 6-7)
