//! EditorGUILayout class (matches Unity's EditorGUILayout).
//!
//! Unity Reference: https://docs.unity3d.com/ScriptReference/EditorGUILayout.html

use super::gui_content::GUIContent;

/// Editor-specific auto-layout (matches Unity's `EditorGUILayout`).
pub struct EditorGUILayout;

impl EditorGUILayout {
    // ── Placeholder methods ──

    pub fn PropertyField(label: &GUIContent, value: &str) -> String {
        let _ = label;
        value.to_string()
    }
    pub fn ObjectField(label: &GUIContent, obj_type: &str) -> Option<String> {
        let _ = (label, obj_type);
        None
    }
    pub fn MaskField(label: &GUIContent, selected: i32, options: &[&str]) -> i32 {
        let _ = (label, options);
        selected
    }
    pub fn Popup(label: &GUIContent, selected: i32, options: &[&str]) -> i32 {
        let _ = (label, options);
        selected
    }
    pub fn EnumPopup(label: &GUIContent, selected: i32) -> i32 {
        let _ = label;
        selected
    }
    pub fn IntPopup(label: &GUIContent, selected: i32, options: &[&str]) -> i32 {
        let _ = (label, options);
        selected
    }
    pub fn FloatPopup(label: &GUIContent, selected: f32, options: &[&str]) -> f32 {
        let _ = (label, options);
        selected
    }
    pub fn LayerField(label: &GUIContent, layer: i32) -> i32 {
        let _ = label;
        layer
    }
    pub fn TagField(label: &GUIContent, tag: &str) -> String {
        let _ = label;
        tag.to_string()
    }
    pub fn LayerMaskField(label: &GUIContent, mask: i32) -> i32 {
        let _ = label;
        mask
    }
    pub fn MinMaxSlider(label: &GUIContent, min: f32, max: f32) -> (f32, f32) {
        let _ = label;
        (min, max)
    }
    pub fn Foldout(foldout: bool, content: &GUIContent) -> bool {
        let _ = content;
        foldout
    }
    pub fn HelpBox(message: &str, message_type: i32) {
        let _ = (message, message_type);
    }
    pub fn Separator() {}
    pub fn PrefixLabel(label: &GUIContent) {
        let _ = label;
    }
    pub fn BeginHorizontal() {}
    pub fn EndHorizontal() {}
    pub fn BeginVertical() {}
    pub fn EndVertical() {}
    pub fn BeginScrollView(pos: [f32; 2]) -> [f32; 2] {
        pos
    }
    pub fn EndScrollView() {}
    pub fn GetLastRect() -> [f32; 4] {
        [0.0; 4]
    }

    // ── Actual egui rendering methods ──

    /// Auto-layout property field using egui (actual rendering).
    /// Unity: EditorGUILayout.PropertyField(SerializedProperty, GUIContent)
    pub fn PropertyFieldEgui(ui: &mut egui::Ui, label: &GUIContent, value: &mut String) -> bool {
        ui.horizontal(|ui| {
            ui.label(&label.text);
            ui.text_edit_singleline(value)
        })
        .inner
        .changed()
    }

    /// Auto-layout object field using egui (actual rendering).
    /// Unity: EditorGUILayout.ObjectField(GUIContent, Object, Type, bool)
    pub fn ObjectFieldEgui(
        ui: &mut egui::Ui,
        label: &GUIContent,
        obj_type: &str,
    ) -> Option<String> {
        ui.horizontal(|ui| {
            ui.label(&label.text);
            ui.label(format!("[{}]", obj_type))
        });
        None
    }

    /// Auto-layout popup using egui (actual rendering).
    /// Unity: EditorGUILayout.Popup(GUIContent, int, string[])
    pub fn PopupEgui(ui: &mut egui::Ui, label: &GUIContent, selected: &mut i32, options: &[&str]) {
        ui.horizontal(|ui| {
            ui.label(&label.text);
            egui::ComboBox::from_id_salt(format!("popup_{}", label.text))
                .selected_text(options.get(*selected as usize).copied().unwrap_or(""))
                .show_ui(ui, |ui| {
                    for (i, option) in options.iter().enumerate() {
                        ui.selectable_value(selected, i as i32, *option);
                    }
                });
        });
    }

    /// Auto-layout enum popup using egui (actual rendering).
    /// Unity: EditorGUILayout.EnumPopup(GUIContent, Enum)
    pub fn EnumPopupEgui(
        ui: &mut egui::Ui,
        label: &GUIContent,
        selected: &mut i32,
        options: &[&str],
    ) {
        Self::PopupEgui(ui, label, selected, options);
    }

    /// Auto-layout int field using egui (actual rendering).
    /// Unity: EditorGUILayout.IntField(GUIContent, int)
    pub fn IntFieldEgui(ui: &mut egui::Ui, label: &GUIContent, value: &mut i32) {
        ui.horizontal(|ui| {
            ui.label(&label.text);
            ui.add(egui::DragValue::new(value));
        });
    }

    /// Auto-layout float field using egui (actual rendering).
    /// Unity: EditorGUILayout.FloatField(GUIContent, float)
    pub fn FloatFieldEgui(ui: &mut egui::Ui, label: &GUIContent, value: &mut f32) {
        ui.horizontal(|ui| {
            ui.label(&label.text);
            ui.add(egui::DragValue::new(value));
        });
    }

    /// Auto-layout layer field using egui (actual rendering).
    /// Unity: EditorGUILayout.LayerField(GUIContent, int)
    pub fn LayerFieldEgui(ui: &mut egui::Ui, label: &GUIContent, layer: &mut i32) {
        let layers = [
            "Default",
            "TransparentFX",
            "Ignore Raycast",
            "Water",
            "UI",
            "Layer 5",
            "Layer 6",
            "Layer 7",
            "Layer 8",
            "Layer 9",
            "Layer 10",
            "Layer 11",
            "Layer 12",
            "Layer 13",
            "Layer 14",
            "Layer 15",
            "Layer 16",
            "Layer 17",
            "Layer 18",
            "Layer 19",
            "Layer 20",
            "Layer 21",
            "Layer 22",
            "Layer 23",
            "Layer 24",
            "Layer 25",
            "Layer 26",
            "Layer 27",
            "Layer 28",
            "Layer 29",
            "Layer 30",
            "Layer 31",
        ];
        Self::PopupEgui(ui, label, layer, &layers);
    }

    /// Auto-layout tag field using egui (actual rendering).
    /// Unity: EditorGUILayout.TagField(GUIContent, string)
    pub fn TagFieldEgui(ui: &mut egui::Ui, label: &GUIContent, tag: &mut String) {
        let tags = [
            "Untagged",
            "Respawn",
            "Finish",
            "EditorOnly",
            "MainCamera",
            "Player",
            "GameController",
        ];
        let selected = tags.iter().position(|&t| t == tag.as_str()).unwrap_or(0) as i32;
        let mut sel = selected;
        Self::PopupEgui(ui, label, &mut sel, &tags);
        if let Some(t) = tags.get(sel as usize) {
            *tag = t.to_string();
        }
    }

    /// Auto-layout foldout using egui (actual rendering).
    /// Unity: EditorGUILayout.Foldout(bool, string, bool)
    pub fn FoldoutEgui(ui: &mut egui::Ui, foldout: &mut bool, content: &GUIContent) {
        *foldout = ui
            .collapsing(&content.text, |_ui| {})
            .header_response
            .clicked();
    }

    /// Auto-layout help box using egui (actual rendering).
    /// Unity: EditorGUILayout.HelpBox(string, MessageType)
    pub fn HelpBoxEgui(ui: &mut egui::Ui, message: &str, message_type: i32) {
        let (icon, color) = match message_type {
            0 => ("ℹ", egui::Color32::BLUE),   // Info
            1 => ("⚠", egui::Color32::YELLOW), // Warning
            2 => ("❌", egui::Color32::RED),   // Error
            _ => ("ℹ", egui::Color32::GRAY),
        };
        ui.colored_label(color, format!("{} {}", icon, message));
    }

    /// Separator using egui (actual rendering).
    /// Unity: EditorGUILayout.Separator()
    pub fn SeparatorEgui(ui: &mut egui::Ui) {
        ui.separator();
    }

    /// Space using egui (actual rendering).
    /// Unity: EditorGUILayout.Space()
    pub fn SpaceEgui(ui: &mut egui::Ui) {
        ui.add_space(10.0);
    }

    /// Begin horizontal layout using egui (actual rendering).
    /// Unity: EditorGUILayout.BeginHorizontal()
    pub fn BeginHorizontalEgui(ui: &mut egui::Ui) -> egui::Ui {
        let mut child_ui = ui.child_ui(
            ui.max_rect(),
            egui::Layout::left_to_right(egui::Align::Min),
            None,
        );
        std::mem::swap(&mut child_ui, ui);
        child_ui
    }

    /// End horizontal layout using egui (actual rendering).
    /// Unity: EditorGUILayout.EndHorizontal()
    pub fn EndHorizontalEgui(ui: &mut egui::Ui, child_ui: &mut egui::Ui) {
        std::mem::swap(ui, child_ui);
    }

    /// Begin vertical layout using egui (actual rendering).
    /// Unity: EditorGUILayout.BeginVertical()
    pub fn BeginVerticalEgui(ui: &mut egui::Ui) -> egui::Ui {
        let mut child_ui = ui.child_ui(
            ui.max_rect(),
            egui::Layout::top_down(egui::Align::Min),
            None,
        );
        std::mem::swap(&mut child_ui, ui);
        child_ui
    }

    /// End vertical layout using egui (actual rendering).
    /// Unity: EditorGUILayout.EndVertical()
    pub fn EndVerticalEgui(ui: &mut egui::Ui, child_ui: &mut egui::Ui) {
        std::mem::swap(ui, child_ui);
    }

    /// Begin scroll view using egui (actual rendering).
    /// Unity: EditorGUILayout.BeginScrollView(Vector2)
    pub fn BeginScrollViewEgui(
        _ui: &mut egui::Ui,
        _scroll_position: &mut egui::Vec2,
    ) -> egui::ScrollArea {
        egui::ScrollArea::vertical().auto_shrink([false, false])
    }
}
