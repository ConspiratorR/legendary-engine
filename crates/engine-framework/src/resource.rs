pub struct FrameworkResource {
    pub delta_time: f32,
    pub frame_count: u64,
}

impl Default for FrameworkResource {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameworkResource {
    pub fn new() -> Self {
        Self {
            delta_time: 0.0,
            frame_count: 0,
        }
    }
}
