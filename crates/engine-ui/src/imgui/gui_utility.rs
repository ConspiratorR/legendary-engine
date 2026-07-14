//! GUIUtility class (matches Unity's GUIUtility).

pub struct GUIUtility;

impl GUIUtility {
    pub fn GetControlID(hint: i32) -> i32 {
        hint
    }
    pub fn GetControlIDWithFocus(hint: i32, focus_type: i32) -> i32 {
        let _ = focus_type;
        hint
    }
    pub fn hotControl() -> i32 {
        0
    }
    pub fn SetHotControl(id: i32) {
        let _ = id;
    }
    pub fn keyboardControl() -> i32 {
        0
    }
    pub fn SetKeyboardControl(id: i32) {
        let _ = id;
    }
    pub fn ExitGUI() {}
    pub fn ScreenToGUIPoint(p: [f32; 2]) -> [f32; 2] {
        p
    }
    pub fn GUIToScreenPoint(p: [f32; 2]) -> [f32; 2] {
        p
    }
    pub fn RotateAroundPivot(angle: f32, pivot: [f32; 2]) {
        let _ = (angle, pivot);
    }
    pub fn ScaleAroundPivot(scale: [f32; 2], pivot: [f32; 2]) {
        let _ = (scale, pivot);
    }
}
