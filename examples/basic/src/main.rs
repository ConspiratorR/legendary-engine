use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;

struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(player_movement);
    }
}

fn player_movement(_world: &mut engine_ecs::world::World) {
    // Placeholder: will update transforms based on input
}

fn main() {
    let mut app = AppBuilder::new();
    app.add_plugin(GamePlugin);
    app.build().run();
}
