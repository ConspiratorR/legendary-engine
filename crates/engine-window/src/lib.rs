//! Window creation and management via winit.

pub mod error;
pub mod window;
pub use error::WindowError;
pub use window::{WindowConfig, create_window};
