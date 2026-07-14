//! GUISkin class (matches Unity's GUISkin).

use super::gui_style::GUIStyle;

/// A collection of styles for IMGUI (matches Unity's `GUISkin`).
#[derive(Debug, Clone)]
pub struct GUISkin {
    pub font: String,
    pub box_style: GUIStyle,
    pub button: GUIStyle,
    pub repeat_button: GUIStyle,
    pub toggle: GUIStyle,
    pub label: GUIStyle,
    pub text_field: GUIStyle,
    pub text_area: GUIStyle,
    pub window: GUIStyle,
    pub horizontal_slider: GUIStyle,
    pub horizontal_slider_thumb: GUIStyle,
    pub vertical_slider: GUIStyle,
    pub vertical_slider_thumb: GUIStyle,
    pub horizontal_scrollbar: GUIStyle,
    pub horizontal_scrollbar_thumb: GUIStyle,
    pub horizontal_scrollbar_left_button: GUIStyle,
    pub horizontal_scrollbar_right_button: GUIStyle,
    pub vertical_scrollbar: GUIStyle,
    pub vertical_scrollbar_thumb: GUIStyle,
    pub vertical_scrollbar_up_button: GUIStyle,
    pub vertical_scrollbar_down_button: GUIStyle,
    pub scroll_view: GUIStyle,
    pub grid: GUIStyle,
}

impl Default for GUISkin {
    fn default() -> Self {
        Self {
            font: String::new(),
            box_style: GUIStyle::default(),
            button: GUIStyle::default(),
            repeat_button: GUIStyle::default(),
            toggle: GUIStyle::default(),
            label: GUIStyle::default(),
            text_field: GUIStyle::default(),
            text_area: GUIStyle::default(),
            window: GUIStyle::default(),
            horizontal_slider: GUIStyle::default(),
            horizontal_slider_thumb: GUIStyle::default(),
            vertical_slider: GUIStyle::default(),
            vertical_slider_thumb: GUIStyle::default(),
            horizontal_scrollbar: GUIStyle::default(),
            horizontal_scrollbar_thumb: GUIStyle::default(),
            horizontal_scrollbar_left_button: GUIStyle::default(),
            horizontal_scrollbar_right_button: GUIStyle::default(),
            vertical_scrollbar: GUIStyle::default(),
            vertical_scrollbar_thumb: GUIStyle::default(),
            vertical_scrollbar_up_button: GUIStyle::default(),
            vertical_scrollbar_down_button: GUIStyle::default(),
            scroll_view: GUIStyle::default(),
            grid: GUIStyle::default(),
        }
    }
}

impl GUISkin {
    pub fn FindStyle(&self, name: &str) -> Option<&GUIStyle> {
        match name {
            "box" | "Box" => Some(&self.box_style),
            "button" | "Button" => Some(&self.button),
            "toggle" | "Toggle" => Some(&self.toggle),
            "label" | "Label" => Some(&self.label),
            "textfield" | "TextField" => Some(&self.text_field),
            "textarea" | "TextArea" => Some(&self.text_area),
            "window" | "Window" => Some(&self.window),
            _ => None,
        }
    }
}
