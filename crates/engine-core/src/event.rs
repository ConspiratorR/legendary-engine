use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::context::Context;

/// Type-safe event bus (like Unity's SendMessage, but type-safe).
pub struct EventBus {
    handlers: HashMap<TypeId, Vec<Box<dyn EventHandler>>>,
}

/// Trait for event handlers.
pub trait EventHandler: Send + Sync {
    fn handle(&mut self, event: &dyn Any, context: &mut Context<'_>);
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
    pub fn send<E: Event + 'static>(&mut self, event: E, context: &mut Context<'_>) {
        if let Some(handlers) = self.handlers.get_mut(&TypeId::of::<E>()) {
            for handler in handlers.iter_mut() {
                handler.handle(&event, context);
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
    F: Fn(&dyn Any, &mut Context<'_>) + Send + Sync,
{
    fn handle(&mut self, event: &dyn Any, context: &mut Context<'_>) {
        (self.handler)(event, context);
    }
}

/// Extension trait for EventBus to add closure-based handlers.
pub trait EventBusExt {
    /// Register a closure handler for an event type.
    fn on_event<E: Event + 'static>(
        &mut self,
        handler: impl Fn(&E, &mut Context<'_>) + Send + Sync + 'static,
    );
}

impl EventBusExt for EventBus {
    fn on_event<E: Event + 'static>(
        &mut self,
        handler: impl Fn(&E, &mut Context<'_>) + Send + Sync + 'static,
    ) {
        let wrapper = ClosureHandler {
            handler: move |event: &dyn Any, context: &mut Context<'_>| {
                if let Some(typed) = event.downcast_ref::<E>() {
                    handler(typed, context);
                }
            },
        };
        self.on::<E>(wrapper);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::time::Time;
    use crate::world::World;

    #[derive(Clone)]
    struct TestEvent {
        value: i32,
    }

    impl Event for TestEvent {}

    struct TestHandler {
        counter: Arc<AtomicUsize>,
    }

    impl EventHandler for TestHandler {
        fn handle(&mut self, event: &dyn Any, _context: &mut Context<'_>) {
            if let Some(e) = event.downcast_ref::<TestEvent>() {
                self.counter.fetch_add(e.value as usize, Ordering::SeqCst);
            }
        }
    }

    #[test]
    fn test_event_bus_send() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut bus = EventBus::new();
        bus.on::<TestEvent>(TestHandler {
            counter: counter.clone(),
        });

        let mut world = World::new();
        let mut events = EventBus::new();
        let mut ctx = Context::new(&mut world, Time::default(), 0, &mut events);

        bus.send(TestEvent { value: 5 }, &mut ctx);
        assert_eq!(counter.load(Ordering::SeqCst), 5);

        bus.send(TestEvent { value: 3 }, &mut ctx);
        assert_eq!(counter.load(Ordering::SeqCst), 8);
    }

    #[test]
    fn test_event_bus_closure() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut bus = EventBus::new();

        let c = counter.clone();
        bus.on_event::<TestEvent>(move |e, _ctx| {
            c.fetch_add(e.value as usize, Ordering::SeqCst);
        });

        let mut world = World::new();
        let mut events = EventBus::new();
        let mut ctx = Context::new(&mut world, Time::default(), 0, &mut events);

        bus.send(TestEvent { value: 10 }, &mut ctx);
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn test_event_bus_multiple_handlers() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut bus = EventBus::new();
        bus.on::<TestEvent>(TestHandler {
            counter: counter.clone(),
        });
        bus.on::<TestEvent>(TestHandler {
            counter: counter.clone(),
        });

        let mut world = World::new();
        let mut events = EventBus::new();
        let mut ctx = Context::new(&mut world, Time::default(), 0, &mut events);

        bus.send(TestEvent { value: 1 }, &mut ctx);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_event_bus_handler_count() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut bus = EventBus::new();
        assert_eq!(bus.handler_count::<TestEvent>(), 0);

        bus.on::<TestEvent>(TestHandler {
            counter: counter.clone(),
        });
        assert_eq!(bus.handler_count::<TestEvent>(), 1);

        bus.on::<TestEvent>(TestHandler {
            counter: counter.clone(),
        });
        assert_eq!(bus.handler_count::<TestEvent>(), 2);
    }
}
