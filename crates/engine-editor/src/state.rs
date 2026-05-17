use egui::Context;
use engine_ui::GuiSkin;

pub struct EditorState;

impl EditorState {
    pub fn new() -> Self {
        Self
    }
    pub fn frame(&mut self, _ctx: &Context, _skin: &GuiSkin) {}
}
