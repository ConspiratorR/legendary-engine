//! Window creation and configuration.
//!
//! Provides [`WindowConfig`] for describing window properties and
//! [`create_window`] for instantiating a [`winit::window::Window`].
//!
//! # Platform behavior
//!
//! - **Size** is in *physical* pixels. On HiDPI displays the actual
//!   framebuffer may be larger than the logical size; winit handles
//!   this transparently.
//! - **Title** must be valid UTF-8. Non-ASCII characters are supported
//!   on all platforms.
//! - **VSync** is a hint to the rendering layer; the window itself
//!   does not enforce it.

use crate::error::WindowError;
use winit::dpi::LogicalSize;
use winit::window::Window;

/// Configuration for creating a window.
///
/// Use the builder methods to customize window properties:
///
/// ```rust,no_run
/// use engine_window::WindowConfig;
///
/// let config = WindowConfig::new()
///     .with_title("My Game")
///     .with_size(1920, 1080)
///     .with_vsync(false);
/// ```
#[derive(Debug)]
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

impl WindowConfig {
    /// Creates a new [`WindowConfig`] with default values.
    ///
    /// Defaults: title "RustEngine", 1280x720, vsync enabled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the window title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Sets the window dimensions in physical pixels.
    ///
    /// Both width and height must be greater than zero.
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Enables or disables vertical sync.
    pub fn with_vsync(mut self, vsync: bool) -> Self {
        self.vsync = vsync;
        self
    }

    /// Validates the configuration, returning an error if values are invalid.
    ///
    /// Checks that width and height are both greater than zero.
    pub fn validate(&self) -> Result<(), WindowError> {
        if self.width == 0 || self.height == 0 {
            return Err(WindowError::InvalidSize {
                width: self.width,
                height: self.height,
            });
        }
        Ok(())
    }
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

/// Creates a [`Window`] from the given configuration and event loop.
///
/// # Errors
///
/// Returns [`WindowError::CreationFailed`] if the platform cannot create the window.
#[allow(deprecated)]
pub fn create_window(
    config: &WindowConfig,
    event_loop: &winit::event_loop::EventLoop<()>,
) -> Result<Window, WindowError> {
    config.validate()?;
        let attrs = Window::default_attributes()
            .with_title(&config.title)
            .with_inner_size(LogicalSize::new(config.width, config.height));
    event_loop
        .create_window(attrs)
        .map_err(|e| WindowError::CreationFailed {
            reason: e.to_string(),
        })
}
