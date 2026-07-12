use crate::context::Context;
use crate::system::System;

/// Execution phase (like Unity's PlayerLoopTiming).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Initialization (Time, Input).
    Initialization,
    /// Before FixedUpdate.
    PreFixedUpdate,
    /// Fixed timestep (physics, animation).
    FixedUpdate,
    /// After FixedUpdate.
    PostFixedUpdate,
    /// Before Update.
    PreUpdate,
    /// Main update (game logic, input).
    Update,
    /// After Update.
    PostUpdate,
    /// Before LateUpdate.
    PreLateUpdate,
    /// Late update (camera follow, etc.).
    LateUpdate,
    /// After LateUpdate.
    PostLateUpdate,
    /// Rendering.
    Render,
    /// After rendering.
    AfterRender,
    /// Cleanup.
    Cleanup,
}

impl Phase {
    /// Get all phases in order.
    pub fn all() -> &'static [Phase] {
        &[
            Phase::Initialization,
            Phase::PreFixedUpdate,
            Phase::FixedUpdate,
            Phase::PostFixedUpdate,
            Phase::PreUpdate,
            Phase::Update,
            Phase::PostUpdate,
            Phase::PreLateUpdate,
            Phase::LateUpdate,
            Phase::PostLateUpdate,
            Phase::Render,
            Phase::AfterRender,
            Phase::Cleanup,
        ]
    }

    /// Get the index of this phase (for ordering).
    pub fn index(&self) -> usize {
        match self {
            Phase::Initialization => 0,
            Phase::PreFixedUpdate => 1,
            Phase::FixedUpdate => 2,
            Phase::PostFixedUpdate => 3,
            Phase::PreUpdate => 4,
            Phase::Update => 5,
            Phase::PostUpdate => 6,
            Phase::PreLateUpdate => 7,
            Phase::LateUpdate => 8,
            Phase::PostLateUpdate => 9,
            Phase::Render => 10,
            Phase::AfterRender => 11,
            Phase::Cleanup => 12,
        }
    }
}

/// The main game loop (like Unity's PlayerLoop).
pub struct PlayerLoop {
    /// Systems indexed by `Phase::index()`. Each slot holds systems registered for that phase.
    phase_systems: Vec<Vec<Box<dyn System>>>,
    startup_systems: Vec<Box<dyn System>>,
    startup_done: bool,
}

impl PlayerLoop {
    /// Create a new PlayerLoop.
    pub fn new() -> Self {
        Self {
            phase_systems: (0..Phase::all().len()).map(|_| Vec::new()).collect(),
            startup_systems: Vec::new(),
            startup_done: false,
        }
    }

    /// Register a system for a specific phase.
    pub fn add_system(&mut self, phase: Phase, system: impl System + 'static) {
        self.phase_systems[phase.index()].push(Box::new(system));
    }

    /// Register a startup system (runs once before first frame).
    pub fn add_startup_system(&mut self, system: impl System + 'static) {
        self.startup_systems.push(Box::new(system));
    }

    /// Execute one frame.
    pub fn run(&mut self, context: &mut Context) {
        // Run startup systems once
        if !self.startup_done {
            for system in &self.startup_systems {
                system.run(context);
            }
            self.startup_done = true;
        }

        // Execute phases in order (vec is already indexed by phase order)
        for systems in &self.phase_systems {
            for system in systems {
                system.run(context);
            }
        }
    }

    /// Get the total number of registered (non-startup) systems.
    pub fn system_count(&self) -> usize {
        self.phase_systems.iter().map(|v| v.len()).sum()
    }

    /// Get the number of startup systems.
    pub fn startup_system_count(&self) -> usize {
        self.startup_systems.len()
    }
}

impl Default for PlayerLoop {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::Time;
    use crate::world::World;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_player_loop_phases() {
        assert_eq!(Phase::all().len(), 13);
        assert_eq!(Phase::Initialization.index(), 0);
        assert_eq!(Phase::Update.index(), 5);
        assert_eq!(Phase::Cleanup.index(), 12);
    }

    #[test]
    fn test_player_loop_run() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let mut loop_ = PlayerLoop::new();
        loop_.add_system(Phase::Update, move |_: &mut Context| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });

        let mut world = World::new();
        let time = Time::default();
        let mut context = Context::new(&mut world, time, 0);

        loop_.run(&mut context);
        assert_eq!(counter.load(Ordering::Relaxed), 1);

        loop_.run(&mut context);
        assert_eq!(counter.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_player_loop_startup() {
        let counter = Arc::new(AtomicU32::new(0));
        let startup_counter = counter.clone();
        let update_counter = counter.clone();

        let mut loop_ = PlayerLoop::new();
        loop_.add_startup_system(move |_: &mut Context| {
            startup_counter.fetch_add(10, Ordering::Relaxed);
        });
        loop_.add_system(Phase::Update, move |_: &mut Context| {
            update_counter.fetch_add(1, Ordering::Relaxed);
        });

        let mut world = World::new();
        let time = Time::default();
        let mut context = Context::new(&mut world, time, 0);

        loop_.run(&mut context); // Startup + Update
        assert_eq!(counter.load(Ordering::Relaxed), 11);

        loop_.run(&mut context); // Only Update (startup already ran)
        assert_eq!(counter.load(Ordering::Relaxed), 12);
    }

    #[test]
    fn test_player_loop_phase_order() {
        let counter = Arc::new(AtomicU32::new(0));
        let late_counter = counter.clone();
        let update_counter = counter.clone();

        let mut loop_ = PlayerLoop::new();
        loop_.add_system(Phase::LateUpdate, move |_: &mut Context| {
            late_counter.fetch_add(100, Ordering::Relaxed);
        });
        loop_.add_system(Phase::Update, move |_: &mut Context| {
            update_counter.fetch_add(1, Ordering::Relaxed);
        });

        let mut world = World::new();
        let time = Time::default();
        let mut context = Context::new(&mut world, time, 0);

        loop_.run(&mut context);
        // Update (1) should run before LateUpdate (100)
        assert_eq!(counter.load(Ordering::Relaxed), 101);
    }
}
