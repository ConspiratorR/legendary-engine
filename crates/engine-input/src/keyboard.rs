#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Released,
    JustPressed,
    Pressed,
    JustReleased,
}

pub use winit::keyboard::KeyCode;
