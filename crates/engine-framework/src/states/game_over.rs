use crate::action::GameStateAction;
use crate::{GameState, StateCtx};
use engine_input::input_manager::InputManager;
use engine_input::keyboard::KeyCode;

pub struct GameOverState {
    score: i32,
}

impl GameOverState {
    pub fn new(score: i32) -> Self {
        Self { score }
    }
}

impl GameState for GameOverState {
    fn on_enter(&mut self, _ctx: &mut StateCtx) {
        println!("╔══════════════════════════════╗");
        println!("║        GAME OVER             ║");
        println!("╠══════════════════════════════╣");
        println!("║  Final Score: {:<15}║", self.score);
        println!("║                              ║");
        println!("║  [R] Restart                 ║");
        println!("║  [Q] Quit to Menu            ║");
        println!("╚══════════════════════════════╝");
    }

    fn on_exit(&mut self, _ctx: &mut StateCtx) {
        println!("Leaving game over screen...");
    }

    fn update(&mut self, ctx: &mut StateCtx, _dt: f32) {
        let action = {
            let input = ctx.world.get_resource::<InputManager>();
            match input {
                Some(input) if input.key_just_pressed(KeyCode::KeyR) => GameStateAction::StartGame,
                Some(input)
                    if input.key_just_pressed(KeyCode::KeyQ)
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
