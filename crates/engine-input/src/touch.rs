//! Touch input support for Android and touch-enabled devices.

/// Touch input phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchPhase {
    /// A new touch point has been created.
    Started,
    /// An existing touch point has moved.
    Moved,
    /// A touch point has been released.
    Ended,
    /// A touch point has been cancelled (e.g., by the system).
    Cancelled,
}

/// A single touch point.
#[derive(Debug, Clone, Copy)]
pub struct TouchPoint {
    /// Unique identifier for this touch point.
    pub id: u64,
    /// X coordinate in logical pixels.
    pub x: f32,
    /// Y coordinate in logical pixels.
    pub y: f32,
    /// Current phase of this touch point.
    pub phase: TouchPhase,
}

/// Touch input state for the current frame.
#[derive(Debug, Default)]
pub struct TouchState {
    /// Active touch points for this frame.
    pub points: Vec<TouchPoint>,
}

impl TouchState {
    /// Check if any touch is active (Started or Moved).
    pub fn is_touching(&self) -> bool {
        self.points
            .iter()
            .any(|p| matches!(p.phase, TouchPhase::Started | TouchPhase::Moved))
    }

    /// Get the first active touch point, if any.
    pub fn primary_touch(&self) -> Option<&TouchPoint> {
        self.points
            .iter()
            .find(|p| matches!(p.phase, TouchPhase::Started | TouchPhase::Moved))
    }

    /// Get all touch points with a specific phase.
    pub fn touches_with_phase(&self, phase: TouchPhase) -> Vec<&TouchPoint> {
        self.points.iter().filter(|p| p.phase == phase).collect()
    }

    /// Clear all touch points (called at end of frame).
    pub fn clear(&mut self) {
        self.points.clear();
    }
}
