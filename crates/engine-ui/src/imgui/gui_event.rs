//! Event class (matches Unity's Event).

/// Event type (matches Unity's `EventType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    MouseDown,
    MouseUp,
    MouseDrag,
    MouseMove,
    ScrollWheel,
    KeyDown,
    KeyUp,
    Repaint,
    Layout,
    Used,
    Ignore,
}

impl Default for EventType {
    fn default() -> Self {
        Self::Repaint
    }
}

/// Key code (matches Unity's `KeyCode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    None,
    Backspace,
    Tab,
    Return,
    Escape,
    Space,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Alpha0,
    Alpha1,
    Alpha2,
    Alpha3,
    Alpha4,
    Alpha5,
    Alpha6,
    Alpha7,
    Alpha8,
    Alpha9,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    LeftShift,
    RightShift,
    LeftControl,
    RightControl,
    LeftAlt,
    RightAlt,
    UpArrow,
    DownArrow,
    LeftArrow,
    RightArrow,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
}

impl Default for KeyCode {
    fn default() -> Self {
        Self::None
    }
}

/// An input event for IMGUI (matches Unity's `Event`).
#[derive(Debug, Clone)]
pub struct Event {
    pub event_type: EventType,
    pub mouse_position: [f32; 2],
    pub button: i32,
    pub key_code: KeyCode,
    pub character: Option<char>,
    pub control: bool,
    pub shift: bool,
    pub alt: bool,
    pub command: bool,
    pub used: bool,
}

impl Default for Event {
    fn default() -> Self {
        Self {
            event_type: EventType::Repaint,
            mouse_position: [0.0, 0.0],
            button: 0,
            key_code: KeyCode::None,
            character: None,
            control: false,
            shift: false,
            alt: false,
            command: false,
            used: false,
        }
    }
}

impl Event {
    pub fn current() -> Self {
        Self::default()
    }

    pub fn Use(&mut self) {
        self.used = true;
        self.event_type = EventType::Used;
    }

    pub fn IsMouse(&self) -> bool {
        matches!(
            self.event_type,
            EventType::MouseDown
                | EventType::MouseUp
                | EventType::MouseDrag
                | EventType::MouseMove
                | EventType::ScrollWheel
        )
    }

    pub fn IsKey(&self) -> bool {
        matches!(self.event_type, EventType::KeyDown | EventType::KeyUp)
    }

    pub fn IsShiftKeyDown(&self) -> bool {
        self.shift
    }

    pub fn IsControlKeyDown(&self) -> bool {
        self.control
    }

    pub fn IsAltKeyDown(&self) -> bool {
        self.alt
    }

    pub fn ScreenToGUIPoint(screen_pos: [f32; 2]) -> [f32; 2] {
        screen_pos
    }

    pub fn GUIToScreenPoint(gui_pos: [f32; 2]) -> [f32; 2] {
        gui_pos
    }

    /// Convert from egui::Event to our Event type.
    pub fn from_egui_event(egui_event: &egui::Event, screen_pos: [f32; 2]) -> Option<Self> {
        match egui_event {
            egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers,
            } => {
                let event_type = if *pressed {
                    EventType::MouseDown
                } else {
                    EventType::MouseUp
                };
                let button = match button {
                    egui::PointerButton::Primary => 0,
                    egui::PointerButton::Secondary => 1,
                    egui::PointerButton::Middle => 2,
                    _ => 3,
                };
                Some(Self {
                    event_type,
                    mouse_position: [pos.x, pos.y],
                    button,
                    key_code: KeyCode::None,
                    character: None,
                    control: modifiers.ctrl,
                    shift: modifiers.shift,
                    alt: modifiers.alt,
                    command: modifiers.mac_cmd,
                    used: false,
                })
            }
            egui::Event::PointerMoved(pos) => Some(Self {
                event_type: EventType::MouseMove,
                mouse_position: [pos.x, pos.y],
                ..Default::default()
            }),
            egui::Event::MouseWheel { delta, .. } => {
                let _ = delta;
                Some(Self {
                    event_type: EventType::ScrollWheel,
                    mouse_position: screen_pos,
                    ..Default::default()
                })
            }
            egui::Event::Key {
                key,
                pressed,
                modifiers,
                ..
            } => {
                let event_type = if *pressed {
                    EventType::KeyDown
                } else {
                    EventType::KeyUp
                };
                let key_code = match key {
                    egui::Key::A => KeyCode::A,
                    egui::Key::B => KeyCode::B,
                    egui::Key::C => KeyCode::C,
                    egui::Key::D => KeyCode::D,
                    egui::Key::E => KeyCode::E,
                    egui::Key::F => KeyCode::F,
                    egui::Key::G => KeyCode::G,
                    egui::Key::H => KeyCode::H,
                    egui::Key::I => KeyCode::I,
                    egui::Key::J => KeyCode::J,
                    egui::Key::K => KeyCode::K,
                    egui::Key::L => KeyCode::L,
                    egui::Key::M => KeyCode::M,
                    egui::Key::N => KeyCode::N,
                    egui::Key::O => KeyCode::O,
                    egui::Key::P => KeyCode::P,
                    egui::Key::Q => KeyCode::Q,
                    egui::Key::R => KeyCode::R,
                    egui::Key::S => KeyCode::S,
                    egui::Key::T => KeyCode::T,
                    egui::Key::U => KeyCode::U,
                    egui::Key::V => KeyCode::V,
                    egui::Key::W => KeyCode::W,
                    egui::Key::X => KeyCode::X,
                    egui::Key::Y => KeyCode::Y,
                    egui::Key::Z => KeyCode::Z,
                    egui::Key::Num0 => KeyCode::Alpha0,
                    egui::Key::Num1 => KeyCode::Alpha1,
                    egui::Key::Num2 => KeyCode::Alpha2,
                    egui::Key::Num3 => KeyCode::Alpha3,
                    egui::Key::Num4 => KeyCode::Alpha4,
                    egui::Key::Num5 => KeyCode::Alpha5,
                    egui::Key::Num6 => KeyCode::Alpha6,
                    egui::Key::Num7 => KeyCode::Alpha7,
                    egui::Key::Num8 => KeyCode::Alpha8,
                    egui::Key::Num9 => KeyCode::Alpha9,
                    egui::Key::Enter => KeyCode::Return,
                    egui::Key::Escape => KeyCode::Escape,
                    egui::Key::Space => KeyCode::Space,
                    egui::Key::Backspace => KeyCode::Backspace,
                    egui::Key::Tab => KeyCode::Tab,
                    egui::Key::ArrowUp => KeyCode::UpArrow,
                    egui::Key::ArrowDown => KeyCode::DownArrow,
                    egui::Key::ArrowLeft => KeyCode::LeftArrow,
                    egui::Key::ArrowRight => KeyCode::RightArrow,
                    egui::Key::Home => KeyCode::Home,
                    egui::Key::End => KeyCode::End,
                    egui::Key::PageUp => KeyCode::PageUp,
                    egui::Key::PageDown => KeyCode::PageDown,
                    egui::Key::Insert => KeyCode::Insert,
                    egui::Key::Delete => KeyCode::Delete,
                    _ => KeyCode::None,
                };
                Some(Self {
                    event_type,
                    mouse_position: screen_pos,
                    button: 0,
                    key_code,
                    character: None,
                    control: modifiers.ctrl,
                    shift: modifiers.shift,
                    alt: modifiers.alt,
                    command: modifiers.mac_cmd,
                    used: false,
                })
            }
            egui::Event::Text(text) => {
                let ch = text.chars().next();
                Some(Self {
                    event_type: EventType::KeyDown,
                    mouse_position: screen_pos,
                    character: ch,
                    ..Default::default()
                })
            }
            _ => None,
        }
    }

    /// Convert from egui::PointerState to get current mouse info.
    pub fn from_pointer_state(pointer: &egui::PointerState, screen_pos: [f32; 2]) -> Self {
        let pos = pointer
            .interact_pos()
            .map(|p| [p.x, p.y])
            .unwrap_or(screen_pos);
        Self {
            event_type: EventType::MouseMove,
            mouse_position: pos,
            ..Default::default()
        }
    }

    /// Convert from egui input to our Event type.
    pub fn from_egui_input(input: &egui::InputState) -> Self {
        let pointer = &input.pointer;
        let mouse_pos = pointer
            .interact_pos()
            .map(|p| [p.x, p.y])
            .unwrap_or([0.0, 0.0]);

        let event_type = if pointer.any_click() {
            if pointer.any_pressed() {
                EventType::MouseDown
            } else {
                EventType::MouseUp
            }
        } else if pointer.any_down() {
            EventType::MouseDrag
        } else if pointer.hover_pos().is_some() {
            EventType::MouseMove
        } else {
            EventType::Repaint
        };

        Self {
            event_type,
            mouse_position: mouse_pos,
            button: 0,
            key_code: KeyCode::None,
            character: None,
            control: input.modifiers.ctrl,
            shift: input.modifiers.shift,
            alt: input.modifiers.alt,
            command: input.modifiers.mac_cmd,
            used: false,
        }
    }

    /// Get the current mouse position from egui input.
    pub fn GetMousePositionEgui(input: &egui::InputState) -> [f32; 2] {
        input
            .pointer
            .interact_pos()
            .map(|p| [p.x, p.y])
            .unwrap_or([0.0, 0.0])
    }

    /// Check if a key is pressed.
    pub fn IsKeyPressedEgui(input: &egui::InputState, key: egui::Key) -> bool {
        input.key_pressed(key)
    }

    /// Check if a modifier is pressed.
    pub fn IsModifierPressed(input: &egui::InputState, modifier: egui::Modifiers) -> bool {
        input.modifiers.contains(modifier)
    }
}
