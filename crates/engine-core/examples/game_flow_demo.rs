//! Comprehensive game flow demo showcasing the RustEngine API surface.
//!
//! Demonstrates:
//! 1. Game states: Menu -> Gameplay -> Pause -> GameOver via StateStack
//! 2. ECS setup: entities with Transform, physics bodies, Camera, DirectionalLight, PbrMaterial
//! 3. Input handling: WASD movement for a player entity
//! 4. Physics: ground plane + falling objects with gravity and collision
//! 5. Rendering setup: camera, directional light, PBR-material objects
//! 6. Audio: AudioManager and AudioMixer channel setup (no files required)
//! 7. Game loop: GameSession score tracking and state transitions

use engine_core::app::{App, AppBuilder};
use engine_core::plugin::Plugin;
use engine_core::time::Time;
use engine_core::transform::Transform;
use engine_ecs::query::QueryPair;
use engine_ecs::system::IntoSystem;
use engine_ecs::world::World;
use engine_framework::{FrameworkPlugin, GameFlowPlugin, GameSession, GameStateAction};
use engine_input::input_manager::InputManager;
use engine_input::keyboard::KeyCode;
use engine_math::{Quat, Vec3};
use engine_render::camera::Camera;
use engine_render::light::DirectionalLight;
use engine_render::resource::material::PbrMaterial;

// ---------------------------------------------------------------------------
// Action queue for bridging systems -> framework state machine
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Local physics types (mirrors engine-physics API without circular dep)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BodyType {
    Static,
    Dynamic,
}

#[derive(Debug, Clone)]
struct RigidBody {
    body_type: BodyType,
    mass: f32,
    velocity: Vec3,
    gravity_scale: f32,
}

impl RigidBody {
    fn new_static() -> Self {
        Self {
            body_type: BodyType::Static,
            mass: 0.0,
            velocity: Vec3::ZERO,
            gravity_scale: 0.0,
        }
    }

    fn new_dynamic(mass: f32) -> Self {
        Self {
            body_type: BodyType::Dynamic,
            mass,
            velocity: Vec3::ZERO,
            gravity_scale: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
enum ColliderShape {
    Box { half_extents: Vec3 },
    Sphere { radius: f32 },
}

#[derive(Debug, Clone)]
struct Collider {
    shape: ColliderShape,
}

impl Collider {
    fn cuboid(hx: f32, hy: f32, hz: f32) -> Self {
        Self {
            shape: ColliderShape::Box {
                half_extents: Vec3::new(hx, hy, hz),
            },
        }
    }

    fn sphere(radius: f32) -> Self {
        Self {
            shape: ColliderShape::Sphere { radius },
        }
    }
}

// ---------------------------------------------------------------------------
// Game-specific components
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Player;

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Health {
    current: f32,
    max: f32,
}

#[derive(Debug, Clone)]
struct Score {
    value: i32,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct FallingObject {
    points: i32,
}

// ---------------------------------------------------------------------------
// Physics simulation plugin
// ---------------------------------------------------------------------------

struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(physics_step_system());
    }
}

/// Simple physics: apply gravity, integrate positions, ground collision.
fn physics_step_system() -> impl IntoSystem {
    |world: &mut World| {
        let dt = 1.0 / 60.0;
        let gravity = Vec3::new(0.0, -9.81, 0.0);
        let ground_y = -1.0;

        // Collect entities with both Transform and RigidBody
        let indices = world.component_entities::<RigidBody>();
        let mut updates: Vec<(u32, Vec3, Vec3)> = Vec::new();

        for &idx in &indices {
            let (body_type, mass, gravity_scale, vel) = {
                match world.get_by_index::<RigidBody>(idx) {
                    Some(body) => (body.body_type, body.mass, body.gravity_scale, body.velocity),
                    None => continue,
                }
            };

            if body_type != BodyType::Dynamic || mass <= 0.0 {
                continue;
            }

            let pos = match world.get_by_index::<Transform>(idx) {
                Some(t) => t.position(),
                None => continue,
            };

            // Apply gravity
            let new_vel = vel + gravity * gravity_scale * dt;

            // Integrate position
            let mut new_pos = pos + new_vel * dt;

            // Ground collision: bounce off ground plane
            let has_collider = world.get_by_index::<Collider>(idx).is_some();
            if has_collider {
                let radius = match world.get_by_index::<Collider>(idx) {
                    Some(Collider {
                        shape: ColliderShape::Sphere { radius },
                        ..
                    }) => *radius,
                    Some(Collider {
                        shape: ColliderShape::Box { half_extents },
                        ..
                    }) => half_extents.y,
                    None => 0.5,
                };

                if new_pos.y - radius < ground_y {
                    new_pos.y = ground_y + radius;
                    updates.push((
                        idx,
                        new_pos,
                        Vec3::new(new_vel.x, -new_vel.y * 0.4, new_vel.z),
                    ));
                    continue;
                }
            }

            updates.push((idx, new_pos, new_vel));
        }

        // Apply updates
        for (idx, new_pos, new_vel) in updates {
            if let Some(body) = world.get_by_index_mut::<RigidBody>(idx) {
                body.velocity = new_vel;
            }
            if let Some(transform) = world.get_by_index_mut::<Transform>(idx) {
                transform.set_position(new_pos);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Gameplay plugin — spawns scene entities and registers systems
// ---------------------------------------------------------------------------

struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let world = app.world_mut();

        // -- Player entity with Transform, RigidBody, Collider, Camera, PbrMaterial --
        let player = world.spawn();
        world.add_component(player, Transform::from_xyz(0.0, 2.0, 5.0));
        world.add_component(player, Player);
        world.add_component(
            player,
            Health {
                current: 100.0,
                max: 100.0,
            },
        );
        world.add_component(player, RigidBody::new_dynamic(1.0));
        world.add_component(player, Collider::sphere(0.5));
        // Camera attached to player — perspective projection
        world.add_component(
            player,
            Camera::perspective(std::f32::consts::FRAC_PI_4, 0.1, 1000.0),
        );
        // PBR material: a reddish character
        world.add_component(player, PbrMaterial::new([0.9, 0.2, 0.2, 1.0], 0.0, 0.6));

        // -- Ground plane: large static box --
        let ground = world.spawn();
        world.add_component(
            ground,
            Transform::from_position_rotation_scale(
                Vec3::new(0.0, -1.5, 0.0),
                Quat::IDENTITY,
                Vec3::new(20.0, 1.0, 20.0),
            ),
        );
        world.add_component(ground, RigidBody::new_static());
        world.add_component(ground, Collider::cuboid(10.0, 0.5, 10.0));
        // Green-ish ground material
        world.add_component(ground, PbrMaterial::new([0.3, 0.6, 0.3, 1.0], 0.0, 0.8));

        // -- Falling objects: spheres at various heights --
        for i in 0..5 {
            let x = (i as f32 - 2.0) * 3.0;
            let y = 8.0 + i as f32 * 2.0;
            let obj = world.spawn();
            world.add_component(obj, Transform::from_xyz(x, y, -5.0));
            world.add_component(obj, RigidBody::new_dynamic(2.0));
            world.add_component(obj, Collider::sphere(0.6));
            world.add_component(
                obj,
                FallingObject {
                    points: 10 * (i + 1),
                },
            );
            // Varying metallic materials (gold, copper, etc.)
            let hue = i as f32 / 5.0;
            world.add_component(obj, PbrMaterial::metallic_color(0.8, 0.6 + hue * 0.3, 0.2));
        }

        // -- Decorative static cubes around the scene --
        for i in 0..4 {
            let angle = i as f32 * std::f32::consts::FRAC_PI_2;
            let cube = world.spawn();
            world.add_component(
                cube,
                Transform::from_position_rotation_scale(
                    Vec3::new(angle.cos() * 8.0, 0.0, angle.sin() * 8.0),
                    Quat::IDENTITY,
                    Vec3::new(1.0, 2.0, 1.0),
                ),
            );
            world.add_component(cube, RigidBody::new_static());
            world.add_component(cube, Collider::cuboid(0.5, 1.0, 0.5));
            // Blue-ish pillars
            world.add_component(cube, PbrMaterial::new([0.2, 0.3, 0.9, 1.0], 0.3, 0.4));
        }

        // -- Directional light (sun) --
        let sun = world.spawn();
        world.add_component(sun, Transform::from_xyz(0.0, 20.0, 10.0));
        world.add_component(
            sun,
            DirectionalLight {
                direction: [0.3, -1.0, -0.5],
                color: [1.0, 0.95, 0.8],
                intensity: 1.2,
                enabled: true,
            },
        );

        // -- Score entity --
        let score_entity = world.spawn();
        world.add_component(score_entity, Score { value: 0 });

        // -- Time resource --
        world.insert_resource(Time::new());

        // -- Register gameplay systems --
        app.add_system(player_movement_system());
        app.add_system(gameplay_update_system());
        app.add_system(pause_check_system());
    }
}

/// WASD movement for the player entity.
fn player_movement_system() -> impl IntoSystem {
    |world: &mut World| {
        let is_running = world
            .get_resource::<GameSession>()
            .is_some_and(|s| s.is_running);
        if !is_running {
            return;
        }

        let player_entities = world.component_entities::<Player>();
        let Some(&player_idx) = player_entities.first() else {
            return;
        };

        let (pressed_w, pressed_s, pressed_a, pressed_d) = {
            let input = world.get_resource::<InputManager>();
            match input {
                Some(i) => (
                    i.key_down(KeyCode::KeyW),
                    i.key_down(KeyCode::KeyS),
                    i.key_down(KeyCode::KeyA),
                    i.key_down(KeyCode::KeyD),
                ),
                None => (false, false, false, false),
            }
        };

        let mut direction = Vec3::ZERO;
        if pressed_w {
            direction.z -= 1.0;
        }
        if pressed_s {
            direction.z += 1.0;
        }
        if pressed_a {
            direction.x -= 1.0;
        }
        if pressed_d {
            direction.x += 1.0;
        }

        let speed = 8.0;
        if let Some(body) = world.get_by_index_mut::<RigidBody>(player_idx) {
            if direction.length_squared() > 0.0001 {
                let normalized = direction.normalize();
                body.velocity.x = normalized.x * speed;
                body.velocity.z = normalized.z * speed;
            } else {
                body.velocity.x *= 0.85; // friction damping
                body.velocity.z *= 0.85;
            }
        }
    }
}

/// Core gameplay logic: health drain, score accumulation, game over check.
fn gameplay_update_system() -> impl IntoSystem {
    |world: &mut World| {
        let is_running = world
            .get_resource::<GameSession>()
            .is_some_and(|s| s.is_running);
        if !is_running {
            return;
        }

        let dt = world
            .get_resource::<Time>()
            .map_or(0.016, |t| t.delta_seconds());

        // Drain player health over time
        let health_query = QueryPair::<&mut Health, &Player>::new();
        for (health, _) in health_query.iter_mut(world) {
            health.current -= 3.0 * dt;
            if health.current <= 0.0 {
                health.current = 0.0;
            }
        }

        // Increment score
        let mut should_game_over = false;
        let mut final_score = 0;

        let score_query = QueryPair::<&mut Score, ()>::new();
        for (score, _) in score_query.iter_mut(world) {
            score.value += 1;
            if score.value >= 600 {
                should_game_over = true;
                final_score = score.value;
            }
        }

        // Read back health and score for logging / game-over check
        let (health_val, current_score) = {
            let hq = QueryPair::<&Health, &Player>::new();
            let health = hq.iter(world).map(|(h, _)| h.current).next().unwrap_or(0.0);
            let sq = QueryPair::<&Score, ()>::new();
            let score = sq.iter(world).map(|(s, _)| s.value).next().unwrap_or(0);
            (health, score)
        };

        // Periodic status output
        if current_score % 120 == 0 && current_score > 0 {
            println!(
                "[Gameplay] Score: {} | Health: {:.1} | Player pos: checking...",
                current_score, health_val
            );
        }

        if health_val <= 0.0 {
            should_game_over = true;
            final_score = current_score;
        }

        if should_game_over && let Some(queue) = world.get_resource_mut::<ActionQueue>() {
            queue.push(GameStateAction::PushGameOver { score: final_score });
        }
    }
}

/// Check for ESC key to trigger pause.
fn pause_check_system() -> impl IntoSystem {
    |world: &mut World| {
        let is_running = world
            .get_resource::<GameSession>()
            .is_some_and(|s| s.is_running);
        if !is_running {
            return;
        }

        let should_pause = world
            .get_resource::<InputManager>()
            .is_some_and(|input| input.key_just_pressed(KeyCode::Escape));

        if should_pause && let Some(queue) = world.get_resource_mut::<ActionQueue>() {
            queue.push(GameStateAction::PushPause);
        }
    }
}

// ---------------------------------------------------------------------------
// Audio setup plugin (demonstrates API without requiring audio files)
// ---------------------------------------------------------------------------

struct AudioSetupPlugin;

impl Plugin for AudioSetupPlugin {
    fn build(&self, app: &mut AppBuilder) {
        use engine_audio::audio_manager::{AudioChannel, AudioManager};
        use engine_audio::mixer::AudioMixer;

        // Create the audio manager and configure channel volumes
        let mut audio = AudioManager::new().unwrap();
        audio.set_master_volume(0.8);
        audio.set_channel_volume(AudioChannel::Music, 0.6);
        audio.set_channel_volume(AudioChannel::Sfx, 1.0);
        app.insert_resource(audio);

        // Create the mixer with named buses
        let mut mixer = AudioMixer::new();
        mixer.set_bus_volume("music", 0.5);
        mixer.set_bus_volume("sfx", 0.8);
        mixer.set_bus_volume("ambient", 0.3);
        app.insert_resource(mixer);

        println!("Audio system initialized (channels: SFX, Music, Ambient)");
    }
}

// ---------------------------------------------------------------------------
// Action sync plugin: bridges system-produced actions into the framework
// ---------------------------------------------------------------------------

struct ActionSyncPlugin;

impl Plugin for ActionSyncPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(ActionQueue::new());

        app.add_post_update_hook(Box::new(|app: &mut App| {
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

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

pub fn main() {
    println!("=== RustEngine Comprehensive Game Flow Demo ===\n");
    println!("This demo showcases the full engine API surface:");
    println!("  - Game states: Menu -> Gameplay -> Pause -> GameOver");
    println!("  - ECS: entities with Transform, RigidBody, Collider, Camera, PbrMaterial");
    println!("  - Input: WASD movement, ESC pause");
    println!("  - Physics: gravity, ground collision, bouncing objects");
    println!("  - Rendering: perspective camera, directional light, PBR materials");
    println!("  - Audio: AudioManager + AudioMixer channel setup");
    println!("  - Game loop: GameSession score tracking\n");

    println!("Controls:");
    println!("  Menu:     [1] New Game  [2] Quit");
    println!("  Gameplay: [WASD] Move   [ESC] Pause");
    println!("  Pause:    [ESC] Resume  [Q] Quit to Menu");
    println!("  GameOver: [R] Restart   [Q] Quit to Menu\n");

    // Build the application with all plugins
    let mut app_builder = AppBuilder::new();

    // Core framework (StateStack, state machine)
    app_builder.add_plugin(FrameworkPlugin);
    // Action bridge (system -> framework)
    app_builder.add_plugin(ActionSyncPlugin);
    // Game flow (MenuState, PauseState, GameOverState, action handling)
    app_builder.add_plugin(GameFlowPlugin);
    // Physics simulation (gravity, integration, collision)
    app_builder.add_plugin(PhysicsPlugin);
    // Audio system (AudioManager, AudioMixer)
    app_builder.add_plugin(AudioSetupPlugin);
    // Gameplay (entities, systems, rendering components)
    app_builder.add_plugin(GameplayPlugin);

    // Pre-update hook: print rendering setup info once on first frame
    let mut printed_info = false;
    app_builder.add_pre_update_hook(Box::new(move |app: &mut App| {
        if !printed_info {
            printed_info = true;

            // Count scene entities by component type
            let transforms = app.world.component_entities::<Transform>().len();
            let bodies = app.world.component_entities::<RigidBody>().len();
            let colliders = app.world.component_entities::<Collider>().len();
            let cameras = app.world.component_entities::<Camera>().len();
            let lights = app.world.component_entities::<DirectionalLight>().len();
            let materials = app.world.component_entities::<PbrMaterial>().len();

            println!("Scene summary:");
            println!("  Transforms:  {}", transforms);
            println!("  RigidBodies: {}", bodies);
            println!("  Colliders:   {}", colliders);
            println!("  Cameras:     {}", cameras);
            println!("  Lights:      {}", lights);
            println!("  Materials:   {}", materials);

            // Show audio state
            if let Some(audio) = app
                .world
                .get_resource::<engine_audio::audio_manager::AudioManager>()
            {
                println!("  Audio master vol: {:.1}", audio.master_volume());
            }
            if let Some(mixer) = app.world.get_resource::<engine_audio::mixer::AudioMixer>() {
                println!("  Mixer buses: {:?}", mixer.bus_names());
            }
            println!();
        }
    }));

    let mut app = app_builder.build();

    // Run up to 600 frames (10 seconds at 60fps)
    for frame in 1..=600 {
        app.run();

        // Print physics state periodically
        if frame % 120 == 0 {
            let body_count = app.world.component_entities::<RigidBody>().len();
            let player_pos = app
                .world
                .component_entities::<Player>()
                .first()
                .and_then(|&idx| app.world.get_by_index::<Transform>(idx))
                .map(|t| {
                    let pos = t.position();
                    format!("({:.1}, {:.1}, {:.1})", pos.x, pos.y, pos.z)
                })
                .unwrap_or_else(|| "N/A".to_string());

            println!(
                "[Frame {:3}] Bodies: {} | Player pos: {}",
                frame, body_count, player_pos
            );
        }

        // Check for quit
        let quit = app
            .world
            .get_resource::<GameSession>()
            .is_some_and(|s| s.quit_requested);
        if quit {
            println!("\nQuit requested. Goodbye!");
            break;
        }
    }

    println!("\n=== Demo Complete ===");
}
