//! Android entry point and lifecycle management.
//!
//! On Android, there is no `main()` function. The entry point is `android_main()`
//! provided by the `android-activity` crate. This module handles the Android
//! application lifecycle and integrates with winit's event loop.

use winit::application::ApplicationHandler;
use winit::event::{ElementState, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::app::{App, AppBuilder};
use crate::engine::Engine;
use crate::error::EngineError;

/// State managed during the Android application lifecycle.
struct AndroidApp {
    window: Option<Window>,
    app: Option<App>,
}

impl ApplicationHandler for AndroidApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attrs = WindowAttributes::default()
                .with_title("RustEngine")
                .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
            match event_loop.create_window(window_attrs) {
                Ok(window) => {
                    self.window = Some(window);
                    log::info!("Android window created");
                }
                Err(e) => {
                    log::error!("Failed to create Android window: {e}");
                    event_loop.exit();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                log::info!("Close requested");
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(app) = &mut self.app {
                    if let Some(r) = app.renderer_mut() {
                        r.resize(size.width, size.height);
                    }
                }
            }
            WindowEvent::KeyboardInput { event: ke, .. } => {
                if let Some(app) = &mut self.app {
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
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(app) = &mut self.app {
                    app.input_mut().mouse_mut().position = (position.x, position.y);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(app) = &mut self.app {
                    let input = app.input_mut();
                    let pressed = state == ElementState::Pressed;
                    match button {
                        winit::event::MouseButton::Left => input.mouse_mut().left_button = pressed,
                        winit::event::MouseButton::Right => {
                            input.mouse_mut().right_button = pressed
                        }
                        winit::event::MouseButton::Middle => {
                            input.mouse_mut().middle_button = pressed
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::Touch(touch) => {
                // Touch input handling - forwarded to input system
                log::debug!("Touch event: id={}, phase={:?}", touch.id, touch.phase);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }

        if let Some(app) = &mut self.app {
            app.run();
            if !app.post_render_hooks.is_empty() {
                let mut hooks = std::mem::take(&mut app.post_render_hooks);
                for hook in &mut hooks {
                    hook(app);
                }
                app.post_render_hooks = hooks;
            }
            app.render_phase();
        }
    }
}

/// Run the engine on Android.
///
/// This function should be called from `android_main()`.
pub fn run_android(mut app_builder: AppBuilder) -> Result<(), EngineError> {
    crate::debug::init_logger();
    log::info!("RustEngine Android starting");

    let event_loop = EventLoop::new().map_err(|e| EngineError::InitFailed(e.to_string()))?;

    let mut state = AndroidApp {
        window: None,
        app: None,
    };

    // Build the app but don't set renderer yet (window not created)
    state.app = Some(app_builder.build());

    event_loop
        .run_app(&mut state)
        .map_err(|e| EngineError::InitFailed(e.to_string()))?;

    Ok(())
}
