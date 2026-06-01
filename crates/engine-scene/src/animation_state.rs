use std::collections::HashMap;

/// A state in an animation state machine.
#[derive(Debug, Clone)]
pub struct AnimationState {
    pub name: String,
    pub clip_name: String,
    pub speed: f32,
    pub looping: bool,
}

impl AnimationState {
    pub fn new(name: impl Into<String>, clip_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            clip_name: clip_name.into(),
            speed: 1.0,
            looping: true,
        }
    }

    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    pub fn with_looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }
}

/// A transition between two animation states.
#[derive(Debug, Clone)]
pub struct AnimationTransition {
    pub from: String,
    pub to: String,
    /// Blend duration in seconds.
    pub blend_duration: f32,
    /// Condition function name (for triggering).
    pub condition: TransitionCondition,
}

/// Condition that triggers a transition.
#[derive(Debug, Clone)]
pub enum TransitionCondition {
    /// Always transition immediately.
    Always,
    /// Transition when a bool parameter is true.
    BoolTrue(String),
    /// Transition when a bool parameter is false.
    BoolFalse(String),
    /// Transition when a float parameter exceeds a threshold.
    FloatGreater(String, f32),
    /// Transition when a float parameter is below a threshold.
    FloatLess(String, f32),
    /// Transition when triggered manually.
    Trigger(String),
}

/// Animation state machine managing states and transitions.
///
/// Controls which animation clip plays based on state transitions
/// and parameter conditions.
#[derive(Debug)]
pub struct AnimationStateMachine {
    pub current_state: String,
    pub states: HashMap<String, AnimationState>,
    pub transitions: Vec<AnimationTransition>,
    pub parameters: AnimationParameters,
    /// Current blend progress (0.0 = fully in current state, 1.0 = fully in target).
    pub blend_progress: f32,
    /// Target state during a blend transition.
    pub blend_target: Option<String>,
    /// Blend duration for the current transition.
    pub blend_duration: f32,
}

/// Parameters used to evaluate transition conditions.
#[derive(Debug, Clone, Default)]
pub struct AnimationParameters {
    pub bools: HashMap<String, bool>,
    pub floats: HashMap<String, f32>,
    pub triggers: Vec<String>,
}

impl AnimationParameters {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_bool(&mut self, name: impl Into<String>, value: bool) {
        self.bools.insert(name.into(), value);
    }

    pub fn get_bool(&self, name: &str) -> bool {
        self.bools.get(name).copied().unwrap_or(false)
    }

    pub fn set_float(&mut self, name: impl Into<String>, value: f32) {
        self.floats.insert(name.into(), value);
    }

    pub fn get_float(&self, name: &str) -> f32 {
        self.floats.get(name).copied().unwrap_or(0.0)
    }

    pub fn set_trigger(&mut self, name: impl Into<String>) {
        self.triggers.push(name.into());
    }

    pub fn consume_trigger(&mut self, name: &str) -> bool {
        if let Some(pos) = self.triggers.iter().position(|t| t == name) {
            self.triggers.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn clear_triggers(&mut self) {
        self.triggers.clear();
    }
}

impl AnimationStateMachine {
    pub fn new(initial_state: impl Into<String>) -> Self {
        Self {
            current_state: initial_state.into(),
            states: HashMap::new(),
            transitions: Vec::new(),
            parameters: AnimationParameters::new(),
            blend_progress: 0.0,
            blend_target: None,
            blend_duration: 0.0,
        }
    }

    /// Add a state to the state machine.
    pub fn add_state(&mut self, state: AnimationState) {
        self.states.insert(state.name.clone(), state);
    }

    /// Add a transition between states.
    pub fn add_transition(&mut self, transition: AnimationTransition) {
        self.transitions.push(transition);
    }

    /// Get the current animation state.
    pub fn current(&self) -> Option<&AnimationState> {
        self.states.get(&self.current_state)
    }

    /// Update the state machine, evaluating transitions.
    ///
    /// Returns the name of the current state (which may have changed).
    pub fn update(&mut self, delta: f32) -> &str {
        // Update blend
        if self.blend_target.is_some() {
            self.blend_progress += delta / self.blend_duration.max(0.001);
            if self.blend_progress >= 1.0 {
                // Blend complete — switch to target
                if let Some(target) = self.blend_target.take() {
                    self.current_state = target;
                }
                self.blend_progress = 0.0;
                self.blend_duration = 0.0;
            }
        }

        // Evaluate transitions (only if not currently blending)
        if self.blend_target.is_none() {
            let transitions: Vec<AnimationTransition> = self.transitions.clone();
            for transition in &transitions {
                if transition.from == self.current_state
                    && self.evaluate_condition(&transition.condition)
                {
                    if transition.blend_duration > 0.0 {
                        // Start blend transition
                        self.blend_target = Some(transition.to.clone());
                        self.blend_duration = transition.blend_duration;
                        self.blend_progress = 0.0;
                    } else {
                        // Instant transition
                        self.current_state = transition.to.clone();
                    }
                    break;
                }
            }
        }

        &self.current_state
    }

    fn evaluate_condition(&self, condition: &TransitionCondition) -> bool {
        match condition {
            TransitionCondition::Always => true,
            TransitionCondition::BoolTrue(name) => self.parameters.get_bool(name),
            TransitionCondition::BoolFalse(name) => !self.parameters.get_bool(name),
            TransitionCondition::FloatGreater(name, threshold) => {
                self.parameters.get_float(name) > *threshold
            }
            TransitionCondition::FloatLess(name, threshold) => {
                self.parameters.get_float(name) < *threshold
            }
            TransitionCondition::Trigger(name) => {
                // Check if trigger was set (non-destructive check)
                self.parameters.triggers.contains(&name.to_string())
            }
        }
    }

    /// Manually trigger a transition.
    pub fn trigger(&mut self, name: &str) {
        self.parameters.set_trigger(name);
    }

    /// Get the blend weight for the current state (1.0 - blend_progress).
    pub fn current_weight(&self) -> f32 {
        if self.blend_target.is_some() {
            1.0 - self.blend_progress
        } else {
            1.0
        }
    }

    /// Get the blend weight for the target state (blend_progress).
    pub fn target_weight(&self) -> f32 {
        if self.blend_target.is_some() {
            self.blend_progress
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_machine_creation() {
        let sm = AnimationStateMachine::new("idle");
        assert_eq!(sm.current_state, "idle");
        assert!(sm.states.is_empty());
    }

    #[test]
    fn test_add_state_and_get() {
        let mut sm = AnimationStateMachine::new("idle");
        sm.add_state(AnimationState::new("idle", "idle_clip"));
        sm.add_state(AnimationState::new("run", "run_clip").with_speed(1.5));

        assert_eq!(sm.current().unwrap().clip_name, "idle_clip");
    }

    #[test]
    fn test_always_transition() {
        let mut sm = AnimationStateMachine::new("idle");
        sm.add_state(AnimationState::new("idle", "idle_clip"));
        sm.add_state(AnimationState::new("run", "run_clip"));
        sm.add_transition(AnimationTransition {
            from: "idle".into(),
            to: "run".into(),
            blend_duration: 0.0,
            condition: TransitionCondition::Always,
        });

        sm.update(0.016);
        assert_eq!(sm.current_state, "run");
    }

    #[test]
    fn test_bool_condition_transition() {
        let mut sm = AnimationStateMachine::new("idle");
        sm.add_state(AnimationState::new("idle", "idle_clip"));
        sm.add_state(AnimationState::new("run", "run_clip"));
        sm.add_transition(AnimationTransition {
            from: "idle".into(),
            to: "run".into(),
            blend_duration: 0.0,
            condition: TransitionCondition::BoolTrue("is_moving".into()),
        });

        // Should not transition yet
        sm.update(0.016);
        assert_eq!(sm.current_state, "idle");

        // Set the parameter
        sm.parameters.set_bool("is_moving", true);
        sm.update(0.016);
        assert_eq!(sm.current_state, "run");
    }

    #[test]
    fn test_float_condition_transition() {
        let mut sm = AnimationStateMachine::new("idle");
        sm.add_state(AnimationState::new("idle", "idle_clip"));
        sm.add_state(AnimationState::new("run", "run_clip"));
        sm.add_transition(AnimationTransition {
            from: "idle".into(),
            to: "run".into(),
            blend_duration: 0.0,
            condition: TransitionCondition::FloatGreater("speed".into(), 0.5),
        });

        sm.update(0.016);
        assert_eq!(sm.current_state, "idle");

        sm.parameters.set_float("speed", 0.8);
        sm.update(0.016);
        assert_eq!(sm.current_state, "run");
    }

    #[test]
    fn test_blend_transition() {
        let mut sm = AnimationStateMachine::new("idle");
        sm.add_state(AnimationState::new("idle", "idle_clip"));
        sm.add_state(AnimationState::new("run", "run_clip"));
        sm.add_transition(AnimationTransition {
            from: "idle".into(),
            to: "run".into(),
            blend_duration: 0.5,
            condition: TransitionCondition::Always,
        });

        // First update: starts blend transition
        sm.update(0.016);
        assert_eq!(sm.blend_target.as_deref(), Some("run"));

        // Second update: blend progresses
        sm.update(0.016);
        assert!(sm.blend_progress > 0.0);
        assert!((sm.current_weight() - (1.0 - sm.blend_progress)).abs() < 1e-6);

        // After enough time, blend should complete
        sm.update(1.0);
        assert_eq!(sm.current_state, "run");
        assert!(sm.blend_target.is_none());
    }

    #[test]
    fn test_trigger_transition() {
        let mut sm = AnimationStateMachine::new("idle");
        sm.add_state(AnimationState::new("idle", "idle_clip"));
        sm.add_state(AnimationState::new("jump", "jump_clip"));
        sm.add_transition(AnimationTransition {
            from: "idle".into(),
            to: "jump".into(),
            blend_duration: 0.0,
            condition: TransitionCondition::Trigger("jump".into()),
        });

        sm.update(0.016);
        assert_eq!(sm.current_state, "idle");

        sm.trigger("jump");
        sm.update(0.016);
        assert_eq!(sm.current_state, "jump");
    }

    #[test]
    fn test_animation_state_builder() {
        let state = AnimationState::new("run", "run_clip")
            .with_speed(2.0)
            .with_looping(false);
        assert!((state.speed - 2.0).abs() < 1e-6);
        assert!(!state.looping);
    }

    #[test]
    fn test_parameters() {
        let mut params = AnimationParameters::new();
        params.set_bool("is_moving", true);
        assert!(params.get_bool("is_moving"));
        assert!(!params.get_bool("is_jumping"));

        params.set_float("speed", 3.5);
        assert!((params.get_float("speed") - 3.5).abs() < 1e-6);

        params.set_trigger("jump");
        assert!(params.consume_trigger("jump"));
        assert!(!params.consume_trigger("jump"));
    }
}
