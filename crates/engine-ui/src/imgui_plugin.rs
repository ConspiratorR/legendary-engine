use crate::GuiSkin;
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;

/// Plugin that registers a default [`GuiSkin`] resource.
///
/// Use this when you want the skinned immediate-mode GUI without the
/// full egui rendering pipeline.
pub struct ImGuiPlugin;

impl Plugin for ImGuiPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(GuiSkin::default());
    }
}
