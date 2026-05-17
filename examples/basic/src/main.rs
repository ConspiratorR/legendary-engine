use engine_core::app::{App, AppBuilder};
use engine_core::engine::run_default;
use engine_core::plugin::Plugin;
use engine_framework::{FrameworkPlugin, GameState, StateCtx, StateStack};
use engine_ui::{EguiPlugin, EguiState, GuiSkin, ImGuiPlugin, Gui, GuiLayout};
use egui::{Rect, Pos2, Vec2};

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
            if !PUSHED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                if let Some(stack) = app.resources.get_mut::<StateStack>() {
                    stack.push(Box::new(MenuState));
                }
            }
        }));

        let mut visible = true;
        let mut opacity = 1.0f32;
        let mut panel_rect = Rect::from_min_size(Pos2::new(10.0, 40.0), Vec2::new(250.0, 300.0));

        app.add_post_update_hook(Box::new(move |app: &mut App| {
            let skin = app.resources.get::<GuiSkin>().cloned().unwrap_or_default();
            let egui_state = app.resources.get_mut::<EguiState>().unwrap();
            let ctx = egui_state.ctx();

            egui::Area::new(egui::Id::new("gui_root"))
                .fixed_pos(Pos2::ZERO)
                .show(ctx, |ui| {
                    let screen = ui.ctx().screen_rect();
                    let mut gui = Gui::new(ui, &skin);
                    gui.box_(Rect::from_min_size(screen.left_top(), Vec2::new(screen.width(), 30.0)), "RustEngine IMGUI Demo");
                });

            GuiLayout::new(ctx, &skin).window("Inspector", &mut panel_rect, |v| {
                v.label("Position:");
                v.horizontal(|h| {
                    h.label("X:");
                    h.text_field("0.0", 60.0);
                });
                v.separator();
                v.label("Visible:");
                v.toggle(&mut visible, "Show Grid");
                v.separator();
                v.label("Opacity:");
                v.slider(&mut opacity, 0.0, 1.0, 200.0);
                v.separator();
                if v.button("Apply") {
                    println!("Apply clicked!");
                }
            });
        }));
    }
}

fn main() {
    let mut builder = AppBuilder::new();
    builder
        .add_plugin(FrameworkPlugin)
        .add_plugin(EguiPlugin)
        .add_plugin(ImGuiPlugin)
        .add_plugin(GamePlugin);
    run_default(builder);
}
