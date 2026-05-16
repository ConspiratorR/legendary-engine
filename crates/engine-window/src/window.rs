use winit::dpi::PhysicalSize;
use winit::window::Window;

pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
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

#[allow(deprecated)]
pub fn create_window(
    config: &WindowConfig,
    event_loop: &winit::event_loop::EventLoop<()>,
) -> Window {
    let attrs = Window::default_attributes()
        .with_title(&config.title)
        .with_inner_size(PhysicalSize::new(config.width, config.height));
    event_loop
        .create_window(attrs)
        .expect("Failed to create window")
}
