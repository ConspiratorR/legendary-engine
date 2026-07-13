use engine_core::World;
use engine_core::Component;
use engine_core::app::AppBuilder;
use engine_core::context::Context;
use engine_core::event::EventBus;
use engine_core::player_loop::Phase;
use engine_core::time::Time;
use engine_core::transform::Transform;
use engine_math::Vec3;

use std::any::Any;

#[derive(Debug)]
struct Position(Vec3);

impl Component for Position {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Debug)]
struct Velocity(Vec3);

impl Component for Velocity {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub fn main() {
    println!("Running 5 frames of simulation...\n");

    let mut builder = AppBuilder::new();

    // Startup system: spawn entities
    builder.add_startup_system(|ctx: &mut Context| {
        let world = &mut ctx.world;

        let player = world.CreateGameObject("Player");
        if let Some(t) = world.GetTransformMut(player) {
            *t = Transform::from_xyz(0.0, 0.0, 0.0);
        }
        world.AddComponent(player, Position(Vec3::new(0.0, 0.0, 0.0)));
        world.AddComponent(player, Velocity(Vec3::new(1.0, 2.0, 0.0)));

        let enemy = world.CreateGameObject("Enemy");
        if let Some(t) = world.GetTransformMut(enemy) {
            *t = Transform::from_xyz(10.0, 5.0, 0.0);
        }
        world.AddComponent(enemy, Position(Vec3::new(10.0, 5.0, 0.0)));
        world.AddComponent(enemy, Velocity(Vec3::new(-1.0, -0.5, 0.0)));
    });

    // Update system: move entities
    builder.add_system_to_phase(Phase::Update, |ctx: &mut Context| {
        let world = &mut ctx.world;
        let handles: Vec<_> = world.GetRootGameObjects();
        for handle in handles {
            let vel = world.GetComponent::<Velocity>(handle).map(|v| v.0);
            if let Some(pos) = world.GetComponentMut::<Position>(handle)
                && let Some(vel) = vel
            {
                pos.0 += vel * 0.016;
            }
        }
    });

    // Print system: display positions each frame
    builder.add_system_to_phase(Phase::Update, |ctx: &mut Context| {
        let world = &ctx.world;
        let handles = world.GetRootGameObjects();
        for handle in handles {
            if let Some(pos) = world.GetComponent::<Position>(handle) {
                let name = world.GetName(handle);
                println!("{}: Position = {:?}", name, pos.0);
            }
        }
    });

    let mut app = builder.build();
    app.set_running(true);

    // Run frames via the PlayerLoop
    let mut world = World::new();
    let time = Time::new();
    let mut events = EventBus::new();

    for frame in 0..5 {
        println!("--- Frame {} ---", frame + 1);
        if app.is_running() {
            let mut ctx = Context::new(&mut world, time.clone(), frame, &mut events);
            app.player_loop_mut().run(&mut ctx);
        }
    }

    println!("\n=== Example Complete ===");
    println!("The engine successfully:");
    println!("1. Built the app via AppBuilder");
    println!("2. Added components (Transform, Position, Velocity) via startup system");
    println!("3. Registered Update phase systems that update positions");
    println!("4. Executed the PlayerLoop for multiple frames");
}
