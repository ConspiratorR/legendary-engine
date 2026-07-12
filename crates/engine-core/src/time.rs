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
    /// Maximum allowed delta time (maximumDeltaTime).
    max_delta_time: f32,
    /// Last frame time for internal delta calculation.
    last_frame_time: std::time::Instant,
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
            max_delta_time: 0.33333334, // ~3 FPS minimum
            last_frame_time: std::time::Instant::now(),
        }
    }
}

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

    /// Get time since last fixed update (like Unity: Time.fixedUnscaledTime).
    pub fn fixedUnscaledTime(&self) -> f32 {
        self.elapsed_time
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

    /// Get delta time for the current step (fixed or regular).
    pub fn stepDeltaTime(&self) -> f32 {
        if self.in_fixed_update {
            self.fixed_delta_time
        } else {
            self.deltaTime()
        }
    }

    // Internal methods for updating time

    /// Update time for a new frame (called by engine).
    pub fn update(&mut self, delta: f32) {
        self.delta_time = delta.min(self.max_delta_time);
        self.elapsed_time += self.deltaTime();
        self.frame_count += 1;
        self.in_fixed_update = false;
    }

    /// Update time for a fixed update step (called by engine).
    pub fn update_fixed(&mut self) {
        self.in_fixed_update = true;
    }

    /// Reset time (for new level, etc.).
    pub fn reset(&mut self) {
        self.delta_time = 0.0;
        self.elapsed_time = 0.0;
        self.frame_count = 0;
        self.in_fixed_update = false;
        self.last_frame_time = std::time::Instant::now();
    }

    /// Update time using internal clock (convenience method for plugins).
    /// Computes delta from last frame time and calls update().
    pub fn update_with_internal_clock(&mut self) {
        let now = std::time::Instant::now();
        let delta = (now - self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        self.update(delta);
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
    
    /// Get delta time in seconds (alias for delta_seconds).
    pub fn delta(&self) -> f32 {
        self.delta_seconds()
    }
    
    /// Get elapsed time in seconds (alias for elapsed_seconds).
    pub fn elapsed(&self) -> f32 {
        self.elapsed_seconds()
    }
    
    /// Get current frames per second.
    pub fn fps(&self) -> f32 {
        if self.delta_time > 0.0 {
            1.0 / self.delta_time
        } else {
            0.0
        }
    }
    
    /// Update time using internal clock (no arguments, for backward compatibility).
    pub fn update_no_args(&mut self) {
        self.update_with_internal_clock();
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
    fn test_time_reset() {
        let mut time = Time::default();
        time.update(0.016);
        time.update(0.016);

        time.reset();
        assert_eq!(time.time(), 0.0);
        assert_eq!(time.frameCount(), 0);
    }
}
