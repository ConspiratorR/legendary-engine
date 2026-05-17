use engine_core::app::{App, AppBuilder};
use engine_core::debug::DebugPlugin;
use engine_core::engine::run_default;
use engine_core::plugin::Plugin;
use engine_editor::EditorPlugin;
use engine_framework::{FrameworkPlugin, GameState, StateCtx, StateStack};
use engine_ui::{EguiPlugin, ImGuiPlugin};

struct MenuState;

impl GameState for MenuState {
    fn on_enter(&mut self, _: &mut StateCtx) {
        println!("Menu entered");
    }
    fn on_exit(&mut self, _: &mut StateCtx) {
        println!("Menu exited");
    }
    fn update(&mut self, _: &mut StateCtx, _dt: f32) {}
}

struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_pre_update_hook(Box::new(|app: &mut App| {
            static PUSHED: std::sync::atomic::AtomicBool =
                std::sync::atomic::AtomicBool::new(false);
            if !PUSHED.swap(true, std::sync::atomic::Ordering::Relaxed)
                && let Some(stack) = app.resources.get_mut::<StateStack>()
            {
                stack.push(Box::new(MenuState));
            }
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
        .add_plugin(EditorPlugin)
        .add_plugin(GamePlugin);
    run_default(builder);
}
