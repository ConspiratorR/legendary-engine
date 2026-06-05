/// The state of a single key across frames.
///
/// Transitions follow this lifecycle:
///
/// ```text
/// Released ──press()──► JustPressed ──update_frame()──► Pressed
///                                                    │
/// Released ◄─update_frame()─ JustReleased ◄─release()─┘
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    /// The key is not pressed.
    Released,
    /// The key was just pressed this frame (transitions to [`Pressed`](Self::Pressed) on next frame).
    JustPressed,
    /// The key is held down.
    Pressed,
    /// The key was just released this frame (transitions to [`Released`](Self::Released) on next frame).
    JustReleased,
}

/// Re-export of [`winit::keyboard::KeyCode`] for convenience.
///
/// See the `winit` documentation for the full list of key variants.
pub use winit::keyboard::KeyCode;
