use engine_core::app::{App, AppBuilder};
use engine_core::color::Color;
use engine_core::plugin::Plugin;
use engine_core::plugins::CorePlugins;
use engine_core::time::Time;

use engine_math::Vec2;
use std::collections::VecDeque;

#[allow(dead_code)]
struct Particle {
    position: Vec2,
    velocity: Vec2,
    lifetime: f32,
    age: f32,
    color: Color,
    size: f32,
}

struct ParticleSystem {
    particles: VecDeque<Particle>,
    max_particles: usize,
    spawn_rate: f32,
    spawn_timer: f32,
}

impl ParticleSystem {
    fn new() -> Self {
        Self {
            particles: VecDeque::new(),
            max_particles: 1000,
            spawn_rate: 10.0,
            spawn_timer: 0.0,
        }
    }

    fn spawn(&mut self, pos: Vec2, vel: Vec2, color: Color, lifetime: f32) {
        if self.particles.len() >= self.max_particles {
            self.particles.pop_front();
        }

        self.particles.push_back(Particle {
            position: pos,
            velocity: vel,
            lifetime,
            age: 0.0,
            color,
            size: 5.0,
        });
    }

    fn update(&mut self, dt: f32) {
        let mut dead = Vec::new();

        for (i, particle) in self.particles.iter_mut().enumerate() {
            particle.age += dt;
            particle.position += particle.velocity * dt;
            particle.velocity.y -= 9.8 * dt;

            if particle.age >= particle.lifetime {
                dead.push(i);
            }
        }

        for i in dead.into_iter().rev() {
            self.particles.remove(i);
        }
    }
}

struct ParticleDemoPlugin;

impl Plugin for ParticleDemoPlugin {
    fn build(&self, app: &mut AppBuilder) {
        // Insert resources into App's registry
        app.insert_resource(ParticleSystem::new());
        app.insert_resource(Time::new());
    }
}

pub fn main() {
    println!("=== RustEngine Particle System Demo ===\n");
    println!("This demo shows a simple particle system with:");
    println!("  - Physics simulation (gravity)");
    println!("  - Particle spawning");
    println!("  - Lifetime management\n");

    let mut app_builder = AppBuilder::new();
    app_builder.add_plugin(CorePlugins);
    app_builder.add_plugin(ParticleDemoPlugin);

    // Add systems as hooks to update particles
    app_builder.add_pre_update_hook(Box::new(|app| {
        // Update time
        if let Some(time) = app.world.get_resource_mut::<Time>() {
            time.update();
        }

        // Spawn particles
        if let Some(system) = app.world.get_resource_mut::<ParticleSystem>() {
            let _dt = system.spawn_timer;
            system.spawn_timer += 0.016;

            let spawn_interval = 1.0 / system.spawn_rate;

            while system.spawn_timer >= spawn_interval {
                system.spawn_timer -= spawn_interval;

                let x = (rand_simple() * 400.0 - 200.0) as f32;
                let y = 200.0;
                let vx = (rand_simple() * 20.0 - 10.0) as f32;
                let vy = (rand_simple() * 50.0 + 100.0) as f32;

                let colors = [Color::RED, Color::ORANGE, Color::YELLOW, Color::CYAN];
                let color = colors[(rand_simple() * 4.0) as usize % 4];

                system.spawn(Vec2::new(x, y), Vec2::new(vx, vy), color, 2.0);
            }
        }
    }));

    // Add particle update hook
    app_builder.add_pre_update_hook(Box::new(|app| {
        if let Some(system) = app.world.get_resource_mut::<ParticleSystem>() {
            system.update(0.016);
        }
    }));

    // Add debug hook to show particle count
    app_builder.add_post_update_hook(Box::new(|app: &mut App| {
        if let Some(time) = app.world.get_resource::<Time>() {
            if time.frame_count() % 60 == 0 {
                let particle_count = app
                    .resources
                    .get::<ParticleSystem>()
                    .map(|s| s.particles.len())
                    .unwrap_or(0);

                println!(
                    "[Frame {}] Particles: {} | FPS: {:.1}",
                    time.frame_count(),
                    particle_count,
                    time.fps()
                );
            }
        }
    }));

    println!("Simulating 300 frames (5 seconds at 60fps)...\n");

    let mut app = app_builder.build();
    for _ in 0..300 {
        app.run();
    }

    println!("\n=== Demo Complete ===");

    // Get final particle count
    if let Some(system) = app.world.get_resource::<ParticleSystem>() {
        println!("Final particle count: {}", system.particles.len());
    }
}

fn rand_simple() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos as f32 % 1000.0) / 1000.0
}
