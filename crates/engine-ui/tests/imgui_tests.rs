use engine_ui::imgui::gui_skin::GUISkin;

#[test]
fn test_gui_skin_default() {
    let skin = GUISkin::default();
    assert!(skin.font.is_empty());
}

#[test]
fn test_gui_skin_find_style() {
    let skin = GUISkin::default();
    assert!(skin.FindStyle("Button").is_some());
    assert!(skin.FindStyle("label").is_some());
    assert!(skin.FindStyle("nonexistent").is_none());
}

use engine_ui::imgui::gui_style::{FontStyle, GUIStyle, TextAnchor};

#[test]
fn test_gui_style_default() {
    let s = GUIStyle::default();
    assert_eq!(s.alignment, TextAnchor::UpperLeft);
    assert_eq!(s.font_style, FontStyle::Normal);
    assert!(!s.word_wrap);
}

#[test]
fn test_gui_style_calc_size() {
    let mut s = GUIStyle::default();
    s.padding = [5.0, 5.0, 5.0, 5.0];
    let size = s.CalcSize([100.0, 20.0]);
    assert_eq!(size, [110.0, 30.0]);
}

#[test]
fn test_gui_style_fixed_size() {
    let mut s = GUIStyle::default();
    s.fixed_width = 200.0;
    s.fixed_height = 50.0;
    let size = s.CalcSize([100.0, 20.0]);
    assert_eq!(size, [200.0, 50.0]);
}

#[test]
fn test_gui_style_content_rect() {
    let s = GUIStyle::default();
    let rect = s.ContentRect([10.0, 20.0, 100.0, 50.0]);
    assert_eq!(rect, [10.0, 20.0, 100.0, 50.0]);
}

use engine_ui::imgui::gui_event::{Event, EventType, KeyCode};

#[test]
fn test_event_default() {
    let e = Event::default();
    assert_eq!(e.event_type, EventType::Repaint);
    assert!(!e.used);
}

#[test]
fn test_event_use() {
    let mut e = Event::default();
    e.Use();
    assert!(e.used);
    assert_eq!(e.event_type, EventType::Used);
}

#[test]
fn test_event_is_mouse() {
    let mut e = Event::default();
    e.event_type = EventType::MouseDown;
    assert!(e.IsMouse());
    e.event_type = EventType::KeyDown;
    assert!(!e.IsMouse());
}

#[test]
fn test_event_modifiers() {
    let mut e = Event::default();
    e.shift = true;
    e.control = true;
    assert!(e.IsShiftKeyDown());
    assert!(e.IsControlKeyDown());
    assert!(!e.IsAltKeyDown());
}

use engine_ui::imgui::gui_content::GUIContent;
use engine_ui::imgui::gui_layout::{GUILayout, LayoutOption};

#[test]
fn test_gUILayout_label() {
    GUILayout::Label(&GUIContent::new("Hello"));
}

#[test]
fn test_gUILayout_button() {
    let result = GUILayout::Button(&GUIContent::new("Click"));
    assert!(!result);
}

#[test]
fn test_gUILayout_options() {
    assert!(matches!(
        GUILayout::Width(200.0),
        LayoutOption::Width(200.0)
    ));
    assert!(matches!(
        GUILayout::Height(30.0),
        LayoutOption::Height(30.0)
    ));
}

#[test]
fn test_gUILayout_groups() {
    GUILayout::BeginHorizontal();
    GUILayout::Label(&GUIContent::new("Left"));
    GUILayout::EndHorizontal();
    GUILayout::BeginVertical();
    GUILayout::Label(&GUIContent::new("Top"));
    GUILayout::EndVertical();
}

#[test]
fn test_gUILayout_spacing() {
    GUILayout::Space(10.0);
    GUILayout::FlexibleSpace();
}

use engine_ui::imgui::gui::{GUI, GUIWindow};

#[test]
fn test_gui_label() {
    GUI::Label([0.0, 0.0, 100.0, 20.0], &GUIContent::new("Hello"));
}

#[test]
fn test_gui_button() {
    let result = GUI::Button([0.0, 0.0, 100.0, 30.0], &GUIContent::new("Click"));
    assert!(!result);
}

#[test]
fn test_gui_toggle() {
    let result = GUI::Toggle([0.0, 0.0, 100.0, 20.0], false, &GUIContent::new("On"));
    assert!(!result);
}

#[test]
fn test_gui_window() {
    let win = GUIWindow::new(1, [10.0, 10.0, 200.0, 100.0], "Test Window");
    assert_eq!(win.id, 1);
    assert_eq!(win.title, "Test Window");
}

use engine_ui::imgui::editor_gui_layout::EditorGUILayout;

#[test]
fn test_editor_gui_layout_property_field() {
    let result = EditorGUILayout::PropertyField(&GUIContent::new("Name"), "Default");
    assert_eq!(result, "Default");
}

#[test]
fn test_editor_gui_layout_foldout() {
    let result = EditorGUILayout::Foldout(false, &GUIContent::new("Section"));
    assert!(!result);
}

#[test]
fn test_editor_gui_layout_popup() {
    let result = EditorGUILayout::Popup(&GUIContent::new("Choice"), 0, &["A", "B", "C"]);
    assert_eq!(result, 0);
}

#[test]
fn test_editor_gui_layout_egui_property_field() {
    use engine_ui::imgui::editor_gui_layout::EditorGUILayout;
    use engine_ui::imgui::gui_content::GUIContent;
    let _: fn(&mut egui::Ui, &GUIContent, &mut String) -> bool = EditorGUILayout::PropertyFieldEgui;
}

#[test]
fn test_editor_gui_layout_egui_popup() {
    use engine_ui::imgui::editor_gui_layout::EditorGUILayout;
    use engine_ui::imgui::gui_content::GUIContent;
    let _: fn(&mut egui::Ui, &GUIContent, &mut i32, &[&str]) = EditorGUILayout::PopupEgui;
}

use engine_ui::imgui::gui_utility::GUIUtility;

#[test]
fn test_gui_utility_get_control_id() {
    let id = GUIUtility::GetControlID(0);
    assert_eq!(id, 0);
}

#[test]
fn test_gui_utility_screen_to_gui() {
    let gui = GUIUtility::ScreenToGUIPoint([100.0, 200.0]);
    assert_eq!(gui, [100.0, 200.0]);
}

#[test]
fn test_editor_gui_layout_separator() {
    EditorGUILayout::Separator();
}

#[test]
fn test_gUILayout_scroll_area() {
    use engine_ui::imgui::gui_layout::GUILayout;
    let _ = GUILayout::ScrollAreaVertical();
    let _ = GUILayout::ScrollAreaHorizontal();
    let _ = GUILayout::ScrollAreaBoth();
}

use engine_math::Vec3;
use engine_ui::imgui::handles::Handles;

#[test]
fn test_handles_wire_sphere() {
    Handles::WireSphere(Vec3::ZERO, 1.0);
}

#[test]
fn test_handles_draw_line() {
    Handles::DrawLine(Vec3::ZERO, Vec3::X);
}

#[test]
fn test_handles_label() {
    Handles::Label(Vec3::ZERO, "Test");
}

#[test]
fn test_handles_position_handle() {
    let pos = Handles::PositionHandle(Vec3::ZERO, engine_math::Quat::IDENTITY);
    assert_eq!(pos, Vec3::ZERO);
}

#[test]
fn test_handles_egui_methods_exist() {
    // Compile-time checks: verify all Egui methods have the expected signatures.
    let _: fn(&egui::Painter, [f32; 2], f32, egui::Color32) = Handles::WireSphereEgui;
    let _: fn(&egui::Painter, [f32; 2], [f32; 2], egui::Color32) = Handles::WireCubeEgui;
    let _: fn(&egui::Painter, [f32; 2], [f32; 2], egui::Color32) = Handles::DrawLineEgui;
    let _: fn(&egui::Painter, [f32; 2], [f32; 2], egui::Color32, f32) = Handles::DrawDottedLineEgui;
    let _: fn(&egui::Painter, [f32; 2], &str, egui::Color32) = Handles::LabelEgui;
    let _: fn(&egui::Painter, [f32; 2], f32, egui::Color32) = Handles::WireDiscEgui;
    let _: fn(&egui::Painter, [f32; 2], f32, f32, f32, egui::Color32) = Handles::ArcEgui;
}

use engine_ui::imgui::text_editor::TextEditor;

#[test]
fn test_text_editor_new() {
    let te = TextEditor::new();
    assert!(te.text.is_empty());
    assert_eq!(te.cursor_index, 0);
}

#[test]
fn test_text_editor_insert() {
    let mut te = TextEditor::new();
    te.Insert("Hello");
    assert_eq!(te.text, "Hello");
    assert_eq!(te.cursor_index, 5);
}

#[test]
fn test_text_editor_copy_paste() {
    let mut te = TextEditor::new();
    te.Insert("Hello World");
    te.select_index = 0;
    te.cursor_index = 5;
    let copied = te.Copy();
    assert_eq!(copied.as_deref(), Some("Hello"));

    te.MoveToEnd();
    te.Paste("!");
    assert_eq!(te.text, "Hello World!");
}

#[test]
fn test_text_editor_select_all() {
    let mut te = TextEditor::new();
    te.Insert("Test");
    te.SelectAll();
    assert!(te.HasSelection());
    assert_eq!(te.SelectedText(), "Test");
}

#[test]
fn test_text_editor_backspace() {
    let mut te = TextEditor::new();
    te.Insert("ABC");
    te.Backspace();
    assert_eq!(te.text, "AB");
    assert_eq!(te.cursor_index, 2);
}

#[test]
fn test_text_editor_cursor_position() {
    let mut te = TextEditor::new();
    te.Insert("Line1\nLine2\nLine3");
    te.cursor_index = 11;
    let (line, col) = te.CursorPosition();
    assert_eq!(line, 1);
    assert_eq!(col, 5);
}

#[test]
fn test_gui_egui_methods_exist() {
    use engine_ui::imgui::gui::GUI;
    use engine_ui::imgui::gui_content::GUIContent;
    let _: fn(&mut egui::Ui, &GUIContent) = GUI::LabelEgui;
    let _: fn(&mut egui::Ui, &GUIContent) -> bool = GUI::ButtonEgui;
    let _: fn(&mut egui::Ui, &mut bool, &GUIContent) = GUI::ToggleEgui;
    let _: fn(&mut egui::Ui, &mut String) -> bool = GUI::TextFieldEgui;
    let _: fn(&mut egui::Ui, &mut String) -> bool = GUI::TextAreaEgui;
    let _: fn(&mut egui::Ui, &mut f32, f32, f32) = GUI::HorizontalSliderEgui;
    let _: fn(&mut egui::Ui) = GUI::SeparatorEgui;
    let _: fn(&mut egui::Ui, &GUIContent) = GUI::BoxEgui;
    let _: fn(&mut egui::Ui, &mut i32, &[&str]) = GUI::ToolbarEgui;
}

#[test]
fn test_gui_layout_egui_methods_exist() {
    use engine_ui::imgui::gui_content::GUIContent;
    use engine_ui::imgui::gui_layout::GUILayout;
    let _: fn(&mut egui::Ui, &GUIContent) = GUILayout::LabelEgui;
    let _: fn(&mut egui::Ui, &GUIContent) -> bool = GUILayout::ButtonEgui;
    let _: fn(&mut egui::Ui, &mut bool, &GUIContent) = GUILayout::ToggleEgui;
    let _: fn(&mut egui::Ui, &mut String) -> bool = GUILayout::TextFieldEgui;
    let _: fn(&mut egui::Ui, &mut String) -> bool = GUILayout::TextAreaEgui;
    let _: fn(&mut egui::Ui, &mut f32, f32, f32) = GUILayout::HorizontalSliderEgui;
    let _: fn(&mut egui::Ui, &mut i32, &[&str]) = GUILayout::ToolbarEgui;
}

#[test]
fn test_event_from_egui_pointer() {
    let egui_event = egui::Event::PointerButton {
        pos: egui::Pos2::new(100.0, 200.0),
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    };
    let event = Event::from_egui_event(&egui_event, [0.0, 0.0]);
    assert!(event.is_some());
    let e = event.unwrap();
    assert_eq!(e.event_type, EventType::MouseDown);
    assert!((e.mouse_position[0] - 100.0).abs() < 0.001);
    assert!((e.mouse_position[1] - 200.0).abs() < 0.001);
}

#[test]
fn test_event_from_egui_pointer_move() {
    let egui_event = egui::Event::PointerMoved(egui::Pos2::new(50.0, 75.0));
    let event = Event::from_egui_event(&egui_event, [0.0, 0.0]);
    assert!(event.is_some());
    let e = event.unwrap();
    assert_eq!(e.event_type, EventType::MouseMove);
    assert!((e.mouse_position[0] - 50.0).abs() < 0.001);
}

#[test]
fn test_event_from_egui_key() {
    let egui_event = egui::Event::Key {
        key: egui::Key::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    let event = Event::from_egui_event(&egui_event, [0.0, 0.0]);
    assert!(event.is_some());
    let e = event.unwrap();
    assert_eq!(e.event_type, EventType::KeyDown);
    assert_eq!(e.key_code, KeyCode::A);
}

#[test]
fn test_event_from_egui_scroll() {
    let egui_event = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, -10.0),
        modifiers: egui::Modifiers::NONE,
    };
    let event = Event::from_egui_event(&egui_event, [50.0, 50.0]);
    assert!(event.is_some());
    let e = event.unwrap();
    assert_eq!(e.event_type, EventType::ScrollWheel);
    assert!((e.mouse_position[0] - 50.0).abs() < 0.001);
}

#[test]
fn test_event_from_egui_text() {
    let egui_event = egui::Event::Text("hello".to_string());
    let event = Event::from_egui_event(&egui_event, [0.0, 0.0]);
    assert!(event.is_some());
    let e = event.unwrap();
    assert_eq!(e.event_type, EventType::KeyDown);
    assert_eq!(e.character, Some('h'));
}

#[test]
fn test_gui_extended_methods() {
    use engine_ui::imgui::gui::GUI;
    GUI::DrawRect([0.0, 0.0, 100.0, 100.0], [1.0, 0.0, 0.0, 1.0]);
    GUI::DrawBorder([0.0, 0.0, 100.0, 100.0], [0.0, 0.0, 0.0, 1.0], 1.0);
    GUI::DrawText([0.0, 0.0, 100.0, 20.0], "Hello", [1.0, 1.0, 1.0, 1.0], 14.0);
    assert_eq!(GUI::GetAvailableRect(), [0.0, 0.0, 0.0, 0.0]);
    assert_eq!(GUI::GetMousePosition(), [0.0, 0.0]);
    assert!(!GUI::MouseOverRect([0.0, 0.0, 100.0, 100.0]));
    assert!(!GUI::RectClicked([0.0, 0.0, 100.0, 100.0]));
    assert!(!GUI::RectDoubleClicked([0.0, 0.0, 100.0, 100.0]));
    assert!(!GUI::RectDragStarted([0.0, 0.0, 100.0, 100.0]));
    assert_eq!(GUI::GetDragDelta(), [0.0, 0.0]);
}

#[test]
fn test_event_from_pointer_state() {
    let mut state = egui::PointerState::default();
    let event = Event::from_pointer_state(&state, [100.0, 200.0]);
    assert_eq!(event.event_type, EventType::MouseMove);
    assert!((event.mouse_position[0] - 100.0).abs() < 0.001);
}

#[test]
fn test_panels_types() {
    use engine_ui::imgui::panels::{Side, TopBottom};
    assert_eq!(Side::Left, Side::Left);
    assert_eq!(Side::Right, Side::Right);
    assert_eq!(TopBottom::Top, TopBottom::Top);
    assert_eq!(TopBottom::Bottom, TopBottom::Bottom);
}

#[test]
fn test_gUILayout_egui_label() {
    use engine_ui::imgui::gui_content::GUIContent;
    use engine_ui::imgui::gui_layout::GUILayout;
    let _: fn(&mut egui::Ui, &GUIContent) = GUILayout::LabelEgui;
}

#[test]
fn test_gUILayout_egui_button() {
    use engine_ui::imgui::gui_content::GUIContent;
    use engine_ui::imgui::gui_layout::GUILayout;
    let _: fn(&mut egui::Ui, &GUIContent) -> bool = GUILayout::ButtonEgui;
}

#[test]
fn test_gUILayout_egui_toggle() {
    use engine_ui::imgui::gui_content::GUIContent;
    use engine_ui::imgui::gui_layout::GUILayout;
    let _: fn(&mut egui::Ui, &mut bool, &GUIContent) = GUILayout::ToggleEgui;
}

#[test]
fn test_gUILayout_egui_text_area() {
    use engine_ui::imgui::gui_layout::GUILayout;
    let _: fn(&mut egui::Ui, &mut String) -> bool = GUILayout::TextAreaEgui;
}

#[test]
fn test_gUILayout_egui_toolbar() {
    use engine_ui::imgui::gui_layout::GUILayout;
    let _: fn(&mut egui::Ui, &mut i32, &[&str]) = GUILayout::ToolbarEgui;
}

#[test]
fn test_event_from_egui_input() {
    use engine_ui::imgui::gui_event::Event;
    // Just verify the method compiles
    let _: fn(&egui::InputState) -> Event = Event::from_egui_input;
}
