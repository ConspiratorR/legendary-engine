use std::sync::Arc;

/// Unique identifier for a registered event listener.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListenerId(usize);

/// Generic synchronous publish/subscribe event channel.
///
/// Listeners are called in registration order when `emit()` is invoked.
/// Uses `Fn(&T)` so multiple listeners can fire without `&mut` conflicts.
pub struct EventChannel<T: Send + 'static> {
    listeners: Vec<(ListenerId, Arc<dyn Fn(&T) + Send + Sync>)>,
    next_id: usize,
}

impl<T: Send + 'static> EventChannel<T> {
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
            next_id: 0,
        }
    }

    /// Register a listener. Returns a `ListenerId` for later removal.
    pub fn subscribe(&mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> ListenerId {
        let id = ListenerId(self.next_id);
        self.next_id += 1;
        self.listeners.push((id, Arc::new(handler)));
        id
    }

    /// Remove a listener by id. No-op if already removed.
    pub fn unsubscribe(&mut self, id: ListenerId) {
        self.listeners.retain(|(lid, _)| *lid != id);
    }

    /// Fire the event to all registered listeners.
    pub fn emit(&self, event: &T) {
        for (_, listener) in &self.listeners {
            listener(event);
        }
    }
}

impl<T: Send + 'static> Default for EventChannel<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_emit_calls_listener() {
        let mut channel = EventChannel::<i32>::new();
        let received = Arc::new(AtomicUsize::new(0));
        let r = received.clone();
        channel.subscribe(move |val| {
            r.store(*val as usize, Ordering::Relaxed);
        });
        channel.emit(&42);
        assert_eq!(received.load(Ordering::Relaxed), 42);
    }

    #[test]
    fn test_multiple_listeners() {
        let mut channel = EventChannel::<i32>::new();
        let sum = Arc::new(AtomicUsize::new(0));

        let s1 = sum.clone();
        channel.subscribe(move |val| {
            s1.fetch_add(*val as usize, Ordering::Relaxed);
        });
        let s2 = sum.clone();
        channel.subscribe(move |val| {
            s2.fetch_add(*val as usize, Ordering::Relaxed);
        });

        channel.emit(&10);
        assert_eq!(sum.load(Ordering::Relaxed), 20);
    }

    #[test]
    fn test_unsubscribe_removes_listener() {
        let mut channel = EventChannel::<i32>::new();
        let received = Arc::new(AtomicUsize::new(0));
        let r = received.clone();
        let id = channel.subscribe(move |val| {
            r.store(*val as usize, Ordering::Relaxed);
        });
        channel.unsubscribe(id);
        channel.emit(&42);
        assert_eq!(received.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_default_is_empty() {
        let channel = EventChannel::<i32>::default();
        channel.emit(&1); // should not panic
    }
}
