//! # engine-window
//!
//! Window management for the RustEngine.
//!
//! Wraps winit to provide cross-platform window creation
//! and event handling. Supports Windows, macOS, and Linux
//! (Wayland/X11).
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use engine_window::WindowConfig;
//!
//! let config = WindowConfig::new()
//!     .with_title("My Game")
//!     .with_size(1280, 720);
//! ```

pub mod error;
pub mod window;
pub use error::WindowError;
pub use window::{WindowConfig, create_window};
