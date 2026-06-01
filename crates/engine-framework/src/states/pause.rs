use crate::action::GameStateAction;
use crate::{GameState, StateCtx};
use engine_input::input_manager::InputManager;
use engine_input::keyboard::KeyCode;

pub struct PauseState;

impl GameState for PauseState {
    fn on_enter(&mut self, _ctx: &mut StateCtx) {
        println!("╔══════════════════════════════╗");
        println!("║          PAUSED              ║");
        println!("╠══════════════════════════════╣");
        println!("║  [ESC] Resume                ║");
        println!("║  [Q]   Quit to Menu          ║");
        println!("╚══════════════════════════════╝");
    }

    fn on_exit(&mut self, _ctx: &mut StateCtx) {
        println!("Resuming...");
    }

    fn update(&mut self, ctx: &mut StateCtx, _dt: f32) {
        let action = {
            let input = ctx.world.get_resource::<InputManager>();
            match input {
                Some(input) if input.key_just_pressed(KeyCode::Escape) => GameStateAction::Pop,
                Some(input) if input.key_just_pressed(KeyCode::KeyQ) => GameStateAction::PushMenu,
                _ => return,
            }
        };
        ctx.resources.insert(action);
    }
}
