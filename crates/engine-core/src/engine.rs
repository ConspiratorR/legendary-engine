use crate::app::AppBuilder;
use engine_window::window::{WindowConfig, create_window};
use std::sync::Arc;

pub struct Engine;

#[allow(clippy::new_ret_no_self)]
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
    let renderer =
        engine_render::renderer::Renderer::new(window.clone()).expect("Failed to create renderer");
    app.set_renderer(renderer);

    use winit::event::{ElementState, Event, MouseButton, WindowEvent};
    use winit::event_loop::ControlFlow;

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
                    if let Some(r) = app.renderer_mut() {
                        r.resize(size.width, size.height);
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput { event: ke, .. },
                    ..
                } => {
                    let input = app.input_mut();
                    use winit::keyboard::PhysicalKey;
                    if let PhysicalKey::Code(key) = ke.physical_key {
                        if ke.state == ElementState::Pressed {
                            input.press(key);
                        } else {
                            input.release(key);
                        }
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::CursorMoved { position, .. },
                    ..
                } => {
                    app.input_mut().mouse_mut().position = (position.x, position.y);
                }
                Event::WindowEvent {
                    event: WindowEvent::MouseInput { state, button, .. },
                    ..
                } => {
                    let input = app.input_mut();
                    let pressed = *state == ElementState::Pressed;
                    match button {
                        MouseButton::Left => input.mouse_mut().left_button = pressed,
                        MouseButton::Right => input.mouse_mut().right_button = pressed,
                        MouseButton::Middle => input.mouse_mut().middle_button = pressed,
                        _ => {}
                    }
                }
                _ => {}
            }
            if let Event::AboutToWait = event {
                app.run();
                if !app.post_render_hooks.is_empty() {
                    let mut hooks = std::mem::take(&mut app.post_render_hooks);
                    for hook in &mut hooks {
                        hook(&mut app);
                    }
                    app.post_render_hooks = hooks;
                }
            }
        })
        .unwrap();
}
