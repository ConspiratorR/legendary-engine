//! GUI class (matches Unity's GUI).

use super::gui_content::GUIContent;
use super::gui_skin::GUISkin;

/// Global GUI settings (matches Unity's `GUI`).
pub struct GUI {
    pub skin: Option<GUISkin>,
    pub color: [f32; 4],
    pub background_color: [f32; 4],
    pub content_color: [f32; 4],
    pub enabled: bool,
    pub depth: i32,
}

impl Default for GUI {
    fn default() -> Self {
        Self {
            skin: None,
            color: [1.0; 4],
            background_color: [1.0; 4],
            content_color: [1.0; 4],
            enabled: true,
            depth: 0,
        }
    }
}

impl GUI {
    pub fn Label(rect: [f32; 4], content: &GUIContent) {
        let _ = (rect, content);
    }
    pub fn Box(rect: [f32; 4], content: &GUIContent) {
        let _ = (rect, content);
    }
    pub fn Button(rect: [f32; 4], content: &GUIContent) -> bool {
        let _ = (rect, content);
        false
    }
    pub fn RepeatButton(rect: [f32; 4], content: &GUIContent) -> bool {
        let _ = (rect, content);
        false
    }
    pub fn Toggle(rect: [f32; 4], value: bool, content: &GUIContent) -> bool {
        let _ = (rect, content);
        value
    }
    pub fn TextField(rect: [f32; 4], text: &str) -> String {
        let _ = rect;
        text.to_string()
    }
    pub fn TextArea(rect: [f32; 4], text: &str) -> String {
        let _ = rect;
        text.to_string()
    }
    pub fn HorizontalSlider(rect: [f32; 4], value: f32, left: f32, right: f32) -> f32 {
        let _ = (rect, left, right);
        value
    }
    pub fn Toolbar(rect: [f32; 4], selected: i32, texts: &[&str]) -> i32 {
        let _ = (rect, texts);
        selected
    }
    pub fn BeginGroup(rect: [f32; 4]) {
        let _ = rect;
    }
    pub fn EndGroup() {}
    pub fn BeginArea(rect: [f32; 4]) {
        let _ = rect;
    }
    pub fn BeginAreaWithTitle(rect: [f32; 4], text: &str) {
        let _ = (rect, text);
    }
    pub fn EndArea() {}
    pub fn DrawTexture(rect: [f32; 4], texture: &str) {
        let _ = (rect, texture);
    }
    pub fn DrawTextureWithTexCoords(rect: [f32; 4], texture: &str, tex_coords: [f32; 4]) {
        let _ = (rect, texture, tex_coords);
    }
    pub fn BringWindowToFront(id: i32) {
        let _ = id;
    }
    pub fn BringWindowToBack(id: i32) {
        let _ = id;
    }
    pub fn DragWindow() {}
    pub fn SetNextWindowName(name: &str) {
        let _ = name;
    }

    pub fn DrawRect(rect: [f32; 4], color: [f32; 4]) {
        let _ = (rect, color);
    }
    pub fn DrawBorder(rect: [f32; 4], color: [f32; 4], width: f32) {
        let _ = (rect, color, width);
    }
    pub fn DrawText(rect: [f32; 4], text: &str, color: [f32; 4], font_size: f32) {
        let _ = (rect, text, color, font_size);
    }
    pub fn GetAvailableRect() -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
    pub fn GetMousePosition() -> [f32; 2] {
        [0.0, 0.0]
    }
    pub fn MouseOverRect(rect: [f32; 4]) -> bool {
        let _ = rect;
        false
    }
    pub fn RectClicked(rect: [f32; 4]) -> bool {
        let _ = rect;
        false
    }
    pub fn RectDoubleClicked(rect: [f32; 4]) -> bool {
        let _ = rect;
        false
    }
    pub fn RectDragStarted(rect: [f32; 4]) -> bool {
        let _ = rect;
        false
    }
    pub fn GetDragDelta() -> [f32; 2] {
        [0.0, 0.0]
    }

    /// Draw a label using egui (actual rendering).
    pub fn LabelEgui(ui: &mut egui::Ui, content: &GUIContent) {
        ui.label(&content.text);
    }

    /// Draw a button using egui (actual rendering).
    pub fn ButtonEgui(ui: &mut egui::Ui, content: &GUIContent) -> bool {
        ui.button(&content.text).clicked()
    }

    /// Draw a toggle using egui (actual rendering).
    /// Unity: GUI.Toggle(Rect, bool, string)
    pub fn ToggleEgui(ui: &mut egui::Ui, value: &mut bool, content: &GUIContent) {
        ui.toggle_value(value, &content.text);
    }

    /// Draw a text field using egui (actual rendering).
    /// Unity: GUI.TextField(Rect, string)
    pub fn TextFieldEgui(ui: &mut egui::Ui, text: &mut String) -> bool {
        ui.text_edit_singleline(text).changed()
    }

    /// Draw a text area using egui (actual rendering).
    /// Unity: GUI.TextArea(Rect, string)
    pub fn TextAreaEgui(ui: &mut egui::Ui, text: &mut String) -> bool {
        ui.text_edit_multiline(text).changed()
    }

    /// Draw a horizontal slider using egui (actual rendering).
    pub fn HorizontalSliderEgui(ui: &mut egui::Ui, value: &mut f32, min: f32, max: f32) {
        ui.add(egui::Slider::new(value, min..=max));
    }

    /// Draw a box using egui (actual rendering).
    /// Unity: GUI.Box(Rect, string)
    pub fn BoxEgui(ui: &mut egui::Ui, content: &GUIContent) {
        ui.group(|ui| {
            ui.label(&content.text);
        });
    }

    /// Draw a separator using egui (actual rendering).
    /// Unity: GUI.Box(Rect, string) with separator style
    pub fn SeparatorEgui(ui: &mut egui::Ui) {
        ui.separator();
    }

    /// Draw a toolbar using egui (actual rendering).
    /// Unity: GUI.Toolbar(Rect, int, string[])
    pub fn ToolbarEgui(ui: &mut egui::Ui, selected: &mut i32, texts: &[&str]) {
        ui.horizontal(|ui| {
            for (i, text) in texts.iter().enumerate() {
                if ui.selectable_label(*selected == i as i32, *text).clicked() {
                    *selected = i as i32;
                }
            }
        });
    }
}

/// Window function type.
pub type WindowFunction = Box<dyn FnMut(i32)>;

/// GUIWindow manages a window instance.
pub struct GUIWindow {
    pub id: i32,
    pub rect: [f32; 4],
    pub title: String,
    pub draggable: bool,
    pub scroll: bool,
    pub background: Option<String>,
}

impl GUIWindow {
    pub fn new(id: i32, rect: [f32; 4], title: &str) -> Self {
        Self {
            id,
            rect,
            title: title.to_string(),
            draggable: true,
            scroll: false,
            background: None,
        }
    }
    pub fn Draw(&mut self) {}
}
