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

/// Called when a collision starts (like Unity's OnCollisionEnter).
#[derive(Debug, Clone)]
pub struct CollisionEnter {
    /// The entity that was collided with.
    pub entity: GameObjectHandle,
    /// Collision data.
    pub collision: Collision,
}
impl Event for CollisionEnter {}

/// Called when a collision ends (like Unity's OnCollisionExit).
#[derive(Debug, Clone)]
pub struct CollisionExit {
    /// The entity that was collided with.
    pub entity: GameObjectHandle,
    /// Collision data.
    pub collision: Collision,
}
impl Event for CollisionExit {}

/// Called when a collision stays (like Unity's OnCollisionStay).
#[derive(Debug, Clone)]
pub struct CollisionStay {
    /// The entity that was collided with.
    pub entity: GameObjectHandle,
    /// Collision data.
    pub collision: Collision,
}
impl Event for CollisionStay {}

/// Called when a trigger is entered (like Unity's OnTriggerEnter).
#[derive(Debug, Clone)]
pub struct TriggerEnter {
    /// The entity that entered the trigger.
    pub entity: GameObjectHandle,
    /// Trigger data.
    pub trigger: TriggerData,
}
impl Event for TriggerEnter {}

/// Called when a trigger is exited (like Unity's OnTriggerExit).
#[derive(Debug, Clone)]
pub struct TriggerExit {
    /// The entity that exited the trigger.
    pub entity: GameObjectHandle,
    /// Trigger data.
    pub trigger: TriggerData,
}
impl Event for TriggerExit {}

/// Called when a trigger stays (like Unity's OnTriggerStay).
#[derive(Debug, Clone)]
pub struct TriggerStay {
    /// The entity that is in the trigger.
    pub entity: GameObjectHandle,
    /// Trigger data.
    pub trigger: TriggerData,
}
impl Event for TriggerStay {}

/// Called when the mouse enters the Collider (like Unity's OnMouseEnter).
#[derive(Debug, Clone)]
pub struct MouseEnter {
    /// The entity the mouse entered.
    pub entity: GameObjectHandle,
}
impl Event for MouseEnter {}

/// Called when the mouse exits the Collider (like Unity's OnMouseExit).
#[derive(Debug, Clone)]
pub struct MouseExit {
    /// The entity the mouse exited.
    pub entity: GameObjectHandle,
}
impl Event for MouseExit {}

/// Called when the mouse is pressed on the Collider (like Unity's OnMouseDown).
#[derive(Debug, Clone)]
pub struct MouseDown {
    /// The entity the mouse was pressed on.
    pub entity: GameObjectHandle,
    /// Which mouse button was pressed.
    pub button: MouseButton,
}
impl Event for MouseDown {}

/// Called when the mouse button is released (like Unity's OnMouseUp).
#[derive(Debug, Clone)]
pub struct MouseUp {
    /// The entity the mouse was released on.
    pub entity: GameObjectHandle,
    /// Which mouse button was released.
    pub button: MouseButton,
}
impl Event for MouseUp {}

/// Called when the mouse is dragged (like Unity's OnMouseDrag).
#[derive(Debug, Clone)]
pub struct MouseDrag {
    /// The entity being dragged.
    pub entity: GameObjectHandle,
    /// Which mouse button is being held.
    pub button: MouseButton,
}
impl Event for MouseDrag {}

/// Called when the mouse is hovering (like Unity's OnMouseOver).
#[derive(Debug, Clone)]
pub struct MouseOver {
    /// The entity being hovered over.
    pub entity: GameObjectHandle,
}
impl Event for MouseOver {}

/// Health changed event.
#[derive(Debug, Clone)]
pub struct HealthChanged {
    /// The entity whose health changed.
    pub entity: GameObjectHandle,
    /// Previous health value.
    pub old_health: f32,
    /// New health value.
    pub new_health: f32,
}
impl Event for HealthChanged {}

/// Entity died event.
#[derive(Debug, Clone)]
pub struct EntityDied {
    /// The entity that died.
    pub entity: GameObjectHandle,
}
impl Event for EntityDied {}

/// Entity spawned event.
#[derive(Debug, Clone)]
pub struct EntitySpawned {
    /// The entity that was spawned.
    pub entity: GameObjectHandle,
}
impl Event for EntitySpawned {}

/// Entity despawned event.
#[derive(Debug, Clone)]
pub struct EntityDespawned {
    /// The entity that was despawned.
    pub entity: GameObjectHandle,
}
impl Event for EntityDespawned {}

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
