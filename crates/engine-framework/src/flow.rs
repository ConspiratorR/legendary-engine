use crate::StateStack;
use crate::action::{GameSession, GameStateAction};
use crate::states::{GameOverState, MenuState, PauseState, TitleState};
use engine_core::app::{App, AppBuilder};
use engine_core::plugin::Plugin;

/// Plugin that wires the standard game-flow state machine.
///
/// Registers [`GameStateAction`] and [`GameSession`] resources, pushes
/// the initial [`TitleState`], and installs a post-update hook that
/// processes queued state transitions.
pub struct GameFlowPlugin;

impl Plugin for GameFlowPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.resources_mut().insert(GameStateAction::None);
        app.world_mut().insert_resource(GameSession::new());

        if let Some(stack) = app.resources_mut().get_mut::<StateStack>() {
            stack.push(Box::new(TitleState));
        }

        app.add_post_update_hook(Box::new(|app: &mut App| {
            let action = app
                .resources_mut()
                .get_mut::<GameStateAction>()
                .map(std::mem::take)
                .unwrap_or(GameStateAction::None);

            match action {
                GameStateAction::None => {}
                GameStateAction::StartGame => {
                    if let Some(session) = app.world.get_resource_mut::<GameSession>() {
                        session.reset();
                        session.is_running = true;
                    }
                    if let Some(stack) = app.resources_mut().get_mut::<StateStack>() {
                        stack.pop();
                    }
                    println!("Starting new game...");
                }
                GameStateAction::PushMenu => {
                    if let Some(stack) = app.resources_mut().get_mut::<StateStack>() {
                        stack.replace(Box::new(MenuState));
                    }
                }
                GameStateAction::PushPause => {
                    if let Some(stack) = app.resources_mut().get_mut::<StateStack>() {
                        stack.push(Box::new(PauseState));
                    }
                }
                GameStateAction::PushGameOver { score } => {
                    if let Some(session) = app.world.get_resource_mut::<GameSession>() {
                        session.is_running = false;
                    }
                    if let Some(stack) = app.resources_mut().get_mut::<StateStack>() {
                        stack.replace(Box::new(GameOverState::new(score)));
                    }
                }
                GameStateAction::PushTitle => {
                    if let Some(stack) = app.resources_mut().get_mut::<StateStack>() {
                        stack.replace(Box::new(TitleState));
                    }
                }
                GameStateAction::Pop => {
                    if let Some(stack) = app.resources_mut().get_mut::<StateStack>() {
                        stack.pop();
                    }
                }
                GameStateAction::Quit => {
                    if let Some(session) = app.world.get_resource_mut::<GameSession>() {
                        session.quit_requested = true;
                    }
                }
            }
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::resource::ResourceRegistry;
    use engine_ecs::world::World;

    #[test]
    fn test_game_session_default() {
        let session = GameSession::new();
        assert_eq!(session.score, 0);
        assert!(!session.is_running);
        assert!(!session.quit_requested);
    }

    #[test]
    fn test_game_session_reset() {
        let mut session = GameSession::new();
        session.score = 100;
        session.is_running = true;
        session.quit_requested = true;
        session.reset();
        assert_eq!(session.score, 0);
        assert!(!session.is_running);
        assert!(!session.quit_requested);
    }

    #[test]
    fn test_game_state_action_default() {
        let action = GameStateAction::default();
        assert!(matches!(action, GameStateAction::None));
    }

    #[test]
    fn test_menu_state_pushes_on_stack() {
        let mut w = World::new();
        let mut r = ResourceRegistry::new();
        let mut stack = StateStack::new();
        stack.push(Box::new(MenuState));
        stack.flush(&mut w, &mut r);
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn test_pause_state_pushes_on_top() {
        let mut w = World::new();
        let mut r = ResourceRegistry::new();
        let mut stack = StateStack::new();
        stack.push(Box::new(MenuState));
        stack.flush(&mut w, &mut r);
        stack.push(Box::new(PauseState));
        stack.flush(&mut w, &mut r);
        assert_eq!(stack.len(), 2);
    }

    #[test]
    fn test_pop_removes_pause_state() {
        let mut w = World::new();
        let mut r = ResourceRegistry::new();
        let mut stack = StateStack::new();
        stack.push(Box::new(MenuState));
        stack.flush(&mut w, &mut r);
        stack.push(Box::new(PauseState));
        stack.flush(&mut w, &mut r);
        assert_eq!(stack.len(), 2);
        stack.pop();
        stack.flush(&mut w, &mut r);
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn test_replace_menu_with_game_over() {
        let mut w = World::new();
        let mut r = ResourceRegistry::new();
        let mut stack = StateStack::new();
        stack.push(Box::new(MenuState));
        stack.flush(&mut w, &mut r);
        assert_eq!(stack.len(), 1);
        stack.replace(Box::new(GameOverState::new(42)));
        stack.flush(&mut w, &mut r);
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn test_game_flow_plugin_initializes_menu() {
        let mut app = AppBuilder::new();
        app.add_plugin(crate::FrameworkPlugin);
        app.add_plugin(GameFlowPlugin);
        let mut app = app.build();

        let stack = app.resources_mut().get::<StateStack>();
        assert!(stack.is_some());
        assert_eq!(stack.unwrap().len(), 0);
        app.run();
        let stack = app.resources_mut().get::<StateStack>();
        assert!(stack.is_some());
        assert_eq!(stack.unwrap().len(), 1);
    }

    #[test]
    fn test_game_flow_start_game_pops_menu() {
        let mut app = AppBuilder::new();
        app.add_plugin(crate::FrameworkPlugin);
        app.add_plugin(GameFlowPlugin);
        let mut app = app.build();

        // First run: flush pushes MenuState onto stack
        app.run();
        assert_eq!(
            app.resources_mut().get::<StateStack>().map(|s| s.len()),
            Some(1)
        );

        // Insert StartGame action; post-update hook queues a pop
        app.resources_mut().insert(GameStateAction::StartGame);
        app.run();

        let session = app.world.get_resource::<GameSession>().unwrap();
        assert!(session.is_running);

        // Pop is pending; one more run to flush it
        app.run();
        let stack = app.resources_mut().get::<StateStack>().unwrap();
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_game_flow_quit_sets_flag() {
        let mut app = AppBuilder::new();
        app.add_plugin(crate::FrameworkPlugin);
        app.add_plugin(GameFlowPlugin);
        let mut app = app.build();

        app.resources_mut().insert(GameStateAction::Quit);
        app.run();

        let session = app.world.get_resource::<GameSession>().unwrap();
        assert!(session.quit_requested);
    }

    #[test]
    fn test_title_state_pushes_on_stack() {
        let mut w = World::new();
        let mut r = ResourceRegistry::new();
        let mut stack = StateStack::new();
        stack.push(Box::new(TitleState));
        stack.flush(&mut w, &mut r);
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn test_push_title_replaces_state() {
        let mut w = World::new();
        let mut r = ResourceRegistry::new();
        let mut stack = StateStack::new();
        stack.push(Box::new(MenuState));
        stack.flush(&mut w, &mut r);
        assert_eq!(stack.len(), 1);
        stack.replace(Box::new(TitleState));
        stack.flush(&mut w, &mut r);
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn test_game_flow_push_title_replaces_state() {
        let mut app = AppBuilder::new();
        app.add_plugin(crate::FrameworkPlugin);
        app.add_plugin(GameFlowPlugin);
        let mut app = app.build();

        // Start with TitleState
        app.run();
        assert_eq!(
            app.resources_mut().get::<StateStack>().map(|s| s.len()),
            Some(1)
        );

        // Push menu via PushMenu action
        app.resources_mut().insert(GameStateAction::PushMenu);
        app.run();
        assert_eq!(
            app.resources_mut().get::<StateStack>().map(|s| s.len()),
            Some(1)
        );

        // PushTitle should replace with TitleState
        app.resources_mut().insert(GameStateAction::PushTitle);
        app.run();
        assert_eq!(
            app.resources_mut().get::<StateStack>().map(|s| s.len()),
            Some(1)
        );
    }

    #[test]
    fn test_game_flow_full_title_to_game_cycle() {
        let mut app = AppBuilder::new();
        app.add_plugin(crate::FrameworkPlugin);
        app.add_plugin(GameFlowPlugin);
        let mut app = app.build();

        // Start: TitleState on stack (flush initial pending push)
        app.run();
        assert_eq!(
            app.resources_mut().get::<StateStack>().map(|s| s.len()),
            Some(1)
        );

        // Title -> Menu: PushMenu queues Replace(MenuState)
        app.resources_mut().insert(GameStateAction::PushMenu);
        app.run();
        // Flush the Replace
        app.run();
        assert_eq!(
            app.resources_mut().get::<StateStack>().map(|s| s.len()),
            Some(1)
        );

        // Menu -> Start game: StartGame queues Pop and sets is_running
        app.resources_mut().insert(GameStateAction::StartGame);
        app.run();
        let session = app.world.get_resource::<GameSession>().unwrap();
        assert!(session.is_running);
        // Flush the Pop
        app.run();
        assert_eq!(
            app.resources_mut().get::<StateStack>().map(|s| s.len()),
            Some(0)
        );

        // Game -> Pause: PushPause queues Push(PauseState)
        app.resources_mut().insert(GameStateAction::PushPause);
        app.run();
        // Flush the Push
        app.run();
        assert_eq!(
            app.resources_mut().get::<StateStack>().map(|s| s.len()),
            Some(1)
        );

        // Pause -> Resume: Pop queues Pop
        app.resources_mut().insert(GameStateAction::Pop);
        app.run();
        // Flush the Pop
        app.run();
        assert_eq!(
            app.resources_mut().get::<StateStack>().map(|s| s.len()),
            Some(0)
        );

        // Game -> GameOver: PushGameOver queues Replace(GameOverState)
        app.resources_mut()
            .insert(GameStateAction::PushGameOver { score: 100 });
        app.run();
        let session = app.world.get_resource::<GameSession>().unwrap();
        assert!(!session.is_running);
        // Flush the Replace
        app.run();
        assert_eq!(
            app.resources_mut().get::<StateStack>().map(|s| s.len()),
            Some(1)
        );

        // GameOver -> Menu: PushMenu queues Replace(MenuState)
        app.resources_mut().insert(GameStateAction::PushMenu);
        app.run();
        // Flush the Replace
        app.run();
        assert_eq!(
            app.resources_mut().get::<StateStack>().map(|s| s.len()),
            Some(1)
        );
    }
}
