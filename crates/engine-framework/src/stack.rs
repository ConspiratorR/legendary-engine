use crate::{GameState, StateCtx};

enum PendingOp {
    Push(Box<dyn GameState>),
    Pop,
    Replace(Box<dyn GameState>),
}

/// A stack of [`GameState`]s with deferred push/pop/replace operations.
///
/// Only the topmost state receives `update` calls. Operations are queued
/// and applied atomically when [`flush`](Self::flush) is called, so
/// mid-frame state changes are safe.
pub struct StateStack {
    states: Vec<Box<dyn GameState>>,
    pending: Vec<PendingOp>,
}

impl Default for StateStack {
    fn default() -> Self {
        Self::new()
    }
}

impl StateStack {
    /// Create an empty state stack.
    pub fn new() -> Self {
        Self {
            states: Vec::new(),
            pending: Vec::new(),
        }
    }
    /// Queue a state to be pushed on the next `flush`.
    pub fn push(&mut self, state: Box<dyn GameState>) {
        self.pending.push(PendingOp::Push(state));
    }
    /// Queue the top state to be popped on the next `flush`.
    pub fn pop(&mut self) {
        self.pending.push(PendingOp::Pop);
    }
    /// Queue a replacement of the top state on the next `flush`.
    pub fn replace(&mut self, state: Box<dyn GameState>) {
        self.pending.push(PendingOp::Replace(state));
    }
    /// Return the number of active (flushed) states.
    pub fn len(&self) -> usize {
        self.states.len()
    }
    /// Returns `true` if there are no active states.
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    /// Apply all pending push/pop/replace operations, calling the
    /// appropriate lifecycle hooks:
    ///
    /// - **Push**: `on_pause` on the current top, then `on_enter` on the new state.
    /// - **Pop**: `on_exit` on the popped state, then `on_resume` on the new top.
    /// - **Replace**: `on_exit` on the old top, then `on_enter` on the new state.
    pub fn flush(
        &mut self,
        world: &mut engine_ecs::world::World,
        resources: &mut engine_core::resource::ResourceRegistry,
    ) {
        let ops = std::mem::take(&mut self.pending);
        for op in ops {
            match op {
                PendingOp::Push(mut s) => {
                    if let Some(top) = self.states.last_mut() {
                        top.on_pause(&mut StateCtx {
                            world,
                            resources,
                            delta: 0.0,
                        });
                    }
                    s.on_enter(&mut StateCtx {
                        world,
                        resources,
                        delta: 0.0,
                    });
                    self.states.push(s);
                }
                PendingOp::Pop => {
                    if let Some(mut s) = self.states.pop() {
                        s.on_exit(&mut StateCtx {
                            world,
                            resources,
                            delta: 0.0,
                        });
                    }
                    if let Some(top) = self.states.last_mut() {
                        top.on_resume(&mut StateCtx {
                            world,
                            resources,
                            delta: 0.0,
                        });
                    }
                }
                PendingOp::Replace(mut s) => {
                    if let Some(mut o) = self.states.pop() {
                        o.on_exit(&mut StateCtx {
                            world,
                            resources,
                            delta: 0.0,
                        });
                    }
                    s.on_enter(&mut StateCtx {
                        world,
                        resources,
                        delta: 0.0,
                    });
                    self.states.push(s);
                }
            }
        }
    }

    /// Call `update` on the topmost state (if any).
    pub fn update_top(
        &mut self,
        world: &mut engine_ecs::world::World,
        resources: &mut engine_core::resource::ResourceRegistry,
        dt: f32,
    ) {
        if let Some(top) = self.states.last_mut() {
            top.update(
                &mut StateCtx {
                    world,
                    resources,
                    delta: dt,
                },
                dt,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{GameState, StateCtx, StateStack};
    use engine_core::resource::ResourceRegistry;
    use engine_ecs::world::World;

    struct TestState;
    impl GameState for TestState {
        fn on_enter(&mut self, _: &mut StateCtx) {}
        fn on_exit(&mut self, _: &mut StateCtx) {}
        fn on_pause(&mut self, _: &mut StateCtx) {}
        fn on_resume(&mut self, _: &mut StateCtx) {}
        fn update(&mut self, _: &mut StateCtx, _: f32) {}
    }

    #[test]
    fn test_push_then_flush_adds_state() {
        let mut w = World::new();
        let mut r = ResourceRegistry::new();
        let mut s = StateStack::new();
        s.push(Box::new(TestState));
        assert_eq!(s.len(), 0);
        s.flush(&mut w, &mut r);
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_pop_removes_state() {
        let mut w = World::new();
        let mut r = ResourceRegistry::new();
        let mut s = StateStack::new();
        s.push(Box::new(TestState));
        s.flush(&mut w, &mut r);
        s.pop();
        s.flush(&mut w, &mut r);
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_replace_swaps() {
        let mut w = World::new();
        let mut r = ResourceRegistry::new();
        let mut s = StateStack::new();
        s.push(Box::new(TestState));
        s.flush(&mut w, &mut r);
        s.replace(Box::new(TestState));
        s.flush(&mut w, &mut r);
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_empty_state() {
        let s = StateStack::new();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_multiple_pending() {
        let mut w = World::new();
        let mut r = ResourceRegistry::new();
        let mut s = StateStack::new();
        s.push(Box::new(TestState));
        s.push(Box::new(TestState));
        s.flush(&mut w, &mut r);
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn test_flush_empty_pending_is_noop() {
        let mut w = World::new();
        let mut r = ResourceRegistry::new();
        let mut s = StateStack::new();
        s.flush(&mut w, &mut r);
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_pop_empty_is_noop() {
        let mut w = World::new();
        let mut r = ResourceRegistry::new();
        let mut s = StateStack::new();
        s.pop();
        s.flush(&mut w, &mut r);
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_replace_empty_acts_as_push() {
        let mut w = World::new();
        let mut r = ResourceRegistry::new();
        let mut s = StateStack::new();
        s.replace(Box::new(TestState));
        s.flush(&mut w, &mut r);
        assert_eq!(s.len(), 1);
    }
}
