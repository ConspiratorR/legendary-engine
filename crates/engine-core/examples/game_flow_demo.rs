use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;
use engine_core::time::Time;
use engine_ecs::query::QueryPair;
use engine_ecs::system::IntoSystem;
use engine_ecs::world::World;
use engine_framework::{FrameworkPlugin, GameFlowPlugin, GameSession, GameStateAction};
use engine_input::input_manager::InputManager;
use engine_input::keyboard::KeyCode;

struct ActionQueue {
    actions: Vec<GameStateAction>,
}

impl ActionQueue {
    fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    fn push(&mut self, action: GameStateAction) {
        self.actions.push(action);
    }

    fn drain(&mut self) -> Vec<GameStateAction> {
        std::mem::take(&mut self.actions)
    }
}

#[derive(Debug, Clone)]
struct Player;

#[derive(Debug, Clone)]
struct Health {
    current: f32,
    max: f32,
}

#[derive(Debug, Clone)]
struct Score {
    value: i32,
}

struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let world = app.world_mut();

        let player = world.spawn();
        world.add_component(player, Player);
        world.add_component(
            player,
            Health {
                current: 100.0,
                max: 100.0,
            },
        );

        let score_entity = world.spawn();
        world.add_component(score_entity, Score { value: 0 });

        world.insert_resource(Time::new());

        app.add_system(gameplay_system());
        app.add_system(pause_check_system());
    }
}

fn gameplay_system() -> impl IntoSystem {
    |world: &mut World| {
        let is_running = world
            .get_resource::<GameSession>()
            .map_or(false, |s| s.is_running);
        if !is_running {
            return;
        }

        let dt = world
            .get_resource::<Time>()
            .map_or(0.016, |t| t.delta_seconds());

        let health_query = QueryPair::<&mut Health, &Player>::new();
        for (health, _) in health_query.iter_mut(world) {
            health.current -= 5.0 * dt;
            if health.current <= 0.0 {
                health.current = 0.0;
            }
        }

        let mut should_game_over = false;
        let mut final_score = 0;

        let score_query = QueryPair::<&mut Score, ()>::new();
        for (score, _) in score_query.iter_mut(world) {
            score.value += 1;
            if score.value >= 300 {
                should_game_over = true;
                final_score = score.value;
            }
        }

        let (health_val, current_score) = {
            let hq = QueryPair::<&Health, &Player>::new();
            let health = hq.iter(world).map(|(h, _)| h.current).next().unwrap_or(0.0);
            let sq = QueryPair::<&Score, ()>::new();
            let score = sq.iter(world).map(|(s, _)| s.value).next().unwrap_or(0);
            (health, score)
        };

        if current_score % 60 == 0 && current_score > 0 {
            println!(
                "[Gameplay] Score: {} | Health: {:.1}",
                current_score, health_val
            );
        }

        if health_val <= 0.0 {
            should_game_over = true;
            final_score = current_score;
        }

        if should_game_over {
            if let Some(queue) = world.get_resource_mut::<ActionQueue>() {
                queue.push(GameStateAction::PushGameOver { score: final_score });
            }
        }
    }
}

fn pause_check_system() -> impl IntoSystem {
    |world: &mut World| {
        let is_running = world
            .get_resource::<GameSession>()
            .map_or(false, |s| s.is_running);
        if !is_running {
            return;
        }

        let should_pause = world
            .get_resource::<InputManager>()
            .map_or(false, |input| input.key_just_pressed(KeyCode::Escape));

        if should_pause {
            if let Some(queue) = world.get_resource_mut::<ActionQueue>() {
                queue.push(GameStateAction::PushPause);
            }
        }
    }
}

struct ActionSyncPlugin;

impl Plugin for ActionSyncPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(ActionQueue::new());

        app.add_post_update_hook(Box::new(|app: &mut engine_core::app::App| {
            let actions = app
                .world
                .get_resource_mut::<ActionQueue>()
                .map(|q| q.drain())
                .unwrap_or_default();
            for action in actions {
                app.resources_mut().insert(action);
            }
        }));
    }
}

pub fn main() {
    println!("=== Game Flow Demo ===\n");
    println!("State transitions: Menu -> Gameplay -> Pause -> GameOver -> Menu\n");
    println!("Controls:");
    println!("  Menu:     [1] New Game  [2] Quit");
    println!("  Gameplay: [ESC] Pause");
    println!("  Pause:    [ESC] Resume  [Q] Quit to Menu");
    println!("  GameOver: [R] Restart   [Q] Quit to Menu\n");

    let mut app_builder = AppBuilder::new();
    app_builder.add_plugin(FrameworkPlugin);
    app_builder.add_plugin(ActionSyncPlugin);
    app_builder.add_plugin(GameFlowPlugin);
    app_builder.add_plugin(GameplayPlugin);
    let mut app = app_builder.build();

    for _frame in 1..=600 {
        app.run();

        let quit = app
            .world
            .get_resource::<GameSession>()
            .map_or(false, |s| s.quit_requested);
        if quit {
            println!("\nQuit requested. Goodbye!");
            break;
        }
    }

    println!("\n=== Demo Complete ===");
}
