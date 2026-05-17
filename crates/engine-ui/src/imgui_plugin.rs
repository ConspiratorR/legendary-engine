use crate::GuiSkin;
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;

pub struct ImGuiPlugin;

impl Plugin for ImGuiPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(GuiSkin::default());
    }
}
