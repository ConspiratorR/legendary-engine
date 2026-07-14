//! GUILayout class (matches Unity's GUILayout).

use super::gui_content::GUIContent;

/// Auto-layout GUI system (matches Unity's `GUILayout`).
pub struct GUILayout;

impl GUILayout {
    pub fn Label(content: &GUIContent) {
        let _ = content;
    }
    pub fn Box(content: &GUIContent) {
        let _ = content;
    }
    pub fn Button(content: &GUIContent) -> bool {
        let _ = content;
        false
    }
    pub fn RepeatButton(content: &GUIContent) -> bool {
        let _ = content;
        false
    }
    pub fn Toggle(value: bool, content: &GUIContent) -> bool {
        let _ = content;
        value
    }
    pub fn TextField(text: &str) -> String {
        text.to_string()
    }
    pub fn TextArea(text: &str) -> String {
        text.to_string()
    }
    pub fn HorizontalSlider(value: f32, left: f32, right: f32) -> f32 {
        let _ = (left, right);
        value
    }
    pub fn Toolbar(selected: i32, texts: &[&str]) -> i32 {
        let _ = texts;
        selected
    }
    pub fn BeginHorizontal() {}
    pub fn BeginHorizontalWithOptions(options: &[&str]) {
        let _ = options;
    }
    pub fn EndHorizontal() {}
    pub fn BeginVertical() {}
    pub fn BeginVerticalWithOptions(options: &[&str]) {
        let _ = options;
    }
    pub fn EndVertical() {}
    pub fn BeginScrollView(pos: [f32; 2]) -> [f32; 2] {
        pos
    }
    pub fn EndScrollView() {}
    /// Create a vertical scroll area (matches Unity's GUILayout.BeginScrollView).
    pub fn ScrollAreaVertical() -> egui::ScrollArea {
        egui::ScrollArea::vertical().auto_shrink([false, false])
    }
    /// Create a horizontal scroll area (matches Unity's GUILayout.BeginScrollView horizontal).
    pub fn ScrollAreaHorizontal() -> egui::ScrollArea {
        egui::ScrollArea::horizontal().auto_shrink([false, false])
    }
    /// Create a scroll area with both axes (matches Unity's GUI.BeginScrollView with both scrollbars).
    pub fn ScrollAreaBoth() -> egui::ScrollArea {
        egui::ScrollArea::both().auto_shrink([false, false])
    }
    pub fn BeginArea(rect: [f32; 4]) {
        let _ = rect;
    }
    pub fn EndArea() {}
    pub fn Window(id: i32, content: &GUIContent) -> bool {
        let _ = (id, content);
        false
    }
    pub fn EndWindow() {}
    pub fn Space(space: f32) {
        let _ = space;
    }
    pub fn FlexibleSpace() {}
    pub fn Width(width: f32) -> LayoutOption {
        LayoutOption::Width(width)
    }
    pub fn Height(height: f32) -> LayoutOption {
        LayoutOption::Height(height)
    }
    pub fn MinWidth(v: f32) -> LayoutOption {
        LayoutOption::MinWidth(v)
    }
    pub fn MaxWidth(v: f32) -> LayoutOption {
        LayoutOption::MaxWidth(v)
    }
    pub fn MinHeight(v: f32) -> LayoutOption {
        LayoutOption::MinHeight(v)
    }
    pub fn MaxHeight(v: f32) -> LayoutOption {
        LayoutOption::MaxHeight(v)
    }
    pub fn ExpandWidth(v: bool) -> LayoutOption {
        LayoutOption::ExpandWidth(v)
    }
    pub fn ExpandHeight(v: bool) -> LayoutOption {
        LayoutOption::ExpandHeight(v)
    }
    pub fn GetLastRect() -> [f32; 4] {
        [0.0; 4]
    }

    /// Draw a label using egui (actual rendering).
    pub fn LabelEgui(ui: &mut egui::Ui, content: &GUIContent) {
        ui.label(&content.text);
    }

    /// Draw a button using egui (actual rendering).
    pub fn ButtonEgui(ui: &mut egui::Ui, content: &GUIContent) -> bool {
        ui.button(&content.text).clicked()
    }

    /// Draw a text field using egui (actual rendering).
    pub fn TextFieldEgui(ui: &mut egui::Ui, text: &mut String) -> bool {
        ui.text_edit_singleline(text).changed()
    }

    /// Draw a toggle/checkbox using egui (actual rendering).
    /// Unity: GUILayout.Toggle(bool, string)
    pub fn ToggleEgui(ui: &mut egui::Ui, value: &mut bool, content: &GUIContent) {
        ui.checkbox(value, &content.text);
    }

    /// Draw a horizontal slider using egui (actual rendering).
    /// Unity: GUILayout.HorizontalSlider(float, float, float)
    pub fn HorizontalSliderEgui(ui: &mut egui::Ui, value: &mut f32, min: f32, max: f32) {
        ui.add(egui::Slider::new(value, min..=max));
    }

    /// Draw a text area using egui (actual rendering).
    /// Unity: GUILayout.TextArea(string)
    pub fn TextAreaEgui(ui: &mut egui::Ui, text: &mut String) -> bool {
        ui.add(egui::TextEdit::multiline(text)).changed()
    }

    /// Draw a toolbar using egui (actual rendering).
    /// Unity: GUILayout.Toolbar(int, string[])
    pub fn ToolbarEgui(ui: &mut egui::Ui, selected: &mut i32, texts: &[&str]) {
        ui.horizontal(|ui| {
            for (i, text) in texts.iter().enumerate() {
                if ui.selectable_label(*selected == i as i32, *text).clicked() {
                    *selected = i as i32;
                }
            }
        });
    }

    /// Begin a horizontal layout group using egui (actual rendering).
    /// Unity: GUILayout.BeginHorizontal()
    pub fn BeginHorizontalEgui(ui: &mut egui::Ui) -> egui::Ui {
        let mut child_ui = ui.child_ui(
            ui.max_rect(),
            egui::Layout::left_to_right(egui::Align::Min),
            None,
        );
        std::mem::swap(&mut child_ui, ui);
        child_ui
    }

    /// End a horizontal layout group using egui (actual rendering).
    /// Unity: GUILayout.EndHorizontal()
    pub fn EndHorizontalEgui(ui: &mut egui::Ui, child_ui: &mut egui::Ui) {
        std::mem::swap(ui, child_ui);
    }

    /// Begin a vertical layout group using egui (actual rendering).
    /// Unity: GUILayout.BeginVertical()
    pub fn BeginVerticalEgui(ui: &mut egui::Ui) -> egui::Ui {
        let mut child_ui = ui.child_ui(
            ui.max_rect(),
            egui::Layout::top_down(egui::Align::Min),
            None,
        );
        std::mem::swap(&mut child_ui, ui);
        child_ui
    }

    /// End a vertical layout group using egui (actual rendering).
    /// Unity: GUILayout.EndVertical()
    pub fn EndVerticalEgui(ui: &mut egui::Ui, child_ui: &mut egui::Ui) {
        std::mem::swap(ui, child_ui);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LayoutOption {
    Width(f32),
    Height(f32),
    MinWidth(f32),
    MaxWidth(f32),
    MinHeight(f32),
    MaxHeight(f32),
    ExpandWidth(bool),
    ExpandHeight(bool),
}
