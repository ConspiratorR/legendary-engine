use engine_core::app::{App, AppBuilder};
use engine_core::debug::DebugPlugin;
use engine_core::engine::run_default;
use engine_core::plugin::Plugin;
use engine_framework::{FrameworkPlugin, GameState, StateCtx, StateStack};
use engine_ui::{EguiPlugin, EguiState, GuiSkin, ImGuiPlugin};
mod editor;
use editor::EditorLayout;

struct MenuState;

impl GameState for MenuState {
    fn on_enter(&mut self, _: &mut StateCtx) { println!("Menu entered"); }
    fn on_exit(&mut self, _: &mut StateCtx) { println!("Menu exited"); }
    fn update(&mut self, _: &mut StateCtx, _dt: f32) {}
}

struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_pre_update_hook(Box::new(|app: &mut App| {
            static PUSHED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            if !PUSHED.swap(true, std::sync::atomic::Ordering::Relaxed)
                && let Some(stack) = app.resources.get_mut::<StateStack>()
            {
                stack.push(Box::new(MenuState));
            }
        }));

        let mut editor = EditorLayout::new();
        app.add_post_update_hook(Box::new(move |app: &mut App| {
            let skin = app.resources.get::<GuiSkin>().cloned().unwrap_or_default();
            let egui_state = app.resources.get_mut::<EguiState>().unwrap();
            let ctx = egui_state.ctx();
            editor.frame(ctx, &skin);
        }));
    }
}

fn main() {
    let mut builder = AppBuilder::new();
    builder
        .add_plugin(FrameworkPlugin)
        .add_plugin(DebugPlugin)
        .add_plugin(EguiPlugin)
        .add_plugin(ImGuiPlugin)
        .add_plugin(GamePlugin);
    run_default(builder);
}
