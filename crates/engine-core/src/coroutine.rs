//! Coroutine system (matches Unity's MonoBehaviour coroutines).
//!
//! # Unity Documentation
//! <https://docs.unity3d.com/Manual/Coroutines.html>
//!
//! Unity coroutines are `IEnumerator`s that `yield` between steps. Rust has no
//! generator-based coroutines in stable, so this module models a routine as an
//! explicit list of [`CoroutineStep`]s executed over time by the World.
//!
//! ## Example
//!
//! ```rust
//! use engine_core::world::World;
//! use engine_core::coroutine::CoroutineStep;
//!
//! let mut world = World::new();
//! let obj = world.CreateGameObject("Flash");
//! let handle = world.StartCoroutine(
//!     obj,
//!     "Blink",
//!     vec![
//!         CoroutineStep::Wait(0.25),
//!         CoroutineStep::SetActive(false),
//!         CoroutineStep::Wait(0.25),
//!         CoroutineStep::SetActive(true),
//!     ],
//! );
//! ```

use crate::gameobject::GameObjectHandle;
use std::collections::VecDeque;

/// Handle to a running coroutine (matches Unity's `Coroutine`).
///
/// # Unity Documentation
/// <https://docs.unity3d.com/ScriptReference/Coroutine.html>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoroutineId(pub u64);

impl CoroutineId {
    pub const INVALID: Self = Self(u64::MAX);
}

/// One step of a coroutine.
///
/// Steps run in order. `Wait` pauses the routine for N seconds (scaled by
/// `Time.timeScale`); other steps complete in a single frame.
#[derive(Clone)]
pub enum CoroutineStep {
    /// Pause for N seconds (matches `yield return new WaitForSeconds(n)`).
    Wait(f32),
    /// Wait until end of current frame (matches `yield return null` / `WaitForEndOfFrame`).
    WaitEndOfFrame,
    /// Wait until next FixedUpdate (matches `WaitForFixedUpdate`).
    WaitFixedUpdate,
    /// Set the owner GameObject's active state.
    SetActive(bool),
    /// Send a message to the owner (method name on MonoBehaviours).
    Call(String),
    /// Custom action with access to World and owner handle.
    Action(std::sync::Arc<dyn Fn(&mut crate::world::World, GameObjectHandle) + Send + Sync>),
}

impl std::fmt::Debug for CoroutineStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoroutineStep::Wait(s) => write!(f, "Wait({s})"),
            CoroutineStep::WaitEndOfFrame => write!(f, "WaitEndOfFrame"),
            CoroutineStep::WaitFixedUpdate => write!(f, "WaitFixedUpdate"),
            CoroutineStep::SetActive(a) => write!(f, "SetActive({a})"),
            CoroutineStep::Call(m) => write!(f, "Call({m})"),
            CoroutineStep::Action(_) => write!(f, "Action(..)"),
        }
    }
}

/// A running coroutine instance.
pub struct Coroutine {
    id: CoroutineId,
    owner: GameObjectHandle,
    name: String,
    steps: VecDeque<CoroutineStep>,
    /// Remaining wait time (seconds), if currently waiting on Wait.
    wait_remaining: Option<f32>,
    /// Waiting for end of frame (resumes next tick).
    wait_end_of_frame: bool,
    /// Waiting for FixedUpdate phase.
    wait_fixed: bool,
    finished: bool,
}

impl Coroutine {
    pub fn id(&self) -> CoroutineId {
        self.id
    }

    pub fn owner(&self) -> GameObjectHandle {
        self.owner
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Advance this coroutine. Returns true if still running.
    fn advance(&mut self, world: &mut crate::world::World, dt: f32, in_fixed: bool) -> bool {
        if self.finished || !world.is_valid(self.owner) {
            self.finished = true;
            return false;
        }

        // Handle active waits
        if let Some(remaining) = self.wait_remaining.take() {
            let left = remaining - dt;
            if left > 0.0 {
                self.wait_remaining = Some(left);
                return true;
            }
            // Wait finished; fall through to run next steps this frame
        }
        if self.wait_end_of_frame {
            self.wait_end_of_frame = false;
            // continue to run steps after end-of-frame yield
        }
        if self.wait_fixed {
            if !in_fixed {
                self.wait_fixed = true;
                return true;
            }
            self.wait_fixed = false;
        }

        // Run steps until we hit a wait or finish
        while let Some(step) = self.steps.front() {
            match step {
                CoroutineStep::Wait(secs) => {
                    let secs = *secs;
                    self.steps.pop_front();
                    if secs > 0.0 {
                        // Count this frame's dt toward the wait
                        let left = secs - dt;
                        if left > 0.0 {
                            self.wait_remaining = Some(left);
                            return true;
                        }
                        // wait fully consumed this frame; continue
                    }
                }
                CoroutineStep::WaitEndOfFrame => {
                    self.steps.pop_front();
                    self.wait_end_of_frame = true;
                    return true;
                }
                CoroutineStep::WaitFixedUpdate => {
                    self.steps.pop_front();
                    self.wait_fixed = true;
                    return true;
                }
                _ => {
                    let step = self.steps.pop_front().unwrap();
                    self.execute_step(world, step);
                }
            }
        }

        self.finished = true;
        false
    }

    fn execute_step(&mut self, world: &mut crate::world::World, step: CoroutineStep) {
        let owner = self.owner;
        match step {
            CoroutineStep::SetActive(active) => {
                world.SetActive(owner, active);
            }
            CoroutineStep::Call(method) => {
                world.SendMessage(owner, &method);
            }
            CoroutineStep::Action(f) => {
                f(world, owner);
            }
            // handled in advance()
            CoroutineStep::Wait(_)
            | CoroutineStep::WaitEndOfFrame
            | CoroutineStep::WaitFixedUpdate => {}
        }
    }
}

/// Manages all running coroutines on a World.
#[derive(Default)]
pub struct CoroutineRunner {
    coroutines: Vec<Coroutine>,
    next_id: u64,
}

impl CoroutineRunner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a coroutine on `owner`. Returns a handle for StopCoroutine.
    pub fn start(
        &mut self,
        owner: GameObjectHandle,
        name: impl Into<String>,
        steps: Vec<CoroutineStep>,
    ) -> CoroutineId {
        let id = CoroutineId(self.next_id);
        self.next_id += 1;
        self.coroutines.push(Coroutine {
            id,
            owner,
            name: name.into(),
            steps: steps.into(),
            wait_remaining: None,
            wait_end_of_frame: false,
            wait_fixed: false,
            finished: false,
        });
        id
    }

    /// Stop a specific coroutine.
    pub fn stop(&mut self, id: CoroutineId) -> bool {
        let before = self.coroutines.len();
        self.coroutines.retain(|c| c.id != id);
        before != self.coroutines.len()
    }

    /// Stop all coroutines owned by a GameObject.
    pub fn stop_all_for(&mut self, owner: GameObjectHandle) {
        self.coroutines.retain(|c| c.owner != owner);
    }

    /// Stop every coroutine.
    pub fn stop_all(&mut self) {
        self.coroutines.clear();
    }

    /// Whether a coroutine is still running.
    pub fn is_running(&self, id: CoroutineId) -> bool {
        self.coroutines.iter().any(|c| c.id == id && !c.finished)
    }

    /// Number of live coroutines.
    pub fn count(&self) -> usize {
        self.coroutines.iter().filter(|c| !c.finished).count()
    }

    /// Tick all coroutines (called once per frame from World).
    pub fn tick(&mut self, world: &mut crate::world::World, dt: f32, in_fixed: bool) {
        let mut list = std::mem::take(&mut self.coroutines);
        for mut co in list.drain(..) {
            if co.advance(world, dt, in_fixed) {
                self.coroutines.push(co);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::World;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn test_coroutine_wait_then_finish() {
        let mut world = World::new();
        let obj = world.CreateGameObject("Obj");
        let mut runner = CoroutineRunner::new();
        let id = runner.start(obj, "Test", vec![CoroutineStep::Wait(0.1)]);
        assert_eq!(runner.count(), 1);

        runner.tick(&mut world, 0.05, false);
        assert!(runner.is_running(id));

        runner.tick(&mut world, 0.06, false);
        assert!(!runner.is_running(id));
        assert_eq!(runner.count(), 0);
    }

    #[test]
    fn test_coroutine_action_runs() {
        let mut world = World::new();
        let obj = world.CreateGameObject("Obj");
        let hits = Arc::new(AtomicU32::new(0));
        let hits2 = hits.clone();
        let mut runner = CoroutineRunner::new();
        runner.start(
            obj,
            "Act",
            vec![CoroutineStep::Action(Arc::new(move |_w, _o| {
                hits2.fetch_add(1, Ordering::Relaxed);
            }))],
        );
        runner.tick(&mut world, 0.016, false);
        assert_eq!(hits.load(Ordering::Relaxed), 1);
        assert_eq!(runner.count(), 0);
    }

    #[test]
    fn test_coroutine_set_active() {
        let mut world = World::new();
        let obj = world.CreateGameObject("Obj");
        let mut runner = CoroutineRunner::new();
        runner.start(obj, "Hide", vec![CoroutineStep::SetActive(false)]);
        runner.tick(&mut world, 0.016, false);
        assert!(!world.IsActive(obj));
    }

    #[test]
    fn test_stop_coroutine() {
        let mut world = World::new();
        let obj = world.CreateGameObject("Obj");
        let mut runner = CoroutineRunner::new();
        let id = runner.start(obj, "Long", vec![CoroutineStep::Wait(10.0)]);
        assert!(runner.stop(id));
        assert_eq!(runner.count(), 0);
    }

    #[test]
    fn test_stop_all_for_owner() {
        let mut world = World::new();
        let obj = world.CreateGameObject("Obj");
        let other = world.CreateGameObject("Other");
        let mut runner = CoroutineRunner::new();
        runner.start(obj, "A", vec![CoroutineStep::Wait(1.0)]);
        runner.start(obj, "B", vec![CoroutineStep::Wait(1.0)]);
        runner.start(other, "C", vec![CoroutineStep::Wait(1.0)]);
        runner.stop_all_for(obj);
        assert_eq!(runner.count(), 1);
    }
}
