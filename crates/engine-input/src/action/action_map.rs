use crate::action::Binding;
use crate::input_manager::InputManager;
use crate::keyboard::KeyCode;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct ActionState {
    pub value: f32,
    just_pressed: bool,
    just_released: bool,
    #[allow(dead_code)]
    previous: f32,
}

impl ActionState {
    pub fn just_pressed(&self) -> bool {
        self.just_pressed
    }
    pub fn pressed(&self) -> bool {
        self.value != 0.0
    }
    pub fn just_released(&self) -> bool {
        self.just_released
    }
}

pub struct ActionMap {
    bindings: Vec<(String, Binding)>,
    states: HashMap<String, ActionState>,
}

impl Default for ActionMap {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionMap {
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
            states: HashMap::new(),
        }
    }

    pub fn bind_key(&mut self, action: &str, key: KeyCode) {
        self.bindings.push((action.to_string(), Binding::Key(key)));
    }

    pub fn bind_axis(&mut self, action: &str, positive: KeyCode, negative: KeyCode) {
        self.bindings
            .push((action.to_string(), Binding::Axis { positive, negative }));
    }

    pub fn bind_all(&mut self, bindings: impl IntoIterator<Item = (String, Binding)>) {
        self.bindings.extend(bindings);
    }

    pub fn bindings(&self) -> &[(String, Binding)] {
        &self.bindings
    }

    pub fn action(&self, name: &str) -> ActionState {
        self.states.get(name).copied().unwrap_or(ActionState {
            value: 0.0,
            just_pressed: false,
            just_released: false,
            previous: 0.0,
        })
    }

    pub fn update(&mut self, input: &InputManager) {
        let mut values: HashMap<String, f32> = HashMap::new();
        for (name, binding) in &self.bindings {
            let v: f32 = match binding {
                Binding::Key(key) => {
                    if input.key_down(*key) {
                        1.0
                    } else {
                        0.0
                    }
                }
                Binding::Axis { positive, negative } => {
                    let mut v = 0.0;
                    if input.key_down(*positive) {
                        v += 1.0;
                    }
                    if input.key_down(*negative) {
                        v -= 1.0;
                    }
                    v
                }
            };
            let e = values.entry(name.clone()).or_insert(0.0);
            if v.abs() > e.abs() {
                *e = v;
            }
        }
        for (name, value) in &values {
            let prev = self
                .states
                .get(name.as_str())
                .map(|s| s.value)
                .unwrap_or(0.0);
            let was = prev != 0.0;
            let now = *value != 0.0;
            self.states.insert(
                name.clone(),
                ActionState {
                    value: *value,
                    just_pressed: !was && now,
                    just_released: was && !now,
                    previous: prev,
                },
            );
        }
    }
}

pub fn action_update_system(_world: &mut engine_ecs::world::World) {}

#[cfg(test)]
mod tests {
    use crate::action::Binding;
    use crate::action::action_map::ActionMap;
    use crate::input_manager::InputManager;
    use crate::keyboard::KeyCode;

    #[test]
    fn test_bind_key_not_pressed_initially() {
        let mut map = ActionMap::new();
        map.bind_key("jump", KeyCode::Space);
        assert!(!map.action("jump").pressed());
    }

    #[test]
    fn test_just_pressed_after_key_down() {
        let mut map = ActionMap::new();
        map.bind_key("fire", KeyCode::KeyE);
        let mut input = InputManager::new();
        input.press(KeyCode::KeyE);
        map.update(&input);
        assert!(map.action("fire").just_pressed());
        assert!(map.action("fire").pressed());
    }

    #[test]
    fn test_just_released_after_key_up() {
        let mut map = ActionMap::new();
        map.bind_key("fire", KeyCode::KeyE);
        let mut input = InputManager::new();
        input.press(KeyCode::KeyE);
        map.update(&input);
        input.release(KeyCode::KeyE);
        map.update(&input);
        assert!(map.action("fire").just_released());
        assert!(!map.action("fire").pressed());
    }

    #[test]
    fn test_axis_positive_value() {
        let mut map = ActionMap::new();
        map.bind_axis("move_x", KeyCode::KeyD, KeyCode::KeyA);
        let mut input = InputManager::new();
        input.press(KeyCode::KeyD);
        map.update(&input);
        assert!((map.action("move_x").value - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_axis_negative_value() {
        let mut map = ActionMap::new();
        map.bind_axis("move_x", KeyCode::KeyD, KeyCode::KeyA);
        let mut input = InputManager::new();
        input.press(KeyCode::KeyA);
        map.update(&input);
        assert!((map.action("move_x").value - (-1.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_unknown_action_returns_default() {
        let map = ActionMap::new();
        assert_eq!(map.action("nonexistent").value, 0.0);
        assert!(!map.action("nonexistent").pressed());
    }

    #[test]
    fn test_bind_all_batch() {
        let mut map = ActionMap::new();
        map.bind_all(vec![
            ("jump".to_string(), Binding::Key(KeyCode::Space)),
            ("crouch".to_string(), Binding::Key(KeyCode::ShiftLeft)),
        ]);
        assert_eq!(map.bindings().len(), 2);
    }
}
