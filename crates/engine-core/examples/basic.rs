use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;
use engine_ecs::query::QueryPair;
use engine_ecs::system::IntoSystem;
use engine_ecs::world::World;
use engine_math::Vec3;

struct Position(Vec3);
struct Velocity(Vec3);

struct SetupPlugin;

impl Plugin for SetupPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let world = app.world_mut();
        let player = world.spawn();
        world.add_component(player, Position(Vec3::new(0.0, 0.0, 0.0)));
        world.add_component(player, Velocity(Vec3::new(1.0, 2.0, 0.0)));

        let enemy = world.spawn();
        world.add_component(enemy, Position(Vec3::new(10.0, 5.0, 0.0)));
        world.add_component(enemy, Velocity(Vec3::new(-1.0, -0.5, 0.0)));

        app.add_system(movement_system());
        app.add_system(print_system());
    }
}

fn movement_system() -> impl IntoSystem {
    |world: &mut World| {
        let query = QueryPair::<Position, Velocity>::new();
        for (pos, vel) in query.iter_mut(world) {
            pos.0 += vel.0 * 0.016;
        }
    }
}

fn print_system() -> impl IntoSystem {
    |world: &mut World| {
        let query = QueryPair::<Position, Velocity>::new();
        for (pos, _vel) in query.iter(world) {
            println!("Position = {:?}", pos.0);
        }
    }
}

pub fn main() {
    let mut app_builder = AppBuilder::new();
    app_builder.add_plugin(SetupPlugin);
    let mut app = app_builder.build();

    println!("Running 5 frames of simulation...\n");

    for frame in 1..=5 {
        println!("--- Frame {} ---", frame);
        app.run();
    }

    println!("\n=== Example Complete ===");
    println!("The engine successfully:");
    println!("1. Created an ECS world with entities");
    println!("2. Added components (Position, Velocity) via plugin");
    println!("3. Registered systems that update positions");
    println!("4. Executed the schedule for multiple frames");
}
