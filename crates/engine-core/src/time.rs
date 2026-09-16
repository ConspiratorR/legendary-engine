/// Time information (like Unity's Time class).
#[derive(Debug, Clone)]
pub struct Time {
    /// Time since last frame (deltaTime).
    delta_time: f32,
    /// Total time since application start (time).
    elapsed_time: f32,
    /// Fixed timestep (fixedDeltaTime).
    fixed_delta_time: f32,
    /// Time scale (timeScale).
    time_scale: f32,
    /// Frame count (frameCount).
    frame_count: u64,
    /// Whether we're in FixedUpdate.
    in_fixed_update: bool,
    /// Whether any FixedUpdate step ran since the last `Time::update`.
    /// Used by WaitForFixedUpdate to resume at end-of-frame.
    fixed_steps_ran: bool,
    /// Maximum allowed delta time (maximumDeltaTime).
    max_delta_time: f32,
    /// Time at the last FixedUpdate step (for fixedUnscaledTime).
    last_fixed_time: f32,
    /// Accumulator for fixed-timestep FixedUpdate (Unity PlayerLoop).
    fixed_accumulator: f32,
    /// Maximum FixedUpdate steps per rendered frame (spiral-of-death guard).
    max_fixed_steps: u32,
}

impl Default for Time {
    fn default() -> Self {
        Self {
            delta_time: 0.0,
            elapsed_time: 0.0,
            fixed_delta_time: 0.02, // 50 Hz
            time_scale: 1.0,
            frame_count: 0,
            in_fixed_update: false,
            fixed_steps_ran: false,
            max_delta_time: 0.33333334, // ~3 FPS minimum
            last_fixed_time: 0.0,
            fixed_accumulator: 0.0,
            max_fixed_steps: 8,
        }
    }
}

#[allow(non_snake_case)]
impl Time {
    /// Create a new Time with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get delta time (scaled by timeScale) (like Unity: Time.deltaTime).
    pub fn deltaTime(&self) -> f32 {
        self.delta_time * self.time_scale
    }

    /// Get unscaled delta time (like Unity: Time.unscaledDeltaTime).
    pub fn unscaledDeltaTime(&self) -> f32 {
        self.delta_time
    }

    /// Get fixed delta time (like Unity: Time.fixedDeltaTime).
    pub fn fixedDeltaTime(&self) -> f32 {
        self.fixed_delta_time
    }

    /// Get total elapsed time (like Unity: Time.time).
    pub fn time(&self) -> f32 {
        self.elapsed_time
    }

    /// Get unscaled total time (like Unity: Time.unscaledTime).
    pub fn unscaledTime(&self) -> f32 {
        self.elapsed_time
    }

    /// Get time at the last FixedUpdate step (like Unity: Time.fixedUnscaledTime).
    pub fn fixedUnscaledTime(&self) -> f32 {
        self.last_fixed_time
    }

    /// Get frame count (like Unity: Time.frameCount).
    pub fn frameCount(&self) -> u64 {
        self.frame_count
    }

    /// Get time scale (like Unity: Time.timeScale).
    pub fn timeScale(&self) -> f32 {
        self.time_scale
    }

    /// Set time scale (like Unity: Time.timeScale).
    pub fn set_timeScale(&mut self, scale: f32) {
        self.time_scale = scale.clamp(0.0, 100.0);
    }

    /// Get maximum delta time (like Unity: Time.maximumDeltaTime).
    pub fn maximumDeltaTime(&self) -> f32 {
        self.max_delta_time
    }

    /// Set maximum delta time.
    pub fn set_maximumDeltaTime(&mut self, max: f32) {
        self.max_delta_time = max.max(0.0);
    }

    /// Check if we're in FixedUpdate (like Unity: Time.inFixedTimeStep).
    pub fn inFixedTimeStep(&self) -> bool {
        self.in_fixed_update
    }

    /// Whether any FixedUpdate step ran this frame (after `begin_fixed_update` / `end_fixed_update`).
    pub fn fixed_steps_ran(&self) -> bool {
        self.fixed_steps_ran
    }

    /// Get delta time for the current step (fixed or regular).
    pub fn stepDeltaTime(&self) -> f32 {
        if self.in_fixed_update {
            self.fixed_delta_time
        } else {
            self.deltaTime()
        }
    }

    /// Update time for a new frame (called by engine).
    ///
    /// Also advances the FixedUpdate accumulator by the scaled delta, matching
    /// Unity's PlayerLoop: FixedUpdate runs 0+ times per rendered frame.
    pub fn update(&mut self, delta: f32) {
        self.delta_time = delta.min(self.max_delta_time);
        self.elapsed_time += self.deltaTime();
        self.frame_count += 1;
        self.in_fixed_update = false;
        self.fixed_steps_ran = false;
        // Accumulate scaled time for FixedUpdate (Unity uses Time.deltaTime).
        self.fixed_accumulator += self.deltaTime();
    }

    /// How many FixedUpdate steps should run this frame (Unity: 0+ times).
    ///
    /// Does not consume the accumulator — call [`Time::begin_fixed_update`] /
    /// [`Time::end_fixed_update`] around each step.
    pub fn pending_fixed_steps(&self) -> u32 {
        let step = self.fixed_delta_time.max(f32::EPSILON);
        let steps = (self.fixed_accumulator / step).floor() as i64;
        steps.clamp(0, self.max_fixed_steps as i64) as u32
    }

    /// Enter FixedUpdate for one fixed step (called by engine before FixedUpdate).
    pub fn begin_fixed_update(&mut self) {
        self.in_fixed_update = true;
        self.last_fixed_time = self.elapsed_time;
    }

    /// Leave FixedUpdate and consume one fixed step from the accumulator.
    pub fn end_fixed_update(&mut self) {
        self.fixed_accumulator = (self.fixed_accumulator - self.fixed_delta_time).max(0.0);
        self.in_fixed_update = false;
        self.fixed_steps_ran = true;
    }

    /// Update time for a fixed update step (called by engine).
    pub fn update_fixed(&mut self) {
        self.begin_fixed_update();
    }

    /// Get current FixedUpdate accumulator remaining (debug/diagnostics).
    pub fn fixed_accumulator(&self) -> f32 {
        self.fixed_accumulator
    }

    /// Set maximum FixedUpdate steps per rendered frame (default 8).
    pub fn set_max_fixed_steps(&mut self, max: u32) {
        self.max_fixed_steps = max.max(1);
    }

    /// Get maximum FixedUpdate steps per rendered frame.
    pub fn max_fixed_steps(&self) -> u32 {
        self.max_fixed_steps
    }

    /// Reset time (for new level, etc.).
    pub fn reset(&mut self) {
        self.delta_time = 0.0;
        self.elapsed_time = 0.0;
        self.frame_count = 0;
        self.in_fixed_update = false;
        self.fixed_steps_ran = false;
        self.last_fixed_time = 0.0;
        self.fixed_accumulator = 0.0;
    }

    // Backward-compatible snake_case methods for existing code

    /// Get delta time in seconds (snake_case alias for deltaTime).
    pub fn delta_seconds(&self) -> f32 {
        self.deltaTime()
    }

    /// Get elapsed time in seconds (snake_case alias for time).
    pub fn elapsed_seconds(&self) -> f32 {
        self.time()
    }

    /// Get frame count (snake_case alias for frameCount).
    pub fn frame_count(&self) -> u64 {
        self.frameCount()
    }

    /// Get time scale (snake_case alias for timeScale).
    pub fn time_scale(&self) -> f32 {
        self.timeScale()
    }

    /// Set time scale (snake_case alias for set_timeScale).
    pub fn set_time_scale(&mut self, scale: f32) {
        self.set_timeScale(scale);
    }

    /// Get maximum delta time (snake_case alias for maximumDeltaTime).
    pub fn maximum_delta_time(&self) -> f32 {
        self.maximumDeltaTime()
    }

    /// Set maximum delta time (snake_case alias for set_maximumDeltaTime).
    pub fn set_maximum_delta_time(&mut self, max: f32) {
        self.set_maximumDeltaTime(max);
    }

    /// Check if we're in FixedUpdate (snake_case alias for inFixedTimeStep).
    pub fn in_fixed_time_step(&self) -> bool {
        self.inFixedTimeStep()
    }

    /// Get delta time for the current step (snake_case alias for stepDeltaTime).
    pub fn step_delta_time(&self) -> f32 {
        self.stepDeltaTime()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_default() {
        let time = Time::default();
        assert_eq!(time.deltaTime(), 0.0);
        assert_eq!(time.time(), 0.0);
        assert_eq!(time.timeScale(), 1.0);
        assert_eq!(time.frameCount(), 0);
    }

    #[test]
    fn test_time_update() {
        let mut time = Time::default();

        time.update(0.016); // 60 FPS
        assert_eq!(time.deltaTime(), 0.016);
        assert_eq!(time.time(), 0.016);
        assert_eq!(time.frameCount(), 1);

        time.update(0.016);
        assert_eq!(time.time(), 0.032);
        assert_eq!(time.frameCount(), 2);
    }

    #[test]
    fn test_time_scale() {
        let mut time = Time::default();
        time.set_timeScale(0.5);

        time.update(0.016);
        assert_eq!(time.deltaTime(), 0.008); // Scaled
        assert_eq!(time.unscaledDeltaTime(), 0.016); // Unscaled
    }

    #[test]
    fn test_time_max_delta() {
        let mut time = Time::default();
        time.set_maximumDeltaTime(0.1);

        time.update(1.0); // Very large delta
        assert_eq!(time.deltaTime(), 0.1); // Clamped
    }

    #[test]
    fn test_time_fixed_update() {
        let mut time = Time::default();

        time.update(0.016);
        assert!(!time.inFixedTimeStep());

        time.update_fixed();
        assert!(time.inFixedTimeStep());
        assert_eq!(time.stepDeltaTime(), time.fixedDeltaTime());
    }

    #[test]
    fn test_fixed_accumulator_zero_or_more_steps() {
        let mut time = Time::default(); // fixed_delta = 0.02

        // Small frame: 0.01s scaled → 0 FixedUpdate steps
        time.update(0.01);
        assert_eq!(time.pending_fixed_steps(), 0);

        // Another small frame: accumulator = 0.02 → 1 step
        time.update(0.01);
        assert_eq!(time.pending_fixed_steps(), 1);

        time.begin_fixed_update();
        time.end_fixed_update();
        assert!((time.fixed_accumulator() - 0.0).abs() < 1e-6);

        // Large frame: 0.05 → 2 steps (0.05 / 0.02 = 2.5 → 2)
        time.update(0.05);
        assert_eq!(time.pending_fixed_steps(), 2);
    }

    #[test]
    fn test_fixed_accumulator_capped() {
        let mut time = Time::default();
        time.set_max_fixed_steps(3);
        // Huge hitch clamped by maximumDeltaTime first (0.333), then step cap
        time.update(10.0);
        assert!(time.pending_fixed_steps() <= 3);
    }

    #[test]
    fn test_fixed_accumulator_time_scale() {
        let mut time = Time::default();
        time.set_timeScale(0.0);
        time.update(0.05);
        // Paused: scaled delta is 0, no FixedUpdate steps
        assert_eq!(time.pending_fixed_steps(), 0);
    }

    #[test]
    fn test_time_reset() {
        let mut time = Time::default();
        time.update(0.016);
        time.update(0.016);

        time.reset();
        assert_eq!(time.time(), 0.0);
        assert_eq!(time.frameCount(), 0);
    }
}
