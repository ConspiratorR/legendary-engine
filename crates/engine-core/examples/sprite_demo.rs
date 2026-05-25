use engine_math::Mat4;
use engine_render::renderer::Renderer;
use engine_render::sprite::SpriteBatch;
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

    event_loop.run(move |event, elwt| {
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

            let sprites: Vec<SpriteBatch> = Vec::new();
            let _ = renderer.present(&camera_matrix, &sprites);
        }
    }).unwrap();
}
