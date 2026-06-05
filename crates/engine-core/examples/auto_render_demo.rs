//! ECS Auto-Render Demo — validates the new RenderPlugin2D integration.
//!
//! This demo creates Camera and Sprite as ECS components.
//! The engine's render_phase() automatically collects and renders them.
//!
//! You should see a dark window with three colored rectangles (sprites).

use engine_asset::asset::Handle;
use engine_asset::types::Texture;
use engine_core::app::AppBuilder;
use engine_core::plugins::CorePlugins;
use engine_math::{Mat4, Vec2, Vec3};
use engine_render::camera::{Camera, Color};
use engine_render::plugin::RenderPlugin2D;
use engine_render::sprite::Sprite;
use engine_render::texture_bridge::TextureBridge;
use engine_window::{window::WindowConfig, window::create_window};
use std::path::PathBuf;
use std::sync::Arc;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Create window
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window = Arc::new(
        create_window(
            &WindowConfig {
                title: "ECS Auto-Render Demo".to_string(),
                width: 800,
                height: 600,
                vsync: true,
            },
            &event_loop,
        )
        .unwrap(),
    );

    // Build app
    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins);

    // Set up renderer via RenderPlugin2D
    let mut plugin = RenderPlugin2D::new(window.clone());
    plugin.build(builder.world_mut());
    let renderer = plugin.take_renderer().unwrap();

    // Create a 1x1 white texture and register it with the TextureBridge
    let white_tex = Texture {
        id: "white".into(),
        width: 1,
        height: 1,
        data: vec![255, 255, 255, 255],
        channels: 4,
        asset_path: PathBuf::new(),
    };
    let tex_handle = Handle::new(white_tex);

    // Register the texture handle with the bridge so resolve() works
    {
        let bridge = builder
            .world_mut()
            .get_resource_mut::<TextureBridge>()
            .unwrap();
        bridge.request(&tex_handle, "");
    }

    // Spawn entities into the ECS world
    let world = builder.world_mut();

    // Camera entity
    let cam_entity = world.spawn();
    let mut camera = Camera::orthographic(0.0, 800.0, 600.0, 0.0);
    camera.clear_color = Some(Color::new(0.1, 0.1, 0.15, 1.0));
    world.add_component(cam_entity, camera);

    // Red sprite at center-left
    let sprite1 = world.spawn();
    world.add_component(
        sprite1,
        Sprite {
            texture: tex_handle.clone(),
            color: [0.9, 0.2, 0.2, 1.0],
            size: Vec2::new(128.0, 128.0),
            transform: Mat4::from_translation(Vec3::new(200.0, 300.0, 0.0)),
            flip_x: false,
            flip_y: false,
            uv_region: [0.0, 0.0, 1.0, 1.0],
        },
    );

    // Green sprite at center-right
    let sprite2 = world.spawn();
    world.add_component(
        sprite2,
        Sprite {
            texture: tex_handle.clone(),
            color: [0.2, 0.8, 0.3, 1.0],
            size: Vec2::new(128.0, 128.0),
            transform: Mat4::from_translation(Vec3::new(500.0, 300.0, 0.0)),
            flip_x: false,
            flip_y: false,
            uv_region: [0.0, 0.0, 1.0, 1.0],
        },
    );

    // Blue sprite at bottom-center
    let sprite3 = world.spawn();
    world.add_component(
        sprite3,
        Sprite {
            texture: tex_handle.clone(),
            color: [0.2, 0.3, 0.9, 1.0],
            size: Vec2::new(200.0, 80.0),
            transform: Mat4::from_translation(Vec3::new(400.0, 100.0, 0.0)),
            flip_x: false,
            flip_y: false,
            uv_region: [0.0, 0.0, 1.0, 1.0],
        },
    );

    let mut app = builder.build();
    app.set_renderer(renderer);

    println!("=== ECS Auto-Render Demo ===");
    println!("If you see a window with colored rectangles, the integration works!");

    // Event loop with auto-render
    #[allow(deprecated)]
    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(winit::event_loop::ControlFlow::Poll);

            match &event {
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::CloseRequested,
                    ..
                } => elwt.exit(),
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::Resized(size),
                    ..
                } => {
                    if let Some(r) = app.renderer_mut() {
                        r.resize(size.width, size.height);
                    }
                }
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::KeyboardInput { event: ke, .. },
                    ..
                } => {
                    let input = app.input_mut();
                    use winit::keyboard::PhysicalKey;
                    if let PhysicalKey::Code(key) = ke.physical_key {
                        if ke.state == winit::event::ElementState::Pressed {
                            input.press(key);
                        } else {
                            input.release(key);
                        }
                    }
                }
                _ => {}
            }

            if let winit::event::Event::AboutToWait = event {
                app.run();
                app.render_phase();
            }
        })
        .unwrap();
}
