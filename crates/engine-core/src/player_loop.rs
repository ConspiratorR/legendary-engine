use crate::context::Context;
use crate::system::System;
use std::collections::HashMap;

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
        Phase::all().iter().position(|&p| p == *self).unwrap_or(0)
    }
}

/// A system registered for a specific phase.
struct PhaseSystem {
    phase: Phase,
    system: Box<dyn System>,
}

/// The main game loop (like Unity's PlayerLoop).
pub struct PlayerLoop {
    systems: Vec<PhaseSystem>,
    startup_systems: Vec<Box<dyn System>>,
    startup_done: bool,
}

impl PlayerLoop {
    /// Create a new PlayerLoop.
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
            startup_systems: Vec::new(),
            startup_done: false,
        }
    }

    /// Register a system for a specific phase.
    pub fn add_system(&mut self, phase: Phase, system: impl System + 'static) {
        self.systems.push(PhaseSystem {
            phase,
            system: Box::new(system),
        });
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

        // Sort systems by phase order
        let mut phase_groups: HashMap<usize, Vec<&PhaseSystem>> = HashMap::new();
        for system in &self.systems {
            let index = system.phase.index();
            phase_groups.entry(index).or_default().push(system);
        }

        // Execute phases in order
        let mut indices: Vec<usize> = phase_groups.keys().copied().collect();
        indices.sort();

        for index in indices {
            if let Some(systems) = phase_groups.get(&index) {
                for system in systems {
                    system.system.run(context);
                }
            }
        }
    }

    /// Get the number of registered systems.
    pub fn system_count(&self) -> usize {
        self.systems.len()
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
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

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
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        let mut world = World::new();
        let time = Time::default();
        let mut context = Context::new(&mut world, time, 0);

        loop_.run(&mut context);
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        loop_.run(&mut context);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_player_loop_startup() {
        let counter = Arc::new(AtomicU32::new(0));
        let startup_counter = counter.clone();
        let update_counter = counter.clone();

        let mut loop_ = PlayerLoop::new();
        loop_.add_startup_system(move |_: &mut Context| {
            startup_counter.fetch_add(10, Ordering::SeqCst);
        });
        loop_.add_system(Phase::Update, move |_: &mut Context| {
            update_counter.fetch_add(1, Ordering::SeqCst);
        });

        let mut world = World::new();
        let time = Time::default();
        let mut context = Context::new(&mut world, time, 0);

        loop_.run(&mut context); // Startup + Update
        assert_eq!(counter.load(Ordering::SeqCst), 11);

        loop_.run(&mut context); // Only Update (startup already ran)
        assert_eq!(counter.load(Ordering::SeqCst), 12);
    }

    #[test]
    fn test_player_loop_phase_order() {
        let counter = Arc::new(AtomicU32::new(0));
        let late_counter = counter.clone();
        let update_counter = counter.clone();

        let mut loop_ = PlayerLoop::new();
        loop_.add_system(Phase::LateUpdate, move |_: &mut Context| {
            late_counter.fetch_add(100, Ordering::SeqCst);
        });
        loop_.add_system(Phase::Update, move |_: &mut Context| {
            update_counter.fetch_add(1, Ordering::SeqCst);
        });

        let mut world = World::new();
        let time = Time::default();
        let mut context = Context::new(&mut world, time, 0);

        loop_.run(&mut context);
        // Update (1) should run before LateUpdate (100)
        assert_eq!(counter.load(Ordering::SeqCst), 101);
    }
}
