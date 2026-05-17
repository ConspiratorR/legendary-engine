use std::time::Duration;

/// Time resource, tracking delta time and total elapsed time.
#[derive(Debug, Clone)]
pub struct Time {
    delta_time: Duration,
    delta_seconds: f32,
    elapsed: Duration,
    elapsed_seconds: f32,
    frame_count: u64,
    last_frame_time: std::time::Instant,
}

impl Default for Time {
    fn default() -> Self {
        Self::new()
    }
}

impl Time {
    /// Creates a new Time instance.
    pub fn new() -> Self {
        Self {
            delta_time: Duration::from_secs_f32(0.016), // 60fps default
            delta_seconds: 0.016,
            elapsed: Duration::default(),
            elapsed_seconds: 0.0,
            frame_count: 0,
            last_frame_time: std::time::Instant::now(),
        }
    }

    /// Updates the time state, should be called once per frame.
    pub fn update(&mut self) {
        let now = std::time::Instant::now();
        self.delta_time = now - self.last_frame_time;
        self.delta_seconds = self.delta_time.as_secs_f32();
        self.elapsed += self.delta_time;
        self.elapsed_seconds = self.elapsed.as_secs_f32();
        self.frame_count += 1;
        self.last_frame_time = now;
    }

    /// Gets the time since the last frame as a Duration.
    pub fn delta(&self) -> Duration {
        self.delta_time
    }

    /// Gets the time since the last frame in seconds.
    pub fn delta_seconds(&self) -> f32 {
        self.delta_seconds
    }

    /// Gets the total time elapsed since the Time was created.
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Gets the total elapsed time in seconds.
    pub fn elapsed_seconds(&self) -> f32 {
        self.elapsed_seconds
    }

    /// Gets the current frame count.
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Gets the current frames per second.
    pub fn fps(&self) -> f32 {
        if self.delta_seconds > 0.0 {
            1.0 / self.delta_seconds
        } else {
            0.0
        }
    }
}
