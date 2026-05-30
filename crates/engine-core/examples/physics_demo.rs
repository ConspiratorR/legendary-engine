//! Physics demo example - shows ECS-based physics simulation.
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;
use engine_math::Vec3;

#[derive(Debug, Clone)]
enum BodyType {
    Static,
    Dynamic,
}

#[derive(Debug, Clone)]
struct RigidBody {
    body_type: BodyType,
    velocity: Vec3,
}

impl RigidBody {
    fn new_static() -> Self {
        Self {
            body_type: BodyType::Static,
            velocity: Vec3::new(0.0, 0.0, 0.0),
        }
    }

    fn new_dynamic() -> Self {
        Self {
            body_type: BodyType::Dynamic,
            velocity: Vec3::new(0.0, 0.0, 0.0),
        }
    }

    fn set_linear_velocity(&mut self, vel: Vec3) {
        self.velocity = vel;
    }
}

#[derive(Debug, Clone)]
struct Position(Vec3);

#[derive(Debug, Clone)]
struct Collider {
    half_extents: Vec3,
}

impl Collider {
    fn cuboid(hx: f32, hy: f32, hz: f32) -> Self {
        Self {
            half_extents: Vec3::new(hx, hy, hz),
        }
    }
}

/// Simple physics plugin that spawns demo entities.
struct PhysicsDemoPlugin;

impl Plugin for PhysicsDemoPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let world = app.world_mut();

        // Create a floor
        let floor = world.spawn();
        world.add_component(floor, Position(Vec3::new(0.0, -0.5, 0.0)));
        world.add_component(floor, RigidBody::new_static());
        world.add_component(floor, Collider::cuboid(50.0, 0.5, 50.0));

        // Create some cubes
        for i in 0..20 {
            let cube = world.spawn();
            let mut body = RigidBody::new_dynamic();
            body.set_linear_velocity(Vec3::new(
                (i as f32 * 0.5).sin() * 5.0,
                10.0 + i as f32,
                (i as f32 * 0.5).cos() * 5.0,
            ));
            world.add_component(cube, Position(Vec3::new(0.0, 5.0 + i as f32, 0.0)));
            world.add_component(cube, body);
            world.add_component(cube, Collider::cuboid(0.5, 0.5, 0.5));
        }

        println!("Physics demo initialized with 20 cubes!");
    }
}

fn step_physics(world: &mut engine_ecs::world::World, gravity: Vec3, dt: f32) {
    let indices = world.component_entities::<RigidBody>();

    // Phase 1: read positions and velocities, compute updates
    let mut updates: Vec<(u32, Vec3, Vec3)> = Vec::new();
    for &idx in &indices {
        if let Some(body) = world.get_by_index::<RigidBody>(idx) {
            if matches!(body.body_type, BodyType::Dynamic) {
                let vel = body.velocity;
                if let Some(pos) = world.get_by_index::<Position>(idx) {
                    let new_vel = vel + gravity * dt;
                    let new_pos = pos.0 + new_vel * dt;
                    updates.push((idx, new_vel, new_pos));
                }
            }
        }
    }

    // Phase 2: apply updates
    for (idx, vel, pos) in updates {
        if let Some(body) = world.get_by_index_mut::<RigidBody>(idx) {
            body.velocity = vel;
        }
        if let Some(pos_comp) = world.get_by_index_mut::<Position>(idx) {
            pos_comp.0 = pos;
        }
    }
}

fn main() {
    println!("=== RustEngine Physics Demo ===\n");

    let mut app_builder = AppBuilder::new();
    app_builder.add_plugin(PhysicsDemoPlugin);
    let mut app = app_builder.build();

    println!("Running physics simulation...\n");

    let gravity = Vec3::new(0.0, -9.81, 0.0);
    let dt = 1.0 / 60.0;

    // Simulate 500 frames
    for frame in 0..500 {
        step_physics(&mut app.world, gravity, dt);

        if frame % 60 == 0 {
            let body_count = app.world.component_entities::<RigidBody>().len();
            println!(
                "Frame {} - Bodies: {}, Gravity: ({:.1}, {:.1}, {:.1})",
                frame, body_count, gravity.x, gravity.y, gravity.z
            );
        }
    }

    println!("\n=== Simulation complete! ===");
}
