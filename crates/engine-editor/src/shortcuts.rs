use engine_input::keyboard::KeyCode;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub key: KeyCode,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl KeyBinding {
    pub fn new(key: KeyCode) -> Self {
        Self {
            key,
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    pub fn with_ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    pub fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }

    pub fn matches(&self, key: KeyCode, ctrl: bool, shift: bool, alt: bool) -> bool {
        self.key == key && self.ctrl == ctrl && self.shift == shift && self.alt == alt
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorAction {
    SaveScene,
    LoadScene,
    NewScene,
    Undo,
    Redo,
    Copy,
    Paste,
    Duplicate,
    Delete,
    SelectAll,
    DeselectAll,
    FocusOnSelection,
    TranslateTool,
    RotateTool,
    ScaleTool,
    Play,
    Pause,
    Stop,
    NextFrame,
    PrevFrame,
    ToggleGrid,
    ToggleGizmos,
    ShowConsole,
    ShowHierarchy,
    ShowInspector,
    ShowProject,
}

pub struct ShortcutManager {
    bindings: HashMap<EditorAction, KeyBinding>,
    action_handlers: HashMap<EditorAction, Box<dyn Fn()>>,
}

impl ShortcutManager {
    pub fn new() -> Self {
        let mut manager = Self {
            bindings: HashMap::new(),
            action_handlers: HashMap::new(),
        };
        manager.register_defaults();
        manager
    }

    fn register_defaults(&mut self) {
        self.bind(
            EditorAction::SaveScene,
            KeyBinding::new(KeyCode::KeyS).with_ctrl(),
        );
        self.bind(
            EditorAction::LoadScene,
            KeyBinding::new(KeyCode::KeyO).with_ctrl(),
        );
        self.bind(
            EditorAction::NewScene,
            KeyBinding::new(KeyCode::KeyN).with_ctrl(),
        );
        self.bind(
            EditorAction::Undo,
            KeyBinding::new(KeyCode::KeyZ).with_ctrl(),
        );
        self.bind(
            EditorAction::Redo,
            KeyBinding::new(KeyCode::KeyY).with_ctrl(),
        );
        self.bind(
            EditorAction::Copy,
            KeyBinding::new(KeyCode::KeyC).with_ctrl(),
        );
        self.bind(
            EditorAction::Paste,
            KeyBinding::new(KeyCode::KeyV).with_ctrl(),
        );
        self.bind(
            EditorAction::Duplicate,
            KeyBinding::new(KeyCode::KeyD).with_ctrl(),
        );
        self.bind(EditorAction::Delete, KeyBinding::new(KeyCode::Delete));
        self.bind(
            EditorAction::SelectAll,
            KeyBinding::new(KeyCode::KeyA).with_ctrl(),
        );
        self.bind(
            EditorAction::FocusOnSelection,
            KeyBinding::new(KeyCode::KeyF),
        );
        self.bind(EditorAction::TranslateTool, KeyBinding::new(KeyCode::KeyW));
        self.bind(EditorAction::RotateTool, KeyBinding::new(KeyCode::KeyE));
        self.bind(EditorAction::ScaleTool, KeyBinding::new(KeyCode::KeyR));
        self.bind(
            EditorAction::Play,
            KeyBinding::new(KeyCode::KeyP).with_ctrl(),
        );
        self.bind(EditorAction::Pause, KeyBinding::new(KeyCode::Pause));
        self.bind(EditorAction::Stop, KeyBinding::new(KeyCode::F5).with_ctrl());
        self.bind(
            EditorAction::NextFrame,
            KeyBinding::new(KeyCode::Period).with_ctrl(),
        );
        self.bind(
            EditorAction::PrevFrame,
            KeyBinding::new(KeyCode::Comma).with_ctrl(),
        );
    }

    pub fn bind(&mut self, action: EditorAction, binding: KeyBinding) {
        self.bindings.insert(action, binding);
    }

    pub fn unbind(&mut self, action: EditorAction) {
        self.bindings.remove(&action);
    }

    pub fn get_binding(&self, action: &EditorAction) -> Option<&KeyBinding> {
        self.bindings.get(action)
    }

    pub fn get_action(
        &self,
        key: KeyCode,
        ctrl: bool,
        shift: bool,
        alt: bool,
    ) -> Option<EditorAction> {
        for (action, binding) in &self.bindings {
            if binding.matches(key, ctrl, shift, alt) {
                return Some(*action);
            }
        }
        None
    }

    pub fn register_handler<F>(&mut self, action: EditorAction, handler: F)
    where
        F: Fn() + 'static,
    {
        self.action_handlers.insert(action, Box::new(handler));
    }

    pub fn execute(&self, action: &EditorAction) {
        if let Some(handler) = self.action_handlers.get(action) {
            handler();
        }
    }

    pub fn format_shortcut(&self, action: &EditorAction) -> String {
        if let Some(binding) = self.bindings.get(action) {
            let mut parts = Vec::new();
            if binding.ctrl {
                parts.push("Ctrl");
            }
            if binding.shift {
                parts.push("Shift");
            }
            if binding.alt {
                parts.push("Alt");
            }
            let key_str = format!("{:?}", binding.key).replace("Key", "");
            parts.push(key_str.as_str());
            parts.join("+")
        } else {
            "None".to_string()
        }
    }
}

impl Default for ShortcutManager {
    fn default() -> Self {
        Self::new()
    }
}
