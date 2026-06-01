use crate::state::EditorState;
use engine_core::app::{App, AppBuilder};
use engine_core::plugin::Plugin;
use engine_ui::EguiState;
use engine_ui::GuiSkin;

/// Plugin that registers the editor state and hooks the editor UI into
/// the post-update phase.
pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(EditorState::new());
        app.add_post_update_hook(Box::new(|app: &mut App| {
            let skin = app.resources.get::<GuiSkin>().cloned().unwrap_or_default();
            let ctx = match app.resources.get::<EguiState>() {
                Some(s) => s.ctx().clone(),
                None => return,
            };
            let state = match app.resources.get_mut::<EditorState>() {
                Some(s) => s,
                None => return,
            };
            state.frame(&ctx, &skin);
        }));
    }
}
