/// The state of a single key across frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    /// The key is not pressed.
    Released,
    /// The key was just pressed this frame.
    JustPressed,
    /// The key is held down.
    Pressed,
    /// The key was just released this frame.
    JustReleased,
}

/// Re-export of [`winit::keyboard::KeyCode`] for convenience.
pub use winit::keyboard::KeyCode;
