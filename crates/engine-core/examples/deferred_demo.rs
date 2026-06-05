//! Deferred rendering pipeline demo.
//!
//! Demonstrates:
//! 1. Engine setup with CorePlugins
//! 2. Camera entity with perspective projection
//! 3. DirectionalLight entity (sun)
//! 4. 3D scene with ground plane and objects using PbrMaterial
//! 5. MeshRenderer components for deferred geometry submission

use engine_core::app::AppBuilder;
use engine_core::plugins::CorePlugins;
use engine_core::time::Time;
use engine_core::transform::Transform;
use engine_math::Vec3;
use engine_render::camera::Camera;
use engine_render::light::DirectionalLight;
use engine_render::mesh_bridge::MeshRenderer;
use engine_render::resource::material::PbrMaterial;

fn main() {
    println!("=== RustEngine Deferred Rendering Demo ===\n");
    println!("This demo sets up a 3D scene for the deferred rendering pipeline:");
    println!("  - Perspective camera");
    println!("  - Directional light (sun)");
    println!("  - Ground plane with PBR material");
    println!("  - Colored objects (cubes/spheres) with varying materials");
    println!("  - MeshRenderer components for GPU submission\n");

    let mut app = AppBuilder::new();
    app.add_plugin(CorePlugins);

    let world = app.world_mut();

    // ── Camera ──────────────────────────────────────────────
    let camera = world.spawn();
    world.add_component(camera, Transform::from_xyz(0.0, 5.0, 10.0));
    world.add_component(
        camera,
        Camera::perspective(std::f32::consts::FRAC_PI_4, 0.1, 1000.0),
    );

    // ── Directional light (sun) ─────────────────────────────
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

    // ── Ground plane ────────────────────────────────────────
    let ground = world.spawn();
    world.add_component(
        ground,
        Transform::from_xyz(0.0, -1.0, 0.0).with_scale(Vec3::new(20.0, 0.2, 20.0)),
    );
    world.add_component(ground, PbrMaterial::new([0.3, 0.6, 0.3, 1.0], 0.0, 0.8));
    world.add_component(
        ground,
        MeshRenderer {
            mesh_id: 0, // placeholder — would be set by mesh upload system
            material_id: 0,
            cast_shadow: true,
        },
    );

    // ── Scene objects ───────────────────────────────────────
    let objects = [
        // position,              scale,               color (RGBA),           metallic, roughness
        (
            Vec3::new(-3.0, 0.0, -3.0),
            Vec3::new(1.0, 2.0, 1.0),
            [0.9, 0.2, 0.2, 1.0],
            0.0,
            0.4,
        ),
        (
            Vec3::new(0.0, 0.0, -4.0),
            Vec3::new(1.5, 1.5, 1.5),
            [0.2, 0.5, 0.9, 1.0],
            0.5,
            0.3,
        ),
        (
            Vec3::new(3.0, 0.5, -2.0),
            Vec3::new(1.0, 1.0, 1.0),
            [0.9, 0.8, 0.1, 1.0],
            0.9,
            0.2,
        ),
        (
            Vec3::new(-1.5, 0.0, -6.0),
            Vec3::new(2.0, 2.0, 2.0),
            [0.6, 0.3, 0.8, 1.0],
            0.1,
            0.7,
        ),
    ];

    for (pos, scale, color, metallic, roughness) in objects {
        let entity = world.spawn();
        world.add_component(
            entity,
            Transform::from_xyz(pos.x, pos.y, pos.z).with_scale(scale),
        );
        world.add_component(entity, PbrMaterial::new(color, metallic, roughness));
        world.add_component(
            entity,
            MeshRenderer {
                mesh_id: 0,
                material_id: 0,
                cast_shadow: true,
            },
        );
    }

    // ── Time resource ───────────────────────────────────────
    world.insert_resource(Time::new());

    // ── Scene summary ───────────────────────────────────────
    let transforms = world.component_entities::<Transform>().len();
    let cameras = world.component_entities::<Camera>().len();
    let lights = world.component_entities::<DirectionalLight>().len();
    let materials = world.component_entities::<PbrMaterial>().len();
    let mesh_renderers = world.component_entities::<MeshRenderer>().len();

    println!("Scene summary:");
    println!("  Transforms:    {}", transforms);
    println!("  Cameras:       {}", cameras);
    println!("  Lights:        {}", lights);
    println!("  PBR Materials: {}", materials);
    println!("  Mesh Renderers: {}", mesh_renderers);

    // Build and run a few frames to validate ECS setup
    let mut app = app.build();
    println!("\nSimulating 60 frames (1 second at 60fps)...\n");

    for frame in 1..=60 {
        app.run();

        if frame % 30 == 0
            && let Some(time) = app.world.get_resource::<Time>()
        {
            println!(
                "[Frame {}] Elapsed: {:.2}s | FPS: {:.1}",
                frame,
                time.elapsed_seconds(),
                time.fps()
            );
        }
    }

    println!("\n=== Demo Complete ===");
    println!("To render with a window, use: engine_core::engine::run_default(app_builder);");
    println!("Try:");
    println!("  cargo run --example deferred_demo -p engine-core");
    println!("  cargo run --example game_flow_demo -p engine-core");
}
