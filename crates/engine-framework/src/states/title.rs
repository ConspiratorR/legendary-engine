use crate::action::GameStateAction;
use crate::{GameState, StateCtx};
use engine_input::input_manager::InputManager;
use engine_input::keyboard::KeyCode;

pub struct TitleState;

impl GameState for TitleState {
    fn on_enter(&mut self, _ctx: &mut StateCtx) {
        println!("╔══════════════════════════════╗");
        println!("║       RUST ENGINE            ║");
        println!("║                              ║");
        println!("║    Press any key to start    ║");
        println!("╚══════════════════════════════╝");
    }

    fn on_exit(&mut self, _ctx: &mut StateCtx) {
        println!("Leaving title screen...");
    }

    fn update(&mut self, ctx: &mut StateCtx, _dt: f32) {
        let action = {
            let input = ctx.world.get_resource::<InputManager>();
            match input {
                Some(input)
                    if input.key_just_pressed(KeyCode::Space)
                        || input.key_just_pressed(KeyCode::Enter)
                        || input.key_just_pressed(KeyCode::Escape) =>
                {
                    GameStateAction::PushMenu
                }
                _ => return,
            }
        };
        ctx.resources.insert(action);
    }
}
