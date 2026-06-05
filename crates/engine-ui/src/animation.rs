//! UI animation system with easing functions, tweening, and gesture recognition.
//!
//! Provides [`Easing`] curves, [`Tween`] for value interpolation,
//! [`UiAnimation`] for widget property animation, and [`GestureRecognizer`]
//! for detecting swipe/pinch/tap gestures.

use std::collections::HashMap;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Easing functions
// ---------------------------------------------------------------------------

/// Easing curve types for animation interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Easing {
    /// Linear interpolation (no easing).
    #[default]
    Linear,
    /// Ease-in (slow start).
    EaseIn,
    /// Ease-out (slow end).
    EaseOut,
    /// Ease-in-out (slow start and end).
    EaseInOut,
    /// Ease-in cubic.
    EaseInCubic,
    /// Ease-out cubic.
    EaseOutCubic,
    /// Ease-in-out cubic.
    EaseInOutCubic,
    /// Spring-like overshoot.
    BackIn,
    /// Spring-like settle.
    BackOut,
    /// Bounce effect at the end.
    BounceOut,
}

impl Easing {
    /// Apply the easing function to a normalized `t` value (0.0..=1.0).
    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::EaseIn => t * t,
            Easing::EaseOut => t * (2.0 - t),
            Easing::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
            Easing::EaseInCubic => t * t * t,
            Easing::EaseOutCubic => {
                let t1 = t - 1.0;
                t1 * t1 * t1 + 1.0
            }
            Easing::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    let t1 = 2.0 * t - 2.0;
                    0.5 * t1 * t1 * t1 + 1.0
                }
            }
            Easing::BackIn => {
                let s = 1.70158;
                t * t * ((s + 1.0) * t - s)
            }
            Easing::BackOut => {
                let s = 1.70158;
                let t1 = t - 1.0;
                t1 * t1 * ((s + 1.0) * t1 + s) + 1.0
            }
            Easing::BounceOut => {
                if t < 1.0 / 2.75 {
                    7.5625 * t * t
                } else if t < 2.0 / 2.75 {
                    let t2 = t - 1.5 / 2.75;
                    7.5625 * t2 * t2 + 0.75
                } else if t < 2.5 / 2.75 {
                    let t2 = t - 2.25 / 2.75;
                    7.5625 * t2 * t2 + 0.9375
                } else {
                    let t2 = t - 2.625 / 2.75;
                    7.5625 * t2 * t2 + 0.984375
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tween
// ---------------------------------------------------------------------------

/// A value interpolation between `start` and `end` over a duration.
#[derive(Debug, Clone)]
pub struct Tween {
    /// Start value.
    pub start: f32,
    /// End value.
    pub end: f32,
    /// Total duration.
    pub duration: Duration,
    /// Elapsed time.
    pub elapsed: Duration,
    /// Easing curve.
    pub easing: Easing,
    /// Whether the tween is currently running.
    pub running: bool,
    /// Whether to loop the animation.
    pub looping: bool,
    /// Whether to reverse direction on each loop (ping-pong).
    pub ping_pong: bool,
    /// Internal: direction for ping-pong (true = forward).
    forward: bool,
}

impl Tween {
    /// Create a new tween from `start` to `end` over `duration`.
    pub fn new(start: f32, end: f32, duration: Duration) -> Self {
        Self {
            start,
            end,
            duration,
            elapsed: Duration::ZERO,
            easing: Easing::default(),
            running: true,
            looping: false,
            ping_pong: false,
            forward: true,
        }
    }

    /// Set the easing curve.
    pub fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// Enable looping.
    pub fn with_looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    /// Enable ping-pong (reverse on each loop).
    pub fn with_ping_pong(mut self, ping_pong: bool) -> Self {
        self.ping_pong = ping_pong;
        self
    }

    /// Advance the tween by `dt` and return the current interpolated value.
    pub fn tick(&mut self, dt: Duration) -> f32 {
        if !self.running {
            return self.current_value();
        }

        self.elapsed += dt;

        let total = self.duration.as_secs_f32();
        if total <= 0.0 {
            self.running = false;
            return if self.forward { self.end } else { self.start };
        }

        let mut t = self.elapsed.as_secs_f32() / total;

        if t >= 1.0 {
            if self.looping {
                if self.ping_pong {
                    self.forward = !self.forward;
                }
                self.elapsed = Duration::from_secs_f32(t - 1.0);
                t -= 1.0;
            } else {
                self.running = false;
                t = 1.0;
            }
        }

        let eased = self.easing.apply(t);
        if self.forward {
            self.start + (self.end - self.start) * eased
        } else {
            self.end - (self.end - self.start) * eased
        }
    }

    /// Get the current value without advancing.
    pub fn current_value(&self) -> f32 {
        let total = self.duration.as_secs_f32();
        if total <= 0.0 {
            return if self.forward { self.end } else { self.start };
        }
        let t = (self.elapsed.as_secs_f32() / total).clamp(0.0, 1.0);
        let eased = self.easing.apply(t);
        if self.forward {
            self.start + (self.end - self.start) * eased
        } else {
            self.end - (self.end - self.start) * eased
        }
    }

    /// Whether the tween has finished (not running and not looping).
    pub fn is_finished(&self) -> bool {
        !self.running
    }

    /// Reset the tween to the beginning.
    pub fn reset(&mut self) {
        self.elapsed = Duration::ZERO;
        self.running = true;
        self.forward = true;
    }
}

// ---------------------------------------------------------------------------
// UI Animation (per-widget property animation)
// ---------------------------------------------------------------------------

/// Animatable properties of a widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimProperty {
    /// Opacity (0.0..=1.0).
    Opacity,
    /// Horizontal position offset.
    OffsetX,
    /// Vertical position offset.
    OffsetY,
    /// Scale factor.
    Scale,
    /// Rotation in degrees.
    Rotation,
}

/// A running animation targeting a specific widget property.
#[derive(Debug, Clone)]
pub struct UiAnimation {
    /// The widget being animated.
    pub widget_id: u64,
    /// The property being animated.
    pub property: AnimProperty,
    /// The underlying tween.
    pub tween: Tween,
}

impl UiAnimation {
    pub fn new(widget_id: u64, property: AnimProperty, tween: Tween) -> Self {
        Self {
            widget_id,
            property,
            tween,
        }
    }
}

// ---------------------------------------------------------------------------
// Animation manager
// ---------------------------------------------------------------------------

/// Manages all active UI animations.
pub struct AnimationManager {
    animations: Vec<UiAnimation>,
}

impl AnimationManager {
    pub fn new() -> Self {
        Self {
            animations: Vec::new(),
        }
    }

    /// Start a new animation.
    pub fn animate(&mut self, animation: UiAnimation) {
        self.animations.push(animation);
    }

    /// Tick all animations by `dt`, returning a map of (widget_id, property) → value
    /// for all changed properties.
    pub fn tick(&mut self, dt: Duration) -> HashMap<(u64, AnimProperty), f32> {
        let mut changes = HashMap::new();
        for anim in &mut self.animations {
            let value = anim.tween.tick(dt);
            changes.insert((anim.widget_id, anim.property), value);
        }
        // Remove finished non-looping animations.
        self.animations.retain(|a| !a.tween.is_finished());
        changes
    }

    /// Number of active animations.
    pub fn len(&self) -> usize {
        self.animations.len()
    }

    /// Whether there are no active animations.
    pub fn is_empty(&self) -> bool {
        self.animations.is_empty()
    }

    /// Cancel all animations for a specific widget.
    pub fn cancel_for_widget(&mut self, widget_id: u64) {
        self.animations.retain(|a| a.widget_id != widget_id);
    }

    /// Cancel all animations.
    pub fn cancel_all(&mut self) {
        self.animations.clear();
    }
}

impl Default for AnimationManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Gesture recognizer
// ---------------------------------------------------------------------------

/// Types of gestures that can be recognized.
#[derive(Debug, Clone, PartialEq)]
pub enum Gesture {
    /// A quick tap (short duration, minimal movement).
    Tap { x: f32, y: f32 },
    /// A long press (held for > threshold).
    LongPress { x: f32, y: f32 },
    /// A swipe in a direction.
    Swipe { dx: f32, dy: f32, velocity: f32 },
    /// A pinch gesture (two fingers).
    Pinch { scale: f32 },
}

/// Configuration for gesture recognition thresholds.
#[derive(Debug, Clone)]
pub struct GestureConfig {
    /// Maximum duration for a tap (ms).
    pub tap_max_duration_ms: u64,
    /// Maximum movement for a tap (points).
    pub tap_max_distance: f32,
    /// Minimum duration for a long press (ms).
    pub long_press_min_duration_ms: u64,
    /// Minimum distance for a swipe (points).
    pub swipe_min_distance: f32,
    /// Minimum velocity for a swipe (points/sec).
    pub swipe_min_velocity: f32,
}

impl Default for GestureConfig {
    fn default() -> Self {
        Self {
            tap_max_duration_ms: 300,
            tap_max_distance: 10.0,
            long_press_min_duration_ms: 500,
            swipe_min_distance: 50.0,
            swipe_min_velocity: 200.0,
        }
    }
}

/// Tracks touch/mouse input and recognizes gestures.
pub struct GestureRecognizer {
    config: GestureConfig,
    /// Start position of the current touch/click.
    start_pos: Option<(f32, f32)>,
    /// Current position.
    current_pos: Option<(f32, f32)>,
    /// Accumulated time for long press detection.
    elapsed: Duration,
    /// Recognized gesture (consumed by the caller).
    pending_gesture: Option<Gesture>,
}

impl GestureRecognizer {
    pub fn new(config: GestureConfig) -> Self {
        Self {
            config,
            start_pos: None,
            current_pos: None,
            elapsed: Duration::ZERO,
            pending_gesture: None,
        }
    }

    /// Called when a touch/click begins.
    pub fn on_pointer_down(&mut self, x: f32, y: f32) {
        self.start_pos = Some((x, y));
        self.current_pos = Some((x, y));
        self.elapsed = Duration::ZERO;
        self.pending_gesture = None;
    }

    /// Called when the pointer moves.
    pub fn on_pointer_move(&mut self, x: f32, y: f32) {
        self.current_pos = Some((x, y));
    }

    /// Called when the touch/click ends.
    pub fn on_pointer_up(&mut self) {
        let (start, current) = match (self.start_pos, self.current_pos) {
            (Some(s), Some(c)) => (s, c),
            _ => return,
        };

        let dx = current.0 - start.0;
        let dy = current.1 - start.1;
        let distance = (dx * dx + dy * dy).sqrt();
        let duration_ms = self.elapsed.as_millis() as u64;

        if distance <= self.config.tap_max_distance
            && duration_ms <= self.config.tap_max_duration_ms
        {
            self.pending_gesture = Some(Gesture::Tap {
                x: current.0,
                y: current.1,
            });
        } else if distance >= self.config.swipe_min_distance {
            let duration_secs = self.elapsed.as_secs_f32().max(0.001);
            let velocity = distance / duration_secs;
            if velocity >= self.config.swipe_min_velocity {
                self.pending_gesture = Some(Gesture::Swipe { dx, dy, velocity });
            }
        }

        self.start_pos = None;
        self.current_pos = None;
    }

    /// Tick the recognizer (call each frame) to detect long presses.
    pub fn tick(&mut self, dt: Duration) {
        if self.start_pos.is_none() {
            return;
        }
        self.elapsed += dt;

        if self.elapsed.as_millis() as u64 >= self.config.long_press_min_duration_ms
            && self.pending_gesture.is_none()
        {
            let (x, y) = self
                .start_pos
                .expect("start_pos must be Some when elapsed exceeds threshold");
            self.pending_gesture = Some(Gesture::LongPress { x, y });
            // Clear to prevent re-triggering.
            self.start_pos = None;
        }
    }

    /// Consume the pending gesture, if any.
    pub fn take_gesture(&mut self) -> Option<Gesture> {
        self.pending_gesture.take()
    }

    /// Whether a gesture is pending.
    pub fn has_gesture(&self) -> bool {
        self.pending_gesture.is_some()
    }
}

impl Default for GestureRecognizer {
    fn default() -> Self {
        Self::new(GestureConfig::default())
    }
}

// ---------------------------------------------------------------------------
// Transition helper (for theme/opacity transitions)
// ---------------------------------------------------------------------------

/// A simple 0→1 transition that can be driven each frame.
#[derive(Debug, Clone)]
pub struct Transition {
    pub progress: f32,
    pub duration_secs: f32,
    pub easing: Easing,
    running: bool,
}

impl Transition {
    pub fn new(duration_secs: f32) -> Self {
        Self {
            progress: 0.0,
            duration_secs,
            easing: Easing::EaseInOut,
            running: true,
        }
    }

    /// Advance by `dt` seconds. Returns the eased progress (0.0..=1.0).
    pub fn tick(&mut self, dt: f32) -> f32 {
        if !self.running {
            return self.easing.apply(self.progress);
        }
        if self.duration_secs > 0.0 {
            self.progress = (self.progress + dt / self.duration_secs).min(1.0);
        } else {
            self.progress = 1.0;
        }
        if self.progress >= 1.0 {
            self.running = false;
        }
        self.easing.apply(self.progress)
    }

    pub fn is_complete(&self) -> bool {
        self.progress >= 1.0
    }

    pub fn reset(&mut self) {
        self.progress = 0.0;
        self.running = true;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Easing tests -------------------------------------------------------

    #[test]
    fn test_easing_linear_endpoints() {
        assert!((Easing::Linear.apply(0.0) - 0.0).abs() < f32::EPSILON);
        assert!((Easing::Linear.apply(1.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_easing_clamps_input() {
        assert!((Easing::Linear.apply(-0.5) - 0.0).abs() < f32::EPSILON);
        assert!((Easing::Linear.apply(1.5) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_ease_in_starts_slow() {
        let early = Easing::EaseIn.apply(0.25);
        let linear = Easing::Linear.apply(0.25);
        assert!(early < linear, "ease-in should be below linear early");
    }

    #[test]
    fn test_ease_out_ends_slow() {
        let late = Easing::EaseOut.apply(0.75);
        let linear = Easing::Linear.apply(0.75);
        assert!(late > linear, "ease-out should be above linear late");
    }

    #[test]
    fn test_bounce_out_endpoints() {
        assert!((Easing::BounceOut.apply(0.0) - 0.0).abs() < f32::EPSILON);
        assert!((Easing::BounceOut.apply(1.0) - 1.0).abs() < f32::EPSILON);
    }

    // -- Tween tests --------------------------------------------------------

    #[test]
    fn test_tween_basic_interpolation() {
        let mut tween = Tween::new(0.0, 100.0, Duration::from_secs(1));
        let v = tween.tick(Duration::from_millis(500));
        assert!((v - 50.0).abs() < 1.0, "halfway should be ~50, got {v}");
    }

    #[test]
    fn test_tween_finishes() {
        let mut tween = Tween::new(0.0, 10.0, Duration::from_millis(100));
        tween.tick(Duration::from_millis(200));
        assert!(tween.is_finished());
    }

    #[test]
    fn test_tween_looping() {
        let mut tween = Tween::new(0.0, 100.0, Duration::from_millis(100)).with_looping(true);
        // Tick past the end — should wrap around.
        let _ = tween.tick(Duration::from_millis(150));
        assert!(!tween.is_finished(), "looping tween should not finish");
    }

    #[test]
    fn test_tween_ping_pong() {
        let mut tween = Tween::new(0.0, 100.0, Duration::from_millis(100))
            .with_looping(true)
            .with_ping_pong(true);
        // First half: forward.
        let v1 = tween.tick(Duration::from_millis(100));
        assert!((v1 - 100.0).abs() < 2.0, "should reach end, got {v1}");
        // Second half: reversed.
        let v2 = tween.tick(Duration::from_millis(100));
        assert!(
            (v2 - 0.0).abs() < 2.0,
            "ping-pong should return to start, got {v2}"
        );
    }

    #[test]
    fn test_tween_reset() {
        let mut tween = Tween::new(0.0, 100.0, Duration::from_millis(100));
        tween.tick(Duration::from_millis(200));
        assert!(tween.is_finished());
        tween.reset();
        assert!(!tween.is_finished());
        let v = tween.current_value();
        assert!((v - 0.0).abs() < f32::EPSILON);
    }

    // -- AnimationManager tests ---------------------------------------------

    #[test]
    fn test_animation_manager_tick() {
        let mut mgr = AnimationManager::new();
        mgr.animate(UiAnimation::new(
            1,
            AnimProperty::Opacity,
            Tween::new(0.0, 1.0, Duration::from_millis(100)),
        ));
        assert_eq!(mgr.len(), 1);

        let changes = mgr.tick(Duration::from_millis(50));
        assert!(changes.contains_key(&(1, AnimProperty::Opacity)));
    }

    #[test]
    fn test_animation_manager_removes_finished() {
        let mut mgr = AnimationManager::new();
        mgr.animate(UiAnimation::new(
            1,
            AnimProperty::Opacity,
            Tween::new(0.0, 1.0, Duration::from_millis(100)),
        ));
        mgr.tick(Duration::from_millis(200));
        assert!(mgr.is_empty(), "finished animation should be removed");
    }

    #[test]
    fn test_animation_manager_cancel_for_widget() {
        let mut mgr = AnimationManager::new();
        mgr.animate(UiAnimation::new(
            1,
            AnimProperty::Opacity,
            Tween::new(0.0, 1.0, Duration::from_secs(10)),
        ));
        mgr.animate(UiAnimation::new(
            2,
            AnimProperty::Scale,
            Tween::new(1.0, 2.0, Duration::from_secs(10)),
        ));
        mgr.cancel_for_widget(1);
        assert_eq!(mgr.len(), 1);
    }

    // -- GestureRecognizer tests --------------------------------------------

    #[test]
    fn test_gesture_tap() {
        let mut gr = GestureRecognizer::default();
        gr.on_pointer_down(100.0, 100.0);
        gr.tick(Duration::from_millis(50));
        gr.on_pointer_up();
        let gesture = gr.take_gesture();
        assert!(
            matches!(gesture, Some(Gesture::Tap { .. })),
            "expected tap, got {gesture:?}"
        );
    }

    #[test]
    fn test_gesture_long_press() {
        let mut gr = GestureRecognizer::default();
        gr.on_pointer_down(100.0, 100.0);
        gr.tick(Duration::from_millis(600));
        let gesture = gr.take_gesture();
        assert!(
            matches!(gesture, Some(Gesture::LongPress { .. })),
            "expected long press, got {gesture:?}"
        );
    }

    #[test]
    fn test_gesture_swipe() {
        let mut gr = GestureRecognizer::default();
        gr.on_pointer_down(100.0, 100.0);
        gr.on_pointer_move(200.0, 100.0);
        gr.tick(Duration::from_millis(100));
        gr.on_pointer_up();
        let gesture = gr.take_gesture();
        assert!(
            matches!(gesture, Some(Gesture::Swipe { .. })),
            "expected swipe, got {gesture:?}"
        );
    }

    #[test]
    fn test_gesture_no_tap_if_too_far() {
        let mut gr = GestureRecognizer::default();
        gr.on_pointer_down(100.0, 100.0);
        gr.on_pointer_move(200.0, 200.0);
        gr.tick(Duration::from_millis(50));
        gr.on_pointer_up();
        let gesture = gr.take_gesture();
        // Should be a swipe, not a tap.
        assert!(!matches!(gesture, Some(Gesture::Tap { .. })));
    }

    // -- Transition tests ---------------------------------------------------

    #[test]
    fn test_transition_progress() {
        let mut t = Transition::new(1.0);
        let eased = t.tick(0.5);
        assert!(eased > 0.0 && eased < 1.0);
        assert!(!t.is_complete());
    }

    #[test]
    fn test_transition_completes() {
        let mut t = Transition::new(0.5);
        t.tick(1.0);
        assert!(t.is_complete());
    }

    #[test]
    fn test_transition_reset() {
        let mut t = Transition::new(0.5);
        t.tick(1.0);
        assert!(t.is_complete());
        t.reset();
        assert!(!t.is_complete());
        assert!((t.progress - 0.0).abs() < f32::EPSILON);
    }
}
