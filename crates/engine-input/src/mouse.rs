#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseState {
    pub position: (f64, f64),
    pub delta: (f64, f64),
    pub left_button: bool,
    pub right_button: bool,
}

impl Default for MouseState {
    fn default() -> Self {
        Self {
            position: (0.0, 0.0),
            delta: (0.0, 0.0),
            left_button: false,
            right_button: false,
        }
    }
}
