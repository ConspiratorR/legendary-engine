use crate::app::AppBuilder;
use engine_window::window::{WindowConfig, create_window};
use std::sync::Arc;

pub struct Engine;

impl Engine {
    pub fn new() -> AppBuilder {
        AppBuilder::new()
    }
}

#[allow(deprecated)]
pub fn run_default(app_builder: AppBuilder) {
    let mut app = app_builder.build();
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window = Arc::new(create_window(&WindowConfig::default(), &event_loop));
    let mut renderer = engine_render::renderer::Renderer::new(window.clone());

    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(winit::event_loop::ControlFlow::Poll);

            match event {
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::CloseRequested,
                    ..
                } => {
                    elwt.exit();
                }
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::Resized(size),
                    ..
                } => {
                    renderer.resize(size.width, size.height);
                }
                winit::event::Event::AboutToWait => {
                    app.run();
                    let _ = renderer.present();
                }
                _ => {}
            }
        })
        .unwrap();
}
