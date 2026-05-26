use engine_math::{Mat4, Vec2, Vec3};
use engine_render::camera::{Camera, Color, Viewport};
use engine_render::renderer::Renderer;
use engine_render::sprite::SpriteDraw;
use engine_window::{window::WindowConfig, window::create_window};
use log::info;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("Sprite Demo — Multi-Camera");

    let event_loop = EventLoop::new().unwrap();
    let window = std::sync::Arc::new(create_window(
        &WindowConfig {
            title: "Sprite Demo — Multi-Camera".to_string(),
            width: 800,
            height: 600,
            vsync: true,
        },
        &event_loop,
    ));

    let mut renderer = Renderer::new(window);

    let texture_id = renderer
        .texture_store
        .load(
            &renderer.device,
            &renderer.queue,
            &renderer.sprite_pipeline.texture_bind_group_layout,
            "assets/test.png",
        )
        .unwrap_or_else(|e| {
            info!("Could not load texture: {} — using fallback", e);
            0
        });

    // Define scene sprites
    let all_sprites = vec![
        SpriteDraw {
            world_matrix: Mat4::from_translation(Vec3::new(200.0, 300.0, 0.0)),
            color: [1.0, 1.0, 1.0, 1.0],
            size: Vec2::new(128.0, 128.0),
            texture_id,
            flip_x: false,
            flip_y: false,
        },
        SpriteDraw {
            world_matrix: Mat4::from_translation(Vec3::new(600.0, 300.0, 0.0)),
            color: [0.0, 1.0, 0.0, 1.0],
            size: Vec2::new(128.0, 128.0),
            texture_id,
            flip_x: false,
            flip_y: false,
        },
    ];

    // Main camera — full screen
    let mut main_camera = Camera::orthographic(0.0, 800.0, 600.0, 0.0);
    main_camera.priority = 0;
    main_camera.clear_color = Some(Color::new(0.1, 0.1, 0.1, 1.0));

    // Top-right mini camera (picture-in-picture)
    let mut mini_camera = Camera::orthographic(0.0, 400.0, 300.0, 0.0);
    mini_camera.priority = 1;
    mini_camera.viewport = Viewport::Relative {
        x: 0.6,
        y: 0.0,
        width: 0.4,
        height: 0.4,
    };
    mini_camera.clear_color = Some(Color::new(0.2, 0.2, 0.3, 1.0));

    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            match &event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => elwt.exit(),
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    renderer.resize(size.width, size.height);
                }
                _ => {}
            }

            if let Event::AboutToWait = event {
                let cameras: Vec<&Camera> = vec![&main_camera, &mini_camera];
                let _ = renderer.render_frame(&cameras, &all_sprites);
            }
        })
        .unwrap();
}
