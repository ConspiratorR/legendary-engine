/// Snapshot of the mouse cursor and button state for the current frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseState {
    /// Cursor position in window coordinates `(x, y)`.
    pub position: (f64, f64),
    /// Cursor movement delta since the last frame `(dx, dy)`.
    pub delta: (f64, f64),
    /// Whether the left mouse button is currently held.
    pub left_button: bool,
    /// Whether the right mouse button is currently held.
    pub right_button: bool,
    /// Whether the middle mouse button is currently held.
    pub middle_button: bool,
}

impl Default for MouseState {
    fn default() -> Self {
        Self {
            position: (0.0, 0.0),
            delta: (0.0, 0.0),
            left_button: false,
            right_button: false,
            middle_button: false,
        }
    }
}
