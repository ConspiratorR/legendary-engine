use crate::keyboard::{KeyCode, KeyState};
use crate::mouse::MouseState;
use crate::touch::{TouchPoint, TouchState};
use std::collections::HashMap;

/// Central input state tracker.
///
/// Maintains the current state of every key, the mouse, and touch input.
/// Call [`press`](Self::press) / [`release`](Self::release) from your event
/// loop, and [`update_frame`](Self::update_frame) once per tick to
/// advance the just-pressed/just-released transitions.
pub struct InputManager {
    keys: HashMap<KeyCode, KeyState>,
    mouse: MouseState,
    touch: TouchState,
}

impl Default for InputManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InputManager {
    /// Create a new input manager with all keys released.
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            mouse: MouseState::default(),
            touch: TouchState::default(),
        }
    }

    /// Record a key press event.
    pub fn press(&mut self, key: KeyCode) {
        let state = self.keys.entry(key).or_insert(KeyState::Released);
        if *state == KeyState::Released || *state == KeyState::JustReleased {
            *state = KeyState::JustPressed;
        }
    }

    /// Record a key release event.
    pub fn release(&mut self, key: KeyCode) {
        let state = self.keys.entry(key).or_insert(KeyState::Released);
        if *state == KeyState::Pressed || *state == KeyState::JustPressed {
            *state = KeyState::JustReleased;
        }
    }

    /// Return the current state of a key.
    pub fn key_state(&self, key: KeyCode) -> KeyState {
        self.keys.get(&key).copied().unwrap_or(KeyState::Released)
    }

    /// Returns `true` if the key is currently held (`Pressed` or `JustPressed`).
    pub fn key_down(&self, key: KeyCode) -> bool {
        matches!(
            self.key_state(key),
            KeyState::Pressed | KeyState::JustPressed
        )
    }

    /// Returns `true` only on the first frame the key is down.
    pub fn key_just_pressed(&self, key: KeyCode) -> bool {
        self.key_state(key) == KeyState::JustPressed
    }

    /// Returns `true` only on the first frame the key is released.
    pub fn key_just_released(&self, key: KeyCode) -> bool {
        self.key_state(key) == KeyState::JustReleased
    }

    /// Advance the frame: `JustPressed` → `Pressed`, `JustReleased` → `Released`,
    /// and reset mouse delta.
    pub fn update_frame(&mut self) {
        for state in self.keys.values_mut() {
            match state {
                KeyState::JustPressed => *state = KeyState::Pressed,
                KeyState::JustReleased => *state = KeyState::Released,
                _ => {}
            }
        }
        self.mouse.delta = (0.0, 0.0);
    }

    /// Get a shared reference to the mouse state.
    pub fn mouse(&self) -> &MouseState {
        &self.mouse
    }

    /// Get an exclusive reference to the mouse state.
    pub fn mouse_mut(&mut self) -> &mut MouseState {
        &mut self.mouse
    }

    /// Get a shared reference to the touch state.
    pub fn touch(&self) -> &TouchState {
        &self.touch
    }

    /// Get an exclusive reference to the touch state.
    pub fn touch_mut(&mut self) -> &mut TouchState {
        &mut self.touch
    }

    /// Add a touch point to the current frame's touch state.
    pub fn add_touch(&mut self, point: TouchPoint) {
        self.touch.points.push(point);
    }
}

#[cfg(test)]
mod tests {
    use crate::input_manager::InputManager;
    use crate::keyboard::{KeyCode, KeyState};

    #[test]
    fn test_key_default_released() {
        let input = InputManager::new();
        assert_eq!(input.key_state(KeyCode::Space), KeyState::Released);
    }

    #[test]
    fn test_key_press_and_release() {
        let mut input = InputManager::new();
        input.press(KeyCode::KeyA);
        assert_eq!(input.key_state(KeyCode::KeyA), KeyState::JustPressed);
        input.update_frame();
        assert_eq!(input.key_state(KeyCode::KeyA), KeyState::Pressed);
        input.release(KeyCode::KeyA);
        assert_eq!(input.key_state(KeyCode::KeyA), KeyState::JustReleased);
        input.update_frame();
        assert_eq!(input.key_state(KeyCode::KeyA), KeyState::Released);
    }

    #[test]
    fn test_key_down_and_just_pressed() {
        let mut input = InputManager::new();
        assert!(!input.key_down(KeyCode::Space));
        assert!(!input.key_just_pressed(KeyCode::Space));
        assert!(!input.key_just_released(KeyCode::Space));

        input.press(KeyCode::Space);
        assert!(input.key_down(KeyCode::Space));
        assert!(input.key_just_pressed(KeyCode::Space));

        input.update_frame();
        assert!(input.key_down(KeyCode::Space));
        assert!(!input.key_just_pressed(KeyCode::Space));

        input.release(KeyCode::Space);
        assert!(!input.key_down(KeyCode::Space));
        assert!(input.key_just_released(KeyCode::Space));

        input.update_frame();
        assert!(!input.key_down(KeyCode::Space));
        assert!(!input.key_just_released(KeyCode::Space));
    }
}
