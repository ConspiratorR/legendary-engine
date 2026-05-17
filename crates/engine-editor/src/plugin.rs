use engine_core::app::{App, AppBuilder};
use engine_core::plugin::Plugin;
use engine_ui::EguiState;
use engine_ui::GuiSkin;
use crate::state::EditorState;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(EditorState::new());
        app.add_post_update_hook(Box::new(|app: &mut App| {
            let skin = app.resources.get::<GuiSkin>().cloned().unwrap_or_default();
            let ctx = app.resources.get::<EguiState>().unwrap().ctx().clone();
            let state = app.resources.get_mut::<EditorState>().unwrap();
            state.frame(&ctx, &skin);
        }));
    }
}
