use crate::app::AppBuilder;
use crate::error::EngineError;
use engine_window::window::{WindowConfig, create_window};
use std::sync::Arc;

/// High-level engine entry point.
///
/// Provides a convenient way to create an [`AppBuilder`] with default settings.
/// For more control, use `AppBuilder::new()` directly.
pub struct Engine;

#[allow(clippy::new_ret_no_self)]
impl Engine {
    /// Create a new [`AppBuilder`].
    pub fn new() -> AppBuilder {
        AppBuilder::new()
    }
}

/// Run the application with a default window and renderer.
///
/// This creates a window, initializes the wgpu renderer via [`RenderPlugin2D`],
/// and enters the winit event loop. Input events are forwarded to the app's
/// input system. The app's `run()` method is called each frame, followed by
/// an automatic render phase that collects Camera and Sprite components from
/// the ECS world and renders them to the window.
#[allow(deprecated)]
pub fn run_default(mut app_builder: AppBuilder) -> Result<(), EngineError> {
    let event_loop =
        winit::event_loop::EventLoop::new().map_err(|e| EngineError::InitFailed(e.to_string()))?;
    let window = Arc::new(create_window(&WindowConfig::default(), &event_loop)?);

    // Use RenderPlugin2D to set up renderer and texture bridge
    let mut plugin = engine_render::plugin::RenderPlugin2D::new(window.clone());
    plugin.build(app_builder.world_mut());
    let renderer = plugin
        .take_renderer()
        .ok_or_else(|| EngineError::InitFailed("Renderer already taken".into()))?;

    let mut app = app_builder.build();
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
                // Run post-render hooks (for user extensions)
                if !app.post_render_hooks.is_empty() {
                    let mut hooks = std::mem::take(&mut app.post_render_hooks);
                    for hook in &mut hooks {
                        hook(&mut app);
                    }
                    app.post_render_hooks = hooks;
                }
                // Automatic render phase: collect cameras/sprites from ECS and render
                app.render_phase();
            }
        })
        .map_err(|e| EngineError::InitFailed(e.to_string()))?;
    Ok(())
}
