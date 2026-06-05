use engine_input::action::{ActionMap, Binding};
use engine_input::input_manager::InputManager;
use engine_input::keyboard::{KeyCode, KeyState};
use engine_input::mouse::MouseState;

#[test]
fn input_manager_creation() {
    let input = InputManager::new();
    assert_eq!(input.key_state(KeyCode::Space), KeyState::Released);
    assert_eq!(input.key_state(KeyCode::KeyA), KeyState::Released);
    assert_eq!(input.mouse().position, (0.0, 0.0));
    assert_eq!(input.mouse().delta, (0.0, 0.0));
    assert!(!input.mouse().left_button);
}

#[test]
fn register_action_and_query() {
    let mut map = ActionMap::new();
    map.bind_key("jump", KeyCode::Space);

    let state = map.action("jump");
    assert!(!state.pressed());
    assert!(!state.just_pressed());
    assert!(!state.just_released());
    assert_eq!(state.value, 0.0);
}

#[test]
fn duplicate_action_multiple_bindings() {
    let mut map = ActionMap::new();
    map.bind_key("jump", KeyCode::Space);
    map.bind_key("jump", KeyCode::KeyJ);

    assert_eq!(map.bindings().len(), 2);

    let mut input = InputManager::new();

    input.press(KeyCode::KeyJ);
    map.update(&input);
    assert!(map.action("jump").pressed());
    assert_eq!(map.action("jump").value, 1.0);
    input.release(KeyCode::KeyJ);
    input.update_frame();

    input.press(KeyCode::Space);
    map.update(&input);
    assert!(map.action("jump").pressed());
    assert_eq!(map.action("jump").value, 1.0);
}

#[test]
fn key_binding_press_release_cycle() {
    let mut map = ActionMap::new();
    map.bind_key("fire", KeyCode::KeyE);
    let mut input = InputManager::new();

    map.update(&input);
    assert!(!map.action("fire").pressed());

    input.press(KeyCode::KeyE);
    map.update(&input);
    assert!(map.action("fire").just_pressed());
    assert!(map.action("fire").pressed());

    input.update_frame();
    map.update(&input);
    assert!(!map.action("fire").just_pressed());
    assert!(map.action("fire").pressed());

    input.release(KeyCode::KeyE);
    map.update(&input);
    assert!(map.action("fire").just_released());
    assert!(!map.action("fire").pressed());

    input.update_frame();
    map.update(&input);
    assert!(!map.action("fire").just_released());
    assert!(!map.action("fire").pressed());
}

#[test]
fn mouse_state_tracking() {
    let mut input = InputManager::new();

    input.mouse_mut().position = (100.0, 200.0);
    input.mouse_mut().delta = (5.0, -3.0);
    input.mouse_mut().left_button = true;

    assert_eq!(input.mouse().position, (100.0, 200.0));
    assert_eq!(input.mouse().delta, (5.0, -3.0));
    assert!(input.mouse().left_button);
    assert!(!input.mouse().right_button);
    assert!(!input.mouse().middle_button);

    input.update_frame();
    assert_eq!(input.mouse().delta, (0.0, 0.0));
    assert_eq!(input.mouse().position, (100.0, 200.0));
}

#[test]
fn axis_input_positive() {
    let mut map = ActionMap::new();
    map.bind_axis("move_x", KeyCode::KeyD, KeyCode::KeyA);
    let mut input = InputManager::new();

    input.press(KeyCode::KeyD);
    map.update(&input);
    let state = map.action("move_x");
    assert!(state.pressed());
    assert!((state.value - 1.0).abs() < f32::EPSILON);
}

#[test]
fn axis_input_negative() {
    let mut map = ActionMap::new();
    map.bind_axis("move_x", KeyCode::KeyD, KeyCode::KeyA);
    let mut input = InputManager::new();

    input.press(KeyCode::KeyA);
    map.update(&input);
    let state = map.action("move_x");
    assert!(state.pressed());
    assert!((state.value - (-1.0)).abs() < f32::EPSILON);
}

#[test]
fn axis_input_cancel() {
    let mut map = ActionMap::new();
    map.bind_axis("move_x", KeyCode::KeyD, KeyCode::KeyA);
    let mut input = InputManager::new();

    input.press(KeyCode::KeyD);
    input.press(KeyCode::KeyA);
    map.update(&input);
    let state = map.action("move_x");
    assert!(!state.pressed());
    assert_eq!(state.value, 0.0);
}

#[test]
fn unknown_action_returns_default() {
    let map = ActionMap::new();
    let state = map.action("nonexistent");
    assert_eq!(state.value, 0.0);
    assert!(!state.pressed());
    assert!(!state.just_pressed());
    assert!(!state.just_released());
}

#[test]
fn bind_all_batch() {
    let mut map = ActionMap::new();
    map.bind_all(vec![
        ("jump".to_string(), Binding::Key(KeyCode::Space)),
        ("crouch".to_string(), Binding::Key(KeyCode::ShiftLeft)),
        (
            "move_x".to_string(),
            Binding::Axis {
                positive: KeyCode::KeyD,
                negative: KeyCode::KeyA,
            },
        ),
    ]);
    assert_eq!(map.bindings().len(), 3);
}

#[test]
fn default_trait_implementation() {
    let input = InputManager::default();
    assert_eq!(input.key_state(KeyCode::Space), KeyState::Released);

    let map = ActionMap::default();
    assert_eq!(map.action("test").value, 0.0);
}

#[test]
fn mouse_state_default() {
    let mouse = MouseState::default();
    assert_eq!(mouse.position, (0.0, 0.0));
    assert_eq!(mouse.delta, (0.0, 0.0));
    assert!(!mouse.left_button);
    assert!(!mouse.right_button);
    assert!(!mouse.middle_button);
}

#[test]
fn key_down_reflects_held_state() {
    let mut input = InputManager::new();
    assert!(!input.key_down(KeyCode::KeyW));

    input.press(KeyCode::KeyW);
    // JustPressed counts as down
    assert!(input.key_down(KeyCode::KeyW));

    input.update_frame();
    // Pressed still counts as down
    assert!(input.key_down(KeyCode::KeyW));

    input.release(KeyCode::KeyW);
    // JustReleased does NOT count as down
    assert!(!input.key_down(KeyCode::KeyW));
}

#[test]
fn key_just_pressed_only_first_frame() {
    let mut input = InputManager::new();

    input.press(KeyCode::KeyW);
    assert!(input.key_just_pressed(KeyCode::KeyW));

    input.update_frame();
    // Pressed is no longer "just" pressed
    assert!(!input.key_just_pressed(KeyCode::KeyW));
}

#[test]
fn key_just_released_only_first_frame() {
    let mut input = InputManager::new();

    input.press(KeyCode::KeyW);
    input.update_frame();
    input.release(KeyCode::KeyW);
    assert!(input.key_just_released(KeyCode::KeyW));

    input.update_frame();
    // Fully released is no longer "just" released
    assert!(!input.key_just_released(KeyCode::KeyW));
}

#[test]
fn double_press_does_not_reset_to_just_pressed() {
    let mut input = InputManager::new();

    input.press(KeyCode::KeyW);
    assert_eq!(input.key_state(KeyCode::KeyW), KeyState::JustPressed);

    // Calling press again while already pressed should not re-trigger JustPressed
    input.press(KeyCode::KeyW);
    assert_eq!(input.key_state(KeyCode::KeyW), KeyState::JustPressed);

    input.update_frame();
    assert_eq!(input.key_state(KeyCode::KeyW), KeyState::Pressed);

    // Press again while held — should stay Pressed, not go back to JustPressed
    input.press(KeyCode::KeyW);
    assert_eq!(input.key_state(KeyCode::KeyW), KeyState::Pressed);
}

#[test]
fn release_on_already_released_key_is_noop() {
    let mut input = InputManager::new();

    // Release a key that was never pressed
    input.release(KeyCode::KeyW);
    assert_eq!(input.key_state(KeyCode::KeyW), KeyState::Released);
}

#[test]
fn multiple_keys_independent() {
    let mut input = InputManager::new();

    input.press(KeyCode::KeyW);
    input.press(KeyCode::KeyA);
    assert!(input.key_down(KeyCode::KeyW));
    assert!(input.key_down(KeyCode::KeyA));

    input.release(KeyCode::KeyW);
    assert!(!input.key_down(KeyCode::KeyW));
    assert!(input.key_down(KeyCode::KeyA));
}

#[test]
fn action_rebind_at_runtime() {
    let mut map = ActionMap::new();
    map.bind_key("jump", KeyCode::Space);

    let mut input = InputManager::new();
    input.press(KeyCode::Space);
    map.update(&input);
    assert!(map.action("jump").just_pressed());
    input.release(KeyCode::Space);
    input.update_frame();

    // Rebind: clear and re-bind to a different key
    let mut map = ActionMap::new();
    map.bind_key("jump", KeyCode::KeyJ);

    // Space no longer triggers jump
    input.press(KeyCode::Space);
    map.update(&input);
    assert!(!map.action("jump").pressed());
    input.release(KeyCode::Space);
    input.update_frame();

    // KeyJ now triggers jump
    input.press(KeyCode::KeyJ);
    map.update(&input);
    assert!(map.action("jump").just_pressed());
}

#[test]
fn action_multiple_bindings_same_action_last_wins_on_equal_value() {
    let mut map = ActionMap::new();
    map.bind_key("fire", KeyCode::KeyE);
    map.bind_key("fire", KeyCode::KeyF);
    map.bind_key("fire", KeyCode::KeyG);

    let mut input = InputManager::new();
    // All three pressed — value should be 1.0 (all produce same magnitude)
    input.press(KeyCode::KeyE);
    input.press(KeyCode::KeyF);
    input.press(KeyCode::KeyG);
    map.update(&input);
    assert!(map.action("fire").pressed());
    assert_eq!(map.action("fire").value, 1.0);
}

#[test]
fn action_held_across_frames_stays_pressed_not_just_pressed() {
    let mut map = ActionMap::new();
    map.bind_key("run", KeyCode::ShiftLeft);

    let mut input = InputManager::new();
    input.press(KeyCode::ShiftLeft);
    map.update(&input);
    assert!(map.action("run").just_pressed());
    assert!(map.action("run").pressed());

    // Next frame: key still held
    input.update_frame();
    map.update(&input);
    assert!(!map.action("run").just_pressed());
    assert!(map.action("run").pressed());
}

#[test]
fn mouse_buttons_all_tracked() {
    let mut input = InputManager::new();

    input.mouse_mut().left_button = true;
    input.mouse_mut().right_button = true;
    input.mouse_mut().middle_button = true;

    assert!(input.mouse().left_button);
    assert!(input.mouse().right_button);
    assert!(input.mouse().middle_button);

    input.mouse_mut().left_button = false;
    assert!(!input.mouse().left_button);
    assert!(input.mouse().right_button);
}
