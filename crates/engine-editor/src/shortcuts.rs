//! Keyboard shortcut manager — maps key bindings to editor actions with
//! support for configurable shortcut profiles.

use engine_input::keyboard::KeyCode;
use std::collections::HashMap;

/// A keyboard shortcut with modifier keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub key: KeyCode,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl KeyBinding {
    /// Creates a key binding with no modifier keys.
    pub fn new(key: KeyCode) -> Self {
        Self {
            key,
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    /// Adds the Ctrl modifier.
    pub fn with_ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    /// Adds the Shift modifier.
    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    /// Adds the Alt modifier.
    pub fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }

    /// Returns `true` if this binding matches the given key + modifiers.
    pub fn matches(&self, key: KeyCode, ctrl: bool, shift: bool, alt: bool) -> bool {
        self.key == key && self.ctrl == ctrl && self.shift == shift && self.alt == alt
    }
}

/// All bindable editor actions.
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
    TerrainTool,
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

/// Maps [`EditorAction`]s to key bindings.
#[derive(Debug, Clone)]
pub struct ShortcutManager {
    bindings: HashMap<EditorAction, KeyBinding>,
}

impl ShortcutManager {
    /// Creates a new shortcut manager with default key bindings.
    pub fn new() -> Self {
        let mut manager = Self {
            bindings: HashMap::new(),
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
        self.bind(EditorAction::TerrainTool, KeyBinding::new(KeyCode::KeyT));
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

    /// Binds an action to a key binding.
    pub fn bind(&mut self, action: EditorAction, binding: KeyBinding) {
        self.bindings.insert(action, binding);
    }

    /// Removes the binding for an action.
    pub fn unbind(&mut self, action: EditorAction) {
        self.bindings.remove(&action);
    }

    /// Returns the key binding for an action, if any.
    pub fn get_binding(&self, action: &EditorAction) -> Option<&KeyBinding> {
        self.bindings.get(action)
    }

    /// Looks up the action bound to the given key + modifiers.
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

    /// Returns a human-readable string for the action's key binding (e.g. "Ctrl+S").
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
