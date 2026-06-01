/// Framework-level resource tracking delta time and frame count.
pub struct FrameworkResource {
    /// Time elapsed since the last frame in seconds.
    pub delta_time: f32,
    /// Total number of frames elapsed.
    pub frame_count: u64,
}

impl Default for FrameworkResource {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameworkResource {
    /// Create a new framework resource with zeroed values.
    pub fn new() -> Self {
        Self {
            delta_time: 0.0,
            frame_count: 0,
        }
    }
}
