use engine_math::{Mat4, Vec2};
use engine_render::renderer::Renderer;
use engine_render::sprite::{SpriteBatch, SpriteDraw};
use engine_window::{window::WindowConfig, window::create_window};
use log::info;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("Starting Sprite Demo");

    let event_loop = EventLoop::new().unwrap();
    let window = std::sync::Arc::new(create_window(
        &WindowConfig {
            title: "Sprite Demo".to_string(),
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

    let mut batch = SpriteBatch::new(texture_id);
    let draw = SpriteDraw {
        world_matrix: Mat4::IDENTITY,
        color: [1.0, 1.0, 1.0, 1.0],
        size: Vec2::new(128.0, 128.0),
        texture_id,
        flip_x: false,
        flip_y: false,
    };
    batch.push(&draw);
    batch.upload(&renderer.device);

    let batches = vec![batch];

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
                let width = renderer.config.width as f32;
                let height = renderer.config.height as f32;
                let proj = Mat4::orthographic_rh(0.0, width, height, 0.0, -1.0, 1.0);
                let view = Mat4::IDENTITY;
                let camera_matrix = proj * view;

                let _ = renderer.present(&camera_matrix, &batches);
            }
        })
        .unwrap();
}
