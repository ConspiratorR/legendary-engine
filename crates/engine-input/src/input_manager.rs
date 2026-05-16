use std::collections::HashMap;
use crate::keyboard::{KeyCode, KeyState};
use crate::mouse::MouseState;

pub struct InputManager {
    keys: HashMap<KeyCode, KeyState>,
    mouse: MouseState,
}

impl Default for InputManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            mouse: MouseState::default(),
        }
    }

    pub fn press(&mut self, key: KeyCode) {
        let state = self.keys.entry(key).or_insert(KeyState::Released);
        if *state == KeyState::Released || *state == KeyState::JustReleased {
            *state = KeyState::JustPressed;
        }
    }

    pub fn release(&mut self, key: KeyCode) {
        let state = self.keys.entry(key).or_insert(KeyState::Released);
        if *state == KeyState::Pressed || *state == KeyState::JustPressed {
            *state = KeyState::JustReleased;
        }
    }

    pub fn key_state(&self, key: KeyCode) -> KeyState {
        self.keys.get(&key).copied().unwrap_or(KeyState::Released)
    }

    pub fn key_down(&self, key: KeyCode) -> bool {
        matches!(self.key_state(key), KeyState::Pressed | KeyState::JustPressed)
    }

    pub fn key_just_pressed(&self, key: KeyCode) -> bool {
        self.key_state(key) == KeyState::JustPressed
    }

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

    pub fn mouse(&self) -> &MouseState {
        &self.mouse
    }

    pub fn mouse_mut(&mut self) -> &mut MouseState {
        &mut self.mouse
    }
}

#[cfg(test)]
mod tests {
    use crate::keyboard::{KeyCode, KeyState};
    use crate::input_manager::InputManager;

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
}
