use crate::action::GameStateAction;
use crate::{GameState, StateCtx};
use engine_input::input_manager::InputManager;
use engine_input::keyboard::KeyCode;

/// The main menu state.
///
/// Offers New Game and Quit options. Transitions to gameplay on `[1]`
/// or requests quit on `[2]` / Escape.
pub struct MenuState;

impl GameState for MenuState {
    fn on_enter(&mut self, _ctx: &mut StateCtx) {
        println!("╔══════════════════════════════╗");
        println!("║         MAIN MENU            ║");
        println!("╠══════════════════════════════╣");
        println!("║  [1] New Game                ║");
        println!("║  [2] Quit                    ║");
        println!("╚══════════════════════════════╝");
    }

    fn on_exit(&mut self, _ctx: &mut StateCtx) {
        println!("Leaving menu...");
    }

    fn update(&mut self, ctx: &mut StateCtx, _dt: f32) {
        let action = {
            let input = ctx.world.get_resource::<InputManager>();
            match input {
                Some(input) if input.key_just_pressed(KeyCode::Digit1) => {
                    GameStateAction::StartGame
                }
                Some(input)
                    if input.key_just_pressed(KeyCode::Digit2)
                        || input.key_just_pressed(KeyCode::Escape) =>
                {
                    GameStateAction::Quit
                }
                _ => return,
            }
        };
        ctx.resources.insert(action);
    }
}
