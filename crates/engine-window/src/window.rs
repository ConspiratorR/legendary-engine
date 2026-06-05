use crate::error::WindowError;
use winit::dpi::PhysicalSize;
use winit::window::Window;

/// Configuration for creating a window.
pub struct WindowConfig {
    /// The window title.
    pub title: String,
    /// The window width in physical pixels.
    pub width: u32,
    /// The window height in physical pixels.
    pub height: u32,
    /// Whether to enable vertical sync.
    pub vsync: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "RustEngine".to_string(),
            width: 1280,
            height: 720,
            vsync: true,
        }
    }
}

/// Create a [`Window`] from the given configuration and event loop.
#[allow(deprecated)]
pub fn create_window(
    config: &WindowConfig,
    event_loop: &winit::event_loop::EventLoop<()>,
) -> Result<Window, WindowError> {
    let attrs = Window::default_attributes()
        .with_title(&config.title)
        .with_inner_size(PhysicalSize::new(config.width, config.height));
    event_loop
        .create_window(attrs)
        .map_err(|e| WindowError::CreationFailed {
            reason: e.to_string(),
        })
}
