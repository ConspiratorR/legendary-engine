use engine_core::resource::ResourceRegistry;
use engine_ecs::world::World;
use engine_framework::{GameState, StateCtx, StateStack};
use std::cell::RefCell;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

type Log = Rc<RefCell<Vec<String>>>;

struct TrackingState {
    name: String,
    log: Log,
}

impl GameState for TrackingState {
    fn on_enter(&mut self, _: &mut StateCtx) {
        self.log
            .borrow_mut()
            .push(format!("{}:on_enter", self.name));
    }
    fn on_exit(&mut self, _: &mut StateCtx) {
        self.log.borrow_mut().push(format!("{}:on_exit", self.name));
    }
    fn on_pause(&mut self, _: &mut StateCtx) {
        self.log
            .borrow_mut()
            .push(format!("{}:on_pause", self.name));
    }
    fn on_resume(&mut self, _: &mut StateCtx) {
        self.log
            .borrow_mut()
            .push(format!("{}:on_resume", self.name));
    }
    fn update(&mut self, _: &mut StateCtx, dt: f32) {
        self.log
            .borrow_mut()
            .push(format!("{}:update({})", self.name, dt));
    }
}

fn new_ctx() -> (World, ResourceRegistry) {
    (World::new(), ResourceRegistry::new())
}

// ---------------------------------------------------------------------------
// Tests: push / pop / replace
// ---------------------------------------------------------------------------

#[test]
fn push_adds_state_after_flush() {
    let (mut w, mut r) = new_ctx();
    let mut s = StateStack::new();
    assert_eq!(s.len(), 0);

    s.push(Box::new(TrackingState {
        name: "A".into(),
        log: Rc::new(RefCell::new(vec![])),
    }));
    assert_eq!(s.len(), 0, "push is deferred");

    s.flush(&mut w, &mut r);
    assert_eq!(s.len(), 1, "flush applies pending push");
}

#[test]
fn pop_removes_state_after_flush() {
    let (mut w, mut r) = new_ctx();
    let mut s = StateStack::new();

    s.push(Box::new(TrackingState {
        name: "A".into(),
        log: Rc::new(RefCell::new(vec![])),
    }));
    s.flush(&mut w, &mut r);
    assert_eq!(s.len(), 1);

    s.pop();
    assert_eq!(s.len(), 1, "pop is deferred");
    s.flush(&mut w, &mut r);
    assert_eq!(s.len(), 0, "flush applies pending pop");
}

#[test]
fn replace_swaps_top_state() {
    let (mut w, mut r) = new_ctx();
    let mut s = StateStack::new();

    s.push(Box::new(TrackingState {
        name: "A".into(),
        log: Rc::new(RefCell::new(vec![])),
    }));
    s.flush(&mut w, &mut r);
    assert_eq!(s.len(), 1);

    s.replace(Box::new(TrackingState {
        name: "B".into(),
        log: Rc::new(RefCell::new(vec![])),
    }));
    s.flush(&mut w, &mut r);
    assert_eq!(s.len(), 1, "replace keeps stack at same depth");
}

#[test]
fn push_multiple_states() {
    let (mut w, mut r) = new_ctx();
    let mut s = StateStack::new();

    s.push(Box::new(TrackingState {
        name: "A".into(),
        log: Rc::new(RefCell::new(vec![])),
    }));
    s.push(Box::new(TrackingState {
        name: "B".into(),
        log: Rc::new(RefCell::new(vec![])),
    }));
    s.push(Box::new(TrackingState {
        name: "C".into(),
        log: Rc::new(RefCell::new(vec![])),
    }));
    s.flush(&mut w, &mut r);
    assert_eq!(s.len(), 3);
}

#[test]
fn pop_empty_stack_is_noop() {
    let (mut w, mut r) = new_ctx();
    let mut s = StateStack::new();

    s.pop();
    s.flush(&mut w, &mut r);
    assert_eq!(s.len(), 0, "pop on empty stack does nothing");
}

#[test]
fn replace_empty_stack_pushes() {
    let (mut w, mut r) = new_ctx();
    let mut s = StateStack::new();

    s.replace(Box::new(TrackingState {
        name: "A".into(),
        log: Rc::new(RefCell::new(vec![])),
    }));
    s.flush(&mut w, &mut r);
    assert_eq!(s.len(), 1, "replace on empty stack acts as push");
}

// ---------------------------------------------------------------------------
// Tests: lifecycle callbacks (on_enter, on_exit, on_update)
// ---------------------------------------------------------------------------

#[test]
fn push_calls_on_enter() {
    let (mut w, mut r) = new_ctx();
    let log: Log = Rc::new(RefCell::new(vec![]));
    let mut s = StateStack::new();

    s.push(Box::new(TrackingState {
        name: "A".into(),
        log: log.clone(),
    }));
    s.flush(&mut w, &mut r);

    assert_eq!(*log.borrow(), vec!["A:on_enter"]);
}

#[test]
fn pop_calls_on_exit() {
    let (mut w, mut r) = new_ctx();
    let log: Log = Rc::new(RefCell::new(vec![]));
    let mut s = StateStack::new();

    s.push(Box::new(TrackingState {
        name: "A".into(),
        log: log.clone(),
    }));
    s.flush(&mut w, &mut r);
    log.borrow_mut().clear();

    s.pop();
    s.flush(&mut w, &mut r);

    assert_eq!(*log.borrow(), vec!["A:on_exit"]);
}

#[test]
fn replace_calls_exit_then_enter() {
    let (mut w, mut r) = new_ctx();
    let log: Log = Rc::new(RefCell::new(vec![]));
    let mut s = StateStack::new();

    s.push(Box::new(TrackingState {
        name: "A".into(),
        log: log.clone(),
    }));
    s.flush(&mut w, &mut r);
    log.borrow_mut().clear();

    s.replace(Box::new(TrackingState {
        name: "B".into(),
        log: log.clone(),
    }));
    s.flush(&mut w, &mut r);

    let entries = log.borrow().clone();
    assert_eq!(entries, vec!["A:on_exit", "B:on_enter"]);
}

#[test]
fn update_top_calls_topmost_state_only() {
    let (mut w, mut r) = new_ctx();
    let log_a: Log = Rc::new(RefCell::new(vec![]));
    let log_b: Log = Rc::new(RefCell::new(vec![]));
    let mut s = StateStack::new();

    s.push(Box::new(TrackingState {
        name: "A".into(),
        log: log_a.clone(),
    }));
    s.push(Box::new(TrackingState {
        name: "B".into(),
        log: log_b.clone(),
    }));
    s.flush(&mut w, &mut r);
    log_a.borrow_mut().clear();
    log_b.borrow_mut().clear();

    s.update_top(&mut w, &mut r, 0.5);

    assert!(log_a.borrow().is_empty(), "bottom state should not update");
    assert_eq!(*log_b.borrow(), vec!["B:update(0.5)"]);
}

#[test]
fn update_top_on_empty_stack_is_noop() {
    let (mut w, mut r) = new_ctx();
    let mut s = StateStack::new();
    // Should not panic
    s.update_top(&mut w, &mut r, 0.016);
}

#[test]
fn push_multiple_flushes_call_enter_in_order() {
    let (mut w, mut r) = new_ctx();
    let log: Log = Rc::new(RefCell::new(vec![]));
    let mut s = StateStack::new();

    s.push(Box::new(TrackingState {
        name: "A".into(),
        log: log.clone(),
    }));
    s.push(Box::new(TrackingState {
        name: "B".into(),
        log: log.clone(),
    }));
    s.push(Box::new(TrackingState {
        name: "C".into(),
        log: log.clone(),
    }));
    s.flush(&mut w, &mut r);

    assert_eq!(
        *log.borrow(),
        vec![
            "A:on_enter",
            "A:on_pause",
            "B:on_enter",
            "B:on_pause",
            "C:on_enter"
        ]
    );
}

#[test]
fn pop_multiple_exits_in_lifo_order() {
    let (mut w, mut r) = new_ctx();
    let log: Log = Rc::new(RefCell::new(vec![]));
    let mut s = StateStack::new();

    s.push(Box::new(TrackingState {
        name: "A".into(),
        log: log.clone(),
    }));
    s.push(Box::new(TrackingState {
        name: "B".into(),
        log: log.clone(),
    }));
    s.push(Box::new(TrackingState {
        name: "C".into(),
        log: log.clone(),
    }));
    s.flush(&mut w, &mut r);
    log.borrow_mut().clear();

    s.pop();
    s.pop();
    s.pop();
    s.flush(&mut w, &mut r);

    assert_eq!(
        *log.borrow(),
        vec![
            "C:on_exit",
            "B:on_resume",
            "B:on_exit",
            "A:on_resume",
            "A:on_exit"
        ]
    );
}

// ---------------------------------------------------------------------------
// Tests: empty / len
// ---------------------------------------------------------------------------

#[test]
fn new_stack_is_empty() {
    let s = StateStack::new();
    assert!(s.is_empty());
    assert_eq!(s.len(), 0);
}

#[test]
fn len_reflects_flushed_states() {
    let (mut w, mut r) = new_ctx();
    let mut s = StateStack::new();

    for i in 0..5 {
        s.push(Box::new(TrackingState {
            name: format!("S{i}"),
            log: Rc::new(RefCell::new(vec![])),
        }));
    }
    assert_eq!(s.len(), 0, "pending ops don't affect len before flush");

    s.flush(&mut w, &mut r);
    assert_eq!(s.len(), 5);
    assert!(!s.is_empty());
}

#[test]
fn len_drops_to_zero_after_popping_all() {
    let (mut w, mut r) = new_ctx();
    let mut s = StateStack::new();

    s.push(Box::new(TrackingState {
        name: "A".into(),
        log: Rc::new(RefCell::new(vec![])),
    }));
    s.push(Box::new(TrackingState {
        name: "B".into(),
        log: Rc::new(RefCell::new(vec![])),
    }));
    s.flush(&mut w, &mut r);
    assert_eq!(s.len(), 2);

    s.pop();
    s.pop();
    s.flush(&mut w, &mut r);
    assert_eq!(s.len(), 0);
    assert!(s.is_empty());
}

#[test]
fn deferred_ops_do_not_affect_len_until_flush() {
    let (mut w, mut r) = new_ctx();
    let mut s = StateStack::new();

    s.push(Box::new(TrackingState {
        name: "A".into(),
        log: Rc::new(RefCell::new(vec![])),
    }));
    assert_eq!(s.len(), 0);

    s.flush(&mut w, &mut r);
    assert_eq!(s.len(), 1);

    s.pop();
    assert_eq!(s.len(), 1, "pop is deferred");

    s.flush(&mut w, &mut r);
    assert_eq!(s.len(), 0);
}

// ---------------------------------------------------------------------------
// Tests: on_pause / on_resume lifecycle
// ---------------------------------------------------------------------------

#[test]
fn push_calls_on_pause_on_previous_top() {
    let (mut w, mut r) = new_ctx();
    let log: Log = Rc::new(RefCell::new(vec![]));
    let mut s = StateStack::new();

    s.push(Box::new(TrackingState {
        name: "A".into(),
        log: log.clone(),
    }));
    s.flush(&mut w, &mut r);
    log.borrow_mut().clear();

    s.push(Box::new(TrackingState {
        name: "B".into(),
        log: log.clone(),
    }));
    s.flush(&mut w, &mut r);

    let entries = log.borrow().clone();
    assert_eq!(entries, vec!["A:on_pause", "B:on_enter"]);
}

#[test]
fn pop_calls_on_resume_on_new_top() {
    let (mut w, mut r) = new_ctx();
    let log: Log = Rc::new(RefCell::new(vec![]));
    let mut s = StateStack::new();

    s.push(Box::new(TrackingState {
        name: "A".into(),
        log: log.clone(),
    }));
    s.push(Box::new(TrackingState {
        name: "B".into(),
        log: log.clone(),
    }));
    s.flush(&mut w, &mut r);
    log.borrow_mut().clear();

    s.pop();
    s.flush(&mut w, &mut r);

    let entries = log.borrow().clone();
    assert_eq!(entries, vec!["B:on_exit", "A:on_resume"]);
}

#[test]
fn pop_empty_stack_no_exit_callback() {
    let (mut w, mut r) = new_ctx();
    let log: Log = Rc::new(RefCell::new(vec![]));
    let mut s = StateStack::new();

    // Push a state so the log is in scope, then clear
    s.push(Box::new(TrackingState {
        name: "A".into(),
        log: log.clone(),
    }));
    s.flush(&mut w, &mut r);
    log.borrow_mut().clear();

    // Pop A, then pop again on empty stack
    s.pop();
    s.flush(&mut w, &mut r);
    assert_eq!(*log.borrow(), vec!["A:on_exit"]);

    log.borrow_mut().clear();
    s.pop();
    s.flush(&mut w, &mut r);
    assert!(
        log.borrow().is_empty(),
        "no callbacks on pop of empty stack"
    );
}

#[test]
fn replace_on_empty_stack_no_exit_callback() {
    let (mut w, mut r) = new_ctx();
    let log: Log = Rc::new(RefCell::new(vec![]));
    let mut s = StateStack::new();

    s.replace(Box::new(TrackingState {
        name: "A".into(),
        log: log.clone(),
    }));
    s.flush(&mut w, &mut r);

    assert_eq!(*log.borrow(), vec!["A:on_enter"]);
}

#[test]
fn replace_with_same_state_type() {
    let (mut w, mut r) = new_ctx();
    let log_a: Log = Rc::new(RefCell::new(vec![]));
    let log_b: Log = Rc::new(RefCell::new(vec![]));
    let mut s = StateStack::new();

    s.push(Box::new(TrackingState {
        name: "A".into(),
        log: log_a.clone(),
    }));
    s.flush(&mut w, &mut r);
    log_a.borrow_mut().clear();

    // Replace with a new instance of the same name
    s.replace(Box::new(TrackingState {
        name: "A".into(),
        log: log_b.clone(),
    }));
    s.flush(&mut w, &mut r);

    assert_eq!(s.len(), 1);
    // Old A got on_exit, new A got on_enter
    assert_eq!(*log_a.borrow(), vec!["A:on_exit"]);
    assert_eq!(*log_b.borrow(), vec!["A:on_enter"]);
}

#[test]
fn nested_push_pop_preserves_stack() {
    let (mut w, mut r) = new_ctx();
    let log: Log = Rc::new(RefCell::new(vec![]));
    let mut s = StateStack::new();

    // Push A
    s.push(Box::new(TrackingState {
        name: "A".into(),
        log: log.clone(),
    }));
    s.flush(&mut w, &mut r);
    assert_eq!(s.len(), 1);

    // Push B
    s.push(Box::new(TrackingState {
        name: "B".into(),
        log: log.clone(),
    }));
    s.flush(&mut w, &mut r);
    assert_eq!(s.len(), 2);

    // Push C
    s.push(Box::new(TrackingState {
        name: "C".into(),
        log: log.clone(),
    }));
    s.flush(&mut w, &mut r);
    assert_eq!(s.len(), 3);

    log.borrow_mut().clear();

    // Pop C
    s.pop();
    s.flush(&mut w, &mut r);
    assert_eq!(s.len(), 2);
    assert_eq!(*log.borrow(), vec!["C:on_exit", "B:on_resume"]);

    log.borrow_mut().clear();

    // Pop B
    s.pop();
    s.flush(&mut w, &mut r);
    assert_eq!(s.len(), 1);
    assert_eq!(*log.borrow(), vec!["B:on_exit", "A:on_resume"]);
}

#[test]
fn replace_does_not_call_on_pause() {
    // Replace should call on_exit (not on_pause) on the old state
    let (mut w, mut r) = new_ctx();
    let log: Log = Rc::new(RefCell::new(vec![]));
    let mut s = StateStack::new();

    s.push(Box::new(TrackingState {
        name: "A".into(),
        log: log.clone(),
    }));
    s.flush(&mut w, &mut r);
    log.borrow_mut().clear();

    s.replace(Box::new(TrackingState {
        name: "B".into(),
        log: log.clone(),
    }));
    s.flush(&mut w, &mut r);

    let entries = log.borrow().clone();
    assert_eq!(entries, vec!["A:on_exit", "B:on_enter"]);
    assert!(
        !entries.iter().any(|e| e.contains("on_pause")),
        "replace should not call on_pause"
    );
    assert!(
        !entries.iter().any(|e| e.contains("on_resume")),
        "replace should not call on_resume"
    );
}

#[test]
fn push_then_pop_single_state_full_lifecycle() {
    let (mut w, mut r) = new_ctx();
    let log: Log = Rc::new(RefCell::new(vec![]));
    let mut s = StateStack::new();

    s.push(Box::new(TrackingState {
        name: "A".into(),
        log: log.clone(),
    }));
    s.flush(&mut w, &mut r);
    log.borrow_mut().clear();

    s.pop();
    s.flush(&mut w, &mut r);

    // Only on_exit; no on_resume because stack is now empty
    assert_eq!(*log.borrow(), vec!["A:on_exit"]);
}
